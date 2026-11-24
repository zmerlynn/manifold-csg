//! Safe wrapper around a manifold3d CrossSection object (2D region).
//!
//! [`CrossSection`] provides 2D boolean operations, geometric offset, convex
//! hull, and transforms. Cross-sections can be extruded to 3D via
//! [`Manifold::extrude`](crate::Manifold::extrude).

use crate::owned;
use manifold_csg_sys::*;
use std::ops;
use std::sync::Mutex;

use crate::manifold::read_polygons;
use crate::quality::CircularQuality;
use crate::rect::Rect;
use crate::types::{OpType, PanicPayload, store_panic, take_stored_panic};

/// Join type for [`CrossSection::offset`].
///
/// Determines how corners are handled when inflating/deflating a 2D shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
/// interpreted when creating a [`CrossSection`]. The rule only decides how the
/// input is read: the result is regularized like any other cross-section.
///
/// Upstream master (the upcoming manifold 4.0) dropped the `Negative` rule. Reverse the contours and
/// use [`Positive`](Self::Positive) for the same result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FillRule {
    /// Alternating inside/outside based on crossing count parity. Use this
    /// for input authored for an even-odd tool such as OpenSCAD.
    EvenOdd,
    /// Inside where the winding number is non-zero, so a winding of two is
    /// still filled.
    NonZero,
    /// Inside where the winding number is positive. The upstream default.
    Positive,
}

/// A C constructor taking one kind of polygon input.
type PolygonCtor<P> = unsafe extern "C" fn(*const P) -> *mut ManifoldCrossSection;

impl FillRule {
    /// The C constructor that reads polygons with this rule.
    pub(crate) fn polygons_ctor(self) -> PolygonCtor<ManifoldPolygons> {
        match self {
            FillRule::EvenOdd => manifold_cross_section_even_odd_polygons,
            FillRule::NonZero => manifold_cross_section_non_zero_polygons,
            FillRule::Positive => manifold_cross_section_of_polygons,
        }
    }

    /// The C constructor that reads one contour with this rule.
    fn simple_polygon_ctor(self) -> PolygonCtor<ManifoldSimplePolygon> {
        match self {
            FillRule::EvenOdd => manifold_cross_section_even_odd_simple_polygon,
            FillRule::NonZero => manifold_cross_section_non_zero_simple_polygon,
            FillRule::Positive => manifold_cross_section_of_simple_polygon,
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
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }
}

/// A safe wrapper around a manifold3d CrossSection object.
///
/// Represents a 2D region suitable for Boolean operations, offsetting,
/// and extrusion to 3D. Memory is automatically freed when dropped.
///
/// See the [upstream `CrossSection` docs](https://elalish.github.io/manifold/docs/html/classmanifold_1_1_cross_section.html)
/// for details on the underlying algorithms and parameter semantics.
///
/// # Example
///
/// ```
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
            .field("ptr", &self.ptr)
            .finish_non_exhaustive()
    }
}

// SAFETY: Same rationale as Manifold — single-ownership transfer across
// threads is sound. The underlying polygon data is an owned heap allocation.
unsafe impl Send for CrossSection {}

// SAFETY: The C++ CrossSection class synchronizes lazy path evaluation, so
// concurrent const access from multiple threads is safe.
unsafe impl Sync for CrossSection {}

impl Clone for CrossSection {
    fn clone(&self) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_copy(self.ptr) });
        Self { ptr }
    }
}

impl Drop for CrossSection {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr came from a manifold_* constructor and is freed only here.
            unsafe { manifold_delete_cross_section(self.ptr) };
            self.ptr = std::ptr::null_mut();
        }
    }
}

impl Default for CrossSection {
    fn default() -> Self {
        Self::empty()
    }
}

impl CrossSection {
    // ── Constructors ────────────────────────────────────────────────

