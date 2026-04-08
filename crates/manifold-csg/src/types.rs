//! Common types and error definitions.

/// Errors from manifold3d operations.
#[derive(Debug, thiserror::Error)]
pub enum CsgError {
    #[error("manifold3d status: {0:?}")]
    ManifoldStatus(manifold_csg_sys::ManifoldError),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("empty mesh (no faces)")]
    EmptyMesh,
}
