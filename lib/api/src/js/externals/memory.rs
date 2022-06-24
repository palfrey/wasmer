use crate::js::context::{
    AsContextMut, AsContextRef, ContextHandle, ContextObjects, InternalContextHandle,
};
use crate::js::export::VMMemory;
use crate::js::exports::{ExportError, Exportable};
use crate::js::externals::Extern;
use crate::js::{MemoryAccessError, MemoryType};
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::slice;
use thiserror::Error;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasmer_types::{Bytes, Pages};

/// Error type describing things that can go wrong when operating on Wasm Memories.
#[derive(Error, Debug, Clone, PartialEq, Hash)]
pub enum MemoryError {
    /// The operation would cause the size of the memory to exceed the maximum or would cause
    /// an overflow leading to unindexable memory.
    #[error("The memory could not grow: current size {} pages, requested increase: {} pages", current.0, attempted_delta.0)]
    CouldNotGrow {
        /// The current size in pages.
        current: Pages,
        /// The attempted amount to grow by in pages.
        attempted_delta: Pages,
    },
    /// A user defined error value, used for error cases not listed above.
    #[error("A user-defined error occurred: {0}")]
    Generic(String),
}

#[wasm_bindgen]
extern "C" {
    /// [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WebAssembly/Memory)
    #[wasm_bindgen(js_namespace = WebAssembly, extends = js_sys::Object, typescript_type = "WebAssembly.Memory")]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub type JSMemory;

    /// The `grow()` protoype method of the `Memory` object increases the
    /// size of the memory instance by a specified number of WebAssembly
    /// pages.
    ///
    /// Takes the number of pages to grow (64KiB in size) and returns the
    /// previous size of memory, in pages.
    ///
    /// # Reimplementation
    ///
    /// We re-implement `WebAssembly.Memory.grow` because it is
    /// different from what `wasm-bindgen` declares. It marks the function
    /// as `catch`, which means it can throw an exception.
    ///
    /// See [the opened patch](https://github.com/rustwasm/wasm-bindgen/pull/2599).
    ///
    /// # Exceptions
    ///
    /// A `RangeError` is thrown if adding pages would exceed the maximum
    /// memory.
    ///
    /// [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WebAssembly/Memory/grow)
    #[wasm_bindgen(catch, method, js_namespace = WebAssembly)]
    pub fn grow(this: &JSMemory, pages: u32) -> Result<u32, JsValue>;
}

/// A WebAssembly `memory` instance.
///
/// A memory instance is the runtime representation of a linear memory.
/// It consists of a vector of bytes and an optional maximum size.
///
/// The length of the vector always is a multiple of the WebAssembly
/// page size, which is defined to be the constant 65536 – abbreviated 64Ki.
/// Like in a memory type, the maximum size in a memory instance is
/// given in units of this page size.
///
/// A memory created by the host or in WebAssembly code will be accessible and
/// mutable from both host and WebAssembly.
///
/// Spec: <https://webassembly.github.io/spec/core/exec/runtime.html#memory-instances>
#[derive(Debug, Clone)]
pub struct Memory {
    pub(crate) handle: ContextHandle<VMMemory>,
    #[allow(dead_code)]
    view: js_sys::Uint8Array,
}

unsafe impl Send for Memory {}
unsafe impl Sync for Memory {}

