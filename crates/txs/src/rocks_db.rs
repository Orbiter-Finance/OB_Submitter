use super::*;
use core::cmp::Ordering;
use ethers::types::Address;
use primitives::{error::Result, types::CrossTxData};
use rocksdb::Direction;
use std::fmt::format;

pub struct TxsRocksDB {
    inner: DB,
}

// todo get_profit_by_tx_hash
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
        //fixme
        let iter = self.inner.iterator_opt(
            IteratorMode::From(&bincode::serialize(&lower_bound)?, Direction::Forward),
            &read_opts,
        );
        let mut txs = Vec::new();
        for (key, value) in iter {
            let k: CrossTxData = bincode::deserialize(&key)?;
            let v: CrossTxProfit = bincode::deserialize(&value)?;
            println!("key: {:?}, value: {:?}", k, v);
            println!(
                "start_timestamp: {}, end_timestamp: {}",
                start_timestamp, end_timestamp
            );
            println!("tx timestamp: {:?}", k.target_time);
            if k.target_time != end_timestamp {
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

// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.051000000000001101", source_chain: 5, source_id: "0x9077dc48e3b0c857b2fac9a333321d991553544f3d3ae20a281e831b2af87e12", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694679156000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.049995000000000008", target_chain: 420, target_id: H256([121, 242, 35, 171, 20, 205, 49, 78, 161, 32, 160, 123, 186, 102, 193, 204, 163, 165, 73, 90, 240, 149, 74, 14, 5, 206, 160, 230, 136, 34, 108, 212]), target_maker: None, target_symbol: "ETH", target_time: 1694679214000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694679214000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.051000000000001102", source_chain: 420, source_id: "0x3cdd4b287257977e83769443c6c3be2895e3feffabe9e42e640ea7193834f01e", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694679326000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.049995000000000001", target_chain: 421613, target_id: H256([115, 168, 172, 86, 122, 114, 38, 148, 19, 146, 139, 228, 13, 175, 160, 224, 227, 239, 126, 44, 37, 240, 158, 54, 186, 158, 106, 62, 17, 43, 145, 234]), target_maker: None, target_symbol: "ETH", target_time: 1694679352000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 420, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694679352000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.006000000000001103", source_chain: 420, source_id: "0x039a6e4da9024b345dad5985677fbed660b308ad9f953e2e917090dbbc483707", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694690928000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.004999500000000002", target_chain: 5, target_id: H256([181, 58, 6, 33, 190, 91, 51, 146, 7, 169, 60, 193, 64, 19, 99, 156, 132, 175, 167, 0, 131, 110, 119, 46, 48, 41, 211, 142, 197, 211, 49, 88]), target_maker: None, target_symbol: "ETH", target_time: 1694690976000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 50000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694690976000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.006000000000001101", source_chain: 5, source_id: "0x1fac09d3dfbda0575e69c422119bfab5a7af4654fa826c39600ec60ab241f125", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694762448000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.004999500000000010", target_chain: 420, target_id: H256([249, 24, 52, 197, 78, 173, 249, 48, 98, 172, 62, 244, 202, 98, 201, 171, 43, 13, 104, 46, 240, 50, 65, 121, 119, 90, 184, 94, 124, 173, 19, 192]), target_maker: None, target_symbol: "ETH", target_time: 1694762528000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 50000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694762528000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.005150000000001102", source_chain: 420, source_id: "0xbeb6917fd24102840e4fe06263804f87edd2ff7077d2841d67343f542a1c05be", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694766284000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.004995000000000003", target_chain: 421613, target_id: H256([67, 118, 124, 186, 199, 97, 146, 222, 36, 75, 33, 148, 29, 153, 63, 142, 60, 171, 103, 113, 222, 135, 190, 67, 252, 42, 247, 201, 166, 138, 241, 229]), target_maker: None, target_symbol: "ETH", target_time: 1694770072000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694770072000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 750000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.051000000000001101", source_chain: 421613, source_id: "0xeccd95935c1ab3e0e69252140de9e6851289d845cb82283f9821eeeb8a937e62", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694765890000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.049250000000000006", target_chain: 420, target_id: H256([145, 115, 139, 254, 92, 171, 239, 225, 203, 39, 185, 98, 237, 193, 152, 198, 115, 4, 76, 190, 97, 47, 39, 133, 62, 227, 136, 200, 159, 206, 153, 116]), target_maker: None, target_symbol: "ETH", target_time: 1694770074000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 75000000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694770074000
//
// key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.006000000000001103", source_chain: 421613, source_id: "0x6be1af2d44ba12e76d4a579d04bb01e5ad6924894057d6a95af071c8223a5fae", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694762027000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.004995000000000005", target_chain: 5, target_id: H256([31, 243, 247, 28, 161, 128, 240, 242, 21, 204, 148, 90, 81, 76, 26, 19, 127, 152, 223, 3, 198, 31, 15, 210, 186, 213, 100, 181, 95, 26, 121, 78]), target_maker: None, target_symbol: "ETH", target_time: 1694770080000, target_token: 0x0000000000000000000000000000000000000000 }
// value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
// tx timestamp: 1694770080000

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::TxsRocksDB;

    #[test]
    pub fn test() {
        let db: TxsRocksDB = TxsRocksDB::new(String::from("./db")).unwrap();

        // key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.051000000000001101", source_chain: 5, source_id: "0x9077dc48e3b0c857b2fac9a333321d991553544f3d3ae20a281e831b2af87e12", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694679156000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.049995000000000008", target_chain: 420, target_id: H256([121, 242, 35, 171, 20, 205, 49, 78, 161, 32, 160, 123, 186, 102, 193, 204, 163, 165, 73, 90, 240, 149, 74, 14, 5, 206, 160, 230, 136, 34, 108, 212]), target_maker: None, target_symbol: "ETH", target_time: 1694679214000, target_token: 0x0000000000000000000000000000000000000000 }
        // value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
        // tx timestamp: 1694679214000
        let k_v_1 = (
            CrossTxData {
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(5000000000000u64),
                source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                source_amount: "0.051000000000001101".to_string(),
                source_chain: 5,
                source_id: "0x9077dc48e3b0c857b2fac9a333321d991553544f3d3ae20a281e831b2af87e12"
                    .to_string(),
                source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                source_symbol: "ETH".to_string(),
                source_time: 1694679156000,
                source_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
                target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                target_amount: "0.049995000000000008".to_string(),
                target_chain: 0, //
                target_id: [
                    121, 242, 35, 171, 20, 205, 49, 78, 161, 32, 160, 123, 186, 102, 193, 204, 163,
                    165, 73, 90, 240, 149, 74, 14, 5, 206, 160, 230, 136, 34, 108, 212,
                ]
                .into(),
                target_maker: None,
                target_symbol: "ETH".to_string(),
                target_time: 169467921400, //
                target_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
            },
            CrossTxProfit {
                maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(500000000000u64),
                chain_id: 5,
                token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            },
        );

        // key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 5000000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.051000000000001102", source_chain: 420, source_id: "0x3cdd4b287257977e83769443c6c3be2895e3feffabe9e42e640ea7193834f01e", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694679326000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.049995000000000001", target_chain: 421613, target_id: H256([115, 168, 172, 86, 122, 114, 38, 148, 19, 146, 139, 228, 13, 175, 160, 224, 227, 239, 126, 44, 37, 240, 158, 54, 186, 158, 106, 62, 17, 43, 145, 234]), target_maker: None, target_symbol: "ETH", target_time: 1694679352000, target_token: 0x0000000000000000000000000000000000000000 }
        // value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, chain_id: 420, token: 0x0000000000000000000000000000000000000000 }
        // tx timestamp: 1694679352000
        let k_v_2 = (
            CrossTxData {
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(5000000000000u64),
                source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                source_amount: "0.051000000000001102".to_string(),
                source_chain: 420,
                source_id: "0x3cdd4b287257977e83769443c6c3be2895e3feffabe9e42e640ea7193834f01e"
                    .to_string(),
                source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                source_symbol: "ETH".to_string(),
                source_time: 1694679326000,
                source_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
                target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                target_amount: "0.049995000000000001".to_string(),
                target_chain: 0, //
                target_id: [
                    115, 168, 172, 86, 122, 114, 38, 148, 19, 146, 139, 228, 13, 175, 160, 224,
                    227, 239, 126, 44, 37, 240, 158, 54, 186, 158, 106, 62, 17, 43, 145, 234,
                ]
                .into(),
                target_maker: None,
                target_symbol: "ETH".to_string(),
                target_time: 169467921400, //
                target_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
            },
            CrossTxProfit {
                maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(500000000000u64),
                chain_id: 420,
                token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            },
        );

        // key: CrossTxData { dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 500000000000, source_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, source_amount: "0.006000000000001103", source_chain: 420, source_id: "0x039a6e4da9024b345dad5985677fbed660b308ad9f953e2e917090dbbc483707", source_maker: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, source_symbol: "ETH", source_time: 1694690928000, source_token: 0x0000000000000000000000000000000000000000, target_address: 0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb, target_amount: "0.004999500000000002", target_chain: 5, target_id: H256([181, 58, 6, 33, 190, 91, 51, 146, 7, 169, 60, 193, 64, 19, 99, 156, 132, 175, 167, 0, 131, 110, 119, 46, 48, 41, 211, 142, 197, 211, 49, 88]), target_maker: None, target_symbol: "ETH", target_time: 1694690976000, target_token: 0x0000000000000000000000000000000000000000 }
        // value: CrossTxProfit { maker_address: 0xcc2b58a40a75ddf60ca7273643cafcebe2d34624, dealer_address: 0xdecf6cb214297c3ec7e557f23a8765e06b899c50, profit: 50000000000, chain_id: 5, token: 0x0000000000000000000000000000000000000000 }
        // tx timestamp: 1694690976000
        let k_v_3 = (
            CrossTxData {
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(500000000000u64),
                source_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                source_amount: "0.006000000000001103".to_string(),
                source_chain: 420,
                source_id: "0x039a6e4da9024b345dad5985677fbed660b308ad9f953e2e917090dbbc483707"
                    .to_string(),
                source_maker: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                source_symbol: "ETH".to_string(),
                source_time: 1694690928000,
                source_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
                target_address: Address::from_str("0xe8d9e41276a964b7ab012756b8d1b7107b2b87eb")
                    .unwrap(),
                target_amount: "0.004999500000000002".to_string(),
                target_chain: 0,
                target_id: [
                    181, 58, 6, 33, 190, 91, 51, 146, 7, 169, 60, 193, 64, 19, 99, 156, 132, 175,
                    167, 0, 131, 110, 119, 46, 48, 41, 211, 142, 197, 211, 49, 88,
                ]
                .into(),
                target_maker: None,
                target_symbol: "ETH".to_string(),
                target_time: 169467921400, //
                target_token: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
            },
            CrossTxProfit {
                maker_address: Address::from_str("0xcc2b58a40a75ddf60ca7273643cafcebe2d34624")
                    .unwrap(),
                dealer_address: Address::from_str("0xdecf6cb214297c3ec7e557f23a8765e06b899c50")
                    .unwrap(),
                profit: U256::from(50000000000u64),
                chain_id: 5,
                token: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            },
        );

        db.insert_txs(vec![k_v_1, k_v_2, k_v_3]).unwrap();
        let s = db
            .get_txs_by_timestamp_range(169467921, 1694679213)
            .unwrap();
        // println!("{:?}", s);
    }
}
