#![cfg(test)]

use super::{State, *};
use ethers::{types::U256, utils::keccak256};
use primitives::{func::chain_token_address_convert_to_h256, types::ProfitStateData};
use sparse_merkle_tree::{
    merge::{hash_base_node, merge},
    CompiledMerkleProof,
};
use std::str::FromStr;
use OptimisticTransactionDB;

pub fn into_merge_value<H: Hasher + Default>(key: H256, value: H256, height: u8) -> MergeValue {
    // try keep hash same with MergeWithZero
    if value.is_zero() || height == 0 {
        MergeValue::from_h256(value)
    } else {
        let base_key = key.parent_path(0);
        let base_node = hash_base_node::<H>(0, &base_key, &value);
        let mut zero_bits = key;
        for i in height..=core::u8::MAX {
            if key.get_bit(i) {
                zero_bits.clear_bit(i);
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count: height,
        }
    }
}

fn new_state() -> State<'static, Keccak256Hasher, ProfitStateData> {
    let db = OptimisticTransactionDB::open_default("./db1").unwrap();
    let prefix = b"test";
    State::new(prefix, db)
}

// fn update_db(k_v: Vec<(H256, ProfitStateData)>) -> SMT {
//     let mut tree = SMT::default();
//     for (key, value) in k_v {
//         tree.update(key, value).expect("update");
//     }
//     tree
// }

#[test]
fn test_state() {
    let mut tree = new_state();
    let user: Address = Address::from_str("0x0000000000000000000000000000000000000021").unwrap();
    let token: Address = Address::from_str("0x0000000000000000000000000000000000000011").unwrap();
    let user1: Address = Address::from_str("0x0000000000000000000000000000000000000021").unwrap();
    let user2: Address = Address::from_str("0x0000000000000000000000000000000000000021").unwrap();
    let chain_id: u64 = 1;
    let mut data = ProfitStateData {
        token: token.clone(),
        token_chain_id: chain_id,
        balance: U256::from(100),
        debt: U256::from(60),
    };

    tree.try_update_all(vec![(
        chain_token_address_convert_to_h256(chain_id, token, user),
        data.clone(),
    )])
    .unwrap();
    // let proof = tree
    //     .try_get_proof(chain_token_address_convert_to_h256(chain_id, token, user))
    //     .unwrap();
    let value = tree
        .try_get(chain_token_address_convert_to_h256(chain_id, token, user))
        .unwrap();
    println!("value: {:?}", value);
}

#[test]
fn main() {
    // let data = ProfitStateData {
    //     token: Address::from_str("0x0000000000000000000000000000000000000011").unwrap(),
    //     token_chain_id: 0,
    //     balance: U256::from(100),
    //     debt: U256::from(60)
    // };
    // println!("data: {:?}", data);
    // let e = SmtValue::new(vec![data.clone()]).unwrap();
    // let ee = e.get_serialized_data();
    // println!("encode data: {:?}", ee.clone());
    // println!("hex: {:?}", hex::encode(ee.clone()));

    // let data1 = ProfitStateData {
    //     token: Address::from_str("0x0000000000000000000000000000000000000022").unwrap(),
    //     token_chain_id: 1,
    //     balance: U256::from(80),
    //     debt: U256::from(59),
    // };
    //
    // let datas = vec![data.clone(), data1.clone()];
    // let value = SmtValue::new(datas.clone()).unwrap();
    // println!("smt_value: {:?}", value);
    // println!("-------------------------------------------------------");
    // println!("datas rlp: {:?}", value.get_serialized_data());
    // let datas_h256 = value.to_h256();
    // println!("datas h256: {:?}", datas_h256);
    // // get alice
    // let alice = address_convert_to_h256(
    //     Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
    // );
    // println!(
    //     "address 0x0000000000000000000000000000000000000001 to H256: {:?}",
    //     address_convert_to_h256(
    //         Address::from_str("0x0000000000000000000000000000000000000001").unwrap()
    //     )
    // );
    //
    // let leaf = (alice, datas_h256);
    // println!("-----------------------state test--------------------------------");
    // let mut state = new_state();
    // state.try_clear();
    // let root = state.try_get_root().unwrap();
    // println!("try get root from empty db: {:?}", root);
    // let proof = state.try_get_merkle_proof(vec![alice]).unwrap();
    // println!("try get merkle proof from empty db for alice: {:?}", proof);
    // let future_root = state
    //     .try_get_future_root(proof, vec![(alice, datas.clone())])
    //     .unwrap();
    // println!(
    //     "try get future root from empty db for alice: {:?}",
    //     future_root
    // );
    // let new_root = state.try_update_all(vec![(alice, datas.clone())]).unwrap();
    // assert_eq!(new_root, future_root);
    // state.try_clear();
    // let new_root1 = state.try_get_root().unwrap();
    // assert_eq!(new_root1, root);
    // let new_root_2 = state.try_update_all(vec![(alice, datas.clone())]).unwrap();
    // assert_eq!(new_root_2, new_root);
    // let proof = state.try_get_merkle_proof(vec![alice]).unwrap();
    // println!("try get merkle proof from db for alice: {:?}", proof);
    // let future_root = state
    //     .try_get_future_root(proof.clone(), vec![(alice, datas.clone())])
    //     .unwrap();
    // assert_eq!(future_root, new_root_2);
    // let res = CompiledMerkleProof::from(sparse_merkle_tree::CompiledMerkleProof(proof.clone()))
    //     .verify::<Keccak256Hasher>(&future_root, vec![leaf])
    //     .unwrap();
    // assert!(res);
    // println!("-----------------------end test--------------------------------");
    // println!("proof: {:?}, hex: {}", proof, hex::encode(proof.clone()));
    //
    // println!(
    //     "key: {:?}, hex: {}",
    //     alice,
    //     hex::encode(Into::<[u8; 32]>::into(leaf.0))
    // );
    // println!(
    //     "value: {:?}, hex: {}",
    //     leaf,
    //     hex::encode(Into::<[u8; 32]>::into(leaf.1))
    // );
    // println!(
    //     "root: {:?}, hex: {}",
    //     future_root,
    //     hex::encode(Into::<[u8; 32]>::into(future_root))
    // );
    // let last_root = state.try_update_all(vec![(alice, vec![])]).unwrap();
    // assert_eq!(last_root, root);
}
