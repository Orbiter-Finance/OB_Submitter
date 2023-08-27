use super::*;
use std::cmp::Ordering;
pub fn get_request_builder() -> RequestBuilder {
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Apifox/1.0.0 (https://apifox.com)"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    let url = "http://192.168.1.6:9095/v3/yj6toqvwh1177e1sexfy0u1pxx5j8o47";
    let aa = client.get(url).headers(headers.clone());
    client.get(url).headers(headers)
}

pub async fn get_txs(
    request_builder: RequestBuilder,
    chain_id: u64,
    start_timestamp: u64,
    end_timestamp: u64,
    delay_timestamp: u64,
) -> anyhow::Result<Vec<CrossTxDataAsKey>> {
    let start_timestamp = start_timestamp
        .checked_sub(delay_timestamp)
        .ok_or(anyhow::anyhow!(
            "start_timestamp checked_sub delay_timestamp error"
        ))?;
    let end_timestamp = end_timestamp
        .checked_sub(delay_timestamp)
        .ok_or(anyhow::anyhow!(
            "end_timestamp checked_sub delay_timestamp error"
        ))?;
    let res = request_builder
        .json(&json!({
            "id": "1",
            "jsonrpc": "2.0",
            "method": "orbiter_getBridgeSuccessfulTransaction",
            "params": [{
                "id": chain_id,
                "timestamp": [start_timestamp, end_timestamp]
            }]
        }))
        .send()
        .await?;

    if (res.status() == reqwest::StatusCode::OK) || (res.status() == reqwest::StatusCode::CREATED) {
        let res1: Value = serde_json::from_str(&res.text().await?).unwrap();
        let ress: &Value = &res1["result"][chain_id.to_string()];
        let aaa: Vec<CrossTxDataAsKey> = serde_json::from_value(ress.clone()).unwrap();
        for i in &aaa {
            println!("response: {:#?}", i)
        }
        Ok(aaa)
    } else {
        Err(anyhow::anyhow!("err: {:#?}", res.text().await?))
    }
}

type FF = fn(&[u8], &[u8]) -> Ordering;
pub fn open_rocksdb(path: String, callback: FF) -> anyhow::Result<DB> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_comparator("custom", callback);
    opts.create_if_missing(true);
    let db = DB::open(&opts, path)?;
    Ok(db)
}

pub fn is_chill(block_info: BlockInfo) -> bool {
    let last_submit_timestamp = block_info.storage.last_submit_timestamp;
    let now = block_info.storage.block_timestamp;

    let after_challenge = last_submit_timestamp + block_info.storage.challenge_duration;
    if after_challenge < now {
        let n = (now - after_challenge)
            % (block_info.storage.chill_duration + block_info.storage.withdraw_duration);

        if n > block_info.storage.withdraw_duration {
            return true;
        }
    }
    false
}

pub fn calculate_profit(commission: u64, tx: CrossTxDataAsKey) ->  CrossTxProfit {
    let profit = U256::from_dec_str(tx.profit.as_str()).unwrap();
    let profit = profit * U256::from(commission) / U256::from(100);
    CrossTxProfit {
        maker_address: Address::from_str(&tx.maker_address).unwrap(),
        dealer_address: Address::from_str(&tx.dealer_address).unwrap(),
        profit: profit,
    }
}
