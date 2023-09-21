use super::*;
use core::cmp::Ordering;
use ethers::types::Address;
use primitives::{error::Result, types::CrossTxData};
use rocksdb::Direction;
use std::fmt::format;

pub struct TxsRocksDB {
    inner: DB,
}

impl TxsRocksDB {
    pub fn new(db_path: String) -> anyhow::Result<Self> {
        let path = format!("{}/txs", db_path);
        let db = open_rocksdb(path, tx_compare)?;
        Ok(Self { inner: db })
    }

    pub fn insert_txs(&self, txs: Vec<(CrossTxData, CrossTxProfit)>) -> Result<()> {
        let mut batch = WriteBatch::default();
        for tx in txs {
            let key = bincode::serialize(&tx.0)?;
            let value = bincode::serialize(&tx.1)?;
            batch.put(key, value)?;
        }
        self.inner.write(&batch)?;
        self.inner.flush()?;
        Ok(())
    }

    pub fn get_profit_by_yx_hash(&self, tx_hash: H256) -> Result<Option<CrossTxProfit>> {
        let lower_bound = CrossTxData {
            target_time: 0u64,
            target_chain: 0u64,
            target_id: tx_hash,
            ..Default::default()
        };
        let upper_bound = CrossTxData {
            target_time: u64::MAX,
            target_chain: u64::MAX,
            target_id: tx_hash,
            ..Default::default()
        };
        let mut read_opts = ReadOptions::default();
        read_opts.set_iterate_upper_bound(bincode::serialize(&upper_bound)?);
        let iter = self.inner.iterator_opt(
            IteratorMode::From(&bincode::serialize(&lower_bound)?, Direction::Forward),
            &read_opts,
        );
        let mut profit: Option<CrossTxProfit> = None;
        for (key, value) in iter {
            let k: CrossTxData = bincode::deserialize(&key)?;
            let v: CrossTxProfit = bincode::deserialize(&value)?;
            if k.target_id != tx_hash {
                profit = Some(v);
                break;
            }
        }

        Ok(profit)
    }

    pub fn get_txs_by_timestamp_range(
        &self,
        start_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<Vec<(CrossTxData, CrossTxProfit)>> {
        let start_timestamp = start_timestamp * 1000;
        let end_timestamp = end_timestamp * 1000;
        let mut read_opts = ReadOptions::default();
        let lower_bound = CrossTxData {
            target_time: start_timestamp,
            target_chain: 0u64,
            target_id: [0; 32].into(),
            ..Default::default()
        };
        let upper_bound = CrossTxData {
            target_time: end_timestamp,
            target_chain: u64::MAX,
            target_id: [255; 32].into(),
            ..Default::default()
        };
        read_opts.set_iterate_upper_bound(bincode::serialize(&upper_bound)?);
        let iter = self.inner.iterator_opt(
            IteratorMode::From(&bincode::serialize(&lower_bound)?, Direction::Forward),
            &read_opts,
        );
        let mut txs = Vec::new();
        for (key, value) in iter {
            let k: CrossTxData = bincode::deserialize(&key)?;
            let v: CrossTxProfit = bincode::deserialize(&value)?;
            // println!(
            //     "start_timestamp: {}, end_timestamp: {}",
            //     start_timestamp, end_timestamp
            // );
            // println!("tx timestamp: {:?}", k.target_time);
            if k.target_time != end_timestamp {
                println!("key: {:?}, value: {:?}", k, v);
                txs.push((k, v));
            }
        }

        Ok(txs)
    }
}

type FF = fn(&[u8], &[u8]) -> Ordering;
pub fn open_rocksdb(path: String, callback: FF) -> anyhow::Result<DB> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_comparator("custom", callback);
    let db = DB::open(&opts, path)?;
    Ok(db)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::TxsRocksDB;

