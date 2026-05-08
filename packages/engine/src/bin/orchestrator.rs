use fileflow_engine::start_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    start_server().await?;
    Ok(())
}
