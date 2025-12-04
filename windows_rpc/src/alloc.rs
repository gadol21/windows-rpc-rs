use std::alloc::Layout;

pub extern "system" fn midl_alloc(size: usize) -> *mut core::ffi::c_void {
    let layout =
        unsafe { Layout::from_size_align_unchecked(size + std::mem::size_of::<Layout>(), 1) };
    let ptr = unsafe { std::alloc::alloc(layout) };
    assert!(!ptr.is_null());

    unsafe {
        let layout_ptr = ptr.cast::<Layout>();
        layout_ptr.write(layout);
    }

    unsafe { ptr.add(std::mem::size_of::<Layout>()) as *mut core::ffi::c_void }
}

pub extern "system" fn midl_free(ptr: *mut core::ffi::c_void) {
    let ptr = ptr as *mut u8;
    let ptr = unsafe { ptr.sub(std::mem::size_of::<Layout>()) };
    let layout_ptr = unsafe { *ptr.cast::<Layout>() };
    unsafe { std::alloc::dealloc(ptr, layout_ptr) };
}
