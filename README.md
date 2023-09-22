# submitter

## Introduction

Responsible for the profit statistics of Ethereum L2 cross-chain transactions, 
and submits the Merkel root of the profit data to the Ethereum L1 chain. 
Completely decentralized.
## Installation

### Docker
todo

### Local

1. install git
```asm
sudo apt install git
```
2. install rust
```angular2html
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
3. install clang and llvm
```angular2html
sudo apt install clang llvm
```

4. build submitter
```angular2html
git clone https://github.com/YanOctavian/submitter.git
cd submitter
cargo build --release
```
5. configure environment variables
```shell
# Ethereum node https url
export NETWORK_RPC_URL = "https://eth-goerli.api.onfinality.io/public"
# Ethereum network chain id
export MAINNET_CHAIN_ID = 5

# RFeeManager smart contract address
export ORFeeManager_CONTRACT_ADDRESS = "0xA191028bf304209a14acA85866999d8140BA54d8"
# where to get txs
export TXS_SOURCE_URL = "https://openapi2.orbiter.finance/v3/yj6toqvwh1177e1sexfy0u1pxx5j8o47"
# Where to get chain information
export SUPPORT_CHAINS_SOURCE_URL = "https://api.studio.thegraph.com/query/49058/cabin/version/latest"


# Use directly without changing
export ZK_DELAY_SECONDS = 28800
# Use directly without changing
export OP_DELAY_SECONDS = 604800
# Use directly without changing
export COMMON_DELAY_SECONDS = 900
```
5. run submitter
```angular2html
./target/release/submitter
```
> If you don't want to be a submitter and just want to sync data, then you can use `--no-private-key` in your command line.
> for example, `./target/release/submitter --no-private-key`
6. view log
```shell
# for example
tail -f -n 100 db/logs/submitter.log.2023-09-20
```




