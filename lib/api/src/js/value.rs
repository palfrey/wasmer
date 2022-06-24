use std::convert::TryFrom;
use std::fmt;
use std::string::{String, ToString};

use wasmer_types::Type;

//use crate::ExternRef;
use crate::js::externals::function::Function;

use super::context::AsContextRef;

/// WebAssembly computations manipulate values of basic value types:
/// * Integers (32 or 64 bit width)
/// * Floating-point (32 or 64 bit width)
///
/// Spec: <https://webassembly.github.io/spec/core/exec/runtime.html#values>
#[derive(Clone, PartialEq)]
pub enum Value {
    /// A 32-bit integer.
    ///
    /// In Wasm integers are sign-agnostic, i.e. this can either be signed or unsigned.
    I32(i32),

    /// A 64-bit integer.
    ///
    /// In Wasm integers are sign-agnostic, i.e. this can either be signed or unsigned.
    I64(i64),

    /// A 32-bit float.
    F32(f32),

    /// A 64-bit float.
    F64(f64),

    /// An `externref` value which can hold opaque data to the wasm instance itself.
    //ExternRef(Option<ExternRef>),

    /// A first-class reference to a WebAssembly function.
    FuncRef(Option<Function>),
}

macro_rules! accessors {
    ($bind:ident $(($variant:ident($ty:ty) $get:ident $unwrap:ident $cvt:expr))*) => ($(
        /// Attempt to access the underlying value of this `Value`, returning
        /// `None` if it is not the correct type.
        pub fn $get(&self) -> Option<$ty> {
            if let Self::$variant($bind) = self {
                Some($cvt)
            } else {
                None
            }
        }

        /// Returns the underlying value of this `Value`, panicking if it's the
        /// wrong type.
        ///
        /// # Panics
        ///
        /// Panics if `self` is not of the right type.
        pub fn $unwrap(&self) -> $ty {
            self.$get().expect(concat!("expected ", stringify!($ty)))
        }
    )*)
}

impl Value {
    /// Returns a null `externref` value.
    pub fn null() -> Self {
        Self::FuncRef(None)
    }

    /// Returns the corresponding [`Type`] for this `Value`.
    pub fn ty(&self) -> Type {
        match self {
            Self::I32(_) => Type::I32,
            Self::I64(_) => Type::I64,
            Self::F32(_) => Type::F32,
            Self::F64(_) => Type::F64,
            //Self::ExternRef(_) => Type::ExternRef,
            Self::FuncRef(_) => Type::FuncRef,
        }
    }

    /// Converts the `Value` into a `f64`.
    pub fn as_raw(&self, ctx: &impl AsContextRef) -> f64 {
        match *self {
            Self::I32(v) => v as f64,
            Self::I64(v) => v as f64,
            Self::F32(v) => v as f64,
            Self::F64(v) => v,
            Self::FuncRef(Some(ref f)) => f
                .handle
                .get(ctx.as_context_ref().objects())
                .function
                .as_f64()
                .unwrap_or(0_f64), //TODO is this correct?

            Self::FuncRef(None) => 0_f64,
            //Self::ExternRef(Some(ref e)) => unsafe { *e.address().0 } as .into_raw(),
            //Self::ExternRef(None) =>  externref: 0 },
        }
    }

    /// Converts a `f64` to a `Value`.
    ///
    /// # Safety
    ///
    pub unsafe fn from_raw(_ctx: &impl AsContextRef, ty: Type, raw: f64) -> Self {
        match ty {
            Type::I32 => Self::I32(raw as i32),
            Type::I64 => Self::I64(raw as i64),
            Type::F32 => Self::F32(raw as f32),
            Type::F64 => Self::F64(raw),
            Type::FuncRef => todo!(),
            Type::V128 => todo!(),
            Type::ExternRef => todo!(),
            //Self::ExternRef(
            //{
            //VMExternRef::from_raw(raw).map(|e| ExternRef::from_vm_externref(ctx, e)),
            //),
        }
    }

