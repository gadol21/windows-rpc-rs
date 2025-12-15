//! Windows RPC client and server library for Rust.
//!
//! This crate, together with [`windows_rpc_macros`](https://docs.rs/windows-rpc-macros), provides
//! a way to define Windows RPC interfaces using Rust traits and automatically generate all the
//! necessary client and server code. The generated code handles NDR (Network Data Representation)
//! marshalling, format strings, and Windows RPC runtime integration.
//!
//! # Features
//!
//! - **Simple trait-based interface definition** - Define RPC interfaces using familiar Rust syntax
//! - **Automatic code generation** - Client and server code generated at compile time
//! - **Type safety** - Full Rust type system integration
//! - **NDR marshalling** - Automatic Network Data Representation encoding/decoding
//! - **String support** - Native handling of string parameters and return values
//! - **Integer types** - Support for i8, i16, i32, i64, u8, u16, u32, u64
//! - **ALPC protocol** - Fast local RPC using Advanced Local Procedure Call
//!
//! # Quick Start
//!
//! Define an RPC interface as a trait with the [`rpc_interface`] macro:
//!
//! ```rust
//! use windows_rpc::rpc_interface;
//! use windows_rpc::{ProtocolSequence, client_binding::ClientBinding};
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//!     fn multiply(x: i32, y: i32) -> i32;
//!     fn strlen(string: &str) -> u64;
//!     fn greet(name: &str) -> String;
//! }
//! ```
//!
//! This generates three types:
//! - `CalculatorClient` - for making RPC calls
//! - `CalculatorServerImpl` - trait to implement for the server
//! - `CalculatorServer<T>` - generic server wrapper for RPC dispatch
//!
//! # Server Example
//!
//! Implement the generated `ServerImpl` trait with static methods:
//!
//! ```rust,no_run
//! use windows_rpc::rpc_interface;
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//!     fn greet(name: &str) -> String;
//! }
//!
//! struct CalculatorImpl;
//!
//! impl CalculatorServerImpl for CalculatorImpl {
//!     fn add(a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//!
//!     fn greet(name: &str) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create server with the implementation type
//!     let mut server = CalculatorServer::<CalculatorImpl>::new();
//!     server.register("calculator_endpoint")?;
//!
//!     // Non-blocking: returns immediately, processes calls in background
//!     server.listen_async()?;
//!
//!     println!("Server is running...");
//!
//!     // Keep the server running
//!     std::thread::sleep(std::time::Duration::from_secs(60));
//!
//!     // Clean shutdown
//!     server.stop()?;
//!     Ok(())
//! }
//! ```
//!
//! # Client Example
//!
//! Make RPC calls using the generated client:
//!
//! ```rust,no_run
//! use windows_rpc::rpc_interface;
//! use windows_rpc::{ProtocolSequence, client_binding::ClientBinding};
//!
//! #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
//! trait Calculator {
//!     fn add(a: i32, b: i32) -> i32;
//!     fn greet(name: &str) -> String;
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client binding
//!     let binding = ClientBinding::new(ProtocolSequence::Alpc, "calculator_endpoint")?;
//!     let client = CalculatorClient::new(binding);
//!
//!     // Make RPC calls - integers
//!     let result = client.add(10, 20);
//!     println!("10 + 20 = {result}");  // Prints: 10 + 20 = 30
//!
//!     // Make RPC calls - strings
//!     let greeting = client.greet("Alice");
//!     println!("{greeting}");  // Prints: Hello, Alice!
//!
//!     Ok(())
//! }
//! ```
//!
//! # Complete Example with String Operations
//!
//! Here's a more comprehensive example showcasing various string operations:
//!
//! ```rust,no_run
//! use windows_rpc::{rpc_interface, ProtocolSequence, client_binding::ClientBinding};
//!
//! #[rpc_interface(guid(0xabcdef12_3456_7890_abcd_ef1234567890), version(1.0))]
//! trait StringService {
//!     fn to_uppercase(text: &str) -> String;
//!     fn reverse(text: &str) -> String;
//!     fn count_words(text: &str) -> u32;
//!     fn concat(a: &str, b: &str) -> String;
//! }
//!
//! struct StringServiceImpl;
//!
//! impl StringServiceServerImpl for StringServiceImpl {
//!     fn to_uppercase(text: &str) -> String {
//!         text.to_uppercase()
//!     }
//!
//!     fn reverse(text: &str) -> String {
//!         text.chars().rev().collect()
//!     }
//!
//!     fn count_words(text: &str) -> u32 {
//!         text.split_whitespace().count() as u32
//!     }
//!
//!     fn concat(a: &str, b: &str) -> String {
//!         format!("{}{}", a, b)
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start server
//!     let mut server = StringServiceServer::<StringServiceImpl>::new();
//!     server.register("string_service")?;
//!     server.listen_async()?;
//!
//!     // Create client
//!     let client = StringServiceClient::new(
//!         ClientBinding::new(ProtocolSequence::Alpc, "string_service")?
//!     );
//!
//!     // Test string operations
//!     println!("{}", client.to_uppercase("hello"));              // Output: HELLO
//!     println!("{}", client.reverse("hello"));                   // Output: olleh
//!     println!("{}", client.count_words("hello world"));         // Output: 2
//!     println!("{}", client.concat("Hello, ", "World!"));        // Output: Hello, World!
//!
//!     server.stop()?;
//!     Ok(())
//! }
//! ```
//!
//! # Supported Types
//!
//! The following types can be used for parameters and return values:
//!
//! | Rust Type | Parameters | Return Values | Notes |
//! |-----------|------------|---------------|-------|
//! | `i8`, `u8` | ✓ | ✓ | 8-bit integers |
//! | `i16`, `u16` | ✓ | ✓ | 16-bit integers |
//! | `i32`, `u32` | ✓ | ✓ | 32-bit integers |
//! | `i64`, `u64` | ✓ | ✓ | 64-bit integers |
//! | `&str` | ✓ | ✗ | String input parameters |
//! | `String` | ✗ | ✓ | String return values |
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
//! - **Parameter direction**: Only input (`[in]`) parameters and return values (`[out]`) are
//!   supported. Input-output parameters are not available.
//! - **Types**: Only primitive integers and strings are supported. No pointers, structs,
//!   arrays, unions, or other complex types.
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
//! - Memory allocated by the server for return values is properly managed
//! - Static trait methods are called correctly via monomorphization
//!
//! However, bugs in this crate could lead to memory corruption or undefined behavior.
//!
//! # Implementation Details
//!
//! The server implementation uses:
//! - **Generic server structs**: `{Interface}Server<T>` is generic over the implementation type
//! - **Static trait methods**: Server trait methods don't take `&self`, making implementations stateless
//! - **Monomorphization**: Each instantiation of `Server<ConcreteType>` generates type-specific wrapper functions
//! - **Extern "C" wrappers**: Generated wrapper functions bridge the RPC runtime to Rust static methods
#![cfg(windows)]

#[doc(hidden)]
pub mod alloc;
pub mod client_binding;
pub mod server_binding;

pub use windows_rpc_macros::rpc_interface;

/// Protocol sequence for RPC communication.
///
/// Specifies the transport protocol used for RPC calls.
///
/// # Example
///
/// ```rust,no_run
/// use windows_rpc::{ProtocolSequence, client_binding::ClientBinding};
///
/// # fn main() -> windows::core::Result<()> {
/// // Connect using local RPC (ALPC)
/// let binding = ClientBinding::new(ProtocolSequence::Alpc, "my_endpoint")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtocolSequence {
    /// ALPC (Advanced Local Procedure Call) - local RPC on the same machine.
    ///
    /// Uses the `ncalrpc` protocol sequence. This is the fastest option for
    /// communication between processes on the same Windows machine.
    Alpc,
    // TODO: test and add
    //Tcp,
    //Udp,
    //NamedPipe
}

impl ProtocolSequence {
    fn to_pcwstr(self) -> windows::core::PCWSTR {
        match self {
            ProtocolSequence::Alpc => windows::core::w!("ncalrpc"),
        }
    }
}
