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
    /// `+1` if the ray enters the solid here, `-1` if it exits. Summing these
    /// along the ray gives the change in winding number from origin to end.
    /// Upstream recommends this over the sign of `normal` dotted with the ray
    /// direction, which can disagree in degenerate cases. A ray grazing a
    /// vertex or edge can return a `+1`/`-1` pair at the same distance.
    pub crossing: i32,
}
