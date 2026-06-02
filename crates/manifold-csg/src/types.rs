//! Common types and error definitions.

use std::sync::Mutex;

use manifold_csg_sys::ManifoldOpType;

/// Errors from manifold3d operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CsgError {
    #[error("manifold3d status: {0:?}")]
    ManifoldStatus(manifold_csg_sys::ManifoldError),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl CsgError {
    pub(crate) fn from_status(status: manifold_csg_sys::ManifoldError) -> Result<(), Self> {
        if status.is_ok() {
            Ok(())
        } else {
            Err(Self::ManifoldStatus(status))
        }
    }
}

/// Boolean operation type for generic manifold and cross-section booleans.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    /// Union/addition.
    Add,
    /// Difference/subtraction.
    Subtract,
    /// Intersection.
    Intersect,
}

impl OpType {
    pub(crate) const fn to_ffi(self) -> ManifoldOpType {
        match self {
            Self::Add => ManifoldOpType::Add,
            Self::Subtract => ManifoldOpType::Subtract,
            Self::Intersect => ManifoldOpType::Intersect,
        }
    }
}

pub(crate) type PanicPayload = Box<dyn std::any::Any + Send + 'static>;

pub(crate) fn store_panic(slot: &Mutex<Option<PanicPayload>>, payload: PanicPayload) {
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(payload);
    }
}

pub(crate) fn take_stored_panic(slot: &Mutex<Option<PanicPayload>>) -> Option<PanicPayload> {
    slot.lock().unwrap_or_else(|e| e.into_inner()).take()
}
