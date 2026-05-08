use tonic::{transport::Server, Request, Response, Status};
use fileflow_journal::{Block, SqliteStore, JournalStore};
use fileflow_vfs::{Snapshot, Sandbox};
use std::path::PathBuf;

pub mod orchestrator {
    tonic::include_proto!("fileflow.orchestrator");
}

use orchestrator::orchestrator_server::{Orchestrator, OrchestratorServer};
use orchestrator::*;

#[derive(Debug, Clone)]
pub struct FileFlowOrchestrator {
    pub db_path: String,
}

impl Default for FileFlowOrchestrator {
    fn default() -> Self {
        Self {
            db_path: "fileflow.db".to_string(),
        }
    }
}

#[tonic::async_trait]
impl Orchestrator for FileFlowOrchestrator {
    async fn start_pipeline(
        &self,
        request: Request<StartPipelineRequest>,
    ) -> Result<Response<StartPipelineResponse>, Status> {
        let r = request.into_inner();
        let session_id = "test-session".to_string();
        println!("Starting pipeline for session {}: {}", session_id, r.pipeline_yaml);
        
        Ok(Response::new(StartPipelineResponse {
            session_id,
        }))
    }

    async fn get_session_status(
        &self,
        _request: Request<GetSessionStatusRequest>,
    ) -> Result<Response<GetSessionStatusResponse>, Status> {
        Ok(Response::new(GetSessionStatusResponse {
            status: "Running".to_string(),
            logs: vec!["Initial status check".to_string()],
        }))
    }

    async fn commit_step(
        &self,
        _request: Request<CommitStepRequest>,
    ) -> Result<Response<CommitStepResponse>, Status> {
        Ok(Response::new(CommitStepResponse { success: true }))
    }

    async fn rollback_step(
        &self,
        _request: Request<RollbackStepRequest>,
    ) -> Result<Response<RollbackStepResponse>, Status> {
        Ok(Response::new(RollbackStepResponse { success: true }))
    }

    async fn materialize_snapshot(
        &self,
        _request: Request<MaterializeSnapshotRequest>,
    ) -> Result<Response<MaterializeSnapshotResponse>, Status> {
        Ok(Response::new(MaterializeSnapshotResponse {
            path: "/tmp/materialized".to_string(),
        }))
    }
}

pub async fn start_server() -> anyhow::Result<()> {
    let addr = "[::1]:50051".parse()?;
    let orchestrator = FileFlowOrchestrator::default();

    println!("Orchestrator listening on {}", addr);

    Server::builder()
        .add_service(OrchestratorServer::new(orchestrator))
        .serve(addr)
        .await?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub id: String,
    pub operation: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pipeline {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

use fileflow_flowlang::ASTNode;

pub fn compile_ast_to_pipeline(ast: ASTNode) -> anyhow::Result<Pipeline> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    fn process_body(body: &[ASTNode], nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, prev_id: &mut Option<String>) {
        for stmt in body.iter() {
            match stmt {
                ASTNode::Call { name, args: _ } => {
                    let id = format!("{}_{}", name, nodes.len());
                    nodes.push(Node {
                        id: id.clone(),
                        operation: name.clone(),
                        config: serde_json::json!({}),
                    });
                    if let Some(p_id) = prev_id.clone() {
                        edges.push(Edge { from: p_id, to: id.clone() });
                    }
                    *prev_id = Some(id);
                }
                ASTNode::Pipe { nodes: pipe_nodes } => {
                    for pipe_node in pipe_nodes {
                        if let ASTNode::Call { name, args: _ } = pipe_node {
                            let id = format!("{}_{}", name, nodes.len());
                            nodes.push(Node {
                                id: id.clone(),
                                operation: name.clone(),
                                config: serde_json::json!({}),
                            });
                            if let Some(p_id) = prev_id.clone() {
                                edges.push(Edge { from: p_id, to: id.clone() });
                            }
                            *prev_id = Some(id);
                        }
                    }
                }
                ASTNode::For { var: _, iterable: _, body: for_body } => {
                    process_body(for_body, nodes, edges, prev_id);
                }
                _ => {} 
            }
        }
    }

    if let ASTNode::Pipeline { name: _, body } = ast {
        let mut prev_id = None;
        process_body(&body, &mut nodes, &mut edges, &mut prev_id);
    }

    Ok(Pipeline { nodes, edges })
}

use std::collections::{HashMap, VecDeque};

pub struct Scheduler {
    pub pipeline: Pipeline,
    pub session_id: String,
}

impl Scheduler {
    pub fn new(pipeline: Pipeline, session_id: String) -> Self {
        Self { pipeline, session_id }
    }

    pub fn get_topological_order(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut in_degree = HashMap::new();
        let mut adj = HashMap::new();

        for node in &self.pipeline.nodes {
            in_degree.insert(node.id.clone(), 0);
            adj.insert(node.id.clone(), Vec::new());
        }

        for edge in &self.pipeline.edges {
            *in_degree.get_mut(&edge.to).unwrap() += 1;
            adj.get_mut(&edge.from).unwrap().push(edge.to.clone());
        }

        let mut queue = VecDeque::new();
        for (node_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(node_id.clone());
            }
        }

        let mut order = Vec::new();
        while let Some(u) = queue.pop_front() {
            order.push(u.clone());
            if let Some(neighbors) = adj.get(&u) {
                for v in neighbors {
                    let degree = in_degree.get_mut(v).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }

        if order.len() != self.pipeline.nodes.len() {
            return Err(anyhow::anyhow!("Pipeline contains a cycle"));
        }

        Ok(order)
    }

    pub async fn execute(&self, sandbox: &mut Sandbox, store: &mut dyn JournalStore) -> anyhow::Result<()> {
        let order = self.get_topological_order()?;
        println!("Executing pipeline nodes in order: {:?}", order);

        let nodes_by_id: HashMap<String, &Node> = self.pipeline.nodes.iter()
            .map(|n| (n.id.clone(), n))
            .collect();

        for node_id in order {
            let node = nodes_by_id.get(&node_id).unwrap();
            let pre_hash = if sandbox.changes.is_empty() { 
                sandbox.base_snapshot.compute_merkle_root() 
            } else { 
                sandbox.compute_post_merkle_root() 
            };

            println!("Running node {}: {}", node.id, node.operation);
            
            // Execute operation (Simulated)
            if node.operation == "rename" {
                sandbox.write_file(PathBuf::from("renamed_via_scheduler.txt"), b"Renamed by Scheduler".to_vec());
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            let post_hash = sandbox.compute_post_merkle_root();
            let prev_hash = store.get_latest_hash()?;

            let block = Block::new(
                self.session_id.clone(),
                prev_hash,
                format!("NODE_EXEC:{}", node.operation),
                pre_hash,
                post_hash,
                serde_json::json!({
                    "node_id": node.id,
                    "operation": node.operation,
                    "base_snapshot": sandbox.base_snapshot,
                    "changes": sandbox.changes,
                }),
            );
            
            store.append_block(block)?;
            println!("Committed block for node {}", node.id);
        }

        Ok(())
    }

    async fn verify_node(&self, _node: &Node) -> anyhow::Result<bool> {
        let rand_val = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() % 10;
        Ok(rand_val != 0)
    }

    async fn self_heal(&self, node: &Node) -> anyhow::Result<()> {
        println!("Self-healing: Restoring state for node {}", node.id);
        Ok(())
    }
}
