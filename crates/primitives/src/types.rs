#![allow(unused_imports)]

use crate::keccak256_hasher;
use ethers::{
    abi::{
        self, decode, encode, Detokenize, Error, InvalidOutputType, ParamType, Token, Tokenizable,
        TokenizableItem, Tokenize,
    },
    types::{Address, U256},
    utils::rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
};
use keccak256_hasher::Keccak256Hasher;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sparse_merkle_tree::{h256::H256, merge::MergeValue, traits::Hasher};
use std::{cmp::min, str::FromStr, sync::atomic::Ordering};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ChainType {
    ZK,
    OP,
    Normal,
}

#[serde_as]
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitProof {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub path: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub leave_bitmap: [u8; 32],
    pub token: ProfitStateData,
    pub siblings: Vec<MergeValue>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub root: [u8; 32],
    pub no1_merge_value: (u8, H256),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitStateData {
    pub token: Address,
    pub token_chain_id: u64,
    pub balance: U256,
    pub debt: U256,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitStateDataForRpc {
    pub token: Address,
    pub token_chain_id: u64,
    pub balance: U256,
    pub debt: U256,
    pub total_profit: U256,
    pub total_withdrawn: U256,
}
pub trait Debt {
    fn add_balance(&mut self, amount: U256) -> std::result::Result<(), String>;
    fn sub_balance(&mut self, amount: U256) -> std::result::Result<(), String>;
    fn try_clear(&mut self) -> std::result::Result<(), String>;
}

impl Debt for ProfitStateData {
    fn add_balance(&mut self, amount: U256) -> std::result::Result<(), String> {
        let debt = self.debt;
        let min_debt = min(debt, amount);
        self.debt = debt.checked_sub(min_debt).ok_or("overflow")?;
        let amount = amount.checked_sub(min_debt).ok_or("overflow")?;
        self.balance = self.balance.checked_add(amount).ok_or("overflow")?;
        self.try_clear()?;
        Ok(())
    }

    fn sub_balance(&mut self, amount: U256) -> std::result::Result<(), String> {
        let min_amount = min(amount, self.balance);
        self.balance = self.balance.checked_sub(min_amount).ok_or("overflow")?;
        self.debt = self.debt + (amount - min_amount);
        self.try_clear()?;
        Ok(())
    }

    fn try_clear(&mut self) -> std::result::Result<(), String> {
        if self.balance.is_zero() && self.debt.is_zero() {
            self.token = Address::default();
            self.token_chain_id = 0;
        }
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
            &vec![ParamType::Tuple(vec![
                ParamType::Address,
                ParamType::Uint(64),
                ParamType::Uint(256),
                ParamType::Uint(256),
            ])],
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
                    )))?;
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

#[serde_as]
#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlocksStateData {
    pub block_num: u64,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub root: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub txs: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub profit_root: [u8; 32],
}

pub trait Chain {
    fn into_chain(&mut self, old_block: BlocksStateData);
}

impl Chain for BlocksStateData {
    fn into_chain(&mut self, old_block: BlocksStateData) {
        let mut hasher = Keccak256Hasher::default();
        hasher.write_h256(&H256::from(old_block.root));
        hasher.write_h256(&H256::from(self.txs.clone()));
        hasher.write_h256(&H256::from(self.profit_root));
        self.root = hasher.finish().into();
    }
}

impl TokenizableItem for BlocksStateData {}

impl Tokenizable for BlocksStateData {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType>
    where
        Self: Sized,
    {
        if let Token::Tuple(tuple) = token {
            if tuple.len() == 4 {
                let block_num = tuple[0]
                    .clone()
                    .into_uint()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: block_num"
                    )))?;
                let root = tuple[1]
                    .clone()
                    .into_fixed_bytes()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: root"
                    )))?;
                let txs = tuple[2]
                    .clone()
                    .into_fixed_bytes()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: txs"
                    )))?;
                let profit_root = tuple[3]
                    .clone()
                    .into_fixed_bytes()
                    .ok_or(InvalidOutputType(format!(
                        "BlocksStateData from_token error: profit_root"
                    )))?;

                return Ok(BlocksStateData {
                    block_num: block_num.as_u64(),
                    root: root[..32].to_owned().try_into().unwrap(),
                    txs: txs[..32].to_owned().try_into().unwrap(),
                    profit_root: profit_root[..32].to_owned().try_into().unwrap(),
                });
            }
        }
        return Err(InvalidOutputType(format!(
            "BlocksStateData from_token error: all"
        )));
    }

    fn into_token(self) -> Token {
        let mut tuple = Vec::new();
        tuple.push(Token::Uint(self.block_num.into()));
        tuple.push(Token::FixedBytes(self.root.into()));
        tuple.push(Token::FixedBytes(self.txs.into()));
        tuple.push(Token::FixedBytes(self.profit_root.into()));
        Token::Tuple(tuple)
    }
}