    /// Empty cross-section (identity for union).
    #[must_use]
    pub fn empty() -> Self {
        // SAFETY: takes no arguments and returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_empty() });
        Self { ptr }
    }

    /// Axis-aligned rectangle. If `center` is true, centered at origin.
    #[must_use]
    pub fn square(x: f64, y: f64, center: bool) -> Self {
        // SAFETY: takes plain values and returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_square(x, y, i32::from(center)) });
        Self { ptr }
    }

    /// Circle centered at the origin.
    ///
    /// `quality` is a [`CircularQuality`] or a plain segment count (`0` for
    /// the default quality).
    #[must_use]
    pub fn circle(radius: f64, quality: impl Into<CircularQuality>) -> Self {
        let q = quality.into().to_sys();
        // SAFETY: &q is valid for the call, and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_circle(radius, &q) });
        Self { ptr }
    }

    /// Create a cross-section from polygon rings.
    ///
    /// Uses the upstream default [`FillRule::Positive`], which treats
    /// positively oriented contours as filled regions and negatively oriented
    /// contours as holes. For other winding semantics, use
    /// [`from_polygons_with_fill_rule`](Self::from_polygons_with_fill_rule).
    #[must_use]
    pub fn from_polygons(polygons: &[Vec<[f64; 2]>]) -> Self {
        Self::from_polygons_with_fill_rule(polygons, FillRule::Positive)
    }

    /// Create a cross-section from polygon rings with a specified fill rule.
    ///
    /// The fill rule determines how self-intersecting or overlapping contours
    /// are interpreted. See [`FillRule`] for details.
    #[must_use]
    pub fn from_polygons_with_fill_rule(polygons: &[Vec<[f64; 2]>], fill_rule: FillRule) -> Self {
        if polygons.is_empty() {
            return Self::empty();
        }

        let polys = PolygonsFfi::new(polygons);

        let ctor = fill_rule.polygons_ctor();
        // SAFETY: polys.ptr() is valid until polys drops at the end of this
        // function, and ctor returns a new handle.
        let ptr = owned(unsafe { ctor(polys.ptr()) });
        Self { ptr }
    }

    /// Create a cross-section from a single simple polygon (no holes).
    ///
    /// Uses the upstream default [`FillRule::Positive`]. For other winding
    /// semantics, use
    /// [`from_simple_polygon_with_fill_rule`](Self::from_simple_polygon_with_fill_rule).
    ///
    /// For polygons with holes, use [`from_polygons`](Self::from_polygons).
    #[must_use]
    pub fn from_simple_polygon(points: &[[f64; 2]]) -> Self {
        Self::from_simple_polygon_with_fill_rule(points, FillRule::Positive)
    }

    /// Create a cross-section from a single simple polygon with a specified
    /// fill rule.
    ///
    /// For polygons with holes, use [`from_polygons`](Self::from_polygons).
    #[must_use]
    pub fn from_simple_polygon_with_fill_rule(points: &[[f64; 2]], fill_rule: FillRule) -> Self {
        if points.is_empty() {
            return Self::empty();
        }

        let vec2s: Vec<ManifoldVec2> = points
            .iter()
            .map(|p| ManifoldVec2 { x: p[0], y: p[1] })
            .collect();
        // SAFETY: vec2s outlives the call, and the call returns a new handle.
        let sp = owned(unsafe { manifold_simple_polygon(vec2s.as_ptr(), vec2s.len()) });

        let ctor = fill_rule.simple_polygon_ctor();
        // SAFETY: sp is valid until it is deleted below, and ctor returns a
        // new handle.
        let ptr = owned(unsafe { ctor(sp) });

        // SAFETY: sp is valid and no longer needed.
        unsafe { manifold_delete_simple_polygon(sp) };

        Self { ptr }
    }

