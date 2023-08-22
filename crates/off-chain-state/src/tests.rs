#![cfg(test)]

use super::State;
use super::*;
use ethers::types::U256;
use primitives::types::ProfitStateData;
use sparse_merkle_tree::CompiledMerkleProof;
use std::str::FromStr;
use OptimisticTransactionDB;

fn new_state() -> State<'static, Keccak256Hasher, ProfitStateData> {
    let db = OptimisticTransactionDB::open_default("./db1").unwrap();
    let prefix = b"test";
    State::new(prefix, db)
}

#[test]
fn main() {
    let data = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
        token_chain_id: 0,
        balance: U256::zero(),
        maker: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
    };
    let data1 = ProfitStateData {
        token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
        token_chain_id: 1,
        balance: U256::zero(),
        maker: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
    };

    let datas = vec![data.clone(), data1.clone()];
    let value = SmtValue::new(datas.clone()).unwrap();
    println!("smt_value: {:?}", value);
    println!("-------------------------------------------------------");
    println!("datas rlp: {:?}", value.get_serialized_data());
    let datas_h256 = value.to_h256();
    println!("datas h256: {:?}", datas_h256);
    // get alice
    let alice = address_convert_to_h256(
        Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
    );
    println!(
        "address 0x0000000000000000000000000000000000000001 to H256: {:?}",
        address_convert_to_h256(
            Address::from_str("0x0000000000000000000000000000000000000001").unwrap()
        )
    );

    let leaf = (alice, datas_h256);
    println!("-----------------------state test--------------------------------");
    let mut state = new_state();
    state.try_clear();
    let root = state.try_get_root().unwrap();
    println!("try get root from empty db: {:?}", root);
    let proof = state.try_get_merkle_proof(vec![alice]).unwrap();
    println!("try get merkle proof from empty db for alice: {:?}", proof);
    let future_root = state
        .try_get_future_root(proof, vec![(alice, datas.clone())])
        .unwrap();
    println!(
        "try get future root from empty db for alice: {:?}",
        future_root
    );
    let new_root = state.try_update_all(vec![(alice, datas.clone())]).unwrap();
    assert_eq!(new_root, future_root);
    state.try_clear();
    let new_root1 = state.try_get_root().unwrap();
    assert_eq!(new_root1, root);
    let new_root_2 = state.try_update_all(vec![(alice, datas.clone())]).unwrap();
    assert_eq!(new_root_2, new_root);
    let proof = state.try_get_merkle_proof(vec![alice]).unwrap();
    println!("try get merkle proof from db for alice: {:?}", proof);
    let future_root = state
        .try_get_future_root(proof.clone(), vec![(alice, datas.clone())])
        .unwrap();
    assert_eq!(future_root, new_root_2);
    let res = CompiledMerkleProof::from(sparse_merkle_tree::CompiledMerkleProof(proof.clone()))
        .verify::<Keccak256Hasher>(&future_root, vec![leaf])
        .unwrap();
    assert!(res);
    println!("-----------------------end test--------------------------------");
    println!("proof: {:?}, hex: {}", proof, hex::encode(proof.clone()));

    println!(
        "key: {:?}, hex: {}",
        alice,
        hex::encode(Into::<[u8; 32]>::into(leaf.0))
    );
    println!(
        "value: {:?}, hex: {}",
        leaf,
        hex::encode(Into::<[u8; 32]>::into(leaf.1))
    );
    println!(
        "root: {:?}, hex: {}",
        future_root,
        hex::encode(Into::<[u8; 32]>::into(future_root))
    );
    let last_root = state.try_update_all(vec![(alice, vec![])]).unwrap();
    assert_eq!(last_root, root);
}
