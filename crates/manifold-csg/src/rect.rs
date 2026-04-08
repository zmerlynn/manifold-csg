//! 2D axis-aligned bounding rectangle wrapping `ManifoldRect`.

use manifold_csg_sys::*;

/// A 2D axis-aligned bounding rectangle.
///
/// Wraps the manifold3d `Rect` type, providing spatial queries (containment,
/// overlap), combining operations (union, expand), and transforms.
///
/// Obtain a `Rect` from [`CrossSection::bounds`](crate::CrossSection::bounds),
/// or construct one directly from min/max coordinates.
pub struct Rect {
    pub(crate) ptr: *mut ManifoldRect,
}

// SAFETY: Rect owns its C-allocated ManifoldRect exclusively. The C++ Rect
// type is a simple value type (two vec2s) with no shared/thread-local state,
// so transferring ownership across threads is safe.
unsafe impl Send for Rect {}

impl Clone for Rect {
    fn clone(&self) -> Self {
        Self::new(self.min(), self.max())
    }
}

impl Drop for Rect {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_rect.
            unsafe { manifold_delete_rect(self.ptr) };
        }
    }
}

impl std::fmt::Debug for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rect")
            .field("min", &self.min())
            .field("max", &self.max())
            .finish()
    }
}

impl Rect {
    /// Create a rectangle from min and max corners.
    #[must_use]
    pub fn new(min: [f64; 2], max: [f64; 2]) -> Self {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: ptr is valid from alloc.
        unsafe {
            manifold_rect(ptr, min[0], min[1], max[0], max[1]);
        }
        Self { ptr }
    }

    /// Construct from a raw `ManifoldRect` pointer (takes ownership).
    pub(crate) fn from_ptr(ptr: *mut ManifoldRect) -> Self {
        Self { ptr }
    }

    /// Minimum corner `[x, y]`.
    #[must_use]
    pub fn min(&self) -> [f64; 2] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_rect_min(self.ptr) };
        [v.x, v.y]
    }

    /// Maximum corner `[x, y]`.
    #[must_use]
    pub fn max(&self) -> [f64; 2] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_rect_max(self.ptr) };
        [v.x, v.y]
    }

    /// Dimensions `[width, height]`.
    #[must_use]
    pub fn dimensions(&self) -> [f64; 2] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_rect_dimensions(self.ptr) };
        [v.x, v.y]
    }

    /// Center point `[x, y]`.
    #[must_use]
    pub fn center(&self) -> [f64; 2] {
        // SAFETY: self.ptr is valid (invariant).
        let v = unsafe { manifold_rect_center(self.ptr) };
        [v.x, v.y]
    }

    /// The maximum distance from the center to any corner (half-diagonal).
    #[must_use]
    pub fn scale(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_rect_scale(self.ptr) }
    }

    /// Whether the rectangle is empty (zero area).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_rect_is_empty(self.ptr) != 0 }
    }

    /// Whether the rectangle has finite (non-infinite, non-NaN) bounds.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_rect_is_finite(self.ptr) != 0 }
    }

    /// Whether the rectangle fully contains the given point.
    #[must_use]
    pub fn contains_point(&self, point: [f64; 2]) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_rect_contains_pt(self.ptr, point[0], point[1]) != 0 }
    }

    /// Whether the rectangle fully contains another rectangle.
    #[must_use]
    pub fn contains_rect(&self, other: &Rect) -> bool {
        // SAFETY: both pointers are valid (invariant).
        unsafe { manifold_rect_contains_rect(self.ptr, other.ptr) != 0 }
    }

    /// Whether this rectangle overlaps with another rectangle.
    #[must_use]
    pub fn overlaps_rect(&self, other: &Rect) -> bool {
        // SAFETY: both pointers are valid (invariant).
        unsafe { manifold_rect_does_overlap_rect(self.ptr, other.ptr) != 0 }
    }

    /// Expand this rectangle to include the given point.
    pub fn include_point(&mut self, point: [f64; 2]) {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_rect_include_pt(self.ptr, point[0], point[1]) };
    }

    /// Return the union (smallest rectangle containing both) of two rectangles.
    #[must_use]
    pub fn union(&self, other: &Rect) -> Rect {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_rect_union(ptr, self.ptr, other.ptr) };
        Rect { ptr }
    }

    /// Apply a 3x2 affine transformation matrix (column-major).
    ///
    /// Layout: `[x1, y1, x2, y2, x3, y3]` where columns are:
    /// - col1 `(x1,y1)` — X basis vector
    /// - col2 `(x2,y2)` — Y basis vector
    /// - col3 `(x3,y3)` — translation
    #[must_use]
    pub fn transform(&self, m: &[f64; 6]) -> Rect {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: all pointers are valid.
        unsafe {
            manifold_rect_transform(
                ptr, self.ptr,
                m[0], m[1],
                m[2], m[3],
                m[4], m[5],
            );
        }
        Rect { ptr }
    }

    /// Translate the rectangle by `[x, y]`.
    #[must_use]
    pub fn translate(&self, v: [f64; 2]) -> Rect {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: all pointers are valid.
        unsafe { manifold_rect_translate(ptr, self.ptr, v[0], v[1]) };
        Rect { ptr }
    }

    /// Scale the rectangle by `[x, y]` factors.
    #[must_use]
    pub fn mul(&self, v: [f64; 2]) -> Rect {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: all pointers are valid.
        unsafe { manifold_rect_mul(ptr, self.ptr, v[0], v[1]) };
        Rect { ptr }
    }
}
