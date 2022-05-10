use crate::Pages;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};

/// Implementation styles for WebAssembly linear memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash, RkyvSerialize, RkyvDeserialize, Archive)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
#[archive(as = "Self")]
pub enum MemoryStyle {
    /// The actual memory can be resized and moved.
    Dynamic {
        /// Our chosen offset-guard size.
        ///
        /// It represents the size in bytes of extra guard pages after the end
        /// to optimize loads and stores with constant offsets.
        offset_guard_size: u64,
    },
    /// Address space is allocated up front.
    Static {
        /// The number of mapped and unmapped pages.
        bound: Pages,
        /// Our chosen offset-guard size.
        ///
        /// It represents the size in bytes of extra guard pages after the end
        /// to optimize loads and stores with constant offsets.
        offset_guard_size: u64,
    },
}

impl MemoryStyle {
    /// Returns the offset-guard size
    pub fn offset_guard_size(&self) -> u64 {
        match self {
            Self::Dynamic { offset_guard_size } => *offset_guard_size,
            Self::Static {
                offset_guard_size, ..
            } => *offset_guard_size,
        }
    }
}