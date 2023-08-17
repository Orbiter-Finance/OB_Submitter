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
use tokio::sync::OnceCell;
use tracing::{event, Level};
use tracing_appender::rolling::daily;
use tracing_subscriber::FmtSubscriber;

pub struct Client<State: StataTrait<H256, Data>, Wallet> {
    pub wallet: Arc<Wallet>,
    pub rpc_server_port: u16,
    pub profit_state: Arc<RwLock<State>>,
    pub blocks_state: Arc<RwLock<State>>,
}

impl<'a> Client<State<'a, Blake2bHasher>, LocalWallet> {
    pub fn new(
        wallet: Arc<LocalWallet>,
        // provider: Arc<Provider<Http>>,
        rpc_server_port: u16,
        profit_state: Arc<RwLock<State<'a, Blake2bHasher>>>,
        blocks_state: Arc<RwLock<State<'a, Blake2bHasher>>>,
    ) -> Self {
        Client {
            wallet,
            // provider,
            rpc_server_port,
            profit_state,
            blocks_state,
        }
    }
}

lazy_static! {
    static ref PROFIT_STATE_DB_PATH: OnceCell<String> = OnceCell::new();
    static ref BLOCKS_STATE_DB_PATH: OnceCell<String> = OnceCell::new();
}

pub async fn run() -> Result<()> {
    dotenv().ok();

    let file_appender = daily("logs", "submitter.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_max_level(Level::TRACE)
        .init();

    let args = Args::parse();
    let rpc_server_port = args.rpc_port;
    PROFIT_STATE_DB_PATH
        .set(args.profit_db_path.clone())
        .unwrap();
    BLOCKS_STATE_DB_PATH
        .set(args.blocks_db_path.clone())
        .unwrap();
    assert!(
        PROFIT_STATE_DB_PATH
            .get()
            .expect("profit state db' path not set")
            != BLOCKS_STATE_DB_PATH
                .get()
                .expect("blocks state db' path not set"),
        "profit db's path and blocks db's path can't be the same"
    );
    // for example: 0x0123456789012345678901234567890123456789012345678901234567890123
    let private_key = Password::new()
        .with_prompt("Please enter submitter's private key")
        .interact()?;
    let wallet = Arc::new(LocalWallet::from_str(
        &private_key.trim_end_matches("\n").to_string(),
    )?);
    event!(Level::INFO, "The wallet is created.");

    let profit_state = Arc::new(RwLock::new(State::<'_, Blake2bHasher>::new(
        PROFIT_STATE_DB_PATH
            .get()
            .expect("profit state db' path not set")
            .as_ref(),
        OptimisticTransactionDB::open_default(
            PROFIT_STATE_DB_PATH
                .get()
                .expect("profit state db' path not set"),
        )?,
    )));
    event!(
        Level::INFO,
        "profit state's db is created! path is: {:?}",
        PROFIT_STATE_DB_PATH.get().unwrap()
    );
    let blocks_state = Arc::new(RwLock::new(State::<'_, Blake2bHasher>::new(
        BLOCKS_STATE_DB_PATH
            .get()
            .expect("blocks state db' path not set")
            .as_ref(),
        OptimisticTransactionDB::open_default(
            BLOCKS_STATE_DB_PATH.get().expect("state db' path not set"),
        )?,
    )));
    event!(
        Level::INFO,
        "blocks state's db is created! path is: {:?}",
        BLOCKS_STATE_DB_PATH.get().unwrap()
    );

    let client = Client::new(
        wallet,
        rpc_server_port,
        profit_state.clone(),
        blocks_state.clone(),
    );
    event!(Level::INFO, "The client is created.");
    let server = ServerBuilder::new()
        .build(format!("127.0.0.1:{}", client.rpc_server_port))
        .await?;
    let addr = server.local_addr()?;
    let server_handle = server.start(
        SubmitterApiServerImpl {
            state: profit_state,
        }
        .into_rpc(),
    )?;
    event!(Level::INFO, "Rpc server start at: {:?}", addr);
    tokio::spawn(server_handle.stopped());

    let mut scheduler = Scheduler::new();
    scheduler.every(10.seconds()).run(|| {
        // todo
        event!(Level::INFO, "hello world!");
    });
    tokio::spawn(async move {
        event!(Level::INFO, "Start the scheduled task.");
        loop {
            scheduler.run_pending();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        // todo Regularly update data for state.
    });

    std::future::pending::<()>().await;
    Ok(())
}
