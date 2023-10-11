#![cfg(test)]

use super::func::chain_token_address_convert_to_h256;
use crate::keccak256_hasher::Keccak256Hasher;
use ethers::{types::Address, utils::hex};
use sparse_merkle_tree::traits::Hasher;
use std::str::FromStr;

#[test]
fn main() {
    let token = Address::from_str("0x0000000000000000000000000000000000000021").unwrap();
    let token_chain_id = 101u64;
    let user = Address::from_str("0x0000000000000000000000000000000000000022").unwrap();
    let res = chain_token_address_convert_to_h256(token_chain_id, token, user);
    println!("res: {:?}", res);
    println!("res hex: {:?}", hex::encode(res.as_slice()));
    let mut hasher = Keccak256Hasher::default();
    // hasher.write_h256(&H256::from([0; 32]));
    println!("hash: {:?}", hex::encode(hasher.finish().as_slice()));
}
