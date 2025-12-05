# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library for creating Windows RPC (Remote Procedure Call) clients and servers using procedural macros. The project generates both RPC client and server code from Rust trait definitions, handling the complex NDR (Network Data Representation) marshalling and unmarshalling automatically.

## Architecture

### Workspace Structure

The project is organized as a Cargo workspace with two crates:

- **windows_rpc**: Main library providing RPC runtime support (client/server bindings, memory allocators, thread-local context management)
- **windows_rpc_macros**: Procedural macro crate that generates both RPC client and server code from trait definitions

### Code Generation Flow

1. User defines a Rust trait annotated with `#[rpc_interface(guid(...), version(...))]`
2. The `rpc_interface` macro (in `windows_rpc_macros/src/lib.rs`) parses the trait
3. The macro generates both client and server code:

   **Client Side (`codegen.rs`):**
   - `{Interface}Client` struct with all RPC metadata structures
   - NDR and NDR64 format strings (type descriptors, procedure headers)
   - Method implementations that call `NdrClientCall3` to perform RPC
   - All metadata structures (MIDL_STUBLESS_PROXY_INFO, MIDL_STUB_DESC, RPC_CLIENT_INTERFACE, etc.)

   **Server Side (`server_codegen.rs`):**
   - `{Interface}ServerImpl` trait for users to implement (with `&self` methods)
   - `{Interface}Server` struct with all server metadata structures
   - Extern "C" wrapper functions that bridge RPC callbacks to Rust trait methods
   - Server metadata (MIDL_SERVER_INFO, RPC_SERVER_INTERFACE, RPC_DISPATCH_TABLE, etc.)

### Key Components

**windows_rpc_macros/src/lib.rs**:
- Entry point for the `#[rpc_interface]` procedural macro
- Parses trait definitions and extracts methods, parameters, and return types
- Calls both `compile_client()` and `compile_server()` to generate code

**windows_rpc_macros/src/codegen.rs** (client generation):
- Generates the `{Interface}Client` struct with all RPC metadata
- Creates NDR and NDR64 format strings for parameters and return values
- Handles string parameters by converting Rust `&str` to `HSTRING` to `PCWSTR` for FFI

**windows_rpc_macros/src/server_codegen.rs** (server generation):
- Generates the `{Interface}ServerImpl` trait and `{Interface}Server` struct
- Creates extern "C" wrapper functions that convert FFI types to Rust types
- Handles string parameters by converting `PCWSTR` to Rust `String` using `.to_string()`
- Sets up dispatch tables and server routine tables

**windows_rpc_macros/src/types.rs**:
- Defines the type system: `Type::Simple(BaseType)` for integers, `Type::String` for strings
- Maps Rust types to NDR format codes and parameter attributes
- `to_rust_type()`: Converts internal type to Rust token stream
- `rust_type_to_abi()`: Converts Rust types to ABI-compatible types for client calls

**windows_rpc/src/client_binding.rs**:
- `ClientBinding` wraps RPC binding handles
- Currently supports ALPC protocol (`ncalrpc`)
- Uses `RpcStringBindingComposeW` and `RpcBindingFromStringBindingW`

**windows_rpc/src/server_binding.rs**:
- `ServerBinding` manages RPC server lifecycle
- Methods: `register()`, `listen()` (blocking), `listen_async()` (non-blocking), `stop()`
- Handles protocol sequence registration and interface registration

**windows_rpc/src/server.rs**:
- Thread-local context management for passing trait implementation to wrapper functions
- Stores fat pointers as `(usize, usize)` tuples to preserve vtable information
- `set_context()`, `clear_context()`, and `with_context()` for context management

**windows_rpc/src/alloc.rs**:
- Custom MIDL memory allocator/deallocator for RPC runtime
- Embeds the `Layout` before allocated memory to support proper deallocation

## Development Commands

### Building
```bash
cargo build
```

### Running Tests
```bash
# Run all integration tests (excluding test_sanity which requires external server)
cargo test --test test_client_server --test test_string_params --test test_server_simple

# Run a specific test
cargo test --test test_client_server

# Run a specific test with output
cargo test --test test_string_params -- --nocapture

# Run single-threaded (useful for debugging server tests)
cargo test --test test_client_server -- --test-threads=1
```

### Test Structure
- `test_client_server.rs`: Basic client-server integration test with integer parameters
- `test_string_params.rs`: Tests string parameter handling in both client and server
- `test_server_simple.rs`: Tests server creation and registration without client calls
- `test_sanity.rs`: Requires external RPC server (used for manual testing against C++ server)

## Type System

Currently supported Rust types for RPC parameters and return values:
- **Signed integers**: `i8`, `i16`, `i32`, `i64`
- **Unsigned integers**: `u8`, `u16`, `u32`, `u64`
- **Strings**: `&str` (input parameters only)

