//! RPC server binding management.
//!
//! This module provides types for creating and managing RPC server bindings,
//! which control the server lifecycle: registration, listening, and shutdown.

use std::ffi::c_void;
use windows::Win32::System::Rpc::{
    RPC_C_LISTEN_MAX_CALLS_DEFAULT, RpcMgmtStopServerListening, RpcServerListen,
    RpcServerRegisterIf3, RpcServerUnregisterIf, RpcServerUseProtseqEpW,
};
use windows::core::{Error, HSTRING, PCWSTR};

use crate::ProtocolSequence;

/// Manages the lifecycle of an RPC server.
///
/// This struct handles the low-level details of registering an RPC interface
/// with the Windows RPC runtime and managing the listen/stop lifecycle.
///
/// # Note
///
/// You typically don't create `ServerBinding` directly. Instead, use the
/// generated `{Interface}Server` struct which manages this for you.
///
/// # Example
///
/// ```rust,no_run
/// use windows_rpc::rpc_interface;
///
/// #[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
/// trait MyInterface {
///     fn hello() -> i32;
/// }
///
/// struct MyImpl;
/// impl MyInterfaceServerImpl for MyImpl {
///     fn hello(&self) -> i32 { 42 }
/// }
///
/// # fn main() -> windows::core::Result<()> {
/// let mut server = MyInterfaceServer::new(MyImpl);
/// server.register("my_endpoint")?;
/// server.listen_async()?;
/// // ... server is now accepting calls ...
/// server.stop()?;
/// # Ok(())
/// # }
/// ```
pub struct ServerBinding {
    protocol: ProtocolSequence,
    endpoint: String,
    interface_handle: *const c_void,
    registered: bool,
}

impl ServerBinding {
    /// Creates a new server binding for the specified endpoint.
    ///
    /// This registers the protocol sequence and endpoint with the RPC runtime,
    /// but does not yet register the interface. Call [`register()`](Self::register)
    /// to complete the registration.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol sequence to use
    /// * `endpoint` - The endpoint name clients will connect to
    /// * `interface_handle` - Pointer to the RPC interface specification
    ///
    /// # Errors
    ///
    /// Returns an error if the protocol sequence and endpoint cannot be registered.
    pub fn new(
        protocol: ProtocolSequence,
        endpoint: impl Into<String>,
        interface_handle: *const c_void,
    ) -> Result<Self, Error> {
        let endpoint = endpoint.into();
        let endpoint_hstring = HSTRING::from(&endpoint);

        // Register the protocol sequence and endpoint
        unsafe {
            RpcServerUseProtseqEpW(
                protocol.to_pcwstr(),
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                PCWSTR::from_raw(endpoint_hstring.as_ptr()),
                None, // No security descriptor
            )
            .ok()?;
        }

        Ok(ServerBinding {
            protocol,
            endpoint,
            interface_handle,
            registered: false,
        })
    }

    /// Registers the RPC interface with the runtime.
    ///
    /// After registration, the server can begin accepting calls. This method
    /// is idempotent - calling it multiple times has no effect.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface cannot be registered.
    pub fn register(&mut self) -> Result<(), Error> {
        if self.registered {
            return Ok(());
        }

        unsafe {
            RpcServerRegisterIf3(
                self.interface_handle,
                None, // Interface UUID (use from handle)
                None, // Manager EPV
                0,    // Flags
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                u32::MAX, // Max RPC size
                None,     // Security callback
                None,     // Security descriptor
            )
            .ok()?;
        }

        self.registered = true;
        Ok(())
    }

    /// Starts listening for RPC calls (blocking).
    ///
    /// This method blocks the current thread until [`stop()`](Self::stop) is called
    /// from another thread. Use [`listen_async()`](Self::listen_async) for non-blocking
    /// operation.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The interface has not been registered
    /// - The RPC runtime fails to start listening
    pub fn listen(&self) -> Result<(), Error> {
        if !self.registered {
            return Err(Error::from_hresult(windows::core::HRESULT(-1)));
        }

        unsafe {
            RpcServerListen(
                1, // MinimumCallThreads
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                0, // DontWait = false (blocking)
            )
            .ok()?;
        }

        Ok(())
    }

    /// Starts listening for RPC calls (non-blocking).
    ///
    /// Returns immediately while RPC calls are processed in background threads
    /// managed by the Windows RPC runtime. Call [`stop()`](Self::stop) to shut
    /// down the server.
    ///
    /// This is the recommended mode for most applications as it allows the main
    /// thread to continue other work or wait for a shutdown signal.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The interface has not been registered
    /// - The RPC runtime fails to start listening
    pub fn listen_async(&self) -> Result<(), Error> {
        if !self.registered {
            return Err(Error::from_hresult(windows::core::HRESULT(-1)));
        }

        unsafe {
            RpcServerListen(
                1, // MinimumCallThreads
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                1, // DontWait = true (non-blocking)
            )
            .ok()?;
        }

        Ok(())
    }

    /// Stops the server from accepting new RPC calls.
    ///
    /// Outstanding calls may still complete. For a blocking server, this will
    /// cause [`listen()`](Self::listen) to return.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC runtime fails to stop.
    pub fn stop(&self) -> Result<(), Error> {
        unsafe {
            RpcMgmtStopServerListening(None).ok()?;
        }
        Ok(())
    }

    /// Unregisters the RPC interface.
    ///
    /// This is called automatically when the `ServerBinding` is dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if the interface cannot be unregistered.
    pub fn unregister(&mut self) -> Result<(), Error> {
        if !self.registered {
            return Ok(());
        }

        unsafe {
            RpcServerUnregisterIf(Some(self.interface_handle), None, 1).ok()?;
        }

        self.registered = false;
        Ok(())
    }

    /// Returns the endpoint name.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the protocol sequence.
    pub fn protocol(&self) -> ProtocolSequence {
        self.protocol
    }
}

impl Drop for ServerBinding {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.unregister();
    }
}