impl Memory {
    /// Creates a new host `Memory` from the provided [`MemoryType`].
    ///
    /// This function will construct the `Memory` using the store
    /// [`BaseTunables`][crate::js::tunables::BaseTunables].
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmer::{Memory, MemoryType, Pages, Store, Type, Value};
    /// # let store = Store::default();
    /// #
    /// let m = Memory::new(&store, MemoryType::new(1, None, false)).unwrap();
    /// ```
    pub fn new(ctx: &mut impl AsContextMut, ty: MemoryType) -> Result<Self, MemoryError> {
        let descriptor = js_sys::Object::new();
        js_sys::Reflect::set(&descriptor, &"initial".into(), &ty.minimum.0.into()).unwrap();
        if let Some(max) = ty.maximum {
            js_sys::Reflect::set(&descriptor, &"maximum".into(), &max.0.into()).unwrap();
        }
        js_sys::Reflect::set(&descriptor, &"shared".into(), &ty.shared.into()).unwrap();

        let js_memory = js_sys::WebAssembly::Memory::new(&descriptor)
            .map_err(|_e| MemoryError::Generic("Error while creating the memory".to_owned()))?;

        let vm_memory = VMMemory::new(js_memory, ty);
        Ok(Self::from_vm_export(ctx, vm_memory))
    }

    /// Returns the [`MemoryType`] of the `Memory`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmer::{Memory, MemoryType, Pages, Store, Type, Value};
    /// # let store = Store::default();
    /// #
    /// let mt = MemoryType::new(1, None, false);
    /// let m = Memory::new(&store, mt).unwrap();
    ///
    /// assert_eq!(m.ty(), mt);
    /// ```
    pub fn ty(&self, ctx: &impl AsContextRef) -> MemoryType {
        self.handle.get(ctx.as_context_ref().objects()).ty
    }

    /// Returns the pointer to the raw bytes of the `Memory`.
    #[doc(hidden)]
    pub fn data_ptr(&self) -> *mut u8 {
        unimplemented!("direct data pointer access is not possible in JavaScript");
    }

    /// Returns the size (in bytes) of the `Memory`.
    pub fn data_size(&self, ctx: &impl AsContextRef) -> u64 {
        js_sys::Reflect::get(
            &self
                .handle
                .get(ctx.as_context_ref().objects())
                .memory
                .buffer(),
            &"byteLength".into(),
        )
        .unwrap()
        .as_f64()
        .unwrap() as _
    }

    /// Returns the size (in [`Pages`]) of the `Memory`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmer::{Memory, MemoryType, Pages, Store, Type, Value};
    /// # let store = Store::default();
    /// #
    /// let m = Memory::new(&store, MemoryType::new(1, None, false)).unwrap();
    ///
    /// assert_eq!(m.size(), Pages(1));
    /// ```
    pub fn size(&self, ctx: &impl AsContextRef) -> Pages {
        let bytes = js_sys::Reflect::get(
            &self
                .handle
                .get(ctx.as_context_ref().objects())
                .memory
                .buffer(),
            &"byteLength".into(),
        )
        .unwrap()
        .as_f64()
        .unwrap() as u64;
        Bytes(bytes as usize).try_into().unwrap()
    }

    /// Grow memory by the specified amount of WebAssembly [`Pages`] and return
    /// the previous memory size.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmer::{Memory, MemoryType, Pages, Store, Type, Value, WASM_MAX_PAGES};
    /// # let store = Store::default();
    /// #
    /// let m = Memory::new(&store, MemoryType::new(1, Some(3), false)).unwrap();
    /// let p = m.grow(2).unwrap();
    ///
    /// assert_eq!(p, Pages(1));
    /// assert_eq!(m.size(), Pages(3));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if memory can't be grown by the specified amount
    /// of pages.
    ///
    /// ```should_panic
    /// # use wasmer::{Memory, MemoryType, Pages, Store, Type, Value, WASM_MAX_PAGES};
    /// # let store = Store::default();
    /// #
    /// let m = Memory::new(&store, MemoryType::new(1, Some(1), false)).unwrap();
    ///
    /// // This results in an error: `MemoryError::CouldNotGrow`.
    /// let s = m.grow(1).unwrap();
    /// ```
    pub fn grow<IntoPages>(
        &self,
        ctx: &mut impl AsContextMut,
        delta: IntoPages,
    ) -> Result<Pages, MemoryError>
    where
        IntoPages: Into<Pages>,
    {
        let pages = delta.into();
        let mut ctx_mut = ctx.as_context_mut();
        let js_memory = &self.handle.get_mut(ctx_mut.objects_mut()).memory;
        let our_js_memory: &JSMemory = JsCast::unchecked_from_js_ref(js_memory);
        let new_pages = our_js_memory.grow(pages.0).map_err(|err| {
            if err.is_instance_of::<js_sys::RangeError>() {
                MemoryError::CouldNotGrow {
                    current: self.size(&ctx.as_context_ref()),
                    attempted_delta: pages,
                }
            } else {
                MemoryError::Generic(err.as_string().unwrap())
            }
        })?;
        Ok(Pages(new_pages))
    }

