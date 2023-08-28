use ethers::abi::{Detokenize, InvalidOutputType, Token, Tokenizable, TokenizableItem, Tokenize};
use ethers::{
    abi::{self, decode, encode, Error, ParamType},
    types::{Address, H256, U256},
    utils::rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sparse_merkle_tree::merge::MergeValue;
use std::cmp::min;
use std::sync::atomic::Ordering;

#[serde_as]
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitProof {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub path: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub leave_bitmap: [u8; 32],
    pub token: ProfitStateData,
    pub siblings: Vec<MergeValue>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitStateData {
    pub token: Address,
    pub token_chain_id: u64,
    pub balance: U256,
    pub debt: U256,
}

pub trait Debt {
    fn add_balance(&mut self, amount: U256) -> std::result::Result<(), String>;
    fn sub_balance(&mut self, amount: U256) -> std::result::Result<(), String>;
}

impl Debt for ProfitStateData {
    fn add_balance(&mut self, amount: U256) -> std::result::Result<(), String> {
        let debt = self.debt;
        let min_debt = min(debt, amount);
        self.debt = debt.checked_sub(min_debt).ok_or("overflow")?;
        let amount = amount.checked_sub(min_debt).ok_or("overflow")?;
        self.balance = self.balance.checked_add(amount).ok_or("overflow")?;
        Ok(())
    }

    fn sub_balance(&mut self, amount: U256) -> std::result::Result<(), String> {
        self.balance = self.balance.checked_sub(amount).ok_or("overflow")?;
        Ok(())
    }
}

impl TokenizableItem for ProfitStateData {}

pub trait AbiDecode {
    fn decode(bytes: Vec<u8>) -> std::result::Result<Vec<Token>, Error>;
}

impl AbiDecode for ProfitStateData {
    fn decode(bytes: Vec<u8>) -> std::result::Result<Vec<Token>, Error> {
        decode(
            &[ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Uint(256),
                ParamType::Uint(256),
            ])))],
            &bytes,
        )
    }
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
                        "ProfitStateData from_token error: token"
                    )))?; // .into_address()?;
                let token_chain_id =
                    tuple[1]
                        .clone()
                        .into_uint()
                        .ok_or(InvalidOutputType(format!(
                            "ProfitStateData from_token error: chain_id"
                        )))?;
                let balance = tuple[2]
                    .clone()
                    .into_uint()
                    .ok_or(InvalidOutputType(format!(
                        "ProfitStateData from_token error:balance"
                    )))?;
                let debt = tuple[3]
                    .clone()
                    .into_uint()
                    .ok_or(InvalidOutputType(format!(
                        "ProfitStateData from_token error:debt"
                    )))?;

                return Ok(ProfitStateData {
                    token,
                    token_chain_id: token_chain_id.as_u64(),
                    balance,
                    debt,
                });
            }
        }
        return Err(InvalidOutputType(format!(
            "ProfitStateData from_token error: all"
        )));
    }

    fn into_token(self) -> Token {
        let mut tuple = Vec::new();
        tuple.push(Token::Address(self.token));
        tuple.push(Token::Uint(self.token_chain_id.into()));
        tuple.push(Token::Uint(self.balance));
        tuple.push(Token::Uint(self.debt));
        Token::Tuple(tuple)
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct BlocksStateData {
    pub root: [u8; 32],
    pub txs: [u8; 32],
}

impl TokenizableItem for BlocksStateData {}

impl Tokenizable for BlocksStateData {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Tuple(tuple) = token {
            if tuple.len() == 2 {
                let root = tuple[0]
                    .clone()
                    .into_fixed_bytes()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: root"
                    )))?; // .into_address()?;
                let txs = tuple[1]
                    .clone()
                    .into_fixed_bytes()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: txs"
                    )))?;

                return Ok(BlocksStateData {
                    root: root[..32].to_owned().try_into().unwrap(),
                    txs: txs[..32].to_owned().try_into().unwrap(),
                });
            }
        }
        return Err(InvalidOutputType(format!(
            "BlocksStateData from_token error: all"
        )));
    }

    fn into_token(self) -> Token {
        let mut tuple = Vec::new();
        tuple.push(Token::FixedBytes(self.root.into()));
        tuple.push(Token::FixedBytes(self.txs.into()));
        Token::Tuple(tuple)
    }
}

impl AbiDecode for BlocksStateData {
    fn decode(bytes: Vec<u8>) -> std::result::Result<Vec<Token>, Error> {
        decode(
            &[ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::FixedBytes(32),
                ParamType::FixedBytes(32),
            ])))],
            &bytes,
        )
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossTxDataAsKey {
    pub maker_address: String,
    pub dealer_address: String,
    pub profit: String,
    pub source_address: String,
    pub source_amount: String,
    pub source_chain: String,
    // tx hash
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
pub struct CrossTxProfit {
    pub maker_address: Address,
    pub dealer_address: Address,
    pub profit: U256,
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum Event {
    Deposit(DepositEvent),
    Withdraw(WithdrawEvent),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockInfo {
    pub storage: BlockStorage,
    pub events: Vec<Event>,
}
