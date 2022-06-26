mod function;
mod global;
mod memory;
mod table;

use super::context::wasm_context_t;
pub use function::*;
pub use global::*;
pub use memory::*;
use std::mem::{self, ManuallyDrop};
use std::sync::{Arc, Mutex};
pub use table::*;
use wasmer_api::{Extern, ExternType};

#[allow(non_camel_case_types)]
#[repr(transparent)]
pub struct wasm_extern_t {
    pub(crate) inner: wasm_extern_inner,
}

/// All elements in this union must be `repr(C)` and have a
/// `CApiExternTag` as their first element.
#[allow(non_camel_case_types)]
pub(crate) union wasm_extern_inner {
    function: mem::ManuallyDrop<wasm_func_t>,
    memory: mem::ManuallyDrop<wasm_memory_t>,
    global: mem::ManuallyDrop<wasm_global_t>,
    table: mem::ManuallyDrop<wasm_table_t>,
}

#[cfg(test)]
mod extern_tests {
    use super::*;

    #[test]
    fn externs_are_the_same_size() {
        use std::mem::{align_of, size_of};
        assert_eq!(size_of::<wasm_extern_t>(), size_of::<wasm_func_t>());
        assert_eq!(size_of::<wasm_extern_t>(), size_of::<wasm_memory_t>());
        assert_eq!(size_of::<wasm_extern_t>(), size_of::<wasm_global_t>());
        assert_eq!(size_of::<wasm_extern_t>(), size_of::<wasm_table_t>());

        assert_eq!(align_of::<wasm_extern_t>(), align_of::<wasm_func_t>());
        assert_eq!(align_of::<wasm_extern_t>(), align_of::<wasm_memory_t>());
        assert_eq!(align_of::<wasm_extern_t>(), align_of::<wasm_global_t>());
        assert_eq!(align_of::<wasm_extern_t>(), align_of::<wasm_table_t>());
    }

    #[test]
    fn tags_are_the_same_offset_away() {
        use field_offset::offset_of;

        let func_tag_offset = offset_of!(wasm_func_t => tag).get_byte_offset();
        let memory_tag_offset = offset_of!(wasm_memory_t => tag).get_byte_offset();
        let global_tag_offset = offset_of!(wasm_global_t => tag).get_byte_offset();
        let table_tag_offset = offset_of!(wasm_table_t => tag).get_byte_offset();

        assert_eq!(func_tag_offset, memory_tag_offset);
        assert_eq!(global_tag_offset, table_tag_offset);
        assert_eq!(func_tag_offset, global_tag_offset);
    }
}

impl Drop for wasm_extern_inner {
    fn drop(&mut self) {
        unsafe {
            match self.function.tag {
                CApiExternTag::Function => mem::ManuallyDrop::drop(&mut self.function),
                CApiExternTag::Global => mem::ManuallyDrop::drop(&mut self.global),
                CApiExternTag::Table => mem::ManuallyDrop::drop(&mut self.table),
                CApiExternTag::Memory => mem::ManuallyDrop::drop(&mut self.memory),
            }
        }
    }
}

impl wasm_extern_t {
    pub(crate) fn get_tag(&self) -> CApiExternTag {
        unsafe { self.inner.function.tag }
    }

    pub(crate) fn ty(&self) -> ExternType {
        match self.get_tag() {
            CApiExternTag::Function => unsafe {
                let ctx = self.inner.function.context.as_ref().unwrap();
                let lck = ctx.lock().unwrap();
                ExternType::Function(self.inner.function.inner.ty(&lck.inner))
            },
            CApiExternTag::Memory => unsafe {
                let ctx = self.inner.memory.context.as_ref().unwrap();
                let lck = ctx.lock().unwrap();
                ExternType::Memory(self.inner.memory.inner.ty(&lck.inner))
            },
            CApiExternTag::Global => unsafe {
                let ctx = self.inner.global.context.as_ref().unwrap();
                let lck = ctx.lock().unwrap();
                ExternType::Global(self.inner.global.inner.ty(&lck.inner))
            },
            CApiExternTag::Table => unsafe {
                let ctx = self.inner.table.context.as_ref().unwrap();
                let lck = ctx.lock().unwrap();
                ExternType::Table(self.inner.table.inner.ty(&lck.inner))
            },
        }
    }

