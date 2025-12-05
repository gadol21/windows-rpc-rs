use std::ffi::c_void;
use windows::core::{PCWSTR, Error, HSTRING};
use windows::Win32::System::Rpc::{
    RpcServerRegisterIf3, RpcServerUnregisterIf, RpcServerUseProtseqEpW,
    RpcServerListen, RpcMgmtStopServerListening, RPC_C_LISTEN_MAX_CALLS_DEFAULT,
};

/// Protocol sequences supported by the server
#[derive(Debug, Clone, Copy)]
pub enum ProtocolSequence {
    /// ALPC/LPC (ncalrpc) - Local RPC
    Alpc,
}

impl ProtocolSequence {
    fn to_pcwstr(&self) -> PCWSTR {
        match self {
            ProtocolSequence::Alpc => windows::core::w!("ncalrpc"),
        }
    }
}

/// Server binding wrapper for managing RPC server lifecycle
pub struct ServerBinding {
    protocol: ProtocolSequence,
    endpoint: String,
    interface_handle: *const c_void,
    registered: bool,
}

impl ServerBinding {
    /// Create a new server binding
    /// Note: This does not register the interface yet - call register() to do that
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
            ).ok()?;
        }

        Ok(ServerBinding {
            protocol,
            endpoint,
            interface_handle,
            registered: false,
        })
    }

    /// Register the RPC interface
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
            ).ok()?;
        }

        self.registered = true;
        Ok(())
    }

    /// Start listening for RPC calls (blocking)
    /// This will block until stop() is called from another thread or the server is shut down
    pub fn listen(&self) -> Result<(), Error> {
        if !self.registered {
            return Err(Error::from_hresult(windows::core::HRESULT(-1)));
        }

        unsafe {
            RpcServerListen(
                1, // MinimumCallThreads
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                0, // DontWait = false (blocking)
            ).ok()?;
        }

        Ok(())
    }

    /// Start listening for RPC calls (non-blocking)
    /// Returns immediately and processes calls in background threads
    pub fn listen_async(&self) -> Result<(), Error> {
        if !self.registered {
            return Err(Error::from_hresult(windows::core::HRESULT(-1)));
        }

        unsafe {
            RpcServerListen(
                1, // MinimumCallThreads
                RPC_C_LISTEN_MAX_CALLS_DEFAULT,
                1, // DontWait = true (non-blocking)
            ).ok()?;
        }

        Ok(())
    }

    /// Stop the server from listening
    pub fn stop(&self) -> Result<(), Error> {
        unsafe {
            RpcMgmtStopServerListening(None).ok()?;
        }
        Ok(())
    }

    /// Unregister the interface
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

    /// Get the endpoint name
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get the protocol sequence
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