impl AbiDecode for BlocksStateData {
    fn decode(bytes: Vec<u8>) -> std::result::Result<Vec<Token>, Error> {
        decode(
            &vec![ParamType::Tuple(vec![
                ParamType::Uint(64),
                ParamType::FixedBytes(32),
                ParamType::FixedBytes(32),
                ParamType::FixedBytes(32),
            ])],
            &bytes,
        )
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossTxData {
    pub dealer_address: Address,
    pub profit: U256,

    pub source_address: Address,
    pub source_amount: String,
    pub source_chain: u64,
    // tx_hash
    pub source_id: String,
    pub source_maker: Address,
    pub source_symbol: String,
    pub source_time: u64,
    // token id
    pub source_token: Address,

    pub target_address: Address,
    pub target_amount: String,
    pub target_chain: u64,
    // tx_hash
    pub target_id: H256,
    pub target_maker: Option<Address>,
    pub target_symbol: String,
    pub target_time: u64,
    pub target_token: Address,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossTxRawData {
    pub dealer_address: String,

    // token
    pub source_address: Option<String>,
    pub source_amount: Option<String>,
    pub source_chain: String,
    pub source_id: String,
    //
    pub source_maker: String,
    pub source_symbol: Option<String>,
    pub source_time: u64,
    pub source_token: String,
    // maker address
    pub target_address: String,
    pub target_amount: Option<String>,
    pub target_chain: String,
    // tx_hash
    pub target_id: String,
    pub target_maker: Option<String>,
    pub target_symbol: Option<String>,
    pub target_time: u64,
    pub target_token: String,

    pub trade_fee: String,
    pub trade_fee_decimals: u8,
    pub withholding_fee: Option<String>,
    pub withholding_fee_decimals: u8,
}

impl From<CrossTxRawData> for CrossTxData {
    fn from(value: CrossTxRawData) -> Self {
        let target_id_string = value.target_id.clone();
        let target_id: [u8; 32] = hex::decode(&target_id_string[2..66])
            .unwrap()
            .try_into()
            .unwrap();
        CrossTxData {
            dealer_address: Address::from_str(&value.dealer_address).unwrap(),
            profit: U256::from_dec_str(&value.trade_fee).unwrap(),
            source_address: Address::default(), //value.source_address.parse().unwrap(),
            source_amount: if let Some(source_amount) = value.source_amount {
                source_amount
            } else {
                String::from("0")
            },
            source_chain: value.source_chain.parse().unwrap(),
            source_id: value.source_id,
            source_maker: Address::from_str(&value.source_maker).unwrap(),
            source_symbol: if let Some(source_symbol) = value.source_symbol {
                source_symbol
            } else {
                String::from("")
            },
            source_time: value.source_time,
            source_token: Address::from_str(&value.source_token).unwrap(),
            target_address: Address::from_str(&value.target_address).unwrap(),
            target_amount: if let Some(target_amount) = value.target_amount {
                target_amount
            } else {
                String::from("0")
            },
            target_chain: value.target_chain.parse().unwrap(),
            target_id: target_id.into(),
            target_maker: None,
            target_symbol: if let Some(target_symbol) = value.target_symbol {
                target_symbol
            } else {
                String::from("")
            },
            target_time: value.target_time,
            target_token: Address::from_str(&value.target_token).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct CrossTxProfit {
    pub maker_address: Address,
    pub dealer_address: Address,
    pub profit: U256,
    pub chain_id: u64,
    pub token: Address,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum FeeManagerDuration {
    Lock,
    Challenge,
    Withdraw,
}

impl Default for FeeManagerDuration {
    fn default() -> Self {
        Self::Lock
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BlockStorage {
    pub duration: FeeManagerDuration,
    pub last_update_block: u64,
    pub last_submit_timestamp: u64,
    pub block_timestamp: u64,
    pub block_number: u64,
    pub profit_root: [u8; 32],
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

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfitStatistics {
    pub total_profit: U256,
    pub total_withdrawn: U256,
    pub total_deposit: U256,
}
