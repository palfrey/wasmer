use crate::js::context::AsContextRef;
use crate::js::NativeWasmTypeInto;
use crate::js::{externals::Memory, FromToNativeWasmType};
use crate::{MemoryAccessError, WasmRef, WasmSlice};
use std::convert::TryFrom;
use std::{fmt, marker::PhantomData, mem};
pub use wasmer_types::Memory32;
pub use wasmer_types::Memory64;
pub use wasmer_types::MemorySize;
use wasmer_types::ValueType;

/// Alias for `WasmPtr<T, Memory64>.
pub type WasmPtr64<T> = WasmPtr<T, Memory64>;

/// A zero-cost type that represents a pointer to something in Wasm linear
/// memory.
///
/// This type can be used directly in the host function arguments:
/// ```
/// # use wasmer::Memory;
/// # use wasmer::WasmPtr;
/// pub fn host_import(memory: Memory, ptr: WasmPtr<u32>) {
///     let derefed_ptr = ptr.deref(&memory);
///     let inner_val: u32 = derefed_ptr.read().expect("pointer in bounds");
///     println!("Got {} from Wasm memory address 0x{:X}", inner_val, ptr.offset());
///     // update the value being pointed to
///     derefed_ptr.write(inner_val + 1).expect("pointer in bounds");
/// }
/// ```
///
/// This type can also be used with primitive-filled structs, but be careful of
/// guarantees required by `ValueType`.
/// ```
/// # use wasmer::Memory;
/// # use wasmer::WasmPtr;
/// # use wasmer::ValueType;
///
/// // This is safe as the 12 bytes represented by this struct
/// // are valid for all bit combinations.
/// #[derive(Copy, Clone, Debug, ValueType)]
/// #[repr(C)]
/// struct V3 {
///     x: f32,
///     y: f32,
///     z: f32
/// }
///
/// fn update_vector_3(memory: Memory, ptr: WasmPtr<V3>) {
///     let derefed_ptr = ptr.deref(&memory);
///     let mut inner_val: V3 = derefed_ptr.read().expect("pointer in bounds");
///     println!("Got {:?} from Wasm memory address 0x{:X}", inner_val, ptr.offset());
///     // update the value being pointed to
///     inner_val.x = 10.4;
///     derefed_ptr.write(inner_val).expect("pointer in bounds");
/// }
/// ```
#[repr(transparent)]
pub struct WasmPtr<T, M: MemorySize = Memory32> {
    offset: M::Offset,
    _phantom: PhantomData<*mut T>,
}

impl<T, M: MemorySize> WasmPtr<T, M> {
    /// Create a new `WasmPtr` at the given offset.
    #[inline]
    pub fn new(offset: M::Offset) -> Self {
        Self {
            offset,
            _phantom: PhantomData,
        }
    }

    /// Get the offset into Wasm linear memory for this `WasmPtr`.
    #[inline]
    pub fn offset(self) -> M::Offset {
        self.offset
    }

    /// Casts this `WasmPtr` to a `WasmPtr` of a different type.
    #[inline]
    pub fn cast<U>(self) -> WasmPtr<U, M> {
        WasmPtr {
            offset: self.offset,
            _phantom: PhantomData,
        }
    }

    /// Returns a null `UserPtr`.
    #[inline]
    pub fn null() -> Self {
        WasmPtr::new(M::ZERO)
    }

    /// Checks whether the `WasmPtr` is null.
    #[inline]
    pub fn is_null(self) -> bool {
        self.offset.into() == 0
    }

    /// Calculates an offset from the current pointer address. The argument is
    /// in units of `T`.
    ///
    /// This method returns an error if an address overflow occurs.
    #[inline]
    pub fn add_offset(self, offset: M::Offset) -> Result<Self, MemoryAccessError> {
        let base = self.offset.into();
        let index = offset.into();
        let offset = index
            .checked_mul(mem::size_of::<T>() as u64)
            .ok_or(MemoryAccessError::Overflow)?;
        let address = base
            .checked_add(offset)
            .ok_or(MemoryAccessError::Overflow)?;
        let address = M::Offset::try_from(address).map_err(|_| MemoryAccessError::Overflow)?;
        Ok(WasmPtr::new(address))
    }

    /// Calculates an offset from the current pointer address. The argument is
    /// in units of `T`.
    ///
    /// This method returns an error if an address overflow occurs.
    #[inline]
    pub fn sub_offset(self, offset: M::Offset) -> Result<Self, MemoryAccessError> {
        let base = self.offset.into();
        let index = offset.into();
        let offset = index
            .checked_mul(mem::size_of::<T>() as u64)
            .ok_or(MemoryAccessError::Overflow)?;
        let address = base
            .checked_sub(offset)
            .ok_or(MemoryAccessError::Overflow)?;
        let address = M::Offset::try_from(address).map_err(|_| MemoryAccessError::Overflow)?;
        Ok(WasmPtr::new(address))
    }
}

