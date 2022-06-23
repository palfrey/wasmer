//! Unstable non-standard Wasmer-specific API that contains more WASI
//! API.

use super::super::{
    externals::wasm_extern_t, module::wasm_module_t, store::wasm_store_t, types::wasm_name_t,
    wasi::wasi_env_t,
};
use wasmer_api::{AsContextMut, Extern};
use wasmer_wasi::{generate_import_object_from_ctx, get_wasi_version};

/// Unstable non-standard type wrapping `wasm_extern_t` with the
/// addition of two `wasm_name_t` respectively for the module name and
/// the name of the extern (very likely to be an import). This
/// non-standard type is used by the unstable non-standard
/// `wasi_get_unordered_imports` function.
///
/// The `module`, `name` and `extern` fields are all owned by this type.
#[allow(non_camel_case_types)]
#[derive(Clone)]
pub struct wasmer_named_extern_t {
    module: wasm_name_t,
    name: wasm_name_t,
    r#extern: Box<wasm_extern_t>,
}

wasm_declare_boxed_vec!(named_extern, wasmer);

/// So. Let's explain a dirty hack. `cbindgen` reads the code and
/// collects symbols. What symbols do we need? None of the one
/// declared in `wasm.h`, but for non-standard API, we need to collect
/// all of them. The problem is that `wasmer_named_extern_t` is the only
/// non-standard type where extra symbols are generated by a macro
/// (`wasm_declare_boxed_vec!`). If we want those macro-generated
/// symbols to be collected by `cbindgen`, we need to _expand_ the
/// crate (i.e. running something like `rustc -- -Zunstable-options
/// --pretty=expanded`). Expanding code is unstable and available only
/// on nightly compiler. We _don't want_ to use a nightly compiler
/// only for that. So how can we help `cbindgen` to _see_ those
/// symbols?
///
/// First solution: We write the C code directly in a file, which is
/// then included in the generated header file with the `cbindgen`
/// API. Problem, it's super easy to get it outdated, and it makes the
/// build process more complex.
///
/// Second solution: We write those symbols in a custom module, that
/// is just here for `cbindgen`, never used by our Rust code
/// (otherwise it's duplicated code), with no particular
/// implementation.
///
/// And that's why we have the following `cbindgen_hack`
/// module.
///
/// But this module must not be compiled by `rustc`. How to force
/// `rustc` to ignore a module? With conditional compilation. Because
/// `cbindgen` does not support conditional compilation, it will
/// always _ignore_ the `#[cfg]` attribute, and will always read the
/// content of the module.
///
/// Sorry.
#[doc(hidden)]
#[cfg(__cbindgen_hack__ = "yes")]
mod __cbindgen_hack__ {
    use super::*;

    #[repr(C)]
    pub struct wasmer_named_extern_vec_t {
        pub size: usize,
        pub data: *mut *mut wasmer_named_extern_t,
    }

    #[no_mangle]
    pub unsafe extern "C" fn wasmer_named_extern_vec_new(
        out: *mut wasmer_named_extern_vec_t,
        length: usize,
        init: *const *mut wasmer_named_extern_t,
    ) {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern "C" fn wasmer_named_extern_vec_new_uninitialized(
        out: *mut wasmer_named_extern_vec_t,
        length: usize,
    ) {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern "C" fn wasmer_named_extern_vec_copy(
        out_ptr: &mut wasmer_named_extern_vec_t,
        in_ptr: &wasmer_named_extern_vec_t,
    ) {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern "C" fn wasmer_named_extern_vec_delete(
        ptr: Option<&mut wasmer_named_extern_vec_t>,
    ) {
        unimplemented!()
    }

    #[no_mangle]
    pub unsafe extern "C" fn wasmer_named_extern_vec_new_empty(
        out: *mut wasmer_named_extern_vec_t,
    ) {
        unimplemented!()
    }
}

/// Non-standard function to get the module name of a
/// `wasmer_named_extern_t`.
///
/// The returned value isn't owned by the caller.
#[no_mangle]
pub extern "C" fn wasmer_named_extern_module(
    named_extern: Option<&wasmer_named_extern_t>,
) -> Option<&wasm_name_t> {
    Some(&named_extern?.module)
}

/// Non-standard function to get the name of a `wasmer_named_extern_t`.
///
/// The returned value isn't owned by the caller.
#[no_mangle]
pub extern "C" fn wasmer_named_extern_name(
    named_extern: Option<&wasmer_named_extern_t>,
) -> Option<&wasm_name_t> {
    Some(&named_extern?.name)
}

/// Non-standard function to get the wrapped extern of a
/// `wasmer_named_extern_t`.
///
/// The returned value isn't owned by the caller.
#[no_mangle]
pub extern "C" fn wasmer_named_extern_unwrap(
    named_extern: Option<&wasmer_named_extern_t>,
) -> Option<&wasm_extern_t> {
    Some(named_extern?.r#extern.as_ref())
}

/// Non-standard function to get the imports needed for the WASI
/// implementation with no particular order. Each import has its
/// associated module name and name, so that it can be re-order later
/// based on the `wasm_module_t` requirements.
#[no_mangle]
pub unsafe extern "C" fn wasi_get_unordered_imports(
    store: Option<&wasm_store_t>,
    module: Option<&wasm_module_t>,
    wasi_env: Option<&wasi_env_t>,
    imports: &mut wasmer_named_extern_vec_t,
) -> bool {
    wasi_get_unordered_imports_inner(store, module, wasi_env, imports).is_some()
}

fn wasi_get_unordered_imports_inner(
    store: Option<&wasm_store_t>,
    module: Option<&wasm_module_t>,
    wasi_env: Option<&wasi_env_t>,
    imports: &mut wasmer_named_extern_vec_t,
) -> Option<()> {
    let store = store?;
    let ctx = store.context.as_ref()?;
    let module = module?;
    let _wasi_env = wasi_env?;

    let mut lck = ctx.lock().unwrap();
    let version = c_try!(get_wasi_version(&module.inner, false)
        .ok_or("could not detect a WASI version on the given module"));

    let inner = unsafe { lck.inner.transmute_data::<wasmer_wasi::WasiEnv>() };
    let import_object = generate_import_object_from_ctx(&mut inner.as_context_mut(), version);

    imports.set_buffer(
        import_object
            .into_iter()
            .map(|((module, name), extern_)| {
                let module = module.into();
                let name = name.into();
                let extern_inner = Extern::from_vm_extern(&mut lck.inner, extern_.to_vm_extern());

                Some(Box::new(wasmer_named_extern_t {
                    module,
                    name,
                    r#extern: Box::new(extern_inner.into()),
                }))
            })
            .collect::<Vec<_>>(),
    );

    Some(())
}
