#![allow(unused_imports)]

use super::types::{BlockInfo, CrossTxData};
use crate::types::CrossTxProfit;
use blake2b_rs::{Blake2b, Blake2bBuilder};
use ethers::{
    abi::{encode, Token},
    types::{Address, U256},
    utils::keccak256,
};
use sparse_merkle_tree::H256;
use std::{cmp::Ordering, str::FromStr};
use tiny_keccak::{Hasher, Keccak};

pub fn chain_token_address_convert_to_h256(
    chain_id: u64,
    token_id: Address,
    address: Address,
) -> H256 {
    let mut tuple: Vec<Token> = Vec::new();
    tuple.push(Token::Uint(chain_id.into()));
    tuple.push(Token::Address(token_id));
    tuple.push(Token::Address(address));
    let t = Token::Tuple(tuple);
    let e = encode(&vec![t]);
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(e.as_slice());
    hasher.finalize(&mut output);
    output.into()
}

pub fn block_number_convert_to_h256(block_number: u64) -> H256 {
    let t = Token::Uint(block_number.into());
    let e = encode(&vec![t]);
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(e.as_slice());
    hasher.finalize(&mut output);
    output.into()
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
