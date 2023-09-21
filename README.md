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
./target/release/submitter
```
6. view log
```asm
// for example
tail -f -n 100 db/logs/submitter.log.2023-09-20
```




