# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library for creating Windows RPC (Remote Procedure Call) clients using procedural macros. The project generates RPC client code from Rust trait definitions, handling the complex NDR (Network Data Representation) marshalling and unmarshalling automatically.

## Architecture

### Workspace Structure

The project is organized as a Cargo workspace with two crates:

- **windows_rpc**: Main library providing RPC runtime support (client bindings, memory allocators)
- **windows_rpc_macros**: Procedural macro crate that generates RPC client code from trait definitions

### Code Generation Flow

1. User defines a Rust trait annotated with `#[rpc_interface(guid(...), version(...))]`
2. The `rpc_interface` macro (in `windows_rpc_macros/src/lib.rs`) parses the trait
3. The macro generates:
   - NDR and NDR64 format strings (type descriptors, procedure headers)
   - RPC client struct with all necessary metadata (MIDL_STUBLESS_PROXY_INFO, MIDL_STUB_DESC, etc.)
   - Method implementations that call `NdrClientCall3` to perform RPC

### Key Components

**windows_rpc_macros/src/lib.rs** (procedural macro - ~1100 lines):
- Parses `#[rpc_interface]` attributes (GUID and version)
- Converts Rust types to RPC type descriptors
- Generates two formats: NDR 2.0 (32-bit) and NDR64 (64-bit) format strings
- Creates all required RPC metadata structures with proper circular references
- Supports basic types (i8-i64, u8-u64) and will need extension for strings and complex types

**windows_rpc/src/client_binding.rs**:
- `ClientBinding` wraps RPC binding handles
- Currently supports ALPC protocol (`ncalrpc`)
- Uses `RpcStringBindingComposeW` and `RpcBindingFromStringBindingW`

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
# Run all tests
cargo test

# Run specific test
cargo test test_sanity
```

### Testing the Macro
The primary test is in `windows_rpc/tests/test_sanity.rs` which defines a test RPC interface and calls it.

## Type System

Currently supported Rust types for RPC parameters and return values:
- Signed integers: `i8`, `i16`, `i32`, `i64`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`
- String support is partially implemented but not fully tested

Each type has mappings to:
- FC (Format Code) values for NDR 2.0
- NDR64 format codes for NDR64
- Parameter attributes (IS_IN, IS_OUT, IS_BASE_TYPE, etc.)

## Important Implementation Details

### NDR Format String Generation

The macro generates two separate format string structures:
1. **NDR 2.0 format** (`generate_type_format_string`, `generate_proc_header`): Used on 32-bit systems
2. **NDR64 format** (`generate_ndr64_type_format`, `generate_ndr64_proc_buffer_code`): Used on 64-bit systems

Both formats must be present for the RPC runtime to negotiate the appropriate transfer syntax.

### Memory Layout Constraints

All RPC metadata structures (MIDL_STUBLESS_PROXY_INFO, MIDL_STUB_DESC, format strings, etc.) must remain stable in memory for the lifetime of the client. The generated code uses `Box` to ensure stable addresses and maintains all necessary cross-references.

### Circular Dependencies

There are intentional circular references between:
- `MIDL_STUBLESS_PROXY_INFO.pStubDesc` → `MIDL_STUB_DESC`
- `MIDL_STUB_DESC.ProxyServerInfo` → `MIDL_STUBLESS_PROXY_INFO`
- `MIDL_STUB_DESC.RpcInterfaceInformation` → `RPC_CLIENT_INTERFACE`

These are resolved in the generated `new()` constructor by creating the structures first, then filling in the cross-references.

## Windows Crate Dependencies

The project uses two different Windows crates:
- `windows = "0.62"`: For safe Rust bindings (GUID, RPC_CLIENT_INTERFACE, etc.)
- `windows-sys = "0.61"`: For unsafe low-level bindings (MIDL_* structures, NdrClientCall3)

This split exists because some MIDL structures are only available in windows-sys.

## Edition

Uses Rust edition 2024 (both crates).