impl<T: ValueType, M: MemorySize> WasmPtr<T, M> {
    /// Creates a `WasmRef` from this `WasmPtr` which allows reading and
    /// mutating of the value being pointed to.
    #[inline]
    pub fn deref<'a>(self, ctx: &'a impl AsContextRef, memory: &'a Memory) -> WasmRef<'a, T> {
        WasmRef::new(ctx, memory, self.offset.into())
    }

    /// Reads the address pointed to by this `WasmPtr` in a memory.
    #[inline]
    pub fn read(self, ctx: &impl AsContextRef, memory: &Memory) -> Result<T, MemoryAccessError> {
        self.deref(ctx, memory).read()
    }

    /// Writes to the address pointed to by this `WasmPtr` in a memory.
    #[inline]
    pub fn write(
        self,
        ctx: &impl AsContextRef,
        memory: &Memory,
        val: T,
    ) -> Result<(), MemoryAccessError> {
        self.deref(ctx, memory).write(val)
    }

    /// Creates a `WasmSlice` starting at this `WasmPtr` which allows reading
    /// and mutating of an array of value being pointed to.
    ///
    /// Returns a `MemoryAccessError` if the slice length overflows a 64-bit
    /// address.
    #[inline]
    pub fn slice<'a>(
        self,
        ctx: &'a impl AsContextRef,
        memory: &'a Memory,
        len: M::Offset,
    ) -> Result<WasmSlice<'a, T>, MemoryAccessError> {
        WasmSlice::new(ctx, memory, self.offset.into(), len.into())
    }

    /// Reads a sequence of values from this `WasmPtr` until a value that
    /// matches the given condition is found.
    ///
    /// This last value is not included in the returned vector.
    #[inline]
    pub fn read_until<'a>(
        self,
        ctx: &'a impl AsContextRef,
        memory: &'a Memory,
        mut end: impl FnMut(&T) -> bool,
    ) -> Result<Vec<T>, MemoryAccessError> {
        let mut vec = Vec::new();
        for i in 0u64.. {
            let i = M::Offset::try_from(i).map_err(|_| MemoryAccessError::Overflow)?;
            let val = self.add_offset(i)?.deref(ctx, memory).read()?;
            if end(&val) {
                break;
            }
            vec.push(val);
        }
        Ok(vec)
    }
}

impl<M: MemorySize> WasmPtr<u8, M> {
    /// Reads a UTF-8 string from the `WasmPtr` with the given length.
    ///
    /// This method is safe to call even if the memory is being concurrently
    /// modified.
    #[inline]
    pub fn read_utf8_string<'a>(
        self,
        ctx: &'a impl AsContextRef,
        memory: &'a Memory,
        len: M::Offset,
    ) -> Result<String, MemoryAccessError> {
        let vec = self.slice(ctx, memory, len)?.read_to_vec()?;
        Ok(String::from_utf8(vec)?)
    }

    /// Reads a null-terminated UTF-8 string from the `WasmPtr`.
    ///
    /// This method is safe to call even if the memory is being concurrently
    /// modified.
    #[inline]
    pub fn read_utf8_string_with_nul<'a>(
        self,
        ctx: &'a impl AsContextRef,
        memory: &'a Memory,
    ) -> Result<String, MemoryAccessError> {
        let vec = self.read_until(ctx, memory, |&byte| byte == 0)?;
        Ok(String::from_utf8(vec)?)
    }
}

unsafe impl<T: ValueType, M: MemorySize> FromToNativeWasmType for WasmPtr<T, M>
where
    <M as wasmer_types::MemorySize>::Native: NativeWasmTypeInto,
{
    type Native = M::Native;

    fn to_native(self) -> Self::Native {
        M::offset_to_native(self.offset)
    }
    fn from_native(n: Self::Native) -> Self {
        Self {
            offset: M::native_to_offset(n),
            _phantom: PhantomData,
        }
    }
}

unsafe impl<T: ValueType, M: MemorySize> ValueType for WasmPtr<T, M> {
    fn zero_padding_bytes(&self, _bytes: &mut [mem::MaybeUninit<u8>]) {}
}

impl<T: ValueType, M: MemorySize> Clone for WasmPtr<T, M> {
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            _phantom: PhantomData,
        }
    }
}

impl<T: ValueType, M: MemorySize> Copy for WasmPtr<T, M> {}

impl<T: ValueType, M: MemorySize> PartialEq for WasmPtr<T, M> {
    fn eq(&self, other: &Self) -> bool {
        self.offset.into() == other.offset.into()
    }
}

impl<T: ValueType, M: MemorySize> Eq for WasmPtr<T, M> {}

impl<T: ValueType, M: MemorySize> fmt::Debug for WasmPtr<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "WasmPtr(offset: {}, pointer: {:#x})",
            self.offset.into(),
            self.offset.into()
        )
    }
}
