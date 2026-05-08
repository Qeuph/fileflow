use clap::{Parser, Subcommand};
use fileflow_vfs::{Snapshot, Sandbox};
use fileflow_journal::{Block, SqliteStore, JournalStore};
use fileflow_hyperquery::{Indexer, QueryEngine};
use fileflow_engine::{compile_ast_to_pipeline, Scheduler};
use fileflow_flowlang::parse;
use std::path::PathBuf;
use std::fs;
use anyhow::Context;

#[derive(Parser)]
#[command(name = "fileflow")]
#[command(about = "FileFlow CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short, long)]
        pipeline: String,
        #[arg(short, long)]
        target: PathBuf,
        #[arg(short, long, default_value = "test-session")]
        session: String,
    },
    Snapshot {
        #[command(subcommand)]
        subcommand: SnapshotCommands,
    },
}

#[derive(Subcommand)]
enum SnapshotCommands {
    Create {
        #[arg(short, long)]
        dir: PathBuf,
    },
    Checkout {
        session_id: String,
        #[arg(short, long)]
        step: usize,
        #[arg(short, long)]
        target: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { pipeline, target, session } => {
            println!("Running pipeline {} on {:?} (Session: {})", pipeline, target, session);
            
            // 0. Parse FlowLang Script
            let script_content = fs::read_to_string(&pipeline).context("Failed to read pipeline script")?;
            let ast = parse(&script_content).context("Failed to parse pipeline script")?;
            let pipeline_dag = compile_ast_to_pipeline(ast).context("Failed to compile AST to pipeline")?;
            
            // 1. Index with HyperQuery
            let indexer = Indexer::new("hyperquery.db")?;
            indexer.index_directory(&target)?;
            let engine = QueryEngine::new("hyperquery.db")?;
            let files = engine.query("size > 0")?;
            println!("HyperQuery found {} files to process", files.len());

            // 2. Create Snapshot
            let snapshot = Snapshot::create(&target)?;
            let pre_hash = snapshot.compute_merkle_root();
            println!("Pre-state hash: {:?}", hex::encode(pre_hash));

            // 3. Start Journal
            let mut store = SqliteStore::new("fileflow.db")?;
            let prev_hash = store.get_latest_hash()?;

            // 4. Create Sandbox
            let mut sandbox = Sandbox::new(snapshot.clone(), PathBuf::from("/tmp/fileflow-work"));
            
            // 5. Execute Pipeline via Scheduler
            let scheduler = Scheduler::new(pipeline_dag, session.clone());
            scheduler.execute(&mut sandbox, &mut store).await?;

            // 6. Simulate an additional operation (Optional, for demo)
            println!("Committing final results to journal...");
            sandbox.write_file(PathBuf::from("final_result.txt"), b"Pipeline Finished Successfully".to_vec());

            // 7. Compute Post-state Hash
            let post_hash = sandbox.compute_post_merkle_root();
            println!("Post-state hash: {:?}", hex::encode(post_hash));

            // 8. Commit final block to Journal
            let block = Block::new(
                session,
                store.get_latest_hash()?,
                "SESSION_COMPLETE".to_string(),
                pre_hash,
                post_hash,
                serde_json::json!({
                    "script": pipeline,
                    "base_snapshot": snapshot,
                    "changes": sandbox.changes,
                }),
            );
            let block_hash = store.append_block(block)?;
            println!("Committed final block: {:?}", hex::encode(block_hash));

            // 9. Materialize
            let output_dir = target.join("output");
            sandbox.materialize(&output_dir)?;
            println!("Materialized output to {:?}", output_dir);
        }
        Commands::Snapshot { subcommand } => {
            match subcommand {
                SnapshotCommands::Create { dir } => {
                    let snapshot = Snapshot::create(&dir)?;
                    println!("Snapshot created for {:?} with {} files", dir, snapshot.files.len());
                    println!("Merkle Root: {:?}", hex::encode(snapshot.compute_merkle_root()));
                }
                SnapshotCommands::Checkout { session_id, step, target } => {
                    println!("Checking out session {} at step {} into {:?}", session_id, step, target);
                    
                    let store = SqliteStore::new("fileflow.db")?;
                    
                    // Step 0: Pre-state of the first block
                    // Step n: Post-state of the (n-1)-th block
                    
                    let block_index = if step == 0 { 0 } else { step - 1 };
                    let block = store.get_block_by_step(&session_id, block_index)?
                        .context(format!("No block found for session {} at step/index {}", session_id, block_index))?;
                    
                    let base_snapshot: Snapshot = serde_json::from_value(block.transform_metadata["base_snapshot"].clone())?;
                    let mut sandbox = Sandbox::new(base_snapshot, PathBuf::from("/tmp/fileflow-checkout"));
                    
                    let expected_hash;
                    if step == 0 {
                        println!("Restoring Step 0 (Original State)");
                        expected_hash = block.pre_state_hash;
                        // sandbox.changes remains empty
                    } else {
                        println!("Restoring Step {} (After block {})", step, block_index);
                        expected_hash = block.post_state_hash;
                        let changes: std::collections::HashMap<PathBuf, Vec<u8>> = serde_json::from_value(block.transform_metadata["changes"].clone())?;
                        sandbox.changes = changes;
                    }
                    
                    // Verify hash
                    let actual_hash = if step == 0 { sandbox.base_snapshot.compute_merkle_root() } else { sandbox.compute_post_merkle_root() };
                    
                    if actual_hash != expected_hash {
                        return Err(anyhow::anyhow!(
                            "Hash mismatch! Expected: {:?}, Actual: {:?}",
                            hex::encode(expected_hash),
                            hex::encode(actual_hash)
                        ));
                    }
                    
                    println!("Verification successful! Hash matches: {:?}", hex::encode(actual_hash));
                    
                    sandbox.materialize(&target)?;
                    println!("Checkout complete! Files restored to {:?}", target);
                }
            }
        }
    }

    Ok(())
}
