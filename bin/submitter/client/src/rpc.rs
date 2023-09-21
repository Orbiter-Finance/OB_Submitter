#![allow(unreachable_patterns)]

use async_trait::async_trait;
use ethers::{
    types::{Address, StorageProof, U256},
    utils::hex,
};
use jsonrpsee::{
    core::RpcResult,
    types::{error::ErrorCode, ErrorObject, ErrorObjectOwned},
};
use primitives::{
    constants::*,
    error::Error as StateError,
    func::*,
    traits::{DebugApiServer, StataTrait, SubmitterApiServer},
    types::*,
};
use sparse_merkle_tree::merge::MergeValue;
use state::{Keccak256Hasher, SmtValue, State, Value, H256};
use std::{
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
};
use txs::{
    rocks_db::TxsRocksDB,
    sled_db::{ProfitStatisticsDB, UserTokensDB},
};
use utils::{get_no1_merge_value, SMTBitMap};

pub struct JsonRpcError(pub ErrorObjectOwned);

impl From<JsonRpcError> for ErrorObjectOwned {
    fn from(err: JsonRpcError) -> Self {
        err.0
    }
}

pub const RWLOCK_WRITE_ERROR_CODE: i32 = 888;
pub const PARAMETER_ERROR_CODE: i32 = 889;

impl From<StateError> for JsonRpcError {
    fn from(err: StateError) -> Self {
        JsonRpcError(match err {
            StateError::BincodeError(e) => {
                ErrorObject::owned(STATE_ERROR_CODE, format!("error: {:#?}", e), None::<bool>)
            }

            StateError::RocksDBError(e) => {
                ErrorObject::owned(STATE_ERROR_CODE, format!("error: {:#?}", e), None::<bool>)
            }

            StateError::SparseMerkleTreeError(e) => {
                ErrorObject::owned(STATE_ERROR_CODE, format!("error: {:#?}", e), None::<bool>)
            }
            _ => ErrorObject::owned(
                STATE_ERROR_CODE,
                format!("error: unknown err"),
                None::<bool>,
            ),
        })
    }
}

pub struct SubmitterApiServerImpl<'a> {
    pub state: Arc<RwLock<State<'a, Keccak256Hasher, ProfitStateData>>>,
    pub blocks_state: Arc<RwLock<State<'a, Keccak256Hasher, BlocksStateData>>>,
    pub user_tokens_db: Arc<UserTokensDB>,
    pub profit_statistics_db: Arc<ProfitStatisticsDB>,
    pub txs_db: Arc<TxsRocksDB>,
}

pub struct DebugApiServerImpl<'a> {
    pub state: Arc<RwLock<State<'a, Keccak256Hasher, ProfitStateData>>>,
    pub user_tokens_db: Arc<UserTokensDB>,
}

#[async_trait]
impl DebugApiServer for DebugApiServerImpl<'static> {
    async fn clear_state(&self) -> RpcResult<()> {
        let mut state = self.state.write().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_WRITE_ERROR_CODE,
                format!("error: state write error."),
                None::<bool>,
            )
        })?;
        state
            .try_clear()
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(())
    }

    async fn update_profit(&self, user: Address, mut profit: ProfitStateData) -> RpcResult<H256> {
        let mut state = self.state.write().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_WRITE_ERROR_CODE,
                format!("error: state write error."),
                None::<bool>,
            )
        })?;
        let _ = self
            .user_tokens_db
            .insert_token(user, profit.token_chain_id, profit.token)
            .unwrap();
        let key = chain_token_address_convert_to_h256(profit.token_chain_id, profit.token, user);
        profit.try_clear().unwrap();
        let root = state
            .try_update_all(vec![(key, profit)])
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(root)
    }

    async fn update_profit_by_count(&self, count: u64) -> RpcResult<H256> {
        let mut profit_state = self.state.write().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_WRITE_ERROR_CODE,
                format!("error: state write error."),
                None::<bool>,
            )
        })?;
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
        let root = profit_state
            .try_get_root()
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(root)
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

