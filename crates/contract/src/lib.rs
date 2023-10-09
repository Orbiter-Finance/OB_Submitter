mod tests;

use async_trait::async_trait;
use ethers::abi::{decode, ParamType, Tokenizable};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::prelude::Wallet;
use ethers::prelude::{FunctionCall, Multicall};
use ethers::providers::Http;
use ethers::utils::keccak256;
use ethers::{
    contract::{abigen, Contract, EthEvent},
    middleware::{Middleware, SignerMiddleware},
    prelude::LocalWallet,
    providers::Provider,
    types::{Address, Filter, TransactionReceipt, H160, H256, U256},
};
use primitives::{
    env::{get_fee_manager_contract_address, get_mainnet_chain_id, get_network_https_urls},
    error::{Error as LocalError, Result},
    traits::Contract as ContractTrait,
    types::{BlockInfo, BlockStorage, DepositEvent, Event, FeeManagerDuration, WithdrawEvent},
};

use ethers::types::U64;
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
            Provider::<ethers::providers::Http>::try_from(get_network_https_urls()[0].clone())
                .unwrap();

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
                            tr.block_number,
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
        let provider =
            Provider::<ethers::providers::Http>::try_from(get_network_https_urls()[0].clone())
                .unwrap();

        let fee_manager_contract = FeeManagerContract::new(
            get_fee_manager_contract_address(),
            Arc::new(provider.clone()),
        );

        let duration_check: FunctionCall<
            Arc<ethers_providers::Provider<_>>,
            ethers_providers::Provider<_>,
            u8,
        > = fee_manager_contract.duration_check().block(block_number);
        let submissions: FunctionCall<
            Arc<ethers_providers::Provider<_>>,
            ethers_providers::Provider<_>,
            (u64, u64, u64, [u8; 32], [u8; 32]),
        > = fee_manager_contract.submissions().block(block_number);

        let mut multicall = Multicall::new(provider.clone(), None)
            .await?
            .block(block_number);
        multicall
            .clear_calls()
            .add_get_current_block_timestamp()
            .add_call(duration_check, false)
            .add_call(submissions, false);
        let (timestamp, duration, (start_block, end_block, submit_timestamp, profit_root, _)): (
            u64,
            u8,
            (u64, u64, u64, [u8; 32], [u8; 32]),
        ) = multicall.call().await?;

        let duration = match duration {
            0 => FeeManagerDuration::Lock,
            1 => FeeManagerDuration::Challenge,
            _ => FeeManagerDuration::Withdraw,
        };
        let block_storage = Some(BlockStorage {
            duration,
            last_start_block: start_block,
            last_update_block: end_block,
            last_submit_timestamp: submit_timestamp,
            block_timestamp: timestamp,
            block_number,
            profit_root,
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
            // Ignore ETH
            if token.is_zero() {
                continue;
            }

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
                        block_number,
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

    async fn get_feemanager_contract_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<Event>> {
        let deposit_id = H256::from(keccak256("ETHDeposit(address,uint256)".as_bytes()));
        let withdraw_id = H256::from(keccak256(
            "Withdraw(address,uint64,address,uint256,uint256)".as_bytes(),
        ));

        let filter = Filter::new()
            .address(get_fee_manager_contract_address())
            .topic0(vec![deposit_id, withdraw_id])
            .from_block(from_block)
            .to_block(to_block);
        let logs = self.client.get_logs(&filter).await.unwrap_or(vec![]);

        let mut events: Vec<Event> = vec![];
        for log in logs {
            let log_block = log.block_number.unwrap().as_u64();

            // ETHDeposit event
            if log.topics[0] == deposit_id {
                event!(
                    Level::INFO,
                    "Block #{:?} fee-manager contract {:?} deposit event: {:?}",
                    log_block,
                    get_fee_manager_contract_address(),
                    log.clone(),
                );

                let ts = decode(&vec![ParamType::Uint(256)], &log.data).unwrap();
                let user = H160::from(log.topics[1]);
                let amount = U256::from_token(ts[0].clone()).unwrap();
                events.push(Event::Deposit(DepositEvent {
                    block_number: log_block,
                    address: user,
                    chain_id: get_mainnet_chain_id(),
                    token_address: Default::default(),
                    balance: amount,
                }));
            }

            // Withdraw event
            if log.topics[0] == withdraw_id {
                event!(
                    Level::INFO,
                    "Block #{:?} fee-manager contract {:?} withdraw event: {:?}",
                    log_block,
                    get_fee_manager_contract_address(),
                    log.clone(),
                );

                let ts = decode(
                    &vec![
                        ParamType::Uint(64),
                        ParamType::Address,
                        ParamType::Uint(256),
                        ParamType::Uint(256),
                    ],
                    &log.data,
                )
                .unwrap();

                let user = H160::from(log.topics[1]);
                let chain_id = U256::from_token(ts[0].clone()).unwrap().as_u64();
                let token = H160::from_token(ts[1].clone()).unwrap();
                let amount = U256::from_token(ts[3].clone()).unwrap();
                events.push(Event::Withdraw(WithdrawEvent {
                    block_number: from_block,
                    address: user,
                    chain_id,
                    token_address: token,
                    balance: amount,
                }));
            }
        }

        Ok(events)
    }

    async fn get_block_infos(&self, from_block: u64, to_block: u64) -> Result<Vec<BlockInfo>> {
        let mut handles = vec![];
        for block_number in from_block..to_block + 1 {
            let _self = self.clone();
            handles.push(tokio::spawn(async move {
                let _op = _self.clone().get_block_storage(block_number).await;
                if let Err(err) = _op {
                    event!(
                        Level::WARN,
                        "Block #{:?} get_block_info failed: {:?}",
                        block_number,
                        err,
                    );

                    // Waiting some time
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    return None;
                }

                _op.unwrap()
            }));
        }
        let mut storages = vec![];
        for hd in handles {
            match hd.await {
                Ok(Some(bs)) => {
                    storages.push(bs);
                }
                Ok(None) => {
                    return Ok(vec![]);
                }
                Err(_) => return Ok(vec![]),
            }
        }

        let events = self
            .get_feemanager_contract_events(from_block, to_block)
            .await
            .unwrap();

        // TODO: Currently only supports eth, and will be optimized later.
        // let erc_transfer_events = self
        //     .get_erc20_transfer_events_by_tokens_id(
        //         self.support_mainnet_tokens.as_ref().clone(),
        //         from_block,
        //     )
        //     .await?;
        // events.extend(erc_transfer_events);

        let mut block_infos = vec![];
        for bs in storages {
            let mut _events: Vec<Event> = vec![];
            for e in events.clone() {
                let b_n = {
                    match e.clone() {
                        Event::Withdraw(w_e) => w_e.block_number,
                        Event::Deposit(d_e) => d_e.block_number,
                    }
                };

                if b_n == bs.block_number {
                    _events.push(e);
                }
            }

            let b = BlockInfo {
                storage: bs.clone(),
                events: _events,
            };

            block_infos.push(b.clone());

            event!(
                Level::INFO,
                "Block #{:?} info: {:?}",
                bs.block_number,
                serde_json::to_string(&b.clone()).unwrap(),
            );
        }

        Ok(block_infos)
    }

    async fn get_dealer_profit_percent_by_block(
        &self,
        dealer: Address,
        block_number: u64,
        _token_chian_id: u64,
        _token_id: Address,
    ) -> Result<u64> {
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
