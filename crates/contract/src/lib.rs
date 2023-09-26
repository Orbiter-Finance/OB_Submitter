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
use ethers::types::U64;
use tokio::sync::{broadcast::Sender, RwLock};
use tracing::{event, span, Level};

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
    // let span = span!(Level::INFO, "run");
    // let _enter = span.enter();
    event!(Level::INFO, "latest block crawler is ready.",);
    let mut block_num = 0;
    loop {
        if let Ok(block) = contract.provider.get_block_number().await {
            let mut w = contract.now_block_num.write().await;
            if block.as_u64() == block_num {
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
            block_num = block.as_u64();

            if *w != block_num {
                if let Ok(Some(storage)) = contract.get_block_storage(block_num).await {
                    let b = BlockInfo {
                        storage,
                        events: vec![],
                    };
                    match contract.sender.send(b.clone()) {
                        Ok(_) => {
                            event!(
                                Level::INFO,
                                "send newest block #{:?} info:{:?} success.",
                                block_num,
                                serde_json::to_string(&b.clone()).unwrap(),
                            );
                            *w = block_num;
                        }
                        Err(e) => {
                            event!(
                                Level::WARN,
                                "send newest block #{:?} info fail. error: {:?}",
                                block_num,
                                e
                            );
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
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
    ) -> Result<(H256, Option<U64>)> {
        // let span = span!(Level::INFO, "submit_root");
        // let _enter = span.enter();
        event!(
            Level::INFO,
            "submit root to contract: {:?}",
            get_fee_manager_contract_address(),
        );
        let fee_manager_contract = FeeManagerContract::new(
            get_fee_manager_contract_address(),
            Arc::new(self.client.clone()),
        );

        let res: Option<TransactionReceipt> = fee_manager_contract
            .submit(start, end, profit_root, blocks_root)
            .gas(2000000)
            .send()
            .await?
            .await?;

        match res {
            None => {
                return Err(LocalError::SubmitRootFailed(
                    "transaction receipt is none".to_string(),
                    None,
                ));
            }
            Some(tr) => {
                if let Some(status) = tr.status {
                    if status == 0.into() {
                        return Err(LocalError::SubmitRootFailed(
                            "transaction receipt status is 0".to_string(),
                            tr.block_number
                        ));
                    } else {
                        return Ok((tr.transaction_hash, tr.block_number));
                    }
                } else {
                    return Err(LocalError::SubmitRootFailed(
                        "transaction receipt is none".to_string(),
                        tr.block_number,
                    ));
                }
            }
        }
    }

    async fn get_block_storage(&self, block_number: u64) -> Result<Option<BlockStorage>> {
        // let span = span!(Level::INFO, "get_block_storage");
        // let _enter = span.enter();
        let fee_manager_contract = FeeManagerContract::new(
            get_fee_manager_contract_address(),
            Arc::new(self.client.clone()),
        );
        let mut block_storage: Option<BlockStorage> = None;

        let duration_check: FunctionCall<
            Arc<SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>>,
            SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>,
            u8,
        > = fee_manager_contract.duration_check().block(block_number);
        let duration = duration_check.clone().await?;

        let submissions: FunctionCall<
            Arc<SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>>,
            SignerMiddleware<ethers_providers::Provider<Http>, Wallet<SigningKey>>,
            (u64, u64, u64, [u8; 32], [u8; 32]),
        > = fee_manager_contract.submissions().block(block_number);
        let (startBlock, endBlock, submitTimestamp, profitRoot, _) =
            submissions.clone().block(block_number).await?;

        let mut multicall = Multicall::new(self.client.clone(), None)
            .await?
            .block(block_number);
        multicall
            .clear_calls()
            .add_get_current_block_timestamp()
            .add_call(duration_check, false)
            .add_call(submissions, false);
        let (timestamp, duration, (startBlock, endBlock, submitTimestamp, profitRoot, _)): (
            u64,
            u8,
            (u64, u64, u64, [u8; 32], [u8; 32]),
        ) = multicall.call().await?;

        let duration = match duration {
            0 => FeeManagerDuration::Lock,
            1 => FeeManagerDuration::Challenge,
            _ => FeeManagerDuration::Withdraw,
        };
        block_storage = Some(BlockStorage {
            duration,
            last_start_block: startBlock,
            last_update_block: endBlock,
            last_submit_timestamp: submitTimestamp,
            block_timestamp: timestamp,
            block_number,
            profit_root: profitRoot,
        });
        event!(
            Level::INFO,
            "Block #{:?} storage: {:?}",
            block_number,
            serde_json::to_string(&block_storage).unwrap(),
        );
        Ok(block_storage)
    }

    async fn get_erc20_transfer_events_by_tokens_id(
        &self,
        tokens: Vec<H160>,
        block_number: u64,
    ) -> Result<Vec<Event>> {
        // let span = span!(Level::INFO, "get_erc20_transfer_events_by_tokens_id");
        // let _enter = span.enter();
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
                    let e = Event::Deposit(DepositEvent {
                        address: user,
                        chain_id: get_mainnet_chain_id(),
                        token_address: token,
                        balance: amount,
                    });
                    transfer_los.push(e);
                    event!(
                        Level::INFO,
                        "Block #{:?} erc20 contract address: {:?}, transfer event: {:?}",
                        block_number,
                        token,
                        i
                    );
                }
            }
        }

        Ok(transfer_los)
    }

    async fn get_feemanager_contract_events(&self, block_number: u64) -> Result<Vec<Event>> {
        // let span = span!(Level::INFO, "get_feemanager_contract_events");
        // let _enter = span.enter();
        let fee_manager_contract = FeeManagerContract::new(
            get_fee_manager_contract_address(),
            Arc::new(self.client.clone()),
        );
        let withdraw_logs: Vec<WithdrawFilter> = fee_manager_contract
            .withdraw_filter()
            .from_block(block_number)
            .to_block(block_number)
            .query()
            .await?;
        let deposit_logs: Vec<EthdepositFilter> = fee_manager_contract
            .eth_deposit_filter()
            .from_block(block_number)
            .to_block(block_number)
            .query()
            .await?;
        let mut a: Vec<Event> = vec![];
        for i in withdraw_logs {
            event!(
                Level::INFO,
                "Block #{:?} fee-manager contract {:?} withdraw event: {:?}",
                block_number,
                get_fee_manager_contract_address(),
                i.clone(),
            );
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
            event!(
                Level::INFO,
                "Block #{:?} fee-manager contract {:?} deposit event: {:?}",
                block_number,
                get_fee_manager_contract_address(),
                i.clone(),
            );
            let user = i.sender;
            let amount = i.amount;

            a.push(Event::Deposit(DepositEvent {
                address: user,
                chain_id: get_mainnet_chain_id(),
                token_address: Default::default(),
                balance: amount,
            }));
        }

        Ok(a)
    }

    async fn get_block_info(&self, block_number: u64) -> Result<Option<BlockInfo>> {
        // let span = span!(Level::INFO, "get_block_info");
        // let _enter = span.enter();
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
        let b = BlockInfo {
            storage: storage.unwrap(),
            events,
        };
        event!(
            Level::INFO,
            "Block #{:?} info: {:?}",
            block_number,
            serde_json::to_string(&b.clone()).unwrap(),
        );
        Ok(Some(b))
    }

    async fn get_dealer_profit_percent_by_block(
        &self,
        dealer: Address,
        block_number: u64,
        _token_chian_id: u64,
        _token_id: Address,
    ) -> Result<u64> {
        // let span = span!(Level::INFO, "get_dealer_profit_percent_by_block");
        // let _enter = span.enter();
        let fee_manager_contract_address: H160 = get_fee_manager_contract_address();
        let fee_manager_contract =
            FeeManagerContract::new(fee_manager_contract_address, Arc::new(self.client.clone()));
        let info = fee_manager_contract
            .get_dealer_info(dealer)
            .block(block_number)
            .await?;
        let r = info.fee_ratio.as_u64();
        event!(
            Level::INFO,
            "Block #{:?} dealer: {:?}, profit percent: {:?}",
            block_number,
            dealer,
            r,
        );
        Ok(r)
    }
}
