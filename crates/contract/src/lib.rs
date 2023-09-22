mod tests;

use crate::fee_manager_contract::WithdrawFilter;

use async_trait::async_trait;
use ethers::core::k256::{self, ecdsa::SigningKey, Secp256k1};
use ethers::prelude::Wallet;
use ethers::prelude::{FunctionCall, Multicall};
use ethers::providers::Http;
use ethers::{
    contract::{abigen, Contract, EthEvent},
    middleware::{Middleware, SignerMiddleware},
    prelude::LocalWallet,
    providers::Provider,
    types::{Address, Filter, TransactionReceipt, H160, H256, U256},
};
use ethers_providers::StreamExt;
use primitives::{
    env::{get_fee_manager_contract_address, get_mainnet_chain_id, get_network_https_url},
    error::{Error as LocalError, Result},
    traits::Contract as ContractTrait,
    types::{BlockInfo, BlockStorage, DepositEvent, Event, FeeManagerDuration, WithdrawEvent},
};

use std::{option::Option, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{broadcast::Sender, RwLock};
use tracing::{event, Level};

abigen!(
    FeeManagerContract,
    "./crates/contract/json/ORFeeManager.json",
    derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    ERC20Contract,
    "./crates/contract/json/ERC20.json",
    derives(serde::Deserialize, serde::Serialize)
);

#[ethevent(name = "Transfer")]
#[derive(Debug, Clone, PartialEq, ethers::contract::EthEvent)]
pub struct Transfer {
    #[ethevent(indexed)]
    from: Address,
    #[ethevent(indexed)]
    to: Address,
    value: U256,
}

// #[ethevent(name = "Withdraw")]
// #[derive(Debug, Clone, PartialEq, ethers::contract::EthEvent)]
// pub struct Withdraw {
//     #[ethevent(indexed)]
//     user: Address,
//     chain_id: U256,
//     #[ethevent(indexed)]
//     token: Address,
//     amount: U256,
// }

#[derive(Debug, Clone)]
pub struct SubmitterContract {
    pub sender: Sender<BlockInfo>,
    pub provider: Provider<ethers_providers::Http>,
    pub client: SignerMiddleware<Provider<ethers_providers::Http>, LocalWallet>,
    pub support_mainnet_tokens: Arc<Vec<Address>>,
    pub now_block_num: Arc<RwLock<u64>>,
}

impl SubmitterContract {
    pub async fn new(
        sender: Sender<BlockInfo>,
        wallet: LocalWallet,
        now_block_num: Arc<RwLock<u64>>,
        support_mainnet_tokens: Arc<Vec<Address>>,
    ) -> Self {
        let provider =
            Provider::<ethers::providers::Http>::try_from(get_network_https_url()).unwrap();

        let client: SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>> =
            SignerMiddleware::new_with_provider_chain(provider.clone(), wallet.clone())
                .await
                .unwrap();
        event!(
            Level::INFO,
            "Successfully connected to the ethereum network. Support mainnet tokens: {:?}",
            support_mainnet_tokens.as_ref().clone(),
        );
        Self {
            sender,
            provider,
            client,
            now_block_num,
            support_mainnet_tokens,
        }
    }
}

pub async fn run(contract: Arc<SubmitterContract>) -> Result<()> {
    event!(Level::INFO, "latest  block crawler is ready.",);
    loop {
        if let Ok(block) = contract.provider.get_block_number().await {
            let mut w = contract.now_block_num.write().await;
            let block_num = block.as_u64();
            if *w != block_num {
                if let Ok(Some(storage)) = contract.get_block_storage(block_num).await {
                    let b = BlockInfo {
                        storage,
                        events: vec![],
                    };
                    if contract.sender.send(b).is_ok() {
                        event!(
                            Level::INFO,
                            "send newest block {:?} info success.",
                            block_num
                        );
                        *w = block_num;
                    } else {
                        event!(Level::WARN, "send newest block {:?} info fail.", block_num);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
    Ok(())
}

#[async_trait]
impl ContractTrait for SubmitterContract {
    async fn submit_root(
        &self,
        start: u64,
        end: u64,
        profit_root: [u8; 32],
        blocks_root: [u8; 32],
    ) -> Result<H256> {
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let fee_manager_contract =
            FeeManagerContract::new(fee_manager_contract_address, Arc::new(self.client.clone()));

        let s: FunctionCall<
            Arc<SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>>,
            SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>,
            (),
        > = fee_manager_contract
            .submit(start, end, profit_root, blocks_root)
            .gas(2000000);
        let res: Option<TransactionReceipt> = s.send().await?.await?;

        match res {
            None => {
                event!(Level::INFO, "submit root fail.");
                return Err(LocalError::SubmitRootFailed(
                    "transaction receipt is none".to_string(),
                ));
            }
            Some(tr) => {
                if let Some(status) = tr.status {
                    if status == 0.into() {
                        event!(Level::INFO, "submit root fail.");
                        return Err(LocalError::SubmitRootFailed(
                            "transaction receipt status is 0".to_string(),
                        ));
                    } else {
                        event!(
                            Level::INFO,
                            "submit root success. tx hash: {:?}",
                            tr.transaction_hash
                        );
                        return Ok(tr.transaction_hash);
                    }
                } else {
                    event!(Level::INFO, "submit root fail.");
                    return Err(LocalError::SubmitRootFailed(
                        "transaction receipt is none".to_string(),
                    ));
                }
            }
        }
    }

    async fn get_block_storage(&self, block_number: u64) -> Result<Option<BlockStorage>> {
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let fee_manager_contract =
            FeeManagerContract::new(fee_manager_contract_address, Arc::new(self.client.clone()));
        let mut block_storage: Option<BlockStorage> = None;

        if let Ok(block_info) = self.provider.get_block(block_number).await {
            match block_info {
                None => {}
                Some(b) => {
                    let duration_check: FunctionCall<
                        Arc<SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>>,
                        SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>,
                        u8,
                    > = fee_manager_contract.duration_check().block(block_number);
                    let duration = duration_check.await?;

                    // fixme
                    // let first_call = fee_manager_contract.method::<_, String>("getValue", ()).unwrap();
                    // let mut multicall = Multicall::new(self.client.clone(), None).await.unwrap();
                    // multicall.add_call(first_call, false);
                    // let results = multicall.call().await.unwrap();
                    // let tx_receipt = multicall.send().await?.await.expect("tx dropped");
                    // multicall
                    //     .clear_calls()
                    //     .add_get_eth_balance(address_1, false)
                    //     .add_get_eth_balance(address_2, false);
                    // let balances: (U256, U256) = multicall.call().await?;
                    // let s = fee_manager_contract.submissions();

                    let (_, endBlock, submitTimestamp, profitRoot, _) = fee_manager_contract
                        .submissions()
                        .block(block_number)
                        .await?;
                    let duration = match duration {
                        0 => FeeManagerDuration::Lock,
                        1 => FeeManagerDuration::Challenge,
                        _ => FeeManagerDuration::Withdraw,
                    };
                    block_storage = Some(BlockStorage {
                        duration,
                        last_update_block: endBlock,
                        last_submit_timestamp: submitTimestamp,
                        block_timestamp: b.timestamp.as_u64(),
                        block_number,
                        profit_root: profitRoot,
                    });
                }
            }
        }
        Ok(block_storage)
    }

    async fn get_erc20_transfer_events_by_tokens_id(
        &self,
        tokens: Vec<H160>,
        block_number: u64,
    ) -> Result<Vec<Event>> {
        if tokens.is_empty() {
            return Ok(vec![]);
        }
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let mut transfer_los: Vec<Event> = vec![];
        use ethers::abi::Abi;
        for token in tokens {
            let contract = Contract::new(token, Abi::default(), Arc::new(self.provider.clone()));
            let _event = contract.event::<Transfer>();
            let f = Filter::new()
                .select(block_number)
                .event(&Transfer::name())
                .topic0(
                    H256::from_str(
                        "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    )
                    .unwrap(),
                )
                .topic2(fee_manager_contract_address);
            let logs: Vec<Transfer> = contract.event_with_filter(f).query().await?;
            for i in logs {
                if i.to == fee_manager_contract_address {
                    let user = i.from;
                    let token = token;
                    let amount = i.value;
                    transfer_los.push(Event::Deposit(DepositEvent {
                        address: user,
                        chain_id: get_mainnet_chain_id(),
                        token_address: token,
                        balance: amount,
                    }));
                    event!(Level::INFO, "Block #{:?} erc20 log: {:?}", block_number, i);
                }
            }
        }

        Ok(transfer_los)
    }

    async fn get_feemanager_contract_events(&self, block_number: u64) -> Result<Vec<Event>> {
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let fee_manager_contract =
            FeeManagerContract::new(fee_manager_contract_address, Arc::new(self.client.clone()));
        let withdraw_logs: Vec<WithdrawFilter> = fee_manager_contract
            .withdraw_filter()
            .from_block(block_number)
            .to_block(block_number)
            .query()
            .await?;
        // fixme
        let deposit_logs: Vec<EthdepositFilter> = fee_manager_contract
            .eth_deposit_filter()
            .from_block(block_number)
            .to_block(block_number)
            .query()
            .await?;
        let mut a: Vec<Event> = vec![];
        for i in withdraw_logs {
            let user = i.user;
            let chain_id = i.chain_id;
            let token = i.token;
            let amount = i.amount;
            a.push(Event::Withdraw(WithdrawEvent {
                address: user,
                chain_id: chain_id,
                token_address: token,
                balance: amount,
            }));
        }

        for i in deposit_logs {
            let user = i.sender;
            let amount = i.amount;

            a.push(Event::Deposit(DepositEvent {
                address: user,
                chain_id: get_mainnet_chain_id(),
                token_address: Default::default(),
                balance: amount,
            }));
        }
        if !a.is_empty() {
            event!(
                Level::INFO,
                "Block #{:?} logs: {:?}",
                block_number,
                a.clone()
            );
        }

        Ok(a)
    }

    async fn get_block_info(&self, block_number: u64) -> Result<Option<BlockInfo>> {
        let storage = self.get_block_storage(block_number).await?;
        if storage.is_none() {
            return Ok(None);
        }
        let mut events: Vec<Event> = self.get_feemanager_contract_events(block_number).await?;
        let erc_transfer_events = self
            .get_erc20_transfer_events_by_tokens_id(
                self.support_mainnet_tokens.as_ref().clone(),
                block_number,
            )
            .await?;
        events.extend(erc_transfer_events);
        for i in events.clone() {
            println!("get event: {:?}", i);
        }
        let b = BlockInfo {
            storage: storage.unwrap(),
            events,
        };
        Ok(Some(b))
    }

    async fn get_maker_profit_percent_by_block(
        &self,
        maker: Address,
        block_number: u64,
        _token_chian_id: u64,
        _token_id: Address,
    ) -> Result<u64> {
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let fee_manager_contract =
            FeeManagerContract::new(fee_manager_contract_address, Arc::new(self.client.clone()));
        let info = fee_manager_contract
            .get_dealer_info(maker)
            .block(block_number)
            .await?;
        Ok(info.fee_ratio.as_u64())
    }
}
