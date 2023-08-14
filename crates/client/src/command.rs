use dotenv::dotenv;
use std::env;
use anyhow::Result;
pub async fn run() -> Result<()> {
    dotenv().ok();
    println!("client!");
    Ok(())
}