//! Ray casting against manifold solids.
//!
//! Cast rays against a [`Manifold`](crate::Manifold) to find intersection
//! points, surface normals, and face IDs.

/// Result of a single ray-manifold intersection.
///
/// See the [upstream ray casting docs](https://elalish.github.io/manifold/docs/html/structmanifold_1_1_ray_hit.html)
/// for details on the fields.
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    /// Index of the face that was hit.
    pub face_id: u64,
    /// Distance from the ray origin to the hit point.
    pub distance: f64,
    /// World-space position of the hit.
    pub position: [f64; 3],
    /// Surface normal at the hit point.
    pub normal: [f64; 3],
}
