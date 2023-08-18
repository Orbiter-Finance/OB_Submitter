use anyhow::Result;
use submitter_lib as submitter;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    submitter::run().await?;
    std::future::pending::<()>().await;
    Ok(())
}