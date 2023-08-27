

mod funcs;
use std::str::FromStr;
use ethers::types::{Address, U256};
use ethers::utils::hex::encode;
use funcs::{get_request_builder, get_txs, is_chill, open_rocksdb, calculate_profit};
use lazy_static::lazy_static;
use primitives::types::{CrossTxDataAsKey, CrossTxProfit};
use primitives::types::{BlockInfo, WithdrawEvent};
use primitives::{
    constants::ETH_DELAY_BLOCKS,
    func::{block_info_compare, tx_compare},
    traits::{Contract as ContractTrait, StataTrait},
};
use primitives::{
    traits::Contract,
    types::{BlocksStateData, ProfitStateData},
};
use reqwest::header::{HeaderValue, ACCEPT, CONNECTION, CONTENT_TYPE, USER_AGENT};
use reqwest::{header::HeaderMap, Client, RequestBuilder};
use rocksdb::{
    ops::{Flush, Get, Iterate, Open, WriteOps},
    IteratorMode, ReadOptions,
};
use rocksdb::{Options, WriteBatch, DB};
use serde_json::json;
use serde_json::Value;
use sled::{self, Db, Tree};
use sparse_merkle_tree::H256;
use state::{Keccak256Hasher, State};
use std::ops::Add;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug)]
pub struct TxsDataHandler<ProfitState, BlocksState, Contract> {
    pub profit_state: Arc<RwLock<ProfitState>>,
    pub blocks_state: Arc<RwLock<BlocksState>>,
    pub db_path: String,
    pub start_block: Arc<RwLock<u64>>,
    pub newest_block_sender: Sender<(u64, u64)>,
    pub contract: Contract,
}

#[derive(Debug)]
pub struct ContractTest {
    chanel: Sender<(u64, u64)>,
}

#[async_trait::async_trait]
impl Contract for ContractTest {
    async fn submit_root(
        &self,
        start: u64,
        end: u64,
        blocks_root: [u8; 32],
        root: [u8; 32],
    ) -> Result<(), String> {
        let channel = tokio::sync::broadcast::channel::<(u64, u64)>(1);
        todo!()
    }

    async fn get_block_info(&self, block_number: u64) -> Result<BlockInfo, String> {
        todo!()
    }

    async fn get_maker_commission_by_block(
        &self,
        maker: Address,
        block_number: u64,
    ) -> Result<u32, String> {
        todo!()
    }
}

impl<
        ProfitState: StataTrait<H256, ProfitStateData>,
        BlocksState: StataTrait<H256, BlocksStateData>,
        Contract: ContractTrait,
    > TxsDataHandler<ProfitState, BlocksState, Contract>
{
    pub fn new(
        profit_state: Arc<RwLock<ProfitState>>,
        blocks_state: Arc<RwLock<BlocksState>>,
        db_path: String,
        start_block: Arc<RwLock<u64>>,
        newest_block_sender: Sender<(u64, u64)>,
        contract: Contract,
    ) -> Self {
        TxsDataHandler {
            profit_state,
            blocks_state,
            db_path,
            start_block,
            newest_block_sender,
            contract,
        }
    }

    pub fn put_block_info_to_db(&self, block_number: u64, info: BlockInfo) {
        let db = open_rocksdb(self.db_path.clone() + "/block_info", block_info_compare).unwrap();
        let mut tx = WriteBatch::default();
        let key = block_number.to_le_bytes();
        tx.put(key, bincode::serialize(&info).unwrap());
        db.write(&tx).unwrap();
        db.flush().unwrap();
    }

    pub fn get_block_info_from_db(&self, block_number: u64) -> Option<BlockInfo> {
        let db = open_rocksdb(self.db_path.clone() + "/block_info", block_info_compare).unwrap();
        db.get(block_number.to_le_bytes())
            .unwrap()
            .map(|v| bincode::deserialize(&v).unwrap())
    }

    pub fn put_txs_to_db(&self, txs: Vec<(CrossTxDataAsKey, CrossTxProfit)>) {
        let db = open_rocksdb(self.db_path.clone() + "/txs", tx_compare).unwrap();
        let mut batch = WriteBatch::default();

        for tx in txs {
            let key = bincode::serialize(&tx.0).unwrap();
            let value = bincode::serialize(&tx.1).unwrap();
            batch.put(key, value);
        }
        db.write(&batch).unwrap();
        db.flush().unwrap();
    }

    pub fn get_txs_from_db(&self, start_timestamp: u64, end_timestamp: u64) -> Vec<(CrossTxDataAsKey, CrossTxProfit)> {
        let db = open_rocksdb(self.db_path.clone() + "/txs", tx_compare).unwrap();

        let mut read_opts = ReadOptions::default();
        let lower_bound = CrossTxDataAsKey {
            target_time: start_timestamp,
            ..Default::default()
        };
        let upper_bound = CrossTxDataAsKey {
            target_time: end_timestamp,
            ..Default::default()
        };
        read_opts.set_iterate_lower_bound(bincode::serialize(&lower_bound).unwrap());
        read_opts.set_iterate_upper_bound(bincode::serialize(&upper_bound).unwrap());
        let iter = db.iterator_opt(IteratorMode::Start, &read_opts);
        let mut txs = Vec::new();
        for (key, value) in iter {
            let k: CrossTxDataAsKey = bincode::deserialize(&key).unwrap();
            let v: CrossTxProfit = bincode::deserialize(&value).unwrap();
            println!("key: {:?}, value: {:?}", k, v);
            txs.push((k, v));
        }

        txs
    }

    pub fn get_timestamp_range_by_block_number(&self, block_number: u64) -> Option<(u64, u64)> {
        todo!()
    }
}

