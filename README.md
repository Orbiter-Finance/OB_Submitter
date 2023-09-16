# submitter

## Description

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
5. run submitter
```angular2html
// Options:
//      --rpc-port <RPC_PORT>        rpc server's port [default: 50001]
//      --db-path <DB_PATH>          state db's path [default: db]
//      --debug                      debug mode
//      --start-block <START_BLOCK>  debug mode [default: 0]
```
command
```angular2html
./target/release/submitter --rpc-port 50001 --db-path db
```