    /// Checks whether a value can be used with the given context.
    ///
    /// Primitive (`i32`, `i64`, etc) and null funcref/externref values are not
    /// tied to a context and can be freely shared between contexts.
    ///
    /// Externref and funcref values are tied to a context and can only be used
    /// with that context.
    pub fn is_from_context(&self, ctx: &impl AsContextRef) -> bool {
        match self {
            Self::I32(_)
            | Self::I64(_)
            | Self::F32(_)
            | Self::F64(_)
            //| Self::ExternRef(None)
            | Self::FuncRef(None) => true,
            //Self::ExternRef(Some(e)) => e.is_from_context(ctx),
            Self::FuncRef(Some(f)) => f.is_from_context(ctx),
        }
    }

    accessors! {
        e
        (I32(i32) i32 unwrap_i32 *e)
        (I64(i64) i64 unwrap_i64 *e)
        (F32(f32) f32 unwrap_f32 *e)
        (F64(f64) f64 unwrap_f64 *e)
        //(ExternRef(&Option<ExternRef>) externref unwrap_externref e)
        (FuncRef(&Option<Function>) funcref unwrap_funcref e)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I32(v) => write!(f, "I32({:?})", v),
            Self::I64(v) => write!(f, "I64({:?})", v),
            Self::F32(v) => write!(f, "F32({:?})", v),
            Self::F64(v) => write!(f, "F64({:?})", v),
            //Self::ExternRef(None) => write!(f, "Null ExternRef"),
            //Self::ExternRef(Some(v)) => write!(f, "ExternRef({:?})", v),
            Self::FuncRef(None) => write!(f, "Null FuncRef"),
            Self::FuncRef(Some(v)) => write!(f, "FuncRef({:?})", v),
        }
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::I32(v) => v.to_string(),
            Self::I64(v) => v.to_string(),
            Self::F32(v) => v.to_string(),
            Self::F64(v) => v.to_string(),
            //Self::ExternRef(_) => "externref".to_string(),
            Self::FuncRef(_) => "funcref".to_string(),
        }
    }
}

impl From<i32> for Value {
    fn from(val: i32) -> Self {
        Self::I32(val)
    }
}

impl From<u32> for Value {
    fn from(val: u32) -> Self {
        // In Wasm integers are sign-agnostic, so i32 is basically a 4 byte storage we can use for signed or unsigned 32-bit integers.
        Self::I32(val as i32)
    }
}

impl From<i64> for Value {
    fn from(val: i64) -> Self {
        Self::I64(val)
    }
}

impl From<u64> for Value {
    fn from(val: u64) -> Self {
        // In Wasm integers are sign-agnostic, so i64 is basically an 8 byte storage we can use for signed or unsigned 64-bit integers.
        Self::I64(val as i64)
    }
}

impl From<f32> for Value {
    fn from(val: f32) -> Self {
        Self::F32(val)
    }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self {
        Self::F64(val)
    }
}

impl From<Function> for Value {
    fn from(val: Function) -> Self {
        Self::FuncRef(Some(val))
    }
}

impl From<Option<Function>> for Value {
    fn from(val: Option<Function>) -> Self {
        Self::FuncRef(val)
    }
}

//impl From<ExternRef> for Value {
//    fn from(val: ExternRef) -> Self {
//        Self::ExternRef(Some(val))
//    }
//}
//
//impl From<Option<ExternRef>> for Value {
//    fn from(val: Option<ExternRef>) -> Self {
//        Self::ExternRef(val)
//    }
//}

const NOT_I32: &str = "Value is not of Wasm type i32";
const NOT_I64: &str = "Value is not of Wasm type i64";
const NOT_F32: &str = "Value is not of Wasm type f32";
const NOT_F64: &str = "Value is not of Wasm type f64";
const NOT_FUNCREF: &str = "Value is not of Wasm type funcref";
//const NOT_EXTERNREF: &str = "Value is not of Wasm type externref";

impl TryFrom<Value> for i32 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.i32().ok_or(NOT_I32)
    }
}

impl TryFrom<Value> for u32 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.i32().ok_or(NOT_I32).map(|int| int as Self)
    }
}

impl TryFrom<Value> for i64 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.i64().ok_or(NOT_I64)
    }
}

impl TryFrom<Value> for u64 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.i64().ok_or(NOT_I64).map(|int| int as Self)
    }
}

impl TryFrom<Value> for f32 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.f32().ok_or(NOT_F32)
    }
}

