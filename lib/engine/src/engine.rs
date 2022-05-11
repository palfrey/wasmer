//! Engine trait and associated types.

use crate::tunables::Tunables;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmer_compiler::Target;
use wasmer_engine_universal_artifact::UniversalArtifactBuild;
use wasmer_engine_universal_artifact::UniversalArtifactBuildRef;

use wasmer_types::{CompileError, DeserializeError, FunctionType};
use wasmer_vm::{VMCallerCheckedAnyfunc, VMFuncRef, VMSharedSignatureIndex};

/// A unimplemented Wasmer `Engine`.
///
/// This trait is used by implementors to implement custom engines
/// such as: Universal or Native.
///
/// The product that an `Engine` produces and consumes is the [`Artifact`].
pub trait Engine {
    /// The Artifact type.
    type Artifact;
    /// Gets the target
    fn target(&self) -> &Target;

    /// Register a signature
    fn register_signature(&self, func_type: &FunctionType) -> VMSharedSignatureIndex;

    /// Register a function's data.
    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef;

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType>;

    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError>;

    /// Make a WebAssembly build
    fn compile_build(
        &self,
        binary: &[u8],
        tunables: &dyn Tunables,
    ) -> Result<UniversalArtifactBuild, CompileError>;

    /// Compile a WebAssembly binary
    fn from_build(
        &self,
        build: UniversalArtifactBuildRef<'_>,
    ) -> Result<Self::Artifact, CompileError>
    where
        Self::Artifact: Sized;

    /// Deserializes a WebAssembly module
    ///
    /// # Safety
    ///
    /// The serialized content must represent a serialized WebAssembly module.
    unsafe fn deserialize(&self, bytes: &'_ [u8]) -> Result<Self::Artifact, DeserializeError>;

    /// A unique identifier for this object.
    ///
    /// This exists to allow us to compare two Engines for equality. Otherwise,
    /// comparing two trait objects unsafely relies on implementation details
    /// of trait representation.
    fn id(&self) -> &EngineId;

    /// Clone the engine
    fn cloned(&self) -> Arc<dyn Engine<Artifact = Self::Artifact> + Send + Sync>;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// A unique identifier for an Engine.
pub struct EngineId {
    id: usize,
}

impl EngineId {
    /// Format this identifier as a string.
    pub fn id(&self) -> String {
        format!("{}", &self.id)
    }
}

impl Clone for EngineId {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for EngineId {
    fn default() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, SeqCst),
        }
    }
}
