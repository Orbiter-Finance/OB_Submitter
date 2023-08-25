use super::types::{BlockInfo, CrossTxData};
use blake2b_rs::{Blake2b, Blake2bBuilder};
use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use sparse_merkle_tree::H256;
use std::cmp::Ordering;
use tiny_keccak::{Hasher, Keccak};

pub fn chain_token_address_convert_to_h256(
    chain_id: u64,
    token_id: Address,
    address: Address,
) -> H256 {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&chain_id.to_le_bytes());
    hasher.update(token_id.as_bytes().into());
    hasher.update(address.as_bytes().into());
    hasher.finalize(&mut output);
    output.into()
}

pub fn block_number_convert_to_h256(block_number: u64) -> H256 {
    keccak256(block_number.to_be_bytes()).into()
}

pub fn tx_compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let de_a = bincode::deserialize::<CrossTxData>(a).unwrap();
    let de_b = bincode::deserialize::<CrossTxData>(b).unwrap();
    if de_a.target_time.cmp(&de_b.target_time) == std::cmp::Ordering::Equal {
        if de_a.target_chain.cmp(&de_b.target_chain) == std::cmp::Ordering::Equal {
            de_a.target_id.cmp(&de_b.target_id)
        } else {
            de_a.target_chain.cmp(&de_b.target_chain)
        }
    } else {
        de_a.target_time.cmp(&de_b.target_time)
    }
}

pub fn block_info_compare(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let de_a = bincode::deserialize::<BlockInfo>(a).unwrap();
    let de_b = bincode::deserialize::<BlockInfo>(b).unwrap();
    if de_a.storage.block_number.cmp(&de_b.storage.block_timestamp) == Ordering::Equal {
        de_a.storage
            .block_timestamp
            .cmp(&de_b.storage.block_timestamp)
    } else {
        de_a.storage.block_number.cmp(&de_b.storage.block_number)
    }
}
