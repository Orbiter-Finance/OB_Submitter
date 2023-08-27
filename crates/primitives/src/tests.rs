#![cfg(test)]

use super::types::ProfitStateData;
use crate::types::AbiDecode;
use ethers::abi::Detokenize;
use ethers::abi::ParamType::Tuple;
use ethers::abi::{
    decode, encode, Error, ParamType, Token, Tokenizable, TokenizableItem, Tokenize,
};
use ethers::types::{Address, U256};
use ethers::utils::hex;
use std::str::FromStr;

#[test]
fn main() {
    let data = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000011").unwrap(),
        token_chain_id: 0,
        balance: U256::zero(),
    };
    let data1 = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000010").unwrap(),
        token_chain_id: 1,
        balance: U256::zero(),
    };

    let aa = data.clone().into_token();
    println!("aa:{:?}", aa);
    let bb = ProfitStateData::from_token(aa).unwrap();
    println!("bb: {:?}", bb);
    let data3 = vec![data.clone(), data1.clone()];
    let t = data3.into_token();
    let d = ProfitStateData::decode(encode(&vec![t.clone()])).unwrap();
    println!("d: {:?}", d);
    println!("t: {:?}", t);
    let tt: Vec<ProfitStateData> = Vec::<ProfitStateData>::from_token(d[0].clone()).unwrap();
    println!("tt: {:?}", tt);

    // let tokens1 = data.clone().into_tokens();
    // let tokens2 = data1.clone().into_tokens();
    // let tokens = vec![tokens1, tokens2].into_tokens();
    // println!("tokens: {:?}", tokens);
    // let e = encode(&tokens[..]);
    // let d = decode(
    //     &[ParamType::Tuple(vec![
    //         ParamType::Tuple(vec![
    //             ParamType::Address,
    //             ParamType::Uint(256),
    //             ParamType::Uint(256),
    //         ]),
    //         ParamType::Tuple(vec![
    //             ParamType::Address,
    //             ParamType::Uint(256),
    //             ParamType::Uint(256),
    //         ]),
    //     ])],
    //     &e,
    // )
    //     .unwrap();
    // println!("e: {:?}", d);
}
