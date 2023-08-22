use ethers::types::{Address, U256};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use std::result::Result as StdResult;
use async_trait::async_trait;
use crate::types::BlockInfo;
// local
use super::error::Result;
// The rpc interface provided to the user externally.
#[rpc(server, namespace = "submitter")]
pub trait SubmitterApi {
    #[method(name = "getBalance")]
    async fn get_profit_info(&self, address: Address) -> RpcResult<String>;
    #[method(name = "getRoot")]
    async fn get_root(&self) -> RpcResult<String>;
    #[method(name = "getProof")]
    async fn get_proof(&self, address: Address) -> RpcResult<String>;
    #[method(name = "verify")]
    async fn verify(&self, address: Address, proof: Vec<u8>) -> RpcResult<bool>;
}

use sparse_merkle_tree::H256;

/// Several basic implementations of off-chain state.
pub trait StataTrait<K, V> {
    /// Batch to update kvs, and return the new root.
    fn try_update_all(&mut self, future_k_v: Vec<(K, Vec<V>)>) -> Result<H256>;
    /// clear all data.
    fn try_clear(&mut self) -> Result<()>;
    /// get current merkle proof.
    fn try_get_merkle_proof(&self, keys: Vec<K>) -> Result<Vec<u8>>;
    /// get the future root without changing the state.
    fn try_get_future_root(&self, old_proof: Vec<u8>, future_k_v: Vec<(K, Vec<V>)>)
        -> Result<H256>;
    /// get value by key.
    fn try_get(&self, key: K) -> Result<Option<Vec<V>>>;
    /// get current merkle root.
    fn try_get_root(&self) -> Result<H256>;
}



#[async_trait]
pub trait Contract{
    async fn submit_root(&self, start: u64, end: u64, blocks_root: [u8; 32], root: [u8; 32]) -> StdResult<(), String>;
    async fn get_block_info(&self, block_number: u64) -> StdResult<BlockInfo, String>;
    async fn get_maker_commission_by_block(&self, maker: Address, block_number: u32) -> StdResult<u32, String>;

}