    /// Compute the convex hull of a simple polygon.
    #[must_use]
    pub fn hull_simple_polygon(points: &[[f64; 2]]) -> Self {
        if points.is_empty() {
            return Self::empty();
        }

        let vec2s: Vec<ManifoldVec2> = points
            .iter()
            .map(|p| ManifoldVec2 { x: p[0], y: p[1] })
            .collect();
        // SAFETY: vec2s outlives the call, and the call returns a new handle.
        let sp = owned(unsafe { manifold_simple_polygon(vec2s.as_ptr(), vec2s.len()) });

        // SAFETY: sp is valid until it is deleted below, and the call returns a
        // new handle.
        let ptr = owned(unsafe { manifold_cross_section_hull_simple_polygon(sp) });

        // SAFETY: sp is valid and no longer needed.
        unsafe { manifold_delete_simple_polygon(sp) };

        Self { ptr }
    }

    /// Compute the convex hull of a polygon set (multiple rings).
    #[must_use]
    pub fn hull_polygons(polygons: &[Vec<[f64; 2]>]) -> Self {
        if polygons.is_empty() {
            return Self::empty();
        }

        let polys = PolygonsFfi::new(polygons);

        // SAFETY: polys.ptr() is valid until polys drops, and the call returns a
        // new handle.
        let ptr = owned(unsafe { manifold_cross_section_hull_polygons(polys.ptr()) });

        Self { ptr }
    }

    // ── Booleans ────────────────────────────────────────────────────

