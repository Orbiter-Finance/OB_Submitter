//! This crate maintains Bundler's off-chain state.
//!
//! Usually, in projects related to zero-knowledge proof,
//! some data that originally relied on the chain to be stored can be stored off-chain.
//! In the ERC4337 protocol, we recommend storing the nonce value and deposit on the chain off the chain,
//! so that we can complete the deduction of Gas fee and modification of the Nonce value off the chain,
//! which will greatly reduce the Gas cost.
//!
//! State should not be shared between different chains, each chain has its own state.
//!
//! For the decentralization of bundlers, we will allow anyone to create their bundlers.
//! Before starting the bundler program, the first step you should do is to synchronize the
//! state off the chain to ensure that the Merkle root of your state is the same as that of others,
//! and further verify the legitimacy of the Merkle root stored on the chain.
//! This requires that the details of your synchronization process must update the state in a certain order.
//!

#![allow(dead_code)]
#![allow(unused_imports)]

// todo list
// 1. Modify the Data structure.
// 2. The scheduler regularly updates the data for the state.
// 4. Create a Merkle tree for all blocks, and the value of the leaf is the income Merkle root of this block so far.

// mod tests;
pub mod data_example;
mod keccak256_hasher;
mod tests;

use bincode;
use blake2b_rs::{Blake2b, Blake2bBuilder};
use byte_slice_cast::AsByteSlice;
use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use ethers::utils::rlp::{decode, encode, Decodable, DecoderError, Encodable, Rlp, RlpStream};
pub use keccak256_hasher::Keccak256Hasher;
use primitives::{error::Result, func::address_convert_to_h256, traits::StataTrait};
use rocksdb::prelude::Iterate;
pub use rocksdb::prelude::Open;
pub use rocksdb::{DBVector, OptimisticTransaction, OptimisticTransactionDB};
use rocksdb::{Direction, IteratorMode};
use serde::{Deserialize, Serialize};
use smt_rocksdb_store::default_store::DefaultStoreMultiTree;
pub use sparse_merkle_tree::traits::Hasher;
pub use sparse_merkle_tree::{traits::Value, CompiledMerkleProof, SparseMerkleTree, H256};
use std::fmt::Debug;
use std::marker::PhantomData;
use thiserror::Error;

type DefaultStoreMultiSMT<'a, H, T, W, Data> =
    SparseMerkleTree<H, SmtValue<Data>, DefaultStoreMultiTree<'a, T, W>>;

/// The value stored in the sparse Merkle tree.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SmtValue<Data> {
    datas: Vec<Data>,
    serialized_data: Vec<u8>,
}

impl<Data: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable> Default
    for SmtValue<Data>
{
    fn default() -> Self {
        SmtValue::new(vec![]).unwrap()
    }
}

impl<Data: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable> SmtValue<Data> {
    pub fn new(datas: Vec<Data>) -> Result<Self> {
        let mut stream = RlpStream::new();
        stream.begin_list(datas.len());
        for d in datas.clone() {
            stream.append(&d);
        }

        Ok(SmtValue {
            datas,
            serialized_data: stream.out().to_vec(),
        })
    }

    pub fn get_datas(&self) -> Vec<Data> {
        self.datas.clone()
    }

    pub fn get_serialized_data(&self) -> &[u8] {
        self.serialized_data.as_ref()
    }
}

impl<D: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable> Value for SmtValue<D> {
    fn to_h256(&self) -> H256 {
        match self {
            // H256::zero() is very important and involves the retention of leaf data.
            a if a == &Default::default() => H256::zero(),
            _ => keccak256(self.get_serialized_data()).into(),
        }
    }

    fn zero() -> Self {
        Default::default()
    }
}

impl<D: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable> From<DBVector>
    for SmtValue<D>
{
    fn from(v: DBVector) -> Self {
        let decode_date = Rlp::new(v.as_ref()).as_list::<D>().unwrap();
        SmtValue {
            datas: decode_date,
            serialized_data: v.to_vec(),
        }
    }
}

impl<D: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable> AsRef<[u8]>
    for SmtValue<D>
{
    fn as_ref(&self) -> &[u8] {
        self.get_serialized_data()
    }
}

/// The state of the bundler.
/// stores off-chain state, its merkle root is stored on-chain.
/// Each entry point contract of each chain has a state.
pub struct State<
    'a,
    H: Hasher + Default,
    D: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable,
