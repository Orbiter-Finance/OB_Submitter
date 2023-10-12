use crate::types::ChainType;
use ethers::types::Address;

pub fn get_mainnet_rpc_urls() -> Vec<String> {
    let mainnet_rpc_urls = std::env::var("MAINNET_RPC_URLS").unwrap();
    mainnet_rpc_urls
        .split(";")
        .collect::<Vec<&str>>()
        .iter()
        .map(|f| f.trim().to_string())
        .collect()
}

pub fn get_mainnet_chain_id() -> u64 {
    std::env::var("MAINNET_CHAIN_ID").unwrap().parse().unwrap()
}

pub fn get_block_infos_batch() -> u64 {
    std::env::var("BLOCK_INFOS_BATCH")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(10)
}

pub fn get_start_block() -> u64 {
    std::env::var("START_BLOCK").unwrap().parse().unwrap()
}

pub fn get_dealer_withdraw_delay() -> u64 {
    std::env::var("DEALER_WITHDRAW_DELAY")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(3600)
}

pub fn get_withdraw_duration() -> u64 {
    std::env::var("WITHDRAW_DURATION")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(3360)
}

pub fn get_lock_duration() -> u64 {
    std::env::var("LOCK_DURATION")
        .unwrap_or("".to_string())
        .parse()
        .unwrap_or(240)
}

pub fn get_fee_manager_contract_address() -> Address {
    std::env::var("ORFeeManager_CONTRACT_ADDRESS")
        .unwrap()
        .parse()
        .unwrap()
}

pub fn get_txs_source_url() -> String {
    std::env::var("TXS_SOURCE_URL").expect("TXS_SOURCE_URL is not set")
}

pub fn get_chains_info_source_url() -> String {
    std::env::var("SUPPORT_CHAINS_SOURCE_URL").expect("SUPPORT_CHAINS_SOURCE_URL is not set")
}

pub fn get_delay_seconds_by_chain_type(t: ChainType) -> u64 {
    match t {
        ChainType::Normal => {
            return std::env::var("COMMON_DELAY_SECONDS")
                .unwrap()
                .parse()
                .unwrap()
        }
        ChainType::OP => {
            return std::env::var("COMMON_DELAY_SECONDS")
                .unwrap()
                .parse::<u64>()
                .unwrap()
                + std::env::var("OP_DELAY_SECONDS")
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
        }
        ChainType::ZK => {
            return std::env::var("COMMON_DELAY_SECONDS")
                .unwrap()
                .parse::<u64>()
                .unwrap()
                + std::env::var("ZK_DELAY_SECONDS")
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
        }
    }
}
