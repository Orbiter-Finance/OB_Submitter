use tokio;
use submitter_client;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()>{
    submitter_client::run().await?;
    std::future::pending::<()>().await?;
    Ok(())
}
