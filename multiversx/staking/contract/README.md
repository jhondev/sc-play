# rust config 

```
echo "nightly" > rust-toolchain
rustup override set nightly
```

# build
```
mxpy contract build
```

# tests
```
cargo test --test staking_rust_test
```