#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    reochat::run().await?;
    Ok(())
}
