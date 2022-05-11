use enumset::EnumSet;
use std::any::Any;
use std::convert::TryInto;
use std::path::Path;
use std::{fs, mem};
use wasmer_compiler::{CpuFeature, Features};
use wasmer_types::entity::PrimaryMapRef;
use wasmer_types::{DataInitializer, MemoryIndex, MemoryStyle, ModuleInfo, TableIndex, TableStyle};
use wasmer_types::{DeserializeError, SerializeError};

/// An `Artifact` is the product that the `Engine`
/// implementation produce and use.
///
/// The `Artifact` contains the compiled data for a given
/// module as well as extra information needed to run the
/// module at runtime, such as [`ModuleInfo`] and [`Features`].
pub trait ArtifactCreate: Send + Sync {
    /// Create a `ModuleInfo` for instantiation
    fn create_module_info(&self) -> ModuleInfo;

    /// Returns the features for this Artifact
    fn features(&self) -> &Features;

    /// Returns the CPU features for this Artifact
    fn cpu_features(&self) -> EnumSet<CpuFeature>;

    /// Returns the memory styles associated with this `Artifact`.
    fn memory_styles(&self) -> PrimaryMapRef<MemoryIndex, MemoryStyle>;

    /// Returns the table plans associated with this `Artifact`.
    fn table_styles(&self) -> PrimaryMapRef<TableIndex, TableStyle>;

    /// Returns data initializers to pass to `InstanceHandle::initialize`
    fn data_initializers(&'_ self) -> Vec<DataInitializer<'_>>;

    /// Serializes an artifact into bytes
    fn serialize(&self) -> Result<Vec<u8>, SerializeError>;

    /// Serializes an artifact into a file path
    fn serialize_to_file(&self, path: &Path) -> Result<(), SerializeError> {
        let serialized = self.serialize()?;
        fs::write(&path, serialized)?;
        Ok(())
    }
}

// Implementation of `Upcastable` taken from https://users.rust-lang.org/t/why-does-downcasting-not-work-for-subtraits/33286/7 .
/// Trait needed to get downcasting of `Engine`s to work.
pub trait Upcastable {
    /// upcast ref
    fn upcast_any_ref(&'_ self) -> &'_ dyn Any;
    /// upcast mut ref
    fn upcast_any_mut(&'_ mut self) -> &'_ mut dyn Any;
    /// upcast boxed dyn
    fn upcast_any_box(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + Send + Sync + 'static> Upcastable for T {
    #[inline]
    fn upcast_any_ref(&'_ self) -> &'_ dyn Any {
        self
    }
    #[inline]
    fn upcast_any_mut(&'_ mut self) -> &'_ mut dyn Any {
        self
    }
    #[inline]
    fn upcast_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Metadata header which holds an ABI version and the length of the remaining
/// metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MetadataHeader {
    magic: [u8; 8],
    version: u32,
    len: u32,
}

impl MetadataHeader {
    /// Current ABI version. Increment this any time breaking changes are made
    /// to the format of the serialized data.
    const CURRENT_VERSION: u32 = 1;

    /// Magic number to identify wasmer metadata.
    const MAGIC: [u8; 8] = *b"WASMER\0\0";

    /// Length of the metadata header.
    pub const LEN: usize = 16;

    /// Alignment of the metadata.
    pub const ALIGN: usize = 16;

    /// Creates a new header for metadata of the given length.
    pub fn new(len: usize) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::CURRENT_VERSION,
            len: len.try_into().expect("metadata exceeds maximum length"),
        }
    }

    /// Convert the header into its bytes representation.
    pub fn into_bytes(self) -> [u8; 16] {
        unsafe { mem::transmute(self) }
    }

    /// Parses the header and returns the length of the metadata following it.
    pub fn parse(bytes: &[u8]) -> Result<usize, DeserializeError> {
        if bytes.as_ptr() as usize % 16 != 0 {
            return Err(DeserializeError::CorruptedBinary(
                "misaligned metadata".to_string(),
            ));
        }
        let bytes: [u8; 16] = bytes
            .get(..16)
            .ok_or_else(|| {
                DeserializeError::CorruptedBinary("invalid metadata header".to_string())
            })?
            .try_into()
            .unwrap();
        let header: Self = unsafe { mem::transmute(bytes) };
        if header.magic != Self::MAGIC {
            return Err(DeserializeError::Incompatible(
                "The provided bytes were not serialized by Wasmer".to_string(),
            ));
        }
        if header.version != Self::CURRENT_VERSION {
            return Err(DeserializeError::Incompatible(
                "The provided bytes were serialized by an incompatible version of Wasmer"
                    .to_string(),
            ));
        }
        Ok(header.len as usize)
    }
}
