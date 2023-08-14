use dotenv::dotenv;
use std::env;
use anyhow::Result;
use state::{StataTrait, State, H256, Data, Blake2bHasher, OptimisticTransactionDB, Open};
use std::sync::{Arc, Mutex};
use ethers::prelude::*;
use ethers::signers::LocalWallet;
use super::Args;
use clap::Parser;
use std::str::FromStr;
use clokwerk::{Scheduler, TimeUnits};
use dialoguer::Password;

pub struct Client<State: StataTrait<H256, Data>, Provider, Wallet> {
    pub wallet: Arc<Wallet>,
    // fixme should not be designed like this.
    pub provider: Arc<Provider>,
    pub rpc_server_port: u16,
    pub state: Arc<Mutex<State>>
}


impl<'a> Client<State<'a, Blake2bHasher>,Provider<Http>, LocalWallet> {
    pub fn new(
        wallet: Arc<LocalWallet>,
        provider: Arc<Provider<Http>>,
        rpc_server_port: u16,
        state: Arc<Mutex<State<'a, Blake2bHasher>>>,
    ) -> Self {
        Client {
            wallet,
            provider,
            rpc_server_port,
            state,
        }
    }
}


pub async fn run() -> Result<()> {
    dotenv().ok();
    let args = Args::parse();
    let state_path = args.state_path;
    let rpc_server_port = args.rpc_port;
    println!("state_path: {}", state_path);
    println!("rpc_server_port: {}", rpc_server_port);

    // contains 0x prefix
    // The private key is 32 bytes long
    // for example: 0x0123456789012345678901234567890123456789012345678901234567890123
    let private_key = Password::new()
        .with_prompt("Please enter submitter's private key").interact()?;

    let wallet = Arc::new(LocalWallet::from_str(&private_key.trim_end_matches("\n").to_string())?);
    let provider = Arc::new(Provider::<Http>::try_from(env::var("NETWORK_RPC_URL").expect("NETWORK_RPC_URL is not exists."))?);
    let state = Arc::new(Mutex::new(State::<'_, Blake2bHasher>::new(state_path.as_bytes(), OptimisticTransactionDB::open_default(state_path.clone())?)));
    let _client = Client::new(wallet, provider, rpc_server_port, state);
    tokio::spawn(async move {
        // todo: start rpc server
    });
    let mut scheduler = Scheduler::new();
    scheduler.every(10.seconds()).run(|| {println!("hello world!");});
    tokio::spawn(async move {
        loop {
            scheduler.run_pending();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        // todo Regularly update data for state.
    });

    println!("client!");
    std::future::pending::<()>().await;
    Ok(())
}
