#![allow(unused_variables)]

#[allow(unused_imports)]
use super::*;
use primitives::{
    env::get_mainnet_chain_id,
    types::{CrossTxData, CrossTxRawData},
};
use serde::{Deserialize, Serialize};
use state::{Hasher, Keccak256Hasher};
use std::string::String;

pub struct TxsCrawler {
    url: String,
    headers: HeaderMap,
    method: String,
    client: Client,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct TokenAddress {
    tokenAddress: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChainIds {
    id: String,
}

impl TxsCrawler {
    pub fn new(url: String) -> Self {
        let method = String::from("orbiter_getBridgeSuccessfulTransaction");
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Apifox/1.0.0 (https://apifox.com)"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        // headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        Self {
            url,
            headers,
            method,
            client,
        }
    }

    pub async fn request_txs(
        &self,
        target_chain_id: u64,
        start_timestamp: u64,
        end_timestamp: u64,
        delay_timestamp: u64,
    ) -> anyhow::Result<Vec<CrossTxRawData>> {
        // let  span = tracing::span!(tracing::Level::INFO, "request_txs");
        // let _enter = span.enter();
        let start_timestamp =
            start_timestamp
                .checked_sub(delay_timestamp)
                .ok_or(anyhow::anyhow!(
                    "start_timestamp checked_sub delay_timestamp error"
                ))?
                * 1000;
        let end_timestamp = end_timestamp
            .checked_sub(delay_timestamp)
            .ok_or(anyhow::anyhow!(
                "end_timestamp checked_sub delay_timestamp error"
            ))?
            * 1000;
        let res = self
            .client
            .post(self.url.clone())
            .headers(self.headers.clone())
            .json(&json!({
                "id": "1",
                "jsonrpc": "2.0",
                "method": self.method,
                "params": [{
                    "id": target_chain_id,
                    "timestamp": [start_timestamp, end_timestamp]
                }]
            }))
            .send()
            .await?;

        if (res.status() == reqwest::StatusCode::OK)
            || (res.status() == reqwest::StatusCode::CREATED)
        {
            let res: Value = serde_json::from_str(&res.text().await?)?;
            // event!(Level::INFO, "response: {:#?}", res);
            // println!("response: {:#?}", res);
            let res: &Value = &res["result"][target_chain_id.to_string()];
            event!(
                Level::INFO,
                "start_timestamp: {}, end_timestamp: {}, chain id: {}, res: {:#?}",
                start_timestamp,
                end_timestamp,
                target_chain_id,
                res
            );
            let old_txs: Vec<CrossTxRawData> = serde_json::from_value(res.clone())?;
            let mut new_txs: Vec<CrossTxRawData> = vec![];
            for tx in old_txs {
                // todo check source_time
                event!(Level::INFO, "tx: {:?}", tx);
                let mut tx = tx;
                tx.target_time = tx.target_time + delay_timestamp * 1000;
                new_txs.push(tx);
            }
            return Ok(new_txs);
        } else {
            return Err(anyhow::anyhow!("err: {:#?}", res.text().await?));
        }
    }
}

pub struct SupportChains {
    url: String,
    headers: HeaderMap,
    method: String,
    client: Client,
}

impl SupportChains {
    pub fn new(url: String) -> Self {
        let method = String::from("orbiter_getBridgeSuccessfulTransaction");
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Apifox/1.0.0 (https://apifox.com)"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        // headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        Self {
            url,
            headers,
            method,
            client,
        }
    }

    pub async fn get_mainnet_support_tokens(&self) -> anyhow::Result<Vec<Address>> {
        let graphql_url = &self.url;
        let query = r#"
    {
      tokenRels (where: {chainId: "5"}) {
        tokenAddress

      }
    }
    "#;
        let json_body = serde_json::json!({ "query": query });
        let mut tokens: Vec<Address> = vec![];
        let res = self
            .client
            .post(graphql_url)
            .body(json_body.to_string())
            .headers(self.headers.clone())
            .send()
            .await?;
        if res.status().is_success() {
            let res: Value = serde_json::from_str(&res.text().await?)?;
            let res: &Value = &res["data"]["tokenRels"];
            let ts: Vec<TokenAddress> = serde_json::from_value(res.clone())?;
            for t in ts {
                let token = &t.tokenAddress[26..];
                let token = "0x".to_owned() + token;
                let token = Address::from_str(&token).unwrap();

                if token != Address::zero() {
                    tokens.push(token);
                    // println!("token: {:?}", token);
                }
            }
        } else {
            println!("err: {:?}", res.status());
        }
        tokens.push(Address::default());
        Ok(tokens)
    }

    pub async fn get_support_chains(&self) -> anyhow::Result<Vec<u64>> {
        let graphql_url = &self.url;
        let query = r#"
    {
      chainRels {
    id
  }
    }
    "#;

        let json_body = serde_json::json!({ "query": query });
        let res = self
            .client
            .post(graphql_url)
            .body(json_body.to_string())
            .headers(self.headers.clone())
            .send()
            .await?;
        let mut chains: Vec<u64> = vec![];
        if res.status().is_success() {
            let res: Value = serde_json::from_str(&res.text().await?)?;
            let res: &Value = &res["data"]["chainRels"];
            let cs: Vec<ChainIds> = serde_json::from_value(res.clone())?;
            for i in cs {
                chains.push(i.id.parse::<u64>().unwrap());
            }
            return Ok(chains);
        }
        Ok(chains)
    }
}

pub fn calculate_profit(percent: u64, tx: CrossTxData) -> CrossTxProfit {
    // let span = tracing::span!(tracing::Level::INFO, "calculate_profit");
    // let _enter = span.enter();
    let profit = tx.profit;
    let profit = profit * U256::from(percent) / U256::from(100_0000);
    event!(
        Level::INFO,
        "calculate_profit dealer: {:?}, maker: {:?}, profit: {:?}",
        tx.dealer_address,
        tx.source_maker,
        profit
    );
    CrossTxProfit {
        maker_address: tx.source_maker,
        dealer_address: tx.dealer_address,
        profit: profit,
        chain_id: get_mainnet_chain_id(),
        token: tx.source_token,
    }
}

pub fn get_one_block_txs_hash(mut txs: Vec<H256>) -> H256 {
    if txs.is_empty() {
        return H256::zero();
    }
    let mut hasher = Keccak256Hasher::default();
    txs.sort();
    for tx in txs {
        hasher.write_h256(&tx);
    }
    hasher.finish()
}

pub fn convert_string_to_hash(tx: String) -> H256 {
    let hex_string = &tx[2..];
    let bytes = hex::decode(hex_string).expect("Failed to decode hex string");
    let mut result: [u8; 32] = [0; 32];
    result.copy_from_slice(&bytes[..32]);
    result.into()
}

#[cfg(test)]
pub mod test {
    use crate::{convert_string_to_hash, funcs::TxsCrawler, get_one_block_txs_hash, SupportChains};
    use sparse_merkle_tree::H256;

    #[tokio::test]
    async fn test() {
        // let s = SupportChains::new(
        //     "https://api.studio.thegraph.com/query/49058/cabin/version/latest".to_string(),
        // );
        // let a = s.get_mainnet_support_tokens().await.unwrap();
        // println!("a: {:?}", a);
        // let a = s.get_support_chains().await.unwrap();
        // println!("chains: {:?}", a);
        // https://openapi2.orbiter.finance/v3/yj6toqvwh1177e1sexfy0u1pxx5j8o47
        // {
        //     "id":"1",
        //     "jsonrpc":"2.0",
        //     "method":"orbiter_getBridgeSuccessfulTransaction",
        //     "params":[{
        //         "id":"5",
        //         "timestamp": [0,1694162197302]
        //     }]
        // }

        let s = TxsCrawler::new(
            "https://openapi2.orbiter.finance/v3/yj6toqvwh1177e1sexfy0u1pxx5j8o47".to_string(),
        );
        let end: u64 = 1695284033;
        let duration: u64 = 7200;
        let arb = 421613;
        let op = 420;
        let start = end - duration;
        // let start = 1695023676;
        let a = s.request_txs(op, start, end, 0).await.unwrap();
        println!("a: {:?}", a);
        println!("len: {:?}", a.len());
        for tx in a {
            println!("tx: {:?}", tx);
        }

        let b = convert_string_to_hash(
            "0x9077dc48e3b0c857b2fac9a333321d991553544f3d3ae20a281e831b2af87e12".to_string(),
        );
        println!("b: {:?}", b);
    }
}
