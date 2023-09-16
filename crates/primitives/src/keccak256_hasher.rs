#![allow(unused_imports)]

use ethers::utils::keccak256;
use sparse_merkle_tree::{traits::Hasher, H256};
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

pub struct Keccak256Hasher(pub Keccak);

impl Default for Keccak256Hasher {
    fn default() -> Self {
        Keccak256Hasher(Keccak::v256())
    }
}

impl Hasher for Keccak256Hasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }

    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b][..]);
    }

    fn finish(self) -> H256 {
        let mut buf = [0u8; 32];
        self.0.finalize(&mut buf);
        buf.into()
    }
}