Each type has mappings to:
- FC (Format Code) values for NDR 2.0
- NDR64 format codes for NDR64
- Parameter attributes (IS_IN, IS_OUT, IS_BASE_TYPE, etc.)

### String Handling

Strings require special handling across the FFI boundary:

**Client Side:**
- Rust `&str` → `HSTRING` → `PCWSTR` (via `rust_type_to_abi()`)
- Conversion happens in generated client methods before calling `NdrClientCall3`

**Server Side:**
- `PCWSTR` received in extern "C" wrapper → `String` via `.to_string().unwrap()`
- Converted string passed as `&str` to trait implementation
- Wrapper functions have an extra `binding_handle` parameter (first parameter)

## Important Implementation Details

### NDR Format String Generation

The macro generates two separate format string structures:
1. **NDR 2.0 format** (`generate_type_format_string`, `generate_proc_header`): Used on 32-bit systems
2. **NDR64 format** (`generate_ndr64_type_format`, `generate_ndr64_proc_buffer_code`): Used on 64-bit systems

Both formats must be present for the RPC runtime to negotiate the appropriate transfer syntax.

### Memory Layout Constraints

All RPC metadata structures (MIDL_STUBLESS_PROXY_INFO, MIDL_STUB_DESC, format strings, etc.) must remain stable in memory for the lifetime of the client. The generated code uses `Box` to ensure stable addresses and maintains all necessary cross-references.

### Circular Dependencies

**Client Side:**
- `MIDL_STUBLESS_PROXY_INFO.pStubDesc` → `MIDL_STUB_DESC`
- `MIDL_STUB_DESC.ProxyServerInfo` → `MIDL_STUBLESS_PROXY_INFO`
- `MIDL_STUB_DESC.RpcInterfaceInformation` → `RPC_CLIENT_INTERFACE`

**Server Side:**
- `MIDL_SERVER_INFO.pStubDesc` → `MIDL_STUB_DESC`
- `MIDL_STUB_DESC.ProxyServerInfo` → `MIDL_SERVER_INFO`
- `MIDL_STUB_DESC.RpcInterfaceInformation` → `RPC_SERVER_INTERFACE`
- `MIDL_SYNTAX_INFO[0].DispatchTable` → `dispatch_table_ndr`
- `MIDL_SYNTAX_INFO[1].DispatchTable` → `dispatch_table_ndr64`

These are resolved in the generated `new()` constructor by creating the structures first, then filling in the cross-references using raw pointers.

## Windows Crate Dependencies

The project uses two different Windows crates:
- `windows = "0.62"`: For safe Rust bindings (GUID, RPC_CLIENT_INTERFACE, RPC_SERVER_INTERFACE, PCWSTR, HSTRING)
- `windows-sys = "0.61"`: For unsafe low-level bindings (MIDL_* structures, NdrClientCall3, NdrServerCall2, NdrServerCallAll)

This split exists because some MIDL structures are only available in windows-sys.

### Type Qualification Requirements

**CRITICAL**: When generating code in `codegen.rs` and `server_codegen.rs`, **all types must be fully qualified** to avoid conflicts between the two crates. Examples:
- Use `windows::Win32::System::Rpc::RPC_CLIENT_INTERFACE` (not imported)
- Use `windows_sys::Win32::System::Rpc::MIDL_STUB_DESC` (not imported)
- Use `std::boxed::Box`, `std::option::Option`, `std::ptr::null_mut()`, etc.

This prevents ambiguous type errors when both crates define structures with the same name (like `RPC_DISPATCH_TABLE`).

## Critical Implementation Notes

### Server Metadata Initialization

When generating server code, these fields MUST be initialized correctly (reference from MIDL-generated C code):

1. **`auto_bind_handle`**: Must be created and pointed to by `MIDL_STUB_DESC.IMPLICIT_HANDLE_INFO.pAutoHandle`
2. **`stub_desc.ProxyServerInfo`**: Must point to `server_info` (circular reference)
3. **`MIDL_SYNTAX_INFO[].DispatchTable`**: Must point to respective dispatch tables for both NDR 2.0 and NDR64
4. **`RPC_SERVER_INTERFACE.Flags`**: Should be `0x06000000` (supports NDR64)

Missing any of these will cause runtime errors like `ERROR_STUB_DATA_INVALID` or heap corruption.

### Server Thread-Local Context

The server uses thread-local storage to pass the trait implementation to wrapper functions:
- Fat pointers (trait objects) are stored as `(usize, usize)` tuples via `transmute_copy`
- Context is set in `register()` and cleared in `stop()`
- Wrapper functions call `with_context()` to access the implementation

## Edition

Uses Rust edition 2024 (both crates).