    // #[test]
    // pub fn test() {
    //     let db: TxsRocksDB = TxsRocksDB::new(String::from("./db")).unwrap();
    //
    //     let k_v_1 = (
    //         CrossTxData {
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(5000000000000u64),
    //             source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             source_amount: "0.051000000000001101".to_string(),
    //             source_chain: 5,
    //             source_id: "0x9077dc48e3b0c857b2fac9a333321d991553544f3d3ae20a281e831b2af87e12"
    //                 .to_string(),
    //             source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             source_symbol: "ETH".to_string(),
    //             source_time: 1694679156000,
    //             source_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //             target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             target_amount: "0.049995000000000008".to_string(),
    //             target_chain: 0, //
    //             target_id: [
    //                 121, 242, 35, 171, 20, 205, 49, 78, 161, 32, 160, 123, 186, 102, 193, 204, 163,
    //                 165, 73, 90, 240, 149, 74, 14, 5, 206, 160, 230, 136, 34, 108, 212,
    //             ]
    //             .into(),
    //             target_maker: None,
    //             target_symbol: "ETH".to_string(),
    //             target_time: 169467921400, //
    //             target_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //         },
    //         CrossTxProfit {
    //             maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(500000000000u64),
    //             chain_id: 5,
    //             token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
    //         },
    //     );
    //
    //     let k_v_2 = (
    //         CrossTxData {
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(5000000000000u64),
    //             source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             source_amount: "0.051000000000001102".to_string(),
    //             source_chain: 420,
    //             source_id: "0x3cdd4b287257977e83769443c6c3be2895e3feffabe9e42e640ea7193834f01e"
    //                 .to_string(),
    //             source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             source_symbol: "ETH".to_string(),
    //             source_time: 1694679326000,
    //             source_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //             target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             target_amount: "0.049995000000000001".to_string(),
    //             target_chain: 0, //
    //             target_id: [
    //                 115, 168, 172, 86, 122, 114, 38, 148, 19, 146, 139, 228, 13, 175, 160, 224,
    //                 227, 239, 126, 44, 37, 240, 158, 54, 186, 158, 106, 62, 17, 43, 145, 234,
    //             ]
    //             .into(),
    //             target_maker: None,
    //             target_symbol: "ETH".to_string(),
    //             target_time: 169467921400, //
    //             target_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //         },
    //         CrossTxProfit {
    //             maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(500000000000u64),
    //             chain_id: 420,
    //             token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
    //         },
    //     );
    //
    //     let k_v_3 = (
    //         CrossTxData {
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(500000000000u64),
    //             source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             source_amount: "0.006000000000001103".to_string(),
    //             source_chain: 420,
    //             source_id: "0x039a6e4da9024b345dad5985677fbed660b308ad9f953e2e917090dbbc483707"
    //                 .to_string(),
    //             source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             source_symbol: "ETH".to_string(),
    //             source_time: 1694690928000,
    //             source_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //             target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
    //                 .unwrap(),
    //             target_amount: "0.004999500000000002".to_string(),
    //             target_chain: 0,
    //             target_id: [
    //                 181, 58, 6, 33, 190, 91, 51, 146, 7, 169, 60, 193, 64, 19, 99, 156, 132, 175,
    //                 167, 0, 131, 110, 119, 46, 48, 41, 211, 142, 197, 211, 49, 88,
    //             ]
    //             .into(),
    //             target_maker: None,
    //             target_symbol: "ETH".to_string(),
    //             target_time: 169467921400, //
    //             target_token: Address::from_str("0x0000000000000000000000000000000000000000")
    //                 .unwrap(),
    //         },
    //         CrossTxProfit {
    //             maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
    //                 .unwrap(),
    //             dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
    //                 .unwrap(),
    //             profit: U256::from(50000000000u64),
    //             chain_id: 5,
    //             token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
    //         },
    //     );
    //
    //     db.insert_txs(vec![k_v_1, k_v_2, k_v_3]).unwrap();
    //     let s = db
    //         .get_txs_by_timestamp_range(169467921, 1694679213)
    //         .unwrap();
    //     println!("----------------------------------------------------------------------------");
    //     let profit = db.get_profit_by_yx_hash(
    //         [
    //             115, 168, 172, 86, 122, 114, 38, 148, 19, 146, 139, 228, 13, 175, 160, 224, 227,
    //             239, 126, 44, 37, 240, 158, 54, 186, 158, 106, 62, 17, 43, 145, 234,
    //         ]
    //         .into(),
    //     );
    //     println!("profit: {:?}", profit);
    // }
}
