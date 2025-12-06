//! Windows RPC client and server library for Rust.
//!
//! This crate, together with [`windows_rpc_macros`](https://docs.rs/windows-rpc-macros), provides
//! a way to define Windows RPC interfaces using Rust traits and automatically generate all the
//! necessary client and server code. The generated code handles NDR (Network Data Representation)
//! marshalling, format strings, and Windows RPC runtime integration.
//!
//! # Quick Start
//!
//! Define an RPC interface as a trait with the [`rpc_interface`] macro:
//!
//! ```rust
//! use windows_rpc::rpc_interface;
//! use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//!     fn multiply(x: i32, y: i32) -> i32;
//! }
//! ```
//!
//! This generates three types:
//! - `CalculatorClient` - for making RPC calls
//! - `CalculatorServerImpl` - trait to implement for the server
//! - `CalculatorServer` - wraps your implementation for RPC dispatch
//!
//! # Server Example
//!
//! ```rust,no_run
//! use windows_rpc::rpc_interface;
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//! }
//!
//! struct CalculatorImpl;
//!
//! impl CalculatorServerImpl for CalculatorImpl {
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut server = CalculatorServer::new(CalculatorImpl);
//!     server.register("calculator_endpoint")?;
//!
//!     // Non-blocking: returns immediately, processes calls in background
//!     server.listen_async()?;
//!
//!     // ... do other work or wait for shutdown signal ...
//!
//!     server.stop()?;
//!     Ok(())
//! }
//! ```
//!
//! # Client Example
//!
//! ```rust,no_run
//! use windows_rpc::rpc_interface;
//! use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let binding = ClientBinding::new(ProtocolSequence::Alpc, "calculator_endpoint")?;
//!     let client = CalculatorClient::new(binding);
//!
//!     let result = client.add(10, 20);
//!     println!("10 + 20 = {result}");  // Prints: 10 + 20 = 30
//!
//!     Ok(())
//! }
//! ```
//!
//! # Supported Types
//!
//! The following types can be used for parameters and return values:
//!
//! | Rust Type | Description |
//! |-----------|-------------|
//! | `i8`, `u8` | 8-bit integers |
//! | `i16`, `u16` | 16-bit integers |
//! | `i32`, `u32` | 32-bit integers |
//! | `i64`, `u64` | 64-bit integers |
//! | `&str` | String (input parameters only) |
//!
//! # Protocol Support
//!
//! Currently only ALPC (Advanced Local Procedure Call) is supported via the `ncalrpc`
//! protocol sequence. This allows RPC communication between processes on the same machine.
//!
//! # What This Library Does
//!
//! - Generates all MIDL stub metadata (`MIDL_STUB_DESC`, `MIDL_SERVER_INFO`, etc.)
//! - Handles NDR 2.0 and NDR64 format strings for type marshalling
//! - Manages RPC binding handles and server lifecycle
//! - Converts between Rust types and Windows ABI types
//! - Provides clean async (non-blocking) and sync (blocking) server modes
//!
//! # Limitations
//!
//! This library is currently limited in scope:
//!
//! - **Protocol**: Only local RPC (ALPC/ncalrpc) is supported. TCP, UDP, and named pipes
//!   are not yet implemented.
//! - **Parameter direction**: Only input (`[in]`) parameters are supported. Output and
//!   input-output parameters are not available.
//! - **Types**: Only primitive integers and strings are supported. No pointers, structs,
//!   arrays, unions, or other complex types.
//! - **Strings**: The `&str` type is only supported for input parameters, not return values.
//! - **Security**: No interface security (authentication, authorization, encryption) is
//!   implemented.
//! - **Exceptions**: SEH exceptions from the RPC runtime are not caught or handled.
//! - **Callbacks**: RPC callbacks from server to client are not supported.
//!
//! # Interoperability
//!
//! The generated code produces standard Windows RPC interfaces that are compatible with
//! MIDL-generated C/C++ clients and servers. You can use a Rust server with a C++ client
//! (or vice versa) as long as the interface GUID, version, and method signatures match.
//!
//! # Safety
//!
//! This crate uses `unsafe` code extensively to interact with the Windows RPC runtime.
//! The generated client and server code manages memory carefully to ensure:
//!
//! - RPC metadata structures remain valid for the lifetime of the client/server
//! - String conversions between Rust and Windows types are handled correctly
//! - Thread-local context is used safely for server dispatch
//!
//! However, bugs in this crate could lead to memory corruption or undefined behavior.

#![cfg(windows)]

#[doc(hidden)]
pub mod alloc;
pub mod client_binding;
#[doc(hidden)]
pub mod server;
pub mod server_binding;

pub use windows_rpc_macros::rpc_interface;
