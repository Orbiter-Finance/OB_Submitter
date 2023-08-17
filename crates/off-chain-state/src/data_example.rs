use ethers::types::{Address, U256};
use ethers::utils::{
    keccak256,
    rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
};

// Serialize, Deserialize
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Data {
    pub address: Address,
    pub nonce: u64,
    pub deposit: U256,
}

impl Decodable for Data {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let address = rlp.val_at(0)?;
        let nonce = rlp.val_at(1)?;
        let deposit = rlp.val_at(2)?;
        Ok(Data {
            address,
            nonce,
            deposit,
        })
    }
}

impl Encodable for Data {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(3);
        s.append(&self.address);
        s.append(&self.nonce);
        s.append(&self.deposit);
    }
}
