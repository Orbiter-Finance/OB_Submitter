use ethers::abi::{InvalidOutputType, Token, Tokenizable, Tokenize};
use ethers::{
    abi,
    types::{Address, H256, U256},
    utils::rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
};
use serde::{Deserialize, Serialize};
use sparse_merkle_tree::merge::MergeValue;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitProof<MergeValue> {
    pub path: [u8; 32],
    pub leave_bitmap: [u8; 32],
    pub token: ProfitStateData,
    pub siblings: Vec<MergeValue>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitStateData {
    pub token: Address,
    pub token_chain_id: u64,
    pub balance: U256,
}

impl Tokenizable for ProfitStateData {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Tuple(tuple) = token {
            if tuple.len() == 4 {
                let token = tuple[0]
                    .clone()
                    .into_address()
                    .ok_or(InvalidOutputType(format!(
                        "ProfitStateData from_token error"
                    )))?; // .into_address()?;
                let token_chain_id =
                    tuple[1]
                        .clone()
                        .into_uint()
                        .ok_or(InvalidOutputType(format!(
                            "ProfitStateData from_token error"
                        )))?;
                let balance = tuple[2]
                    .clone()
                    .into_uint()
                    .ok_or(InvalidOutputType(format!(
                        "ProfitStateData from_token error"
                    )))?;

                return Ok(ProfitStateData {
                    token,
                    token_chain_id: token_chain_id.as_u64(),
                    balance,
                });
            }
        }
        return Err(InvalidOutputType(format!(
            "ProfitStateData from_token error"
        )));
    }

    fn into_token(self) -> Token {
        let mut tuple = Vec::new();
        tuple.push(Token::Address(self.token));
        tuple.push(Token::Uint(self.token_chain_id.into()));
        tuple.push(Token::Uint(self.balance));
        Token::Tuple(tuple)
    }
}

impl Encodable for ProfitStateData {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(4);
        s.append(&self.token);
        s.append(&self.token_chain_id);
        s.append(&self.balance);
    }
}

impl Decodable for ProfitStateData {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let token = rlp.val_at(0)?;
        let token_chain_id = rlp.val_at(1)?;
        let balance = rlp.val_at(2)?;
        Ok(ProfitStateData {
            token,
            token_chain_id,
            balance,
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
    pub source_time: u64,
    pub source_token: String,
    pub target_address: String,
    pub target_amount: String,
    pub target_chain: String,
    pub target_id: String,
    pub target_maker: String,
    pub target_symbol: String,
    pub target_time: u64,
    pub target_token: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockStorage {
    pub chill_duration: u64,
    pub challenge_duration: u64,
    pub withdraw_duration: u64,
    pub last_update_block: u64,
    pub last_submit_timestamp: u64,
    pub support_chains: Vec<(u64, u64)>,
    pub block_timestamp: u64,
    pub block_number: u64,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct WithdrawEvent {
    pub address: Address,
    pub chain_id: u64,
    pub token_address: Address,
    pub balance: U256,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DepositEvent {
    pub address: Address,
    pub chain_id: u64,
    pub token_address: Address,
    pub balance: U256,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockInfo {
    pub storage: BlockStorage,
    pub events: Vec<WithdrawEvent>,
}
