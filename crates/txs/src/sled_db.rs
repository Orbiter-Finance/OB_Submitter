use super::*;
use bincode;
use ethers::types::Address;
use primitives::{error::Result, types::ProfitStatistics};
use sled::Db;

#[derive(Clone)]
pub struct MakerProfitDB {
    inner: Tree,
}

impl MakerProfitDB {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        Ok(Self {
            inner: db.open_tree("maker-profit-percent")?,
        })
    }

    pub fn insert_percent(
        &self,
        maker: Address,
        block_num: u64,
        token: Address,
        percent: u64,
    ) -> Result<()> {
        let k = bincode::serialize(&(maker, block_num, token))?;
        let v = bincode::serialize(&percent)?;
        self.inner.insert(k, v)?;
        Ok(())
    }

    pub fn get_percent(
        &self,
        maker: Address,
        block_num: u64,
        token: Address,
    ) -> Result<Option<u64>> {
        let k = bincode::serialize(&(maker, block_num, token))?;
        if let Some(v) = self.inner.get(&k)? {
            return Ok(Some(bincode::deserialize::<u64>(&v)?));
        }
        Ok(None)
    }
}

#[derive(Clone)]
pub struct UserTokensDB {
    inner: Tree,
}

impl UserTokensDB {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        Ok(Self {
            inner: db.open_tree("user-tokens")?,
        })
    }

    pub fn insert_token(&self, user: Address, chain_id: u64, token: Address) -> Result<()> {
        if let Some(v) = self.inner.get(user)? {
            let mut res: Vec<(u64, Address)> = bincode::deserialize(&v)?;
            res.retain(|i| i != &(chain_id, token));
            res.push((chain_id, token));
            let r = bincode::serialize(&res)?;
            self.inner.insert(user, r)?;
        } else {
            let r = bincode::serialize(&vec![(chain_id, token)])?;
            self.inner.insert(user, r)?;
        }
        Ok(())
    }

    pub fn get_tokens(&self, user: Address) -> Result<Vec<(u64, Address)>> {
        if let Some(v) = self.inner.get(user)? {
            let res = bincode::deserialize::<Vec<(u64, Address)>>(&v)?;
            return Ok(res);
        }
        return Ok(vec![]);
    }
}

#[derive(Clone)]
pub struct BlockTxsCountDB {
    inner: Tree,
}

impl BlockTxsCountDB {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        Ok(Self {
            inner: db.open_tree("block-txs-count")?,
        })
    }

    pub fn insert_count(&self, block_num: u64, count: u64) -> Result<()> {
        let k = bincode::serialize(&block_num)?;
        let v = bincode::serialize(&count)?;
        self.inner.insert(k, v)?;
        Ok(())
    }

    pub fn get_count(&self, block_num: u64) -> Result<Option<u64>> {
        let k = bincode::serialize(&block_num)?;
        if let Some(v) = self.inner.get(k)? {
            return Ok(Some(bincode::deserialize::<u64>(&v)?));
        }
        Ok(None)
    }

    pub fn is_txs_completed(&self, start_block: u64, end_block: u64) -> Result<bool> {
        let mut is_completed = true;
        for i in start_block..end_block {
            let k = bincode::serialize(&i)?;
            if self.inner.get(k).unwrap().is_none() {
                is_completed = false;
                event!(
                    Level::INFO,
                    "Block #{}, txs are not quite ready yet, pending......",
                    i
                );
                break;
            }
        }
        Ok(is_completed)
    }
}

#[derive(Clone)]
pub struct ContractBlockInfoDB {
    inner: Tree,
}

impl ContractBlockInfoDB {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        Ok(Self {
            inner: db.open_tree("contract-block-info")?,
        })
    }

    pub fn insert_block_info(&self, block_number: u64, info: BlockInfo) -> Result<()> {
        let k = bincode::serialize(&block_number)?;
        let v = bincode::serialize(&info)?;
        self.inner.insert(k, v)?;
        Ok(())
    }

    pub fn get_block_info(&self, block_number: u64) -> Result<Option<BlockInfo>> {
        let k = bincode::serialize(&block_number)?;
        if let Some(v) = self.inner.get(k)? {
            return Ok(Some(bincode::deserialize::<BlockInfo>(&v)?));
        }
        Ok(None)
    }

    pub fn get_block_num_by_timestamp(
        &self,
        timestamp: u64,
        newest_block_num: u64,
    ) -> Result<Option<u64>> {
        let mut num = newest_block_num;
        if self.inner.is_empty() {
            return Ok(None);
        }
        while num > 0 {
            let k = bincode::serialize(&num)?;
            if let Some(b) = self.inner.get(k)? {
                let block_info = bincode::deserialize::<BlockInfo>(&b)?;
                if block_info.storage.block_timestamp <= timestamp {
                    return Ok(Some(block_info.storage.block_number));
                }
            }
            num -= 1;
        }
        Ok(None)
    }
}

#[derive(Clone)]
pub struct ProfitStatisticsDB {
    inner: Tree,
}

impl ProfitStatisticsDB {
    pub fn new(db: Arc<Db>) -> Result<Self> {
        Ok(Self {
            inner: db.open_tree("profit-statistics")?,
        })
    }

    pub fn update_total_profit(
        &self,
        user: Address,
        chain_id: u64,
        token: Address,
        amount: U256,
    ) -> Result<()> {
        let k = bincode::serialize(&(user, chain_id, token))?;
        if let Some(v) = self.inner.get(k.clone())? {
            let mut profit_statistics = bincode::deserialize::<ProfitStatistics>(&v)?;
            profit_statistics.total_profit += amount;
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        } else {
            let profit_statistics = ProfitStatistics {
                total_profit: amount,
                total_withdrawn: U256::zero(),
                total_deposit: U256::zero(),
            };
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        }

        Ok(())
    }

    pub fn update_total_withdraw(
        &self,
        user: Address,
        chain_id: u64,
        token: Address,
        amount: U256,
    ) -> Result<()> {
        let k = bincode::serialize(&(user, chain_id, token))?;
        if let Some(v) = self.inner.get(k.clone())? {
            let mut profit_statistics = bincode::deserialize::<ProfitStatistics>(&v)?;
            profit_statistics.total_withdrawn += amount;
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        } else {
            let profit_statistics = ProfitStatistics {
                total_profit: U256::zero(),
                total_withdrawn: amount,
                total_deposit: U256::zero(),
            };
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        }

        Ok(())
    }

    pub fn update_total_deposit(
        &self,
        user: Address,
        chain_id: u64,
        token: Address,
        amount: U256,
    ) -> Result<()> {
        let k = bincode::serialize(&(user, chain_id, token))?;
        if let Some(v) = self.inner.get(k.clone())? {
            let mut profit_statistics = bincode::deserialize::<ProfitStatistics>(&v)?;
            profit_statistics.total_deposit += amount;
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        } else {
            let profit_statistics = ProfitStatistics {
                total_profit: U256::zero(),
                total_withdrawn: U256::zero(),
                total_deposit: amount,
            };
            let r = bincode::serialize(&profit_statistics)?;
            self.inner.insert(k, r)?;
        }

        Ok(())
    }

    pub fn get_profit_statistics(
        &self,
        user: Address,
        chain_id: u64,
        token: Address,
    ) -> Result<Option<ProfitStatistics>> {
        let k = bincode::serialize(&(user, chain_id, token))?;
        if let Some(v) = self.inner.get(k)? {
            return Ok(Some(bincode::deserialize::<ProfitStatistics>(&v)?));
        }
        Ok(None)
    }
}
