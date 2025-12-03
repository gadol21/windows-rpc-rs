use std::{ffi::c_void, ptr};

use windows::{
    Win32::System::Rpc::{RpcBindingFromStringBindingW, RpcStringBindingComposeW},
    core::HSTRING,
};

pub enum ProtocolSequence {
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

pub struct ClientBinding {
    handle: *mut c_void,
}

impl ClientBinding {
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

    pub fn handle(&self) -> *mut c_void {
        self.handle
    }
}
