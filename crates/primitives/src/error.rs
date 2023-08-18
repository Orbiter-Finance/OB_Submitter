use thiserror::Error;
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for state.
#[derive(Error, Debug)]
pub enum Error {
    #[error("binary serialization or deserialization errors")]
    BincodeError(#[from] bincode::Error),
    #[error("ckb-rocksdb errors")]
    RocksDBError(#[from] rocksdb::Error),
    #[error("sparse-merkle-tree errors")]
    SparseMerkleTreeError(#[from] sparse_merkle_tree::error::Error),
}
