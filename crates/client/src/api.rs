use ethers::types::{Address, U256};
use state::H256;
use std::ops::Add;

use jsonrpsee::{core::RpcResult, proc_macros::rpc};

// The rpc interface provided to the user externally.
#[rpc(server, namespace = "submitter")]
pub trait SubmitterApi {
    #[method(name = "getBalance")]
    async fn get_balance(&self, address: Address) -> RpcResult<U256>;
    #[method(name = "getRoot")]
    async fn get_root(&self) -> RpcResult<[u8; 32]>;
    #[method(name = "getProof")]
    async fn get_proof(&self, address: Address) -> RpcResult<Vec<u8>>;
    #[method(name = "verify")]
    async fn verify(&self, address: Address, proof: Vec<u8>) -> RpcResult<bool>;
}
