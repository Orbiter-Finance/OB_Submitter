#![cfg(test)]
use super::*;
use crate::{
    erc20_contract::{ERC20Contract, TransferFilter},
    fee_manager_contract::{FeeManagerContract, WithdrawFilter},
};
use ethers::{
    middleware::SignerMiddleware,
    prelude::LocalWallet,
    providers::Provider,
    types::{Address, H160},
};
use std::{str::FromStr, sync::Arc};
use tokio;
use tracing::{event, Level};

// abigen!(
// //     ERC20Contract,
//     r#"[
//         event Transfer(address indexed from, address indexed to, uint256 value)
//         event Approval(address indexed owner, address indexed spender, uint256 value)
//     ]"#,
// );

// #[tokio::test]
// async fn test() {
//     let wallet: LocalWallet = "0xed0e10acdb4b9ad17a0d9ec1b6f92d9e70d9f9c0bbfc609eb1aa03a370aba488"
//         .parse::<LocalWallet>()
//         .unwrap();
//
//     let provider = Provider::<ethers::providers::Http>::try_from(
//         "https://eth-goerli.api.onfinality.io/public",
//     )
//     .unwrap();
//
//     let client = SignerMiddleware::new_with_provider_chain(provider.clone(), wallet.clone())
//         .await
//         .unwrap();
//
//     let a = Address::from_str("0x11612633Db3b966314E7B9DfB2D05289eC5b1B5E").unwrap();
//     // 9651820
//     let block_number = 9651820u64;
//     let entry_point = ERC20Contract::new(a, Arc::new(client.clone()));
//     let logs: Vec<TransferFilter> = entry_point
//         .transfer_filter()
//         .from_block(block_number)
//         .to_block(block_number)
//         .query()
//         .await
//         .unwrap();
//     println!("logs111: {:?}", logs);
//     // event!(
//     //     Level::INFO,
//     //     "hahahahha"
//     // );
// }

#[tokio::test]
async fn local_test() {
    let wallet: LocalWallet = "0xed0e10acdb4b9ad17a0d9ec1b6f92d9e70d9f9c0bbfc609eb1aa03a370aba488"
        .parse::<LocalWallet>()
        .unwrap();

    let provider = Provider::<ethers::providers::Http>::try_from(
        "https://eth-goerli.api.onfinality.io/public",
    )
        .unwrap();

    let client = SignerMiddleware::new_with_provider_chain(provider.clone(), wallet.clone())
        .await
        .unwrap();

    let a = Address::from_str("0xA191028bf304209a14acA85866999d8140BA54d8").unwrap();
    // 9651820
    let block_number = 9651820u64;
    let (s, r) = tokio::sync::broadcast::channel::<BlockInfo>(100);
    let entry_point = FeeManagerContract::new(a, Arc::new(client.clone()));
    let start_num = Arc::new(RwLock::new(064));
    let tokens: Arc<Vec<Address>> = Arc::new(vec![Address::from_str("0xa3a8a6b323e3d38f5284db9337e7c6d74af3366a").unwrap(), Address::from_str("0xa0321efeb50c46c17a7d72a52024eea7221b215a").unwrap(), Address::from_str("0x29b6a77911c1ce3b3849f28721c65dada015c768").unwrap()]);
    let contract = SubmitterContract::new(s.clone(), wallet.clone(), start_num, tokens).await;
    // 9734015
    // 9738031
    let block_info = contract.get_block_info(9734015).await;
    match block_info {
        Ok(b) => {
            println!("block_info: {:?}", b);
        },
        Err(e) => {
            println!("error: {:?}", e);
        }
    }

}

