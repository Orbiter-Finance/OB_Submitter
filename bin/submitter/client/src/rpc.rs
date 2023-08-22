#![allow(unreachable_patterns)]

use async_trait::async_trait;
use ethers::types::{Address, U256};
use ethers::utils::hex;
use jsonrpsee::core::RpcResult;
use jsonrpsee::types::{error::ErrorCode, ErrorObject, ErrorObjectOwned};
use primitives::{constants::*, traits::SubmitterApiServer, types::*};
use primitives::{error::Error as StateError, func::*, traits::StataTrait};
use state::{Keccak256Hasher, State, H256};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

pub struct JsonRpcError(pub ErrorObjectOwned);

impl From<JsonRpcError> for ErrorObjectOwned {
    fn from(err: JsonRpcError) -> Self {
        err.0
    }
}

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

#[async_trait]
impl SubmitterApiServer for SubmitterApiServerImpl<'static> {
    async fn get_profit_info(&self, address: Address) -> RpcResult<String> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let info = state
            .try_get(address_convert_to_h256(address))
            .map_err(|e| Into::<JsonRpcError>::into(e))?
            .ok_or(ErrorObject::owned(
                ACCOUNT_NOT_EXISTS_CODE,
                format!("error: account is not in off-chain-state."),
                None::<bool>,
            ))?;
        Ok(serde_json::to_string(&info).unwrap())
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

    async fn get_proof(&self, address: Address) -> RpcResult<String> {
        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let proof = state
            .try_get_merkle_proof(vec![address_convert_to_h256(address)])
            .map_err(|e| Into::<JsonRpcError>::into(e))?;
        Ok(hex::encode(proof))
    }

    async fn verify(&self, address: Address, proof: Vec<u8>) -> RpcResult<bool> {

        let state = self.state.read().map_err(|_| {
            ErrorObject::owned(
                RWLOCK_READ_ERROR_CODE,
                format!("error: state read error."),
                None::<bool>,
            )
        })?;
        let verify = state
            .try_get_merkle_proof(vec![address_convert_to_h256(address)])
            .map_err(|e| Into::<JsonRpcError>::into(e))?
            == proof;
        Ok(verify)
    }
}
