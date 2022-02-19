### Build
```
$ cargo build --target wasm32-unknown-unknown --release
$ copy target\wasm32-unknown-unknown\release\nft_market.wasm main.wasm 
```
### Deploy
```
$ near deploy --wasmFile main.wasm --accountId account.near
```