> {
    prefix: &'a [u8],
    db: OptimisticTransactionDB,
    _hasher: PhantomData<(H, D)>,
}

impl<
        'a,
        H: Hasher + Default,
        D: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable,
    > State<'a, H, D>
{
    pub fn new(prefix: &'a [u8], db: OptimisticTransactionDB) -> Self {
        State {
            prefix,
            db,
            _hasher: PhantomData,
        }
    }
}

impl<H, Data: Debug + Clone + Default + Eq + PartialEq + Encodable + Decodable>
    StataTrait<H256, Data> for State<'_, H, Data>
where
    H: Hasher + Default,
{
    fn try_update_all(&mut self, future_k_v: Vec<(H256, Vec<Data>)>) -> Result<H256> {
        let kvs = future_k_v
            .into_iter()
            .map(|(k, v)| match SmtValue::new(v) {
                Ok(v) => Ok((k, v)),
                Err(e) => Err(e),
            })
            .collect::<Result<Vec<(H256, SmtValue<Data>)>>>()?;

        let tx = self.db.transaction_default();
        let mut rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, OptimisticTransaction, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::new(self.prefix, &tx))?;
        rocksdb_store_smt.update_all(kvs)?;
        tx.commit()?;
        Ok(*rocksdb_store_smt.root())
    }

    fn try_clear(&mut self) -> Result<()> {
        let snapshot = self.db.snapshot();
        let prefix = self.prefix;
        let prefix_len = prefix.len();
        let leaf_key_len = prefix_len + 32;
        let kvs: Vec<(H256, SmtValue<Data>)> = snapshot
            .iterator(IteratorMode::From(prefix, Direction::Forward))
            .take_while(|(k, _)| k.starts_with(prefix))
            .filter_map(|(k, _)| {
                if k.len() != leaf_key_len {
                    None
                } else {
                    let leaf_key: [u8; 32] = k[prefix_len..].try_into().expect("checked 32 bytes");
                    Some((leaf_key.into(), SmtValue::zero()))
                }
            })
            .collect();

        let tx = self.db.transaction_default();
        let mut rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, OptimisticTransaction, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::new(
            prefix.as_byte_slice(),
            &tx,
        ))?;
        #[cfg(test)]
        println!("root: {:?}", rocksdb_store_smt.root());
        #[cfg(test)]
        println!("clear kvs: {:?}", kvs);
        rocksdb_store_smt.update_all(kvs)?;
        tx.commit()?;
        #[cfg(test)]
        println!("root1: {:?}", self.try_get_root().unwrap());

        assert_eq!(rocksdb_store_smt.root(), &H256::zero());
        Ok(())
    }

    fn try_get_merkle_proof(&self, keys: Vec<H256>) -> Result<Vec<u8>> {
        let snapshot = self.db.snapshot();
        let rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, _, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::<_, ()>::new(
            self.prefix,
            &snapshot,
        ))?;
        let proof = rocksdb_store_smt
            .merkle_proof(keys.clone())?
            .compile(keys)?;
        Ok(proof.0)
    }

    fn try_get_future_root(
        &self,
        old_proof: Vec<u8>,
        future_k_v: Vec<(H256, Vec<Data>)>,
    ) -> Result<H256> {
        let p = CompiledMerkleProof(old_proof);
        let kvs = future_k_v
            .into_iter()
            .map(|(k, v)| match SmtValue::new(v) {
                Ok(v) => Ok((k, v.to_h256())),
                Err(e) => Err(e),
            })
            .collect::<Result<Vec<(H256, H256)>>>()?;

        let f_root = p.compute_root::<H>(kvs)?;
        Ok(f_root)
    }

    fn try_get(&self, key: H256) -> Result<Option<Vec<Data>>> {
        let snapshot = self.db.snapshot();
        let rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, _, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::<_, ()>::new(
            self.prefix,
            &snapshot,
        ))?;
        let v = rocksdb_store_smt.get(&key)?;
        let data = v.get_datas();
        Ok(Some(data.clone()))
    }

    fn try_get_root(&self) -> Result<H256> {
        let snapshot = self.db.snapshot();
        let rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, _, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::<_, ()>::new(
            self.prefix,
            &snapshot,
        ))?;
        let root = *rocksdb_store_smt.root();
        Ok(root)
    }
}
