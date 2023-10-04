use thiserror::Error;
pub type Result<T> = std::result::Result<T, Error>;
use crate::types::BlockInfo;
use ethers::prelude::MulticallError;
use ethers::types::U64;
use ethers::{
    contract::ContractError,
    middleware::SignerMiddleware,
    prelude::LocalWallet,
    providers::{Provider, ProviderError},
};
use sled;
use tokio::sync::broadcast::error::{RecvError, SendError};
/// The error type for state.
#[derive(Error, Debug)]
pub enum Error {
    #[error("binary serialization or deserialization errors")]
    BincodeError(#[from] bincode::Error),
    #[error("ckb-rocksdb errors")]
    RocksDBError(#[from] rocksdb::Error),
    #[error("sparse-merkle-tree errors")]
    SparseMerkleTreeError(#[from] sparse_merkle_tree::error::Error),
    #[error("sled db err")]
    SledDBError(#[from] sled::Error),
    #[error("tokio broadcast send err")]
    BroadcastSendError(#[from] SendError<BlockInfo>),
    #[error("tokio broadcast recv err")]
    BroadcastRecvError(#[from] RecvError),
    #[error("ethers  middleware error")]
    ETHMiddlewareError(#[from] ProviderError),
    #[error("ethers signer middleware error")]
    ETHSignerMiddlewareError(
        #[from] ContractError<SignerMiddleware<Provider<ethers_providers::Http>, LocalWallet>>,
    ),
    #[error("ethers contract error")]
    ETHContractError(#[from] ContractError<Provider<ethers_providers::Http>>),
    #[error("submit root failed")]
    SubmitRootFailed(String, Option<U64>),
    #[error("ethers multicall err")]
    ETHMulticallError(
        #[from] MulticallError<SignerMiddleware<Provider<ethers_providers::Http>, LocalWallet>>,
    ),
}
