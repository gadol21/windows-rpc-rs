# Windows-rpc
[![check](https://github.com/gadol21/windows-rpc-rs/actions/workflows/.github/workflows/check.yml/badge.svg?branch=main)](https://github.com/gadol21/windows-rpc-rs/blob/main/.github/workflows/check.yml)
[![crates.io](https://img.shields.io/crates/v/windows-rpc.svg)](https://crates.io/crates/windows-rpc)
[![docs.rs](https://docs.rs/windows-rpc/badge.svg)](https://docs.rs/crate/windows-rpc)

The `windows-rpc` and `windows-rpc-macros` crates let you generate Windows RPC interfaces, and generate the structs and stubs that are needed to make RPC calls, and host RPC servers in Rust code.

The idea is for you to describe the interface as a trait, and apply a proc macro to mark it as an interface -
```rust
#[windows_rpc_macros::rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
trait TestRpc {
    fn add(a: i32, b: i32) -> i32;
    fn multiply(x: i32, y: i32) -> i32;
    fn strlen(string: &str) -> u64;
}
```

This will auto generate a `TestRpcClient` struct that can be used to make the RPC calls.
Additionally, if you want to implement a server you can implement the trait `TestRpcServerImpl`, and start a `TestRpcServer` with it.

There is a lot of additional work required to support more complex types, add the ability to secure interfaces, and handling SEH exceptions.