impl TryFrom<Value> for f64 {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value.f64().ok_or(NOT_F64)
    }
}

impl TryFrom<Value> for Option<Function> {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::FuncRef(f) => Ok(f),
            _ => Err(NOT_FUNCREF),
        }
    }
}

//impl TryFrom<Value> for Option<ExternRef> {
//    type Error = &'static str;
//
//    fn try_from(value: Value) -> Result<Self, Self::Error> {
//        match value {
//            Value::ExternRef(e) => Ok(e),
//            _ => Err(NOT_EXTERNREF),
//        }
//    }
//}

#[cfg(tests)]
mod tests {
    use super::*;
    /*

    fn test_value_i32_from_u32() {
        let bytes = [0x00, 0x00, 0x00, 0x00];
        let v = Value::<()>::from(u32::from_be_bytes(bytes));
        assert_eq!(v, Value::I32(i32::from_be_bytes(bytes)));

        let bytes = [0x00, 0x00, 0x00, 0x01];
        let v = Value::<()>::from(u32::from_be_bytes(bytes));
        assert_eq!(v, Value::I32(i32::from_be_bytes(bytes)));

        let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
        let v = Value::<()>::from(u32::from_be_bytes(bytes));
        assert_eq!(v, Value::I32(i32::from_be_bytes(bytes)));

        let bytes = [0xFF, 0xFF, 0xFF, 0xFF];
        let v = Value::<()>::from(u32::from_be_bytes(bytes));
        assert_eq!(v, Value::I32(i32::from_be_bytes(bytes)));
    }

    fn test_value_i64_from_u64() {
        let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let v = Value::<()>::from(u64::from_be_bytes(bytes));
        assert_eq!(v, Value::I64(i64::from_be_bytes(bytes)));

        let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let v = Value::<()>::from(u64::from_be_bytes(bytes));
        assert_eq!(v, Value::I64(i64::from_be_bytes(bytes)));

        let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
        let v = Value::<()>::from(u64::from_be_bytes(bytes));
        assert_eq!(v, Value::I64(i64::from_be_bytes(bytes)));

        let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let v = Value::<()>::from(u64::from_be_bytes(bytes));
        assert_eq!(v, Value::I64(i64::from_be_bytes(bytes)));
    }

    fn convert_value_to_i32() {
        let value = Value::<()>::I32(5678);
        let result = i32::try_from(value);
        assert_eq!(result.unwrap(), 5678);

        let value = Value::<()>::from(u32::MAX);
        let result = i32::try_from(value);
        assert_eq!(result.unwrap(), -1);
    }

    fn convert_value_to_u32() {
        let value = Value::<()>::from(u32::MAX);
        let result = u32::try_from(value);
        assert_eq!(result.unwrap(), u32::MAX);

        let value = Value::<()>::I32(-1);
        let result = u32::try_from(value);
        assert_eq!(result.unwrap(), u32::MAX);
    }

    fn convert_value_to_i64() {
        let value = Value::<()>::I64(5678);
        let result = i64::try_from(value);
        assert_eq!(result.unwrap(), 5678);

        let value = Value::<()>::from(u64::MAX);
        let result = i64::try_from(value);
        assert_eq!(result.unwrap(), -1);
    }

    fn convert_value_to_u64() {
        let value = Value::<()>::from(u64::MAX);
        let result = u64::try_from(value);
        assert_eq!(result.unwrap(), u64::MAX);

        let value = Value::<()>::I64(-1);
        let result = u64::try_from(value);
        assert_eq!(result.unwrap(), u64::MAX);
    }

    fn convert_value_to_f32() {
        let value = Value::<()>::F32(1.234);
        let result = f32::try_from(value);
        assert_eq!(result.unwrap(), 1.234);

        let value = Value::<()>::F64(1.234);
        let result = f32::try_from(value);
        assert_eq!(result.unwrap_err(), "Value is not of Wasm type f32");
    }

    fn convert_value_to_f64() {
        let value = Value::<()>::F64(1.234);
        let result = f64::try_from(value);
        assert_eq!(result.unwrap(), 1.234);

        let value = Value::<()>::F32(1.234);
        let result = f64::try_from(value);
        assert_eq!(result.unwrap_err(), "Value is not of Wasm type f64");
    }
    */
}
