use blake2b_rs::{Blake2b, Blake2bBuilder};
use ethers::types::{Address, U256};
use sparse_merkle_tree::H256;

pub fn new_blake2b() -> Blake2b {
    Blake2bBuilder::new(32).personal(b"SMT").build()
}

pub fn address_convert_to_h256(address: Address) -> H256 {
    let mut hasher = new_blake2b();
    let mut buf = [0u8; 32];
    hasher.update(address.as_bytes());
    hasher.finalize(&mut buf);
    buf.into()
}
