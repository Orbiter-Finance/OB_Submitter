use crate::types::{
    BlockInfo, BlockStorage, BlocksStateData, CrossTxProfit, Event, ProfitProof, ProfitStateData,
    ProfitStateDataForRpc,
};
use async_trait::async_trait;
use ethers::types::Address;
use ethers::types::U64;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sparse_merkle_tree::{merge::MergeValue, H256};

// local
use super::error::Result;

#[rpc(server, namespace = "debug")]
pub trait DebugApi {
    #[method(name = "clearState")]
    async fn clear_state(&self) -> RpcResult<()>;
    #[method(name = "updateProfit")]
    async fn update_profit(&self, user: Address, profit: ProfitStateData) -> RpcResult<H256>;
    #[method(name = "updateProfitByCount")]
    async fn update_profit_by_count(&self, count: u64) -> RpcResult<H256>;
}

// The rpc interface provided to the user externally.
#[rpc(server, namespace = "submitter")]
pub trait SubmitterApi {
    #[method(name = "getProfitInfo")]
    async fn get_profit_info(
        &self,
        user: Address,
        tokens: Vec<(u64, Address)>,
    ) -> RpcResult<Vec<ProfitStateDataForRpc>>;

    #[method(name = "getAllProfitInfo")]
    async fn get_all_profit_info(&self, address: Address) -> RpcResult<Vec<ProfitStateDataForRpc>>;
    #[method(name = "getProfitByTxHash")]
    async fn get_profit_by_tx_hash(&self, tx_hash: H256) -> RpcResult<Option<CrossTxProfit>>;
    #[method(name = "getRoot")]
    async fn get_root(&self) -> RpcResult<String>;
    #[method(name = "getProfitProof")]
    async fn get_profit_proof(
        &self,
        user: Address,
        tokens: Vec<(u64, Address)>,
    ) -> RpcResult<Vec<ProfitProof>>;
    #[method(name = "verify")]
    async fn verify(
        &self,
        chain_id: u64,
        token_id: Address,
        address: Address,
        proof: Vec<u8>,
    ) -> RpcResult<bool>;
    #[method(name = "getProfitRootByBlockNum")]
    async fn get_profit_root_by_block_num(&self, block_num: u64) -> RpcResult<BlocksStateData>;
}

/// Several basic implementations of off-chain state.
pub trait StataTrait<K, V> {
    /// Batch to update kvs, and return the new root.
    fn try_update_all(&mut self, future_k_v: Vec<(K, V)>) -> Result<H256>;
    /// clear all data.
    fn try_clear(&mut self) -> Result<()>;
    /// get current merkle proof.
    fn try_get_merkle_proof(&self, keys: Vec<K>) -> Result<Vec<u8>>;
    fn try_get_merkle_proof_1(&self, key: K) -> Result<(H256, Vec<MergeValue>)>;
    /// get the future root without changing the state.
    fn try_get_future_root(&self, old_proof: Vec<u8>, future_k_v: Vec<(K, V)>) -> Result<H256>;
    /// get value by key.
    fn try_get(&self, key: K) -> Result<V>;
    /// get current merkle root.
    fn try_get_root(&self) -> Result<H256>;
}

#[async_trait]
pub trait Contract {
    async fn submit_root(
        &self,
        start: u64,
        end: u64,
        root: [u8; 32],
        blocks_root: [u8; 32],
    ) -> Result<(ethers::types::H256, Option<U64>)>;
    async fn get_block_info(&self, block_number: u64) -> Result<Option<BlockInfo>>;
    async fn get_block_infos(&self, from_block: u64, to_block: u64) -> Result<Vec<BlockInfo>>;
    async fn get_block_storage(&self, block_number: u64) -> Result<Option<BlockStorage>>;
    async fn get_feemanager_contract_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Event>>;
    async fn get_erc20_transfer_events_by_tokens_id(
        &self,
        tokens: Vec<Address>,
        block_number: u64,
    ) -> Result<Vec<Event>>;
    async fn get_dealer_profit_percent_by_block(
        &self,
        maker: Address,
        block_number: u64,
        token_chian_id: u64,
        token_id: Address,
    ) -> Result<u64>;
}