    pub(crate) fn set_context(&mut self, new_val: Option<Arc<Mutex<wasm_context_t>>>) {
        match self.get_tag() {
            CApiExternTag::Function => unsafe {
                (*self.inner.function).context = new_val;
            },
            CApiExternTag::Memory => unsafe {
                (*self.inner.memory).context = new_val;
            },
            CApiExternTag::Global => unsafe {
                (*self.inner.global).context = new_val;
            },
            CApiExternTag::Table => unsafe {
                (*self.inner.table).context = new_val;
            },
        }
    }
}

impl Clone for wasm_extern_t {
    fn clone(&self) -> Self {
        match self.get_tag() {
            CApiExternTag::Function => Self {
                inner: wasm_extern_inner {
                    function: unsafe { self.inner.function.clone() },
                },
            },
            CApiExternTag::Memory => Self {
                inner: wasm_extern_inner {
                    memory: unsafe { self.inner.memory.clone() },
                },
            },
            CApiExternTag::Global => Self {
                inner: wasm_extern_inner {
                    global: unsafe { self.inner.global.clone() },
                },
            },
            CApiExternTag::Table => Self {
                inner: wasm_extern_inner {
                    table: unsafe { self.inner.table.clone() },
                },
            },
        }
    }
}

impl From<Extern> for wasm_extern_t {
    fn from(other: Extern) -> Self {
        match other {
            Extern::Function(function) => Self {
                inner: wasm_extern_inner {
                    function: mem::ManuallyDrop::new(wasm_func_t::new(function)),
                },
            },
            Extern::Memory(memory) => Self {
                inner: wasm_extern_inner {
                    memory: mem::ManuallyDrop::new(wasm_memory_t::new(memory)),
                },
            },
            Extern::Table(table) => Self {
                inner: wasm_extern_inner {
                    table: mem::ManuallyDrop::new(wasm_table_t::new(table)),
                },
            },
            Extern::Global(global) => Self {
                inner: wasm_extern_inner {
                    global: mem::ManuallyDrop::new(wasm_global_t::new(global)),
                },
            },
        }
    }
}

