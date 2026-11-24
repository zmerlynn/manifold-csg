//! Ray casting against manifold solids.
//!
//! Cast rays against a [`Manifold`](crate::Manifold) to find intersection
//! points, surface normals, and face IDs.

/// Result of a single ray-manifold intersection.
///
/// See the [upstream ray casting docs](https://elalish.github.io/manifold/docs/html/structmanifold_1_1_ray_hit.html)
/// for details on the fields.
///
/// Non-exhaustive: upstream adds fields (`crossing` and `original_id` arrived
/// in manifold 4.0), and results only come from
/// [`Manifold::ray_cast`](crate::Manifold::ray_cast).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct RayHit {
    /// The original ID of the input mesh the hit triangle came from, as in
    /// [`MeshGL64::run_original_id`](crate::MeshGL64::run_original_id). `0`
    /// if that mesh was not tracked, e.g. a primitive that never went
    /// through [`as_original`](crate::Manifold::as_original).
    pub original_id: u32,
    /// The face ID the hit triangle got from its input mesh, as in
    /// [`MeshGL64::face_id`](crate::MeshGL64::face_id). `0` if that mesh had
    /// no face IDs. Before manifold 4.0 this was an internal triangle index.
    pub face_id: u32,
    /// Parametric distance along the ray segment, in `[0, 1]`: `0` at the
    /// origin and `1` at the end. Hits exactly at either end are included.
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
