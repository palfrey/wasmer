use super::context::wasm_context_t;
use super::engine::wasm_engine_t;
use std::sync::{Arc, Mutex};
use wasmer_api::Store;

/// Opaque type representing a WebAssembly store.
#[allow(non_camel_case_types)]
pub struct wasm_store_t {
    pub(crate) inner: Store,
    pub(crate) context: Option<Arc<Mutex<wasm_context_t>>>,
}

/// Creates a new WebAssembly store given a specific [engine][super::engine].
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_store_new(
    engine: Option<&wasm_engine_t>,
) -> Option<Box<wasm_store_t>> {
    let engine = engine?;
    let store = Store::new_with_engine(&*engine.inner);

    Some(Box::new(wasm_store_t {
        inner: store,
        context: None,
    }))
}

/// Sets the context for this WebAssembly store.
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_store_context_set(
    store: Option<&mut wasm_store_t>,
    context: Option<Box<wasm_context_t>>,
) {
    let _result = (move |store: Option<&mut wasm_store_t>,
                         context: Option<Box<wasm_context_t>>|
          -> Option<()> {
        let mut store = store?;
        let context = context?;
        store.context = Some(Arc::new(Mutex::new(*context)));
        Some(())
    })(store, context);
}

/// Deletes a WebAssembly store.
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_store_delete(_store: Option<Box<wasm_store_t>>) {}
