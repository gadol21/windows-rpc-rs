//! RPC client binding management.
//!
//! This module provides types for creating and managing RPC client bindings,
//! which are used to connect to RPC servers.

use std::{ffi::c_void, ptr};

use windows::{
    Win32::System::Rpc::{RpcBindingFromStringBindingW, RpcStringBindingComposeW},
    core::HSTRING,
};

/// Protocol sequence for RPC communication.
///
/// Specifies the transport protocol used for RPC calls.
///
/// # Example
///
/// ```rust,no_run
/// use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
///
/// # fn main() -> windows::core::Result<()> {
/// // Connect using local RPC (ALPC)
/// let binding = ClientBinding::new(ProtocolSequence::Alpc, "my_endpoint")?;
/// # Ok(())
/// # }
/// ```
pub enum ProtocolSequence {
    /// ALPC (Advanced Local Procedure Call) - local RPC on the same machine.
    ///
    /// Uses the `ncalrpc` protocol sequence. This is the fastest option for
    /// communication between processes on the same Windows machine.
    Alpc,
    // FIXME: test and add
    //Tcp,
    //Udp,
    //NamedPipe
}

impl ProtocolSequence {
    fn to_string(&self) -> &'static str {
        match self {
            ProtocolSequence::Alpc => "ncalrpc",
            //ProtocolSequence::Tcp => "ncacn_ip_tcp",
            //ProtocolSequence::Http => "ncacn_http",
            //ProtocolSequence::NamedPipe => "ncacn_np",
        }
    }
}

/// An RPC client binding handle.
///
/// Represents a connection endpoint for making RPC calls to a server. The binding
/// encapsulates the protocol, endpoint, and other connection parameters.
///
/// # Example
///
/// ```rust,no_run
/// # use windows_rpc::rpc_interface;
/// #
/// # #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
/// # trait MyInterface {
/// # }
/// use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
///
/// # fn main() -> windows::core::Result<()> {
/// let binding = ClientBinding::new(ProtocolSequence::Alpc, "my_endpoint")?;
/// let client = MyInterfaceClient::new(binding);
/// # Ok(())
/// # }
/// ```
///
/// # Lifetime
///
/// The binding handle must remain valid for the lifetime of any client using it.
/// The generated client structs take ownership of the binding.
pub struct ClientBinding {
    handle: *mut c_void,
}

impl ClientBinding {
    /// Creates a new client binding to the specified endpoint.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol sequence to use for communication
    /// * `endpoint` - The server endpoint name (e.g., "my_rpc_endpoint")
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The binding string cannot be composed
    /// - The binding handle cannot be created from the string
    ///
    /// # Example
    ///
    /// ```rust
    /// use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
    ///
    /// # fn main() -> windows::core::Result<()> {
    /// let binding = ClientBinding::new(ProtocolSequence::Alpc, "calculator_endpoint")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(protocol: ProtocolSequence, endpoint: &str) -> windows::core::Result<Self> {
        let mut string_binding = windows::core::PWSTR::null();
        unsafe {
            RpcStringBindingComposeW(
                // TODO: pass obj uuid, could replace the endpoint/network addr
                None,
                &HSTRING::from(protocol.to_string()),
                None,
                &HSTRING::from(endpoint),
                None,
                Some(&raw mut string_binding),
            )
        }
        .ok()?;

        let mut handle: *mut core::ffi::c_void = ptr::null_mut();
        unsafe { RpcBindingFromStringBindingW(string_binding, &raw mut handle) }.ok()?;

        Ok(Self { handle })
    }

    /// Returns the raw RPC binding handle.
    ///
    /// This is used internally by the generated client code to make RPC calls.
    #[doc(hidden)]
    pub fn handle(&self) -> *mut c_void {
        self.handle
    }
}
