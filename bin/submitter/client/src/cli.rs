use super::{
    rpc::{DebugApiServerImpl, SubmitterApiServerImpl},
    Args,
};
use anyhow::Result;
use clap::Parser;
use clokwerk::{Scheduler, TimeUnits};
use contract::{run as contract_run, SubmitterContract};
use dialoguer::Password;
use dotenv::dotenv;
use ethers::{prelude::*, signers::LocalWallet};
use jsonrpsee::{
    server::{Server, ServerBuilder, ServerHandle},
    Methods,
};
use lazy_static::lazy_static;
use primitives::{
    func::chain_token_address_convert_to_h256,
    traits::{DebugApiServer, StataTrait, SubmitterApiServer},
    types::{BlockInfo, BlocksStateData, ProfitStateData},
};
use sled;
use state::{
    data_example::Data as DataExample, Keccak256Hasher, Open, OptimisticTransactionDB, State, H256,
};
use std::{
    env,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::OnceCell;
use tracing::{event, Level};
use tracing_appender::rolling::daily;
use tracing_subscriber::{fmt::format, FmtSubscriber};
use txs::{funcs::SupportChains, Submitter};

pub struct JsonRpcServer {
    pub mothods: Methods,
}

impl JsonRpcServer {
    pub fn new() -> Self {
        JsonRpcServer {
            mothods: Methods::new(),
        }
    }

    pub fn add_mothod(&mut self, mothoes: impl Into<Methods>) -> anyhow::Result<()> {
        self.mothods.merge(mothoes)?;
        Ok(())
    }
}

pub struct Client<
    Profit: StataTrait<H256, ProfitStateData>,
    Blocks: StataTrait<H256, BlocksStateData>,
    Wallet,
> {
    pub wallet: Arc<Wallet>,
    pub rpc_server_port: u16,
    pub profit_state: Arc<RwLock<Profit>>,
    pub blocks_state: Arc<RwLock<Blocks>>,
}

impl<'a>
    Client<
        State<'a, Keccak256Hasher, ProfitStateData>,
        State<'a, Keccak256Hasher, BlocksStateData>,
        LocalWallet,
    >
{
    pub fn new(
        wallet: Arc<LocalWallet>,
        rpc_server_port: u16,
        profit_state: Arc<RwLock<State<'a, Keccak256Hasher, ProfitStateData>>>,
        blocks_state: Arc<RwLock<State<'a, Keccak256Hasher, BlocksStateData>>>,
    ) -> Self {
        Client {
            wallet,
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

    let args = Args::parse();
    let rpc_server_port = args.rpc_port;
    let mut rpc_server = JsonRpcServer::new();

    let file_appender = daily(format!("{}/logs", args.db_path), "submitter.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_max_level(Level::INFO)
        .init();
    event!(
        Level::INFO,
        "Submitter log init success, and log path: {}/logs/",
        args.db_path
    );

    let profit_state_db_path = format!("{}/profit", args.db_path);
    let blocks_state_db_path = format!("{}/blocks", args.db_path);

    PROFIT_STATE_DB_PATH.set(profit_state_db_path).unwrap();
    BLOCKS_STATE_DB_PATH.set(blocks_state_db_path).unwrap();
    assert_ne!(
        PROFIT_STATE_DB_PATH
            .get()
            .expect("profit state db' path not set"),
        BLOCKS_STATE_DB_PATH
            .get()
            .expect("blocks state db' path not set"),
    );
    let private_key = Password::new()
        .with_prompt("Please enter submitter's private key")
        .interact()?;
    let wallet = Arc::new(LocalWallet::from_str(
        &private_key.trim_end_matches("\n").to_string(),
    )?);
    event!(Level::INFO, "The wallet is created.");

    let profit_state = Arc::new(RwLock::new(
        State::<'_, Keccak256Hasher, ProfitStateData>::new(
            PROFIT_STATE_DB_PATH
                .get()
                .expect("profit state db' path not set")
                .as_ref(),
            OptimisticTransactionDB::open_default(
                PROFIT_STATE_DB_PATH
                    .get()
                    .expect("profit state db' path not set"),
            )?,
        ),
    ));
    event!(
        Level::INFO,
        "Profit state's db is created! path is: {:?}",
        PROFIT_STATE_DB_PATH.get().unwrap()
    );
    let blocks_state = Arc::new(RwLock::new(
        State::<'_, Keccak256Hasher, BlocksStateData>::new(
            BLOCKS_STATE_DB_PATH
                .get()
                .expect("blocks state db' path not set")
                .as_ref(),
            OptimisticTransactionDB::open_default(
                BLOCKS_STATE_DB_PATH.get().expect("state db' path not set"),
            )?,
        ),
    ));
    event!(
        Level::INFO,
        "Blocks state's db is created! path: {:?}",
        BLOCKS_STATE_DB_PATH.get().unwrap()
    );

    let client = Client::new(
        wallet.clone(),
        rpc_server_port,
        profit_state.clone(),
        blocks_state.clone(),
    );
    event!(Level::INFO, "The client is created.");

    let server = ServerBuilder::new()
        .build(format!("127.0.0.1:{}", client.rpc_server_port))
        .await?;
    let addr = server.local_addr()?;
    let sled_db = Arc::new(sled::open(args.db_path.clone()).unwrap());
    let user_tokens_db = Arc::new(txs::sled_db::UserTokensDB::new(sled_db.clone()).unwrap());
    let profit_statistics_db =
        Arc::new(txs::sled_db::ProfitStatisticsDB::new(sled_db.clone()).unwrap());
    rpc_server.add_mothod(
        SubmitterApiServerImpl {
            state: profit_state.clone(),
            user_tokens_db: user_tokens_db.clone(),
            profit_statistics_db: profit_statistics_db.clone(),
        }
        .into_rpc(),
    )?;
    if args.debug {
        rpc_server.add_mothod(
            DebugApiServerImpl {
                state: profit_state.clone(),
                user_tokens_db: user_tokens_db.clone(),
            }
            .into_rpc(),
        )?;
        tokio::spawn(insert_profit_by_count(100_0000, profit_state.clone()));
    }

    let server_handle = server.start(rpc_server.mothods.clone())?;

    event!(Level::INFO, "Rpc server start at: {:?}", addr);
    tokio::spawn(server_handle.stopped());
    let start_block_num1 = Arc::new(tokio::sync::RwLock::new(args.start_block));
    let (s, r) = tokio::sync::broadcast::channel::<BlockInfo>(100);
    let support_chains_crawler = SupportChains::new(
        std::env::var("SUPPORT_CHAINS_SOURCE_URL").expect("SUPPORT_CHAINS_SOURCE_URL is not set"),
    );
    let tokens: Arc<Vec<Address>> =
        Arc::new(support_chains_crawler.get_mainnet_support_tokens().await?);
    let contract = Arc::new(
        SubmitterContract::new(
            s.clone(),
            wallet.as_ref().clone(),
            start_block_num1.clone(),
            tokens,
        )
        .await,
    );
    let c_1 = contract.clone();
    tokio::spawn(async {
        contract_run(c_1).await.unwrap();
        event!(Level::INFO, "contract start");
    });

    let start_block_num = Arc::new(RwLock::new(args.start_block));
    let submitter = Submitter::new(
        profit_state.clone(),
        blocks_state.clone(),
        contract.clone(),
        start_block_num.clone(),
        sled_db.clone(),
        args.db_path,
    );
    tokio::spawn(async move {
        submitter.run().await.unwrap();
    });
    s.send(BlockInfo {
        storage: Default::default(),
        events: vec![],
    })
    .unwrap();
    std::future::pending::<()>().await;
    Ok(())
}

async fn insert_profit_by_count(
    count: u64,
    state: Arc<RwLock<State<'static, Keccak256Hasher, ProfitStateData>>>,
) {
    let mut profit_state = state.write().unwrap();
    for i in 0..count {
        let address: Address = u64_to_ethereum_address(i);
        let token: Address = u64_to_ethereum_address(i + 1);
        let path: H256 = chain_token_address_convert_to_h256(i, token, address);
        let profit = ProfitStateData {
            token,
            token_chain_id: i,
            balance: U256::from_dec_str("1000").unwrap(),
            debt: U256::from(0),
        };
        profit_state.try_update_all(vec![(path, profit)]).unwrap();
        println!("update profit: {:?}", i);
    }
}

fn u64_to_ethereum_address(input: u64) -> Address {
    let mut hex_string = format!("{:x}", input);
    while hex_string.len() < 40 {
        hex_string.insert(0, '0');
    }
    let address_str = format!("0x{}", hex_string);
    Address::from_str(&address_str).expect("Failed to parse Ethereum address")
}
