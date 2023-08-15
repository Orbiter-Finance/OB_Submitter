use anyhow::Result;
use submitter_client;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    submitter_client::run().await?;
    std::future::pending::<()>().await;
    Ok(())
}