    /// Used by tests
    #[doc(hidden)]
    pub fn uint8view(&self, ctx: &impl AsContextRef) -> js_sys::Uint8Array {
        js_sys::Uint8Array::new(
            &self
                .handle
                .get(ctx.as_context_ref().objects())
                .memory
                .buffer(),
        )
    }

    pub(crate) fn buffer<'a>(&'a self, _ctx: &'a impl AsContextRef) -> MemoryBuffer<'a> {
        MemoryBuffer {
            base: &self.view as *const _ as *mut _,
            marker: PhantomData,
        }
    }

    pub(crate) fn from_vm_export(ctx: &mut impl AsContextMut, vm_memory: VMMemory) -> Self {
        let view = js_sys::Uint8Array::new(&vm_memory.memory.buffer());
        Self {
            handle: ContextHandle::new(ctx.as_context_mut().objects_mut(), vm_memory),
            view,
        }
    }

    pub(crate) fn from_vm_extern(
        ctx: &mut impl AsContextMut,
        internal: InternalContextHandle<VMMemory>,
    ) -> Self {
        let view =
            js_sys::Uint8Array::new(&internal.get(ctx.as_context_ref().objects()).memory.buffer());
        Self {
            handle: unsafe {
                ContextHandle::from_internal(ctx.as_context_ref().objects().id(), internal)
            },
            view,
        }
    }

    /// Safely reads bytes from the memory at the given offset.
    ///
    /// The full buffer will be filled, otherwise a `MemoryAccessError` is returned
    /// to indicate an out-of-bounds access.
    ///
    /// This method is guaranteed to be safe (from the host side) in the face of
    /// concurrent writes.
    pub fn read(
        &self,
        _ctx: &impl AsContextRef,
        offset: u64,
        data: &mut [u8],
    ) -> Result<(), MemoryAccessError> {
        let view = &self.view;
        let offset: u32 = offset.try_into().map_err(|_| MemoryAccessError::Overflow)?;
        let len: u32 = data
            .len()
            .try_into()
            .map_err(|_| MemoryAccessError::Overflow)?;
        let end = offset.checked_add(len).ok_or(MemoryAccessError::Overflow)?;
        if end > view.length() {
            Err(MemoryAccessError::HeapOutOfBounds)?;
        }
        view.subarray(offset, end).copy_to(data);
        Ok(())
    }

