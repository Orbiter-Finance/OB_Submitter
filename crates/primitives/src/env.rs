use crate::types::ChainType;
use ethers::types::Address;

pub fn get_network_https_url() -> String {
    std::env::var("NETWORK_RPC_URL").unwrap()
}

pub fn get_mainnet_chain_id() -> u64 {
    std::env::var("MAINNET_CHAIN_ID").unwrap().parse().unwrap()
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