#[async_trait]
impl SubmitterApiServer for SubmitterApiServerImpl<'static> {
    async fn get_profit_info(
        &self,
        user: Address,
        tokens: Vec<(u64, Address)>,
    ) -> RpcResult<Vec<ProfitStateDataForRpc>> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;

        let mut v: Vec<ProfitStateDataForRpc> = vec![];

        for i in tokens {
            let info: ProfitStateData = state
                .try_get(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?;
            if info.balance == U256::zero() && info.debt == U256::zero() {
                continue;
            }
            let mut total_profit = U256::zero();
            let mut total_withdrawn = U256::zero();
            if let Some(profit_statistics) = self
                .profit_statistics_db
                .get_profit_statistics(user, info.token_chain_id, info.token)
                .unwrap()
            {
                total_profit = profit_statistics.total_profit;
                total_withdrawn = profit_statistics.total_withdrawn;
            }

            let info = ProfitStateDataForRpc {
                token: info.token,
                token_chain_id: info.token_chain_id,
                balance: info.balance,
                debt: info.debt,
                total_profit: total_profit,
                total_withdrawn: total_withdrawn,
            };
            v.push(info);
        }

        Ok(v)
    }

    async fn get_all_profit_info(&self, user: Address) -> RpcResult<Vec<ProfitStateDataForRpc>> {
        let tokens = self.user_tokens_db.get_tokens(user).unwrap();
        self.get_profit_info(user, tokens).await
    }

    async fn get_profit_by_tx_hash(&self, tx_hash: H256) -> RpcResult<Option<CrossTxProfit>> {
        self.txs_db.get_profit_by_yx_hash(tx_hash).map_err(|_| {
            ErrorObject::owned(1111, format!("error: get tx's profit err."), None::<bool>)
        })
    }

    async fn get_root(&self) -> RpcResult<String> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let root = state
            .try_get_root()
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(hex::encode(Into::<[u8; 32]>::into(root)))
    }

    async fn get_profit_proof(
        &self,
        user: Address,
        tokens: Vec<(u64, Address)>,
    ) -> RpcResult<Vec<ProfitProof>> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let mut v: Vec<ProfitProof> = vec![];
        if tokens.len() == 0 {
            return Ok(v);
        }

        for i in tokens.clone() {
            let root = state
                .try_get_root()
                .map_err(|e| Into::<JsonRpcError>::into(e))?;
            let bitmap_and_sils = state
                .try_get_merkle_proof_1(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?;
            let old_token = state
                .try_get(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?;
            let no1_merge_value = get_no1_merge_value(
                chain_token_address_convert_to_h256(i.0, i.1, user).into(),
                SmtValue::new(old_token.clone()).unwrap(),
                bitmap_and_sils.0.into(),
            );
            v.push(ProfitProof {
                path: chain_token_address_convert_to_h256(i.0, i.1, user).into(),
                leave_bitmap: bitmap_and_sils.0.into(),
                token: old_token,
                siblings: bitmap_and_sils.1,
                root: root.into(),
                no1_merge_value,
            });
        }
        Ok(v)
    }

    async fn verify(
        &self,
        chain_id: u64,
        token_id: Address,
        address: Address,
        proof: Vec<u8>,
    ) -> RpcResult<bool> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let verify = state
            .try_get_merkle_proof(vec![chain_token_address_convert_to_h256(
                chain_id, token_id, address,
            )])
            .map_err(|e| Into::<JsonRpcError>::into(e))?
            == proof;
        Ok(verify)
    }

    async fn get_profit_root_by_block_num(&self, block_num: u64) -> RpcResult<BlocksStateData> {
        let blocks_state = self.blocks_state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let key = block_number_convert_to_h256(block_num);
        let root = blocks_state
            .try_get(key)
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(root)
    }
}
