use crate::types::ChainType;
use lazy_static::lazy_static;
use std::collections::btree_map::BTreeMap;

lazy_static! {
    pub static ref ChainsType: BTreeMap<u64, ChainType> = {
        let mut map = BTreeMap::new();
        /// mainnet
        // Ethereum
        map.insert(1, ChainType::Normal);
        // Arbitrum
        map.insert(42161, ChainType::OP);
        // OP
        map.insert(10, ChainType::OP);
        // zkSync
        map.insert(324, ChainType::ZK);

        /// testnet
        // Goerli
        map.insert(5, ChainType::Normal);
        // op goerli
        map.insert(420, ChainType::OP);
        // arb goerli
        map.insert(421613, ChainType::OP);
        // zk goerli
        map.insert(280, ChainType::ZK);
        map

    };
}

pub fn get_chain_type(chain_id: u64) -> ChainType {
    ChainsType
        .get(&chain_id)
        .unwrap_or(&ChainType::Normal)
        .clone()
}
