//! Safe wrapper around a manifold3d CrossSection object (2D region).
//!
//! [`CrossSection`] provides 2D boolean operations, geometric offset, convex
//! hull, and transforms. Cross-sections can be extruded to 3D via
//! [`Manifold::extrude`](crate::Manifold::extrude).

use manifold_csg_sys::*;
use std::ops;

use crate::manifold::read_polygons;
use crate::rect::Rect;

/// Join type for [`CrossSection::offset`].
///
/// Determines how corners are handled when inflating/deflating a 2D shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    /// Square (flat) corners.
    Square,
    /// Rounded corners.
    Round,
    /// Mitered (sharp) corners, limited by miter_limit.
    Miter,
    /// Beveled corners.
    Bevel,
}

impl JoinType {
    const fn to_ffi(self) -> ManifoldJoinType {
        match self {
            JoinType::Square => ManifoldJoinType::Square,
            JoinType::Round => ManifoldJoinType::Round,
            JoinType::Miter => ManifoldJoinType::Miter,
            JoinType::Bevel => ManifoldJoinType::Bevel,
        }
    }
}

/// Fill rule for constructing cross-sections from polygons.
///
/// Determines how self-intersecting or overlapping polygon contours are
/// interpreted when creating a [`CrossSection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    /// Alternating inside/outside based on crossing count parity.
    EvenOdd,
    /// Inside if crossing count is non-zero.
    NonZero,
    /// Inside if crossing count is positive.
    Positive,
    /// Inside if crossing count is negative.
    Negative,
}

impl FillRule {
    const fn to_ffi(self) -> ManifoldFillRule {
        match self {
            FillRule::EvenOdd => ManifoldFillRule::EvenOdd,
            FillRule::NonZero => ManifoldFillRule::NonZero,
            FillRule::Positive => ManifoldFillRule::Positive,
            FillRule::Negative => ManifoldFillRule::Negative,
        }
    }
}

/// 2D axis-aligned bounding rectangle (legacy convenience type).
///
/// Consider using [`Rect`] instead for richer spatial queries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2 {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Default for Rect2 {
    fn default() -> Self {
        Self { min_x: 0.0, min_y: 0.0, max_x: 0.0, max_y: 0.0 }
    }
}

/// A safe wrapper around a manifold3d CrossSection object.
///
/// Represents a 2D region suitable for Boolean operations, offsetting,
/// and extrusion to 3D. Memory is automatically freed when dropped.
///
/// # Example
///
/// ```rust,ignore
/// use manifold_csg::{Manifold, CrossSection, JoinType};
///
/// let section = CrossSection::square(10.0, 10.0, true);
/// let expanded = section.offset(2.0, JoinType::Round, 2.0, 16);
/// let solid = Manifold::extrude(&expanded, 20.0);
/// ```
pub struct CrossSection {
    pub(crate) ptr: *mut ManifoldCrossSection,
}

impl std::fmt::Debug for CrossSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrossSection")
            .field("is_empty", &self.is_empty())
            .field("area", &self.area())
            .field("num_vert", &self.num_vert())
            .finish()
    }
}

// SAFETY: Same rationale as Manifold — single-ownership transfer across
// threads is sound. The underlying Clipper2 data is an owned heap allocation.
unsafe impl Send for CrossSection {}

impl Clone for CrossSection {
    fn clone(&self) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_cross_section_copy(ptr, self.ptr) };
        Self { ptr }
    }
}

impl Drop for CrossSection {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_cross_section
            // and has not been freed (Drop runs once).
            unsafe { manifold_delete_cross_section(self.ptr) };
        }
    }
}

impl CrossSection {
    // ── Constructors ────────────────────────────────────────────────