    /// Boolean union: `self ∪ other`.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        // SAFETY: self.ptr and other.ptr are valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_union(self.ptr, other.ptr) });
        Self { ptr }
    }

    /// Boolean difference: `self − other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        // SAFETY: self.ptr and other.ptr are valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_difference(self.ptr, other.ptr) });
        Self { ptr }
    }

    /// Boolean intersection: `self ∩ other`.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        // SAFETY: self.ptr and other.ptr are valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_intersection(self.ptr, other.ptr) });
        Self { ptr }
    }

    /// Generic boolean operation with an explicit operation type.
    ///
    /// Prefer the specific methods ([`union`](Self::union),
    /// [`difference`](Self::difference), [`intersection`](Self::intersection))
    /// or operator overloads for readability.
    #[must_use]
    pub fn boolean(&self, other: &Self, op: OpType) -> Self {
        // SAFETY: self.ptr and other.ptr are valid (invariant), and the call returns a new handle.
        let ptr =
            owned(unsafe { manifold_cross_section_boolean(self.ptr, other.ptr, op.to_ffi()) });
        Self { ptr }
    }

    // ── Offset ──────────────────────────────────────────────────────

    /// Inflate (positive delta) or deflate (negative delta) the cross-section.
    ///
    /// A true geometric offset, not a scale.
    ///
    /// # Arguments
    ///
    /// * `delta` - offset distance (positive = grow, negative = shrink)
    /// * `join_type` - how to handle corners (Square, Round, Miter)
    /// * `miter_limit` - maximum distance for miter joins
    /// * `quality` - resolution for round joins: a [`CircularQuality`] or a
    ///   plain segment count (`0` for the default quality)
    #[must_use]
    pub fn offset(
        &self,
        delta: f64,
        join_type: JoinType,
        miter_limit: f64,
        quality: impl Into<CircularQuality>,
    ) -> Self {
        let q = quality.into().to_sys();
        // SAFETY: self.ptr and &q are valid (invariant), and the call returns
        // a new handle.
        let ptr = owned(unsafe {
            manifold_cross_section_offset(self.ptr, delta, join_type.to_ffi(), miter_limit, &q)
        });
        Self { ptr }
    }

    // ── Hull ────────────────────────────────────────────────────────

    /// Convex hull of this cross-section.
    #[must_use]
    pub fn hull(&self) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_hull(self.ptr) });
        Self { ptr }
    }

    // ── Transforms ──────────────────────────────────────────────────

    /// Translate by (x, y).
    #[must_use]
    pub fn translate(&self, x: f64, y: f64) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_translate(self.ptr, x, y) });
        Self { ptr }
    }

    /// Rotate by `degrees` (counter-clockwise).
    #[must_use]
    pub fn rotate(&self, degrees: f64) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_rotate(self.ptr, degrees) });
        Self { ptr }
    }

    /// Scale by (x, y).
    #[must_use]
    pub fn scale(&self, x: f64, y: f64) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_scale(self.ptr, x, y) });
        Self { ptr }
    }

    /// Mirror across an axis defined by direction (ax_x, ax_y).
    #[must_use]
    pub fn mirror(&self, ax_x: f64, ax_y: f64) -> Self {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_mirror(self.ptr, ax_x, ax_y) });
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
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe {
            manifold_cross_section_transform(self.ptr, m[0], m[1], m[2], m[3], m[4], m[5])
        });
        Self { ptr }
    }

    // ── Decomposition ──────────────────────────────────────────────

    /// Decompose into connected components.
    #[must_use]
    pub fn decompose(&self) -> Vec<Self> {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let vec_ptr = owned(unsafe { manifold_cross_section_decompose(self.ptr) });
        // SAFETY: vec_ptr is valid.
        let n = unsafe { manifold_cross_section_vec_length(vec_ptr) };
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            // SAFETY: vec_ptr is valid for the call, and the call returns a new handle.
            let cs_ptr = owned(unsafe { manifold_cross_section_vec_get(vec_ptr, i) });
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
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let rect_ptr = owned(unsafe { manifold_cross_section_bounds(self.ptr) });
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
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_simplify(self.ptr, epsilon) });
        Self { ptr }
    }

    /// Batch boolean: apply `op` across multiple cross-sections.
    #[must_use]
    pub fn batch_boolean(sections: &[Self], op: OpType) -> Self {
        if sections.is_empty() {
            return Self::empty();
        }
        // SAFETY: takes no arguments and returns a new handle.
        let vec_ptr = owned(unsafe { manifold_cross_section_empty_vec() });
        for cs in sections {
            // SAFETY: vec_ptr and cs.ptr are valid. push_back copies the
            // cross-section into the vector, so cs keeps its own handle.
            unsafe { manifold_cross_section_vec_push_back(vec_ptr, cs.ptr) };
        }
        // SAFETY: vec_ptr is valid until it is deleted below, and the call
        // returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_batch_boolean(vec_ptr, op.to_ffi()) });
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_cross_section_vec(vec_ptr) };
        Self { ptr }
    }

    /// Batch union of multiple cross-sections.
    #[must_use]
    pub fn batch_union(sections: &[Self]) -> Self {
        Self::batch_boolean(sections, OpType::Add)
    }

    /// Batch hull of multiple cross-sections.
    #[must_use]
    pub fn batch_hull(sections: &[Self]) -> Self {
        if sections.is_empty() {
            return Self::empty();
        }
        // SAFETY: takes no arguments and returns a new handle.
        let vec_ptr = owned(unsafe { manifold_cross_section_empty_vec() });
        for cs in sections {
            // SAFETY: vec_ptr and cs.ptr are valid. push_back copies the
            // cross-section into the vector, so cs keeps its own handle.
            unsafe { manifold_cross_section_vec_push_back(vec_ptr, cs.ptr) };
        }
        // SAFETY: vec_ptr is valid until it is deleted below, and the call
        // returns a new handle.
        let ptr = owned(unsafe { manifold_cross_section_batch_hull(vec_ptr) });
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
        struct Context<'a, F> {
            f: &'a mut F,
            panic: Mutex<Option<PanicPayload>>,
        }

        unsafe extern "C" fn trampoline<F>(
            x: f64,
            y: f64,
            ctx: *mut std::ffi::c_void,
        ) -> ManifoldVec2
        where
            F: FnMut(f64, f64) -> [f64; 2],
        {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut Context and is valid for
                // the call duration.
                let ctx = unsafe { &mut *(ctx as *mut Context<'_, F>) };
                (ctx.f)(x, y)
            }));
            match result {
                Ok([rx, ry]) => ManifoldVec2 { x: rx, y: ry },
                Err(payload) => {
                    // Return the original point to avoid UB from unwinding
                    // through C, then resume after the FFI call.
                    // SAFETY: ctx was created from a &mut Context below and is
                    // valid for the duration of the cross-section warp call.
                    let ctx = unsafe { &*(ctx as *const Context<'_, F>) };
                    store_panic(&ctx.panic, payload);
                    ManifoldVec2 { x, y }
                }
            }
        }

        let mut closure = f;
        let mut ctx = Context {
            f: &mut closure,
            panic: Mutex::new(None),
        };
        let ctx_ptr = &mut ctx as *mut Context<'_, F> as *mut std::ffi::c_void;
        // SAFETY: self.ptr is valid (invariant), ctx_ptr is valid for the call, and the call returns a new handle.
        let ptr =
            owned(unsafe { manifold_cross_section_warp(self.ptr, Some(trampoline::<F>), ctx_ptr) });
        if let Some(payload) = take_stored_panic(&ctx.panic) {
            // SAFETY: ptr came from the call above and is not returned.
            unsafe { manifold_delete_cross_section(ptr) };
            std::panic::resume_unwind(payload);
        }
        Self { ptr }
    }

    // ── Extraction ──────────────────────────────────────────────────

    /// Convert to polygon rings.
    ///
    /// Returns a list of contours, each being a list of `[x, y]` points.
    #[must_use]
    pub fn to_polygons(&self) -> Vec<Vec<[f64; 2]>> {
        // SAFETY: self.ptr is valid (invariant), and the call returns a new handle.
        let poly_ptr = owned(unsafe { manifold_cross_section_to_polygons(self.ptr) });

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

/// A C-side `ManifoldPolygons` built from Rust rings.
///
/// The handle has to stay alive until the call that consumes
/// [`ptr`](Self::ptr) returns, and has to be freed afterwards. Owning it here
/// and freeing in `Drop` keeps that true on every path out of the caller,
/// including a panic, which a free at the end of the function body does not.
pub(crate) struct PolygonsFfi {
    ptr: *mut ManifoldPolygons,
}

impl PolygonsFfi {
    pub(crate) fn new(polygons: &[Vec<[f64; 2]>]) -> Self {
        let mut simple: Vec<*mut ManifoldSimplePolygon> = Vec::with_capacity(polygons.len());
        for ring in polygons {
            let vec2s: Vec<ManifoldVec2> = ring
                .iter()
                .map(|p| ManifoldVec2 { x: p[0], y: p[1] })
                .collect();
            // SAFETY: vec2s outlives the call, and the call returns a new handle.
            let sp = owned(unsafe { manifold_simple_polygon(vec2s.as_ptr(), vec2s.len()) });
            simple.push(sp);
        }

        // SAFETY: simple holds simple.len() valid handles, and the cast only adds
        // const (*mut T and *const T have the same layout). The call
        // deep-copies each contour and returns a new handle.
        let ptr = owned(unsafe {
            manifold_polygons(
                simple.as_ptr().cast::<*const ManifoldSimplePolygon>(),
                simple.len(),
            )
        });

        for sp in simple {
            // SAFETY: sp came from manifold_simple_polygon above, and
            // manifold_polygons copied it, so nothing else refers to it.
            unsafe { manifold_delete_simple_polygon(sp) };
        }

        Self { ptr }
    }

    pub(crate) fn ptr(&self) -> *mut ManifoldPolygons {
        self.ptr
    }
}

impl Drop for PolygonsFfi {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr is valid (invariant) and freed exactly once.
            unsafe { manifold_delete_polygons(self.ptr) };
        }
    }
}