impl From<wasm_extern_t> for Extern {
    fn from(mut other: wasm_extern_t) -> Self {
        let out = match other.get_tag() {
            CApiExternTag::Function => unsafe {
                (*ManuallyDrop::take(&mut other.inner.function).inner).into()
            },
            CApiExternTag::Memory => unsafe {
                (*ManuallyDrop::take(&mut other.inner.memory).inner).into()
            },
            CApiExternTag::Table => unsafe {
                (*ManuallyDrop::take(&mut other.inner.table).inner).into()
            },
            CApiExternTag::Global => unsafe {
                (*ManuallyDrop::take(&mut other.inner.global).inner).into()
            },
        };
        mem::forget(other);
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub(crate) enum CApiExternTag {
    Function,
    Global,
    Table,
    Memory,
}

wasm_declare_boxed_vec!(extern);

/// Copy a `wasm_extern_t`.
#[no_mangle]
pub unsafe extern "C" fn wasm_extern_copy(r#extern: &wasm_extern_t) -> Box<wasm_extern_t> {
    Box::new(r#extern.clone())
}

/// Delete an extern.
#[no_mangle]
pub unsafe extern "C" fn wasm_extern_delete(_extern: Option<Box<wasm_extern_t>>) {}

#[no_mangle]
pub extern "C" fn wasm_func_as_extern(func: Option<&wasm_func_t>) -> Option<&wasm_extern_t> {
    unsafe { mem::transmute::<Option<&wasm_func_t>, Option<&wasm_extern_t>>(func) }
}

#[no_mangle]
pub extern "C" fn wasm_global_as_extern(global: Option<&wasm_global_t>) -> Option<&wasm_extern_t> {
    unsafe { mem::transmute::<Option<&wasm_global_t>, Option<&wasm_extern_t>>(global) }
}

#[no_mangle]
pub extern "C" fn wasm_memory_as_extern(memory: Option<&wasm_memory_t>) -> Option<&wasm_extern_t> {
    unsafe { mem::transmute::<Option<&wasm_memory_t>, Option<&wasm_extern_t>>(memory) }
}

#[no_mangle]
pub extern "C" fn wasm_table_as_extern(table: Option<&wasm_table_t>) -> Option<&wasm_extern_t> {
    unsafe { mem::transmute::<Option<&wasm_table_t>, Option<&wasm_extern_t>>(table) }
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_func(r#extern: Option<&wasm_extern_t>) -> Option<&wasm_func_t> {
    let r#extern = r#extern?;

    if r#extern.get_tag() == CApiExternTag::Function {
        Some(unsafe { mem::transmute::<&wasm_extern_t, &wasm_func_t>(r#extern) })
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_global(
    r#extern: Option<&wasm_extern_t>,
) -> Option<&wasm_global_t> {
    let r#extern = r#extern?;

    if r#extern.get_tag() == CApiExternTag::Global {
        Some(unsafe { mem::transmute::<&wasm_extern_t, &wasm_global_t>(r#extern) })
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_memory(
    r#extern: Option<&wasm_extern_t>,
) -> Option<&wasm_memory_t> {
    let r#extern = r#extern?;

    if r#extern.get_tag() == CApiExternTag::Memory {
        Some(unsafe { mem::transmute::<&wasm_extern_t, &wasm_memory_t>(r#extern) })
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn wasm_extern_as_table(r#extern: Option<&wasm_extern_t>) -> Option<&wasm_table_t> {
    let r#extern = r#extern?;

    if r#extern.get_tag() == CApiExternTag::Table {
        Some(unsafe { mem::transmute::<&wasm_extern_t, &wasm_table_t>(r#extern) })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use inline_c::assert_c;

    #[test]
    fn test_extern_copy() {
        (assert_c! {
            #include "tests/wasmer.h"

            int main() {
                wasm_engine_t* engine = wasm_engine_new();
                wasm_store_t* store = wasm_store_new(engine);
                wasm_context_t* ctx = wasm_context_new(store, 0);
                wasm_store_context_set(store, ctx);

                wasm_byte_vec_t wat;
                wasmer_byte_vec_new_from_string(
                    &wat,
                    "(module\n"
                    "  (func (export \"function\")))"
                );
                wasm_byte_vec_t wasm;
                wat2wasm(&wat, &wasm);

                wasm_module_t* module = wasm_module_new(store, &wasm);
                assert(module);

                wasm_extern_vec_t imports = WASM_EMPTY_VEC;
                wasm_trap_t* trap = NULL;

                wasm_instance_t* instance = wasm_instance_new(store, module, &imports, &trap);
                assert(instance);

                wasm_extern_vec_t exports;
                wasm_instance_exports(instance, &exports);

                assert(exports.size == 1);

                wasm_extern_t* function = exports.data[0];
                assert(wasm_extern_kind(function) == WASM_EXTERN_FUNC);

                wasm_extern_t* function_copy = wasm_extern_copy(function);
                assert(wasm_extern_kind(function_copy) == WASM_EXTERN_FUNC);

                wasm_extern_delete(function_copy);
                wasm_extern_vec_delete(&exports);
                wasm_instance_delete(instance);
                wasm_module_delete(module);
                wasm_byte_vec_delete(&wasm);
                wasm_byte_vec_delete(&wat);
                wasm_store_delete(store);
                wasm_engine_delete(engine);

                return 0;
            }
        })
        .success();
    }
}
