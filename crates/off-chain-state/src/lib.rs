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

// mod tests;
pub mod data_example;
mod tests;

use bincode;
use blake2b_rs::{Blake2b, Blake2bBuilder};
use byte_slice_cast::AsByteSlice;
use ethers::{
    abi::{decode, encode, ParamType, Tokenizable, TokenizableItem},
    types::{Address, U256},
    utils::keccak256,
};
pub use primitives::keccak256_hasher::Keccak256Hasher;
use primitives::{error::Result, traits::StataTrait, types::AbiDecode};
use rocksdb::{prelude::Iterate, Direction, IteratorMode};
pub use rocksdb::{prelude::Open, DBVector, OptimisticTransaction, OptimisticTransactionDB};
use serde::{Deserialize, Serialize};
use smt_rocksdb_store::default_store::DefaultStoreMultiTree;
use sparse_merkle_tree::merge::MergeValue;
pub use sparse_merkle_tree::{
    traits::{Hasher, Value},
    CompiledMerkleProof, SparseMerkleTree, H256,
};
use std::{fmt::Debug, marker::PhantomData};
use thiserror::Error;

type DefaultStoreMultiSMT<'a, H, T, W, Data> =
    SparseMerkleTree<H, SmtValue<Data>, DefaultStoreMultiTree<'a, T, W>>;

/// The value stored in the sparse Merkle tree.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SmtValue<Data> {
    data: Data,
    serialized_data: Vec<u8>,
}

impl<
        Data: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode,
    > Default for SmtValue<Data>
{
    fn default() -> Self {
        SmtValue::new(Data::default()).unwrap()
    }
}

impl<
        Data: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode,
    > SmtValue<Data>
{
    pub fn new(data: Data) -> Result<Self> {
        let t = data.clone().into_token();
        let serialized_data = encode(&vec![t.clone()]);

        Ok(SmtValue {
            data,
            serialized_data: serialized_data.to_vec(),
        })
    }

    pub fn get_data(&self) -> Data {
        self.data.clone()
    }

    pub fn get_serialized_data(&self) -> &[u8] {
        self.serialized_data.as_ref()
    }
}

impl<D: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode> Value
    for SmtValue<D>
{
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

impl<D: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode>
    From<DBVector> for SmtValue<D>
{
    fn from(v: DBVector) -> Self {
        let t = D::decode(v.to_vec()).unwrap();
        let decode_date = D::from_token(t[0].clone()).unwrap();
        SmtValue {
            data: decode_date,
            serialized_data: v.to_vec(),
        }
    }
}

impl<D: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode>
    AsRef<[u8]> for SmtValue<D>
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
    D: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode,
> {
    prefix: &'a [u8],
    db: OptimisticTransactionDB,
    _hasher: PhantomData<(H, D)>,
}

impl<
        'a,
        H: Hasher + Default,
        D: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode,
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

impl<
        H,
        Data: Debug + Clone + Default + Eq + PartialEq + TokenizableItem + Tokenizable + AbiDecode,
    > StataTrait<H256, Data> for State<'_, H, Data>
where
    H: Hasher + Default,
{
    fn try_update_all(&mut self, future_k_v: Vec<(H256, Data)>) -> Result<H256> {
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
        let proof = rocksdb_store_smt.merkle_proof(keys.clone()).unwrap();
        let proof = proof.compile(keys)?;
        Ok(proof.0)
    }

    fn try_get_merkle_proof_1(&self, key: H256) -> Result<(H256, Vec<MergeValue>)> {
        let snapshot = self.db.snapshot();
        let rocksdb_store_smt: SparseMerkleTree<
            H,
            SmtValue<Data>,
            DefaultStoreMultiTree<'_, _, ()>,
        > = DefaultStoreMultiSMT::new_with_store(DefaultStoreMultiTree::<_, ()>::new(
            self.prefix,
            &snapshot,
        ))?;
        let proof = rocksdb_store_smt.merkle_proof(vec![key])?;
        let leaves_bitmap = proof.leaves_bitmap();
        let leave_bitmap = leaves_bitmap[0];
        let siblings = proof.merkle_path();
        Ok((leave_bitmap, siblings.clone()))
    }

    fn try_get_future_root(
        &self,
        old_proof: Vec<u8>,
        future_k_v: Vec<(H256, Data)>,
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

    fn try_get(&self, key: H256) -> Result<Data> {
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
        let data = v.get_data();
        Ok(data.clone())
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