pub async fn run(
    profit_state: Arc<RwLock<State<'static, Keccak256Hasher, ProfitStateData>>>,
    blocks_state: Arc<RwLock<State<'static, Keccak256Hasher, BlocksStateData>>>,
    contract: ContractTest,
    start_block: Arc<RwLock<u64>>,
    sender: Sender<(u64, u64)>,
) -> anyhow::Result<()> {
    let db_path = String::from("db");
    let mut newest_block_receiver = sender.subscribe();
    let txs_handler = Arc::new(TxsDataHandler::new(
        profit_state,
        blocks_state,
        db_path,
        start_block,
        sender,
        contract,
    ));

    let txs_handler1 = txs_handler.clone();
    let (sender, mut receiver) =
        tokio::sync::broadcast::channel::<(BlockInfo, Option<BlockInfo>)>(100);
    let sender1 = sender.clone();

    tokio::spawn(async move {
        loop {
            let newest_block = newest_block_receiver.recv().await.unwrap();
            let trusted_block = newest_block
                .0
                .checked_sub(ETH_DELAY_BLOCKS)
                .expect("newest_block.0.checked_sub(eth_delay) error");

            loop {
                let mut start_block = 0;
                {
                    start_block = txs_handler1.start_block.read().unwrap().clone();
                }
                if start_block <= trusted_block {
                    let start_block_info = txs_handler1
                        .contract
                        .get_block_info(start_block)
                        .await
                        .unwrap();
                    txs_handler1.put_block_info_to_db(
                        start_block_info.storage.block_number,
                        start_block_info.clone(),
                    );
                    if start_block != trusted_block {
                        sender1.send((start_block_info.clone(), None)).unwrap();
                    } else {
                        sender1
                            .send((start_block_info.clone(), Some(start_block_info.clone())))
                            .unwrap();
                    }
                    {
                        let mut next_update = txs_handler1.start_block.write().unwrap();
                        *next_update = trusted_block.checked_add(1).unwrap();
                    }
                } else {
                    break;
                }

                // fixme sleep
            }
        }
    });

    let txs_handler2 = txs_handler.clone();
    tokio::spawn(async move {
        loop {
            let mut start_block_info = (BlockInfo::default(), None);
            {
                start_block_info = receiver.recv().await.unwrap();
            }
            let start_block_timestamp = start_block_info.0.storage.block_timestamp;

            let last_block_number = start_block_info.0.storage.block_number - 1;
            let mut last_block_info = txs_handler2.get_block_info_from_db(last_block_number);
            if last_block_info.is_none() {
                last_block_info = Some(
                    txs_handler2
                        .contract
                        .get_block_info(last_block_number)
                        .await
                        .unwrap(),
                );
            }
            let last_block_timestamp = last_block_info.unwrap().storage.block_timestamp;

            let support_chains: Vec<(u64, u64)> = start_block_info.0.storage.support_chains;
            for i in support_chains {
                let txs = get_txs(
                    get_request_builder(),
                    i.0,
                    last_block_timestamp,
                    start_block_timestamp,
                    i.1,
                )
                .await
                .unwrap();
                let mut new_txs: Vec<(CrossTxDataAsKey, CrossTxProfit)> = Vec::new();
                for tx in txs {
                    // fixme db
                    let commission = txs_handler2
                        .contract.get_maker_commission_by_block(Address::from_str(&tx.maker_address).unwrap(), last_block_number).await.unwrap();
                    let profit = calculate_profit(commission as u64, CrossTxDataAsKey::from(tx.clone()));
                    new_txs.push((CrossTxDataAsKey::from(tx), profit));

                }
                txs_handler2.put_txs_to_db(new_txs);
            }
        }
    });

    let txs_handler3 = txs_handler.clone();
    let mut receiver1 = sender.subscribe();

    tokio::spawn(async move {
        loop {
            let start_block_info = receiver1.recv().await.unwrap();
            if let Some(block_info) = start_block_info.1 {
                if is_chill(block_info) {
                    // fixme
                }
            }
        }
    });

    std::future::pending::<()>().await;
    Ok(())
}
