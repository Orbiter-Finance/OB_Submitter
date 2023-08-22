use blake2b_rs::{Blake2b, Blake2bBuilder};
use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use sparse_merkle_tree::H256;
use super::types::{BlockInfo, CrossTxData};

pub fn address_convert_to_h256(address: Address) -> H256 {
    keccak256(address.as_bytes()).into()
}

pub fn tx_compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let de_a = bincode::deserialize::<CrossTxData>(a).unwrap();
    let de_b = bincode::deserialize::<CrossTxData>(b).unwrap();
    if de_a.target_timestamp.cmp(&de_b.target_timestamp) == std::cmp::Ordering::Equal {
        de_a.target_chain.cmp(&de_b.target_chain)
    } else {
        de_a.target_timestamp.cmp(&de_b.target_timestamp)
    }
}

pub fn block_info_compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let de_a = bincode::deserialize::<BlockInfo>(a).unwrap();
    let de_b = bincode::deserialize::<BlockInfo>(b).unwrap();
    de_a.storage.challenge.cmp(&de_b.storage.challenge)

}

