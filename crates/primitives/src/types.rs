use ethers::{
    types::{Address, H256, U256},
    utils::rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
};
use serde::{Deserialize, Serialize};
// use sparse_merkle_tree::H256;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ProfitStateData {
    pub token: Address,
    pub token_chain_id: u64,
    pub balance: U256,
    pub maker: Address,
}

impl Encodable for ProfitStateData {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4);
        s.append(&self.token);
        s.append(&self.token_chain_id);
        s.append(&self.balance);
        s.append(&self.maker);
    }
}

impl Decodable for ProfitStateData {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let token = rlp.val_at(0)?;
        let token_chain_id = rlp.val_at(1)?;
        let balance = rlp.val_at(2)?;
        let maker = rlp.val_at(3)?;
        Ok(ProfitStateData {
            token,
            token_chain_id,
            balance,
            maker,
        })
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct BlocksStateData {
    pub root: H256,
    pub txs: H256,
}

impl Decodable for BlocksStateData {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let root = rlp.val_at(0)?;
        let txs = rlp.val_at(1)?;
        Ok(BlocksStateData { root, txs })
    }
}

impl Encodable for BlocksStateData {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2);
        s.append(&self.root);
        s.append(&self.txs);
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossTxData {
    pub dealer_address: String,
    pub profit: String,
    pub source_address: String,
    pub source_amount: String,
    pub source_chain: String,
    pub source_id: String,
    pub source_maker: String,
    pub source_symbol: String,
    pub source_time: String,
    pub source_token: String,
    pub target_address: String,
    pub target_amount: String,
    pub target_chain: String,
    pub target_id: String,
    pub target_maker: String,
    pub target_symbol: String,
    pub target_time: String,
    pub target_token: String,
}
