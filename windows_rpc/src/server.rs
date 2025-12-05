use std::sync::RwLock;

// Store the fat pointer as two usize values to preserve vtable
// FIXME: ...
//thread_local! {
static SERVER_CONTEXT: RwLock<Option<(usize, usize)>> = RwLock::new(None);
//}

/// Set the server context for the current thread
/// SAFETY: The caller must ensure that the pointer remains valid for the duration
/// of the context (i.e., while executing the RPC call)
pub unsafe fn set_context<T: ?Sized>(ptr: *const T) {
    // Transmute the fat pointer to two usize values (data pointer and vtable)
    let raw: (usize, usize) = unsafe { std::mem::transmute_copy(&ptr) };
    if let Some(_prev_context) = SERVER_CONTEXT.write().unwrap().replace(raw) {
        panic!("Multiple rpc servers are currently not supported");
    }
}

/// Clear the server context for the current thread
pub fn clear_context() {
    let _ = SERVER_CONTEXT.write().unwrap().take();
}

/// Execute a function with access to the server implementation
/// This is used by generated wrapper functions to access the trait implementation
pub fn with_context<T: ?Sized + 'static, R, F>(f: F) -> R
where
    F: FnOnce(&T) -> R,
{
    let raw = SERVER_CONTEXT
        .write()
        .unwrap()
        .expect("No server context available - this should only be called from RPC callbacks");

    let ptr: *const T = unsafe { std::mem::transmute_copy(&raw) };
    let typed_ptr = unsafe { &*ptr };
    f(typed_ptr)
}