    /// Empty cross-section (identity for union).
    #[must_use]
    pub fn empty() -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_cross_section_empty(ptr) };
        Self { ptr }
    }

    /// Axis-aligned rectangle. If `center` is true, centered at origin.
    #[must_use]
    pub fn square(x: f64, y: f64, center: bool) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_cross_section_square(ptr, x, y, i32::from(center)) };
        Self { ptr }
    }

    /// Circle centered at the origin.
    #[must_use]
    pub fn circle(radius: f64, segments: i32) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_cross_section_circle(ptr, radius, segments) };
        Self { ptr }
    }

    /// Create a cross-section from polygon rings.
    ///
    /// The first ring is the outer boundary; subsequent rings are holes.
    /// Uses EvenOdd fill rule. For self-intersecting or overlapping polygons,
    /// use [`from_polygons_with_fill_rule`](Self::from_polygons_with_fill_rule).
    #[must_use]
    pub fn from_polygons(polygons: &[Vec<[f64; 2]>]) -> Self {
        Self::from_polygons_with_fill_rule(polygons, FillRule::EvenOdd)
    }

    /// Create a cross-section from polygon rings with a specified fill rule.
    ///
    /// The fill rule determines how self-intersecting or overlapping contours
    /// are interpreted. See [`FillRule`] for details.
    #[must_use]
    pub fn from_polygons_with_fill_rule(
        polygons: &[Vec<[f64; 2]>],
        fill_rule: FillRule,
    ) -> Self {
        if polygons.is_empty() {
            return Self::empty();
        }

        let (polys_ptr, simple_ptrs) = build_polygons_ffi(polygons);

        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and polys_ptr are valid.
        unsafe {
            manifold_cross_section_of_polygons(ptr, polys_ptr, fill_rule.to_ffi());
        }

        // Clean up polygon allocations.
        // SAFETY: polys_ptr and simple polygon handles are valid and no longer needed.
        unsafe { manifold_delete_polygons(polys_ptr) };
        for sp in simple_ptrs {
            // SAFETY: sp is valid and no longer needed.
            unsafe { manifold_delete_simple_polygon(sp) };
        }

        Self { ptr }
    }

    // ── Booleans ────────────────────────────────────────────────────

    /// Boolean union: `self ∪ other`.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_cross_section_union(ptr, self.ptr, other.ptr) };
        Self { ptr }
    }

    /// Boolean difference: `self − other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_cross_section_difference(ptr, self.ptr, other.ptr) };
        Self { ptr }
    }

    /// Boolean intersection: `self ∩ other`.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_cross_section_intersection(ptr, self.ptr, other.ptr) };
        Self { ptr }
    }

    // ── Offset ──────────────────────────────────────────────────────

    /// Inflate (positive delta) or deflate (negative delta) the cross-section.
    ///
    /// Uses Clipper2's offset algorithm for true geometric offsetting.
    ///
    /// # Arguments
    ///
    /// * `delta` - offset distance (positive = grow, negative = shrink)
    /// * `join_type` - how to handle corners (Square, Round, Miter)
    /// * `miter_limit` - maximum distance for miter joins
    /// * `circular_segments` - resolution for round joins
    #[must_use]
    pub fn offset(
        &self,
        delta: f64,
        join_type: JoinType,
        miter_limit: f64,
        circular_segments: i32,
    ) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe {
            manifold_cross_section_offset(
                ptr,
                self.ptr,
                delta,
                join_type.to_ffi(),
                miter_limit,
                circular_segments,
            );
        }
        Self { ptr }
    }

    // ── Hull ────────────────────────────────────────────────────────

    /// Convex hull of this cross-section.
    #[must_use]
    pub fn hull(&self) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_hull(ptr, self.ptr) };
        Self { ptr }
    }

    // ── Transforms ──────────────────────────────────────────────────

    /// Translate by (x, y).
    #[must_use]
    pub fn translate(&self, x: f64, y: f64) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_translate(ptr, self.ptr, x, y) };
        Self { ptr }
    }

    /// Rotate by `degrees` (counter-clockwise).
    #[must_use]
    pub fn rotate(&self, degrees: f64) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_rotate(ptr, self.ptr, degrees) };
        Self { ptr }
    }

    /// Scale by (x, y).
    #[must_use]
    pub fn scale(&self, x: f64, y: f64) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_scale(ptr, self.ptr, x, y) };
        Self { ptr }
    }

    /// Mirror across an axis defined by direction (ax_x, ax_y).
    #[must_use]
    pub fn mirror(&self, ax_x: f64, ax_y: f64) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_mirror(ptr, self.ptr, ax_x, ax_y) };
        Self { ptr }
    }

    /// Apply a 2D affine transformation via a 3x2 column-major matrix.
    ///
    /// Layout: `[x1, y1, x2, y2, x3, y3]` where columns are:
    /// - col1 `(x1,y1)` — X basis vector
    /// - col2 `(x2,y2)` — Y basis vector
    /// - col3 `(x3,y3)` — translation
    #[must_use]
    pub fn transform(&self, m: &[f64; 6]) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe {
            manifold_cross_section_transform(
                ptr, self.ptr,
                m[0], m[1],
                m[2], m[3],
                m[4], m[5],
            );
        }
        Self { ptr }
    }

    // ── Decomposition ──────────────────────────────────────────────

    /// Decompose into connected components.
    #[must_use]
    pub fn decompose(&self) -> Vec<Self> {
        // SAFETY: manifold_alloc_cross_section_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_cross_section_vec() };
        // SAFETY: vec_ptr and self.ptr are valid.
        unsafe { manifold_cross_section_decompose(vec_ptr, self.ptr) };
        // SAFETY: vec_ptr is valid.
        let n = unsafe { manifold_cross_section_vec_length(vec_ptr) };
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            // SAFETY: manifold_alloc_cross_section returns a valid handle.
            let cs_ptr = unsafe { manifold_alloc_cross_section() };
            // SAFETY: vec_ptr is valid, i is in range.
            unsafe { manifold_cross_section_vec_get(cs_ptr, vec_ptr, i) };
            result.push(Self { ptr: cs_ptr });
        }
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_cross_section_vec(vec_ptr) };
        result
    }

    // ── Queries ─────────────────────────────────────────────────────

    /// Total enclosed area.
    #[must_use]
    pub fn area(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_cross_section_area(self.ptr) }
    }

    /// Number of vertices.
    #[must_use]
    pub fn num_vert(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_cross_section_num_vert(self.ptr) }
    }

    /// Number of contours.
    #[must_use]
    pub fn num_contour(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_cross_section_num_contour(self.ptr) }
    }

    /// Whether the cross-section is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_cross_section_is_empty(self.ptr) != 0 }
    }

    /// Axis-aligned bounding rectangle.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        // SAFETY: manifold_alloc_rect returns a valid handle.
        let rect_ptr = unsafe { manifold_alloc_rect() };
        // SAFETY: rect_ptr and self.ptr are valid.
        unsafe { manifold_cross_section_bounds(rect_ptr, self.ptr) };
        Rect::from_ptr(rect_ptr)
    }

    /// Axis-aligned bounding rectangle as raw min/max values.
    ///
    /// Convenience method returning a simple struct. For spatial queries,
    /// use [`bounds`](Self::bounds) which returns a [`Rect`] with richer methods.
    #[must_use]
    pub fn bounds_rect2(&self) -> Rect2 {
        let r = self.bounds();
        let lo = r.min();
        let hi = r.max();
        Rect2 {
            min_x: lo[0],
            min_y: lo[1],
            max_x: hi[0],
            max_y: hi[1],
        }
    }

    // ── Simplification & Batch ──────────────────────────────────────

    /// Simplify the cross-section, removing vertices closer than `epsilon`.
    #[must_use]
    pub fn simplify(&self, epsilon: f64) -> Self {
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_cross_section_simplify(ptr, self.ptr, epsilon) };
        Self { ptr }
    }

    /// Batch boolean: apply `op` across multiple cross-sections.
    #[must_use]
    pub fn batch_boolean(sections: &[Self], op: crate::OpType) -> Self {
        if sections.is_empty() {
            return Self::empty();
        }
        // SAFETY: manifold_alloc_cross_section_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_cross_section_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_cross_section_empty_vec(vec_ptr) };
        for cs in sections {
            // SAFETY: manifold_alloc_cross_section returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_cross_section() };
            // SAFETY: copy_ptr is valid from alloc, cs.ptr is valid (invariant).
            unsafe { manifold_cross_section_copy(copy_ptr, cs.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid cross-section.
            unsafe { manifold_cross_section_vec_push_back(vec_ptr, copy_ptr) };
        }
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and vec_ptr are valid.
        unsafe { manifold_cross_section_batch_boolean(ptr, vec_ptr, op) };
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_cross_section_vec(vec_ptr) };
        Self { ptr }
    }

    /// Batch union of multiple cross-sections.
    #[must_use]
    pub fn batch_union(sections: &[Self]) -> Self {
        Self::batch_boolean(sections, crate::OpType::Add)
    }

    /// Batch hull of multiple cross-sections.
    #[must_use]
    pub fn batch_hull(sections: &[Self]) -> Self {
        if sections.is_empty() {
            return Self::empty();
        }
        // SAFETY: manifold_alloc_cross_section_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_cross_section_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_cross_section_empty_vec(vec_ptr) };
        for cs in sections {
            // SAFETY: manifold_alloc_cross_section returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_cross_section() };
            // SAFETY: copy_ptr is valid from alloc, cs.ptr is valid (invariant).
            unsafe { manifold_cross_section_copy(copy_ptr, cs.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid cross-section.
            unsafe { manifold_cross_section_vec_push_back(vec_ptr, copy_ptr) };
        }
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and vec_ptr are valid.
        unsafe { manifold_cross_section_batch_hull(ptr, vec_ptr) };
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_cross_section_vec(vec_ptr) };
        Self { ptr }
    }

    /// Compose multiple cross-sections into one (without boolean).
    #[must_use]
    pub fn compose(sections: &[Self]) -> Self {
        // SAFETY: manifold_alloc_cross_section_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_cross_section_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_cross_section_empty_vec(vec_ptr) };
        for cs in sections {
            // SAFETY: manifold_alloc_cross_section returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_cross_section() };
            // SAFETY: copy_ptr is valid from alloc, cs.ptr is valid (invariant).
            unsafe { manifold_cross_section_copy(copy_ptr, cs.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid cross-section.
            unsafe { manifold_cross_section_vec_push_back(vec_ptr, copy_ptr) };
        }
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr and vec_ptr are valid.
        unsafe { manifold_cross_section_compose(ptr, vec_ptr) };
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_cross_section_vec(vec_ptr) };
        Self { ptr }
    }

    // ── Convenience ──────────────────────────────────────────────────

    /// Extrude this cross-section into a 3D manifold along the Z axis.
    ///
    /// Convenience method equivalent to `Manifold::extrude(self, height)`.
    #[must_use]
    pub fn extrude(&self, height: f64) -> crate::Manifold {
        crate::Manifold::extrude(self, height)
    }

    // ── Warp ─────────────────────────────────────────────────────────

    /// Apply a warp function to deform each vertex.
    ///
    /// The closure receives `(x, y)` and returns `[x', y']`.
    #[must_use]
    pub fn warp<F>(&self, f: F) -> Self
    where
        F: FnMut(f64, f64) -> [f64; 2],
    {
        unsafe extern "C" fn trampoline<F>(
            x: f64, y: f64,
            ctx: *mut std::ffi::c_void,
        ) -> ManifoldVec2
        where
            F: FnMut(f64, f64) -> [f64; 2],
        {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut F and is valid for the call duration.
                let f = unsafe { &mut *(ctx as *mut F) };
                f(x, y)
            }));
            match result {
                Ok([rx, ry]) => ManifoldVec2 { x: rx, y: ry },
                // Return the original point on panic to avoid UB from unwinding through C.
                Err(_) => ManifoldVec2 { x, y },
            }
        }

        let mut closure = f;
        let ctx = &mut closure as *mut F as *mut std::ffi::c_void;
        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: ptr valid from alloc, self.ptr valid (invariant), trampoline+ctx valid.
        unsafe { manifold_cross_section_warp_context(ptr, self.ptr, Some(trampoline::<F>), ctx) };
        Self { ptr }
    }

    // ── Extraction ──────────────────────────────────────────────────

    /// Convert to polygon rings.
    ///
    /// Returns a list of contours, each being a list of `[x, y]` points.
    #[must_use]
    pub fn to_polygons(&self) -> Vec<Vec<[f64; 2]>> {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: poly_ptr and self.ptr are valid.
        unsafe { manifold_cross_section_to_polygons(poly_ptr, self.ptr) };

        let result = read_polygons(poly_ptr);

        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        result
    }
}

