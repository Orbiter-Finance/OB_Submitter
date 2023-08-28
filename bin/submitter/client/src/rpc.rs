#![allow(unreachable_patterns)]

use async_trait::async_trait;
use ethers::types::{Address, StorageProof, U256};
use ethers::utils::hex;
use jsonrpsee::core::RpcResult;
use jsonrpsee::types::{error::ErrorCode, ErrorObject, ErrorObjectOwned};
use primitives::{
    constants::*,
    traits::{DebugApiServer, SubmitterApiServer},
    types::*,
};
use primitives::{error::Error as StateError, func::*, traits::StataTrait};
use sparse_merkle_tree::merge::MergeValue;
use state::{Keccak256Hasher, State, Value, H256};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

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
}

pub struct DebugApiServerImpl<'a> {
    pub state: Arc<RwLock<State<'a, Keccak256Hasher, ProfitStateData>>>,
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
    async fn update_profit(
        &self,
        chain_id: u64,
        token_id: Address,
        address: Address,
        amount: U256,
    ) -> RpcResult<H256> {
        let mut state = self.state.write().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_WRITE_ERROR_CODE,
                format!("error: state write error."),
                None::<bool>,
            )
        })?;
        let key = chain_token_address_convert_to_h256(chain_id, token_id, address);
        let value = ProfitStateData {
            token: token_id,
            token_chain_id: chain_id.clone(),
            balance: amount,
            debt: Default::default(),
        };
        let root = state
            .try_update_all(vec![(key, vec![value])])
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(root)
    }
}

#[async_trait]
impl SubmitterApiServer for SubmitterApiServerImpl<'static> {
    async fn get_profit_info(
        &self,
        user: Address,
        tokens: Vec<(u64, Address)>,
    ) -> RpcResult<Vec<ProfitStateData>> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        if tokens.len() == 0 {
            return Err(ErrorObject::owned(
                PARAMETER_ERROR_CODE,
                format!("error: tokens is empty."),
                None::<bool>,
            ));
        }

        let mut v: Vec<ProfitStateData> = vec![];

        for i in tokens {
            let info = state
                .try_get(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?
                .ok_or(ErrorObject::owned(
                    ACCOUNT_NOT_EXISTS_CODE,
                    format!("error: account is not in off-chain-state."),
                    None::<bool>,
                ))?[0]
                .clone();
            v.push(info);
        }

        Ok(v)
    }
    //
    // async fn get_all_profit_info(&self, address: Address) -> RpcResult<Vec<ProfitStateData>> {
    //     let token1 = ProfitStateData {
    //         token: Address::from_str("0x0000000000000000000000000000000000000011").unwrap(),
    //         token_chain_id: 0,
    //         balance: Default::default(),
    //         debt: U256::from(100),
    //     };
    //     let token2 = ProfitStateData {
    //         token: Address::from_str("0x0000000000000000000000000000000000000022").unwrap(),
    //         token_chain_id: 1,
    //         balance: U256::from(200),
    //         debt: Default::default(),
    //     };
    //     let token3 = ProfitStateData {
    //         token: Address::from_str("0x0000000000000000000000000000000000000033").unwrap(),
    //         token_chain_id: 2,
    //         balance: U256::from(100),
    //         debt: Default::default(),
    //     };
    //     Ok(vec![token1, token2, token3])
    // }

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
            let bitmap_and_sils = state
                .try_get_merkle_proof_1(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?;
            let old_token = state
                .try_get(chain_token_address_convert_to_h256(i.0, i.1, user))
                .map_err(|e| Into::<JsonRpcError>::into(e))?
                .ok_or(ErrorObject::owned(
                    ACCOUNT_NOT_EXISTS_CODE,
                    format!("error: account is not in off-chain-state."),
                    None::<bool>,
                ))?[0]
                .clone();

            v.push(ProfitProof {
                path: chain_token_address_convert_to_h256(i.0, i.1, user).into(),
                leave_bitmap: bitmap_and_sils.0.into(),
                token: old_token,
                siblings: bitmap_and_sils.1,
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
}
