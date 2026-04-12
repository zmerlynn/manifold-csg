//! 3D axis-aligned bounding box wrapping `ManifoldBox`.

use manifold_csg_sys::*;

/// A 3D axis-aligned bounding box.
///
/// Wraps the manifold3d `Box` type, providing spatial queries (containment,
/// overlap), combining operations (union, expand), and transforms.
///
/// Obtain a `BoundingBox` from [`Manifold::bounding_box`](crate::Manifold::bounding_box),
/// or construct one directly from min/max coordinates.
pub struct BoundingBox {
    pub(crate) ptr: *mut ManifoldBox,
}

// SAFETY: BoundingBox owns its C-allocated ManifoldBox exclusively. The C++
// Box type is a simple value type (two vec3s) with no shared/thread-local state,
// so transferring ownership across threads is safe.
unsafe impl Send for BoundingBox {}

// SAFETY: BoundingBox is a simple value type (two vec3s) with no lazy evaluation
// or mutable internal state. Concurrent read access is safe.
unsafe impl Sync for BoundingBox {}

impl Clone for BoundingBox {
    fn clone(&self) -> Self {
        Self::new(self.min(), self.max())
    }
}

impl Drop for BoundingBox {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_box.
            unsafe { manifold_delete_box(self.ptr) };
        }
    }
}

impl std::fmt::Debug for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundingBox")
            .field("min", &self.min())
            .field("max", &self.max())
            .finish()
    }
}

impl BoundingBox {
    /// Create a bounding box from min and max corners.
    #[must_use]
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: ptr is valid from alloc.
        unsafe {
            manifold_box(ptr, min[0], min[1], min[2], max[0], max[1], max[2]);
        }
        Self { ptr }
    }

    /// Minimum corner `[x, y, z]`.
    #[must_use]
    pub fn min(&self) -> [f64; 3] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_box_min(self.ptr) };
        [v.x, v.y, v.z]
    }

    /// Maximum corner `[x, y, z]`.
    #[must_use]
    pub fn max(&self) -> [f64; 3] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_box_max(self.ptr) };
        [v.x, v.y, v.z]
    }

    /// Dimensions `[width, height, depth]`.
    #[must_use]
    pub fn dimensions(&self) -> [f64; 3] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_box_dimensions(self.ptr) };
        [v.x, v.y, v.z]
    }

    /// Center point `[x, y, z]`.
    #[must_use]
    pub fn center(&self) -> [f64; 3] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_box_center(self.ptr) };
        [v.x, v.y, v.z]
    }

    /// The maximum distance from the center to any corner (half-diagonal).
    #[must_use]
    pub fn scale(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_box_scale(self.ptr) }
    }

    /// Whether the box is empty (all dimensions are zero or negative).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let d = self.dimensions();
        d[0] <= 0.0 || d[1] <= 0.0 || d[2] <= 0.0
    }

    /// Whether the box has finite (non-infinite, non-NaN) bounds.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_box_is_finite(self.ptr) != 0 }
    }

    /// Whether the box fully contains the given point.
    #[must_use]
    pub fn contains_point(&self, point: [f64; 3]) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_box_contains_pt(self.ptr, point[0], point[1], point[2]) != 0 }
    }

    /// Whether the box fully contains another box.
    #[must_use]
    pub fn contains_box(&self, other: &BoundingBox) -> bool {
        // SAFETY: both pointers are valid (invariant).
        unsafe { manifold_box_contains_box(self.ptr, other.ptr) != 0 }
    }

    /// Whether the box overlaps with a point (same as contains for points).
    #[must_use]
    pub fn overlaps_point(&self, point: [f64; 3]) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_box_does_overlap_pt(self.ptr, point[0], point[1], point[2]) != 0 }
    }

    /// Whether this box overlaps with another box.
    #[must_use]
    pub fn overlaps_box(&self, other: &BoundingBox) -> bool {
        // SAFETY: both pointers are valid (invariant).
        unsafe { manifold_box_does_overlap_box(self.ptr, other.ptr) != 0 }
    }

    /// Expand this box to include the given point.
    pub fn include_point(&mut self, point: [f64; 3]) {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_box_include_pt(self.ptr, point[0], point[1], point[2]) };
    }

    /// Return the union (smallest box containing both) of two bounding boxes.
    #[must_use]
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_box_union(ptr, self.ptr, other.ptr) };
        BoundingBox { ptr }
    }

    /// Apply a 4x3 affine transformation matrix (column-major, same layout as
    /// [`Manifold::transform`](crate::Manifold::transform)).
    #[must_use]
    pub fn transform(&self, m: &[f64; 12]) -> BoundingBox {
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: all pointers are valid.
        unsafe {
            manifold_box_transform(
                ptr, self.ptr, m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10],
                m[11],
            );
        }
        BoundingBox { ptr }
    }

    /// Translate the box by `[x, y, z]`.
    #[must_use]
    pub fn translate(&self, v: [f64; 3]) -> BoundingBox {
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: all pointers are valid.
        unsafe { manifold_box_translate(ptr, self.ptr, v[0], v[1], v[2]) };
        BoundingBox { ptr }
    }

    /// Scale the box by `[x, y, z]` factors.
    #[must_use]
    pub fn mul(&self, v: [f64; 3]) -> BoundingBox {
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: all pointers are valid.
        unsafe { manifold_box_mul(ptr, self.ptr, v[0], v[1], v[2]) };
        BoundingBox { ptr }
    }
}
