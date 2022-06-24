use crate::js::context::{AsContextMut, AsContextRef, InternalContextHandle};
use crate::js::error::WasmError;
use crate::js::wasm_bindgen_polyfill::Global;
use js_sys::Function;
use js_sys::WebAssembly::{Memory, Table};
use std::fmt;
use wasm_bindgen::{JsCast, JsValue};
use wasmer_types::{ExternType, FunctionType, GlobalType, MemoryType, TableType};

#[derive(Clone, Debug, PartialEq)]
pub struct VMMemory {
    pub(crate) memory: Memory,
    pub(crate) ty: MemoryType,
}

unsafe impl Send for VMMemory {}
unsafe impl Sync for VMMemory {}

impl VMMemory {
    pub(crate) fn new(memory: Memory, ty: MemoryType) -> Self {
        Self { memory, ty }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VMGlobal {
    pub(crate) global: Global,
    pub(crate) ty: GlobalType,
}

impl VMGlobal {
    pub(crate) fn new(global: Global, ty: GlobalType) -> Self {
        Self { global, ty }
    }
}

unsafe impl Send for VMGlobal {}
unsafe impl Sync for VMGlobal {}

#[derive(Clone, Debug, PartialEq)]
pub struct VMTable {
    pub(crate) table: Table,
    pub(crate) ty: TableType,
}

unsafe impl Send for VMTable {}
unsafe impl Sync for VMTable {}

impl VMTable {
    pub(crate) fn new(table: Table, ty: TableType) -> Self {
        Self { table, ty }
    }
}

#[derive(Clone)]
pub struct VMFunction {
    pub(crate) function: Function,
    pub(crate) ty: FunctionType,
}

unsafe impl Send for VMFunction {}
unsafe impl Sync for VMFunction {}

impl VMFunction {
    pub(crate) fn new(function: Function, ty: FunctionType) -> Self {
        Self { function, ty }
    }
}

impl PartialEq for VMFunction {
    fn eq(&self, other: &Self) -> bool {
        self.function == other.function
    }
}

impl fmt::Debug for VMFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VMFunction")
            .field("function", &self.function)
            .finish()
    }
}

/// The value of an export passed from one instance to another.
#[derive(Debug, Clone)]
pub enum Export {
    /// A function export value.
    Function(InternalContextHandle<VMFunction>),

    /// A table export value.
    Table(InternalContextHandle<VMTable>),

    /// A memory export value.
    Memory(InternalContextHandle<VMMemory>),

    /// A global export value.
    Global(InternalContextHandle<VMGlobal>),
}

impl Export {
    /// Return the export as a `JSValue`.
    pub fn as_jsvalue<'context>(&self, ctx: &'context impl AsContextRef) -> &'context JsValue {
        match self {
            Self::Memory(js_wasm_memory) => js_wasm_memory
                .get(ctx.as_context_ref().objects())
                .memory
                .as_ref(),
            Self::Function(js_func) => js_func
                .get(ctx.as_context_ref().objects())
                .function
                .as_ref(),
            Self::Table(js_wasm_table) => js_wasm_table
                .get(ctx.as_context_ref().objects())
                .table
                .as_ref(),
            Self::Global(js_wasm_global) => js_wasm_global
                .get(ctx.as_context_ref().objects())
                .global
                .as_ref(),
        }
    }

    /// Convert a `JsValue` into an `Export` within a given `Context`.
    pub fn from_js_value(
        val: JsValue,
        ctx: &mut impl AsContextMut,
        extern_type: ExternType,
    ) -> Result<Self, WasmError> {
        match extern_type {
            ExternType::Memory(memory_type) => {
                if val.is_instance_of::<Memory>() {
                    Ok(Self::Memory(InternalContextHandle::new(
                        &mut ctx.as_context_mut().objects_mut(),
                        VMMemory::new(val.unchecked_into::<Memory>(), memory_type),
                    )))
                } else {
                    Err(WasmError::TypeMismatch(
                        val.js_typeof()
                            .as_string()
                            .map(Into::into)
                            .unwrap_or("unknown".into()),
                        "Memory".into(),
                    ))
                }
            }
            ExternType::Global(global_type) => {
                if val.is_instance_of::<Global>() {
                    Ok(Self::Global(InternalContextHandle::new(
                        &mut ctx.as_context_mut().objects_mut(),
                        VMGlobal::new(val.unchecked_into::<Global>(), global_type),
                    )))
                } else {
                    panic!("Extern type doesn't match js value type");
                }
            }
            ExternType::Function(function_type) => {
                if val.is_instance_of::<Function>() {
                    Ok(Self::Function(InternalContextHandle::new(
                        &mut ctx.as_context_mut().objects_mut(),
                        VMFunction::new(val.unchecked_into::<Function>(), function_type),
                    )))
                } else {
                    panic!("Extern type doesn't match js value type");
                }
            }
            ExternType::Table(table_type) => {
                if val.is_instance_of::<Table>() {
                    Ok(Self::Table(InternalContextHandle::new(
                        &mut ctx.as_context_mut().objects_mut(),
                        VMTable::new(val.unchecked_into::<Table>(), table_type),
                    )))
                } else {
                    panic!("Extern type doesn't match js value type");
                }
            }
        }
    }
}
