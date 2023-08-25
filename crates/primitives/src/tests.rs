#![cfg(test)]

use super::types::ProfitStateData;
use ethers::abi::ParamType::Tuple;
use ethers::abi::{decode, encode, ParamType, Token, Tokenizable, Tokenize};
use ethers::types::{Address, U256};
use ethers::utils::hex;
use std::str::FromStr;

#[test]
fn main() {
    let data = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000011").unwrap(),
        token_chain_id: 0,
        balance: U256::zero(),
        maker: Address::from_str("0x0000000000000000000000000000000000000054").unwrap(),
    };
    let data1 = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000010").unwrap(),
        token_chain_id: 1,
        balance: U256::zero(),
        maker: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
    };

    let tokens1 = data.clone().into_tokens();
    let tokens2 = data1.clone().into_tokens();
    let tokens = vec![Token::Tuple(tokens1), Token::Tuple(tokens2)];
    let e = encode(&tokens[..]);
    let s = hex::encode(Into::<Vec<u8>>::into(e.as_slice()));
    println!("pro: {:?}", s);
    let ds = decode(
        &[ParamType::Tuple(vec![
            ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Uint(256),
                ParamType::Uint(256),
                ParamType::Address,
            ]),
            ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Uint(256),
                ParamType::Uint(256),
                ParamType::Address,
            ]),
        ])],
        &e,
    )
    .unwrap();
    for d in ds {
        println!(": {:?}\n", d);
    }
}
