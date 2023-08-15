use crate::api::SubmitterApiServer;
use async_trait::async_trait;
use ethers::types::{Address, U256};
use jsonrpsee::core::RpcResult;
use state::{address_convert_to_h256, Blake2bHasher, StataTrait, State, H256};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

pub struct SubmitterApiServerImpl<'a> {
    pub state: Arc<RwLock<State<'a, Blake2bHasher>>>,
}

#[async_trait]
impl SubmitterApiServer for SubmitterApiServerImpl<'static> {
    async fn get_balance(&self, address: Address) -> RpcResult<U256> {
        let state = self.state.read().unwrap();
        let balance = state
            .try_get(address_convert_to_h256(address))
            .unwrap()
            .unwrap()
            .deposit;
        Ok(balance)
    }

    async fn get_root(&self) -> RpcResult<[u8; 32]> {
        let state = self.state.read().unwrap();
        let root = state.try_get_root().unwrap();
        Ok(root.into())
    }

    async fn get_proof(&self, address: Address) -> RpcResult<Vec<u8>> {
        let state = self.state.read().unwrap();
        let proof = state
            .try_get_merkle_proof(vec![address_convert_to_h256(address)])
            .unwrap();
        Ok(proof)
    }

    async fn verify(&self, address: Address, proof: Vec<u8>) -> RpcResult<bool> {
        let state = self.state.read().unwrap();
        let verify = state
            .try_get_merkle_proof(vec![address_convert_to_h256(address)])
            .unwrap()
            == proof;
        Ok(verify)
    }
}
