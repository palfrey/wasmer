use crate::wasm_c_api::store::wasm_store_t;
use libc::c_void;
use wasmer_api::Context;

/// Opaque type representing a WebAssembly context.
#[allow(non_camel_case_types)]
pub struct wasm_context_t {
    pub(crate) inner: Context<*mut c_void>,
}

impl core::fmt::Debug for wasm_context_t {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "wasm_context_t")
    }
}

/// Creates a new WebAssembly Context given a specific [engine][super::engine].
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_context_new(
    store: Option<&wasm_store_t>,
    data: *mut c_void,
) -> Option<Box<wasm_context_t>> {
    let store = store?;

    Some(Box::new(wasm_context_t {
        inner: Context::new(&store.inner, data),
    }))
}

/// Deletes a WebAssembly context.
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_context_delete(_context: Option<Box<wasm_context_t>>) {}