// ── CrossSection operator overloads ─────────────────────────────────────

/// `a + b` → Boolean union.
impl ops::Add for &CrossSection {
    type Output = CrossSection;
    fn add(self, rhs: &CrossSection) -> CrossSection {
        self.union(rhs)
    }
}

/// `a - b` → Boolean difference.
impl ops::Sub for &CrossSection {
    type Output = CrossSection;
    fn sub(self, rhs: &CrossSection) -> CrossSection {
        self.difference(rhs)
    }
}

/// `a ^ b` → Boolean intersection.
impl ops::BitXor for &CrossSection {
    type Output = CrossSection;
    fn bitxor(self, rhs: &CrossSection) -> CrossSection {
        self.intersection(rhs)
    }
}

// ── Internal helper: build polygon FFI objects from Rust slices ──────────

/// Build ManifoldPolygons + ManifoldSimplePolygon handles from polygon rings.
///
/// The caller is responsible for freeing both the returned `ManifoldPolygons`
/// and each `ManifoldSimplePolygon` in the vector.
pub(crate) fn build_polygons_ffi(
    polygons: &[Vec<[f64; 2]>],
) -> (*mut ManifoldPolygons, Vec<*mut ManifoldSimplePolygon>) {
    let mut simple_ptrs: Vec<*mut ManifoldSimplePolygon> = Vec::with_capacity(polygons.len());
    for ring in polygons {
        let vec2s: Vec<ManifoldVec2> = ring
            .iter()
            .map(|p| ManifoldVec2 { x: p[0], y: p[1] })
            .collect();
        // SAFETY: manifold_alloc_simple_polygon returns a valid handle.
        let sp = unsafe { manifold_alloc_simple_polygon() };
        // SAFETY: sp is valid, vec2s is a valid slice.
        unsafe { manifold_simple_polygon(sp, vec2s.as_ptr(), vec2s.len()) };
        simple_ptrs.push(sp);
    }

    // SAFETY: manifold_alloc_polygons returns a valid handle.
    let polys_ptr = unsafe { manifold_alloc_polygons() };
    // SAFETY: polys_ptr is valid, simple_ptrs is a valid slice of valid handles.
    unsafe { manifold_polygons(polys_ptr, simple_ptrs.as_ptr(), simple_ptrs.len()) };

    (polys_ptr, simple_ptrs)
}
