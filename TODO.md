# Todo item list
1. Add support for sized strings and buffers **input only**
1. Add support for sized strings and buffers as return value
1. Add support for int/out params (pass &mut u32)
1. Add support for out params (`&mut MaybeUninit<T>`?)
1. Check with heap verifier
1. Add support for binding context to a server instance (to pass &self param)
1. Reexport windows types from windows_rpc, and use them in the macros crate? (to not force users of our crates to add windows and windows-sys dependencies)
1. Handle SEH errors (for example server unavailable is a very common one, access denied too)
1. Expose ways to secure access to servers
1. Test arm64
1. Generate stubs from .idl files

## Consider implementing
1. Pass COM interfaces
1. Test 32 bit Windows
