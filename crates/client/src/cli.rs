use super::rpc::SubmitterApiServerImpl;
use super::Args;
use crate::api::SubmitterApiServer;
use anyhow::Result;
use clap::Parser;
use clokwerk::{Scheduler, TimeUnits};
use dialoguer::Password;
use dotenv::dotenv;
use ethers::prelude::*;
use ethers::signers::LocalWallet;
use jsonrpsee::{
    server::{Server, ServerBuilder, ServerHandle},
    Methods,
};
use lazy_static::lazy_static;
use state::{Blake2bHasher, Data, Open, OptimisticTransactionDB, StataTrait, State, H256};
use std::env;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

pub struct Client<State: StataTrait<H256, Data>, Provider, Wallet> {
    pub wallet: Arc<Wallet>,
    // fixme should not be designed like this.
    pub provider: Arc<Provider>,
    pub rpc_server_port: u16,
    pub state: Arc<RwLock<State>>,
}

impl<'a> Client<State<'a, Blake2bHasher>, Provider<Http>, LocalWallet> {
    pub fn new(
        wallet: Arc<LocalWallet>,
        provider: Arc<Provider<Http>>,
        rpc_server_port: u16,
        state: Arc<RwLock<State<'a, Blake2bHasher>>>,
    ) -> Self {
        Client {
            wallet,
            provider,
            rpc_server_port,
            state,
        }
    }
}

lazy_static! {
    static ref STATE_DB_PATH: String =
        env::var("STATE_DB_PATH").expect("STATE_DB_PATH is not exists.");
}

pub async fn run() -> Result<()> {
    dotenv().ok();
    let args = Args::parse();

    let rpc_server_port = args.rpc_port;
    println!("rpc_server_port: {}", rpc_server_port);

    // contains 0x prefix
    // The private key is 32 bytes long
    // for example: 0x0123456789012345678901234567890123456789012345678901234567890123
    let private_key = Password::new()
        .with_prompt("Please enter submitter's private key")
        .interact()?;

    let wallet = Arc::new(LocalWallet::from_str(
        &private_key.trim_end_matches("\n").to_string(),
    )?);
    let provider = Arc::new(Provider::<Http>::try_from(
        env::var("NETWORK_RPC_URL").expect("NETWORK_RPC_URL is not exists."),
    )?);
    let state = Arc::new(RwLock::new(State::<'_, Blake2bHasher>::new(
        STATE_DB_PATH.as_ref(),
        OptimisticTransactionDB::open_default(STATE_DB_PATH.clone())?,
    )));

    let client = Client::new(wallet, provider, rpc_server_port, state.clone());
    let server = ServerBuilder::new()
        .build(format!("127.0.0.1:{}", client.rpc_server_port.clone()))
        .await?;
    let server_handle = server.start(SubmitterApiServerImpl { state: state }.into_rpc())?;

    tokio::spawn(server_handle.stopped());
    let mut scheduler = Scheduler::new();
    scheduler.every(10.seconds()).run(|| {
        println!("hello world!");
    });

    let service = tokio::spawn(async move {
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