    /// Safely reads bytes from the memory at the given offset.
    ///
    /// This method is similar to `read` but allows reading into an
    /// uninitialized buffer. An initialized view of the buffer is returned.
    ///
    /// The full buffer will be filled, otherwise a `MemoryAccessError` is returned
    /// to indicate an out-of-bounds access.
    ///
    /// This method is guaranteed to be safe (from the host side) in the face of
    /// concurrent writes.
    pub fn read_uninit<'a>(
        &self,
        _ctx: &impl AsContextRef,
        offset: u64,
        buf: &'a mut [MaybeUninit<u8>],
    ) -> Result<&'a mut [u8], MemoryAccessError> {
        let view = &self.view;
        let offset: u32 = offset.try_into().map_err(|_| MemoryAccessError::Overflow)?;
        let len: u32 = buf
            .len()
            .try_into()
            .map_err(|_| MemoryAccessError::Overflow)?;
        let end = offset.checked_add(len).ok_or(MemoryAccessError::Overflow)?;
        if end > view.length() {
            Err(MemoryAccessError::HeapOutOfBounds)?;
        }

        // Zero-initialize the buffer to avoid undefined behavior with
        // uninitialized data.
        for elem in buf.iter_mut() {
            *elem = MaybeUninit::new(0);
        }
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, buf.len()) };

        view.subarray(offset, end).copy_to(buf);
        Ok(buf)
    }

    /// Safely writes bytes to the memory at the given offset.
    ///
    /// If the write exceeds the bounds of the memory then a `MemoryAccessError` is
    /// returned.
    ///
    /// This method is guaranteed to be safe (from the host side) in the face of
    /// concurrent reads/writes.
    pub fn write(
        &self,
        _ctx: &mut impl AsContextMut,
        offset: u64,
        data: &[u8],
    ) -> Result<(), MemoryAccessError> {
        let offset: u32 = offset.try_into().map_err(|_| MemoryAccessError::Overflow)?;
        let len: u32 = data
            .len()
            .try_into()
            .map_err(|_| MemoryAccessError::Overflow)?;
        let view = &self.view;
        let end = offset.checked_add(len).ok_or(MemoryAccessError::Overflow)?;
        if end > view.length() {
            Err(MemoryAccessError::HeapOutOfBounds)?;
        }
        view.subarray(offset, end).copy_from(data);
        Ok(())
    }

    /// Checks whether this `Global` can be used with the given context.
    pub fn is_from_context(&self, ctx: &impl AsContextRef) -> bool {
        self.handle.context_id() == ctx.as_context_ref().objects().id()
    }
}

impl<'a> Exportable<'a> for Memory {
    fn get_self_from_extern(_extern: &'a Extern) -> Result<&'a Self, ExportError> {
        match _extern {
            Extern::Memory(memory) => Ok(memory),
            _ => Err(ExportError::IncompatibleType),
        }
    }
}

/// Underlying buffer for a memory.
#[derive(Copy, Clone)]
pub(crate) struct MemoryBuffer<'a> {
    base: *mut js_sys::Uint8Array,
    marker: PhantomData<(&'a Memory, &'a ContextObjects)>,
}

impl<'a> MemoryBuffer<'a> {
    pub(crate) fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), MemoryAccessError> {
        let end = offset
            .checked_add(buf.len() as u64)
            .ok_or(MemoryAccessError::Overflow)?;
        let view = unsafe { &*(self.base) };
        if end > view.length().into() {
            return Err(MemoryAccessError::HeapOutOfBounds);
        }
        view.subarray(offset as _, end as _)
            .copy_to(unsafe { &mut slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()) });
        Ok(())
    }

    pub(crate) fn read_uninit<'b>(
        &self,
        offset: u64,
        buf: &'b mut [MaybeUninit<u8>],
    ) -> Result<&'b mut [u8], MemoryAccessError> {
        let end = offset
            .checked_add(buf.len() as u64)
            .ok_or(MemoryAccessError::Overflow)?;
        let view = unsafe { &*(self.base) };
        if end > view.length().into() {
            return Err(MemoryAccessError::HeapOutOfBounds);
        }
        let buf_ptr = buf.as_mut_ptr() as *mut u8;
        view.subarray(offset as _, end as _)
            .copy_to(unsafe { &mut slice::from_raw_parts_mut(buf_ptr, buf.len()) });

        Ok(unsafe { slice::from_raw_parts_mut(buf_ptr, buf.len()) })
    }

    pub(crate) fn write(&self, offset: u64, data: &[u8]) -> Result<(), MemoryAccessError> {
        let end = offset
            .checked_add(data.len() as u64)
            .ok_or(MemoryAccessError::Overflow)?;
        let view = unsafe { &mut *(self.base) };
        if end > view.length().into() {
            return Err(MemoryAccessError::HeapOutOfBounds);
        }
        view.subarray(offset as _, end as _).copy_from(data);

        Ok(())
    }
}
