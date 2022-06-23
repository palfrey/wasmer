use super::super::context::wasm_context_t;
use super::super::store::wasm_store_t;
use super::super::types::wasm_memorytype_t;
use super::CApiExternTag;
use std::sync::{Arc, Mutex};
use wasmer_api::{Memory, Pages};

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct wasm_memory_t {
    pub(crate) tag: CApiExternTag,
    pub(crate) inner: Box<Memory>,
    pub(crate) context: Option<Arc<Mutex<wasm_context_t>>>,
}

impl wasm_memory_t {
    pub(crate) fn new(memory: Memory) -> Self {
        Self {
            tag: CApiExternTag::Memory,
            inner: Box::new(memory),
            context: None,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_new(
    store: Option<&mut wasm_store_t>,
    memory_type: Option<&wasm_memorytype_t>,
) -> Option<Box<wasm_memory_t>> {
    let memory_type = memory_type?;
    let store = store?;
    let ctx = store.context.as_mut()?;

    let mut lck = ctx.lock().unwrap();
    let memory_type = memory_type.inner().memory_type;
    let memory = c_try!(Memory::new(&mut lck.inner, memory_type));
    drop(lck);
    let mut retval = Box::new(wasm_memory_t::new(memory));
    retval.context = store.context.clone();
    Some(retval)
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_delete(_memory: Option<Box<wasm_memory_t>>) {}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_copy(memory: &wasm_memory_t) -> Box<wasm_memory_t> {
    // do shallow copy
    Box::new(wasm_memory_t::new((&*memory.inner).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn wasm_memory_type(
    memory: Option<&wasm_memory_t>,
) -> Option<Box<wasm_memorytype_t>> {
    let memory = memory?;
    let ctx = memory.context.as_ref()?;
    let lck = ctx.lock().unwrap();

    Some(Box::new(wasm_memorytype_t::new(
        memory.inner.ty(&lck.inner),
    )))
}

// get a raw pointer into bytes
#[no_mangle]
pub unsafe extern "C" fn wasm_memory_data(memory: &mut wasm_memory_t) -> *mut u8 {
    let ctx = memory.context.as_ref().unwrap();
    let lck = ctx.lock().unwrap();
    memory.inner.data_ptr(&lck.inner)
}

// size in bytes
#[no_mangle]
pub unsafe extern "C" fn wasm_memory_data_size(memory: &wasm_memory_t) -> usize {
    let ctx = memory.context.as_ref().unwrap();
    let lck = ctx.lock().unwrap();
    memory.inner.size(&lck.inner).bytes().0
}

// size in pages
#[no_mangle]
pub unsafe extern "C" fn wasm_memory_size(memory: &wasm_memory_t) -> u32 {
    let ctx = memory.context.as_ref().unwrap();
    let lck = ctx.lock().unwrap();
    memory.inner.size(&lck.inner).0 as _
}

// delta is in pages
#[no_mangle]
pub unsafe extern "C" fn wasm_memory_grow(memory: &mut wasm_memory_t, delta: u32) -> bool {
    let ctx = &mut memory.context.as_ref().unwrap();
    let mut lck = ctx.lock().unwrap();
    memory.inner.grow(&mut lck.inner, Pages(delta)).is_ok()
}
