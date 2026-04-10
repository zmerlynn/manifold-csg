//! Safe wrapper around a manifold3d Manifold object (3D solid).
//!
//! [`Manifold`] wraps the manifold3d C library with a safe Rust API. It
//! implements `Drop` for automatic memory management and provides operator
//! overloads for the three Boolean operations:
//!
//! - `a - b` → difference (subtract b from a)
//! - `a + b` → union
//! - `a ^ b` → intersection (`BitXor`, since `&` is borrowed-ref in Rust)
//!
//! # Example
//!
//! ```rust,ignore
//! use manifold_csg::Manifold;
//!
//! let cube = Manifold::cube(10.0, 10.0, 10.0, true);
//! let hole = Manifold::cylinder(20.0, 3.0, 3.0, 32, false).translate(0.0, 0.0, -10.0);
//! let result = &cube - &hole;
//! assert!(result.volume() < cube.volume());
//! ```

use manifold_csg_sys::*;
use std::ops;

use crate::bounding_box::BoundingBox;
use crate::cross_section::CrossSection;
use crate::types::CsgError;

/// A safe wrapper around a manifold3d Manifold object.
///
/// Represents a closed, manifold 3D solid suitable for Boolean operations.
/// Memory is automatically freed when the value is dropped.
pub struct Manifold {
    pub(crate) ptr: *mut ManifoldManifold,
}

impl std::fmt::Debug for Manifold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manifold")
            .field("is_empty", &self.is_empty())
            .field("num_vert", &self.num_vert())
            .field("num_tri", &self.num_tri())
            .finish()
    }
}

// SAFETY: Single-ownership transfer across threads is sound — ManifoldManifold
// is an owned heap allocation with no thread-local state. All operations that
// produce new geometry return new allocations.
unsafe impl Send for Manifold {}

// SAFETY: The C++ Manifold class synchronizes lazy CSG tree evaluation, so
// concurrent const access (volume, num_vert, etc.) from multiple threads is safe.
unsafe impl Sync for Manifold {}

impl Clone for Manifold {
    /// Clone via `manifold_copy`, producing a new independent C-side handle.
    fn clone(&self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_copy(ptr, self.ptr) };
        Self { ptr }
    }
}

impl Drop for Manifold {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_manifold() and
            // has not been freed (we only free in Drop, which runs once).
            unsafe { manifold_delete_manifold(self.ptr) };
        }
    }
}

impl Manifold {
    // ── Construction from raw mesh data ─────────────────────────────

    /// Create a Manifold from f64 vertex data and u64 triangle indices.
    ///
    /// This uses MeshGL64 internally for full f64 precision — no f32
    /// round-trip that would destroy sub-mm features at large coordinates.
    ///
    /// # Arguments
    ///
    /// * `vert_props` - flat f64 array of vertex properties, `n_props` values
    ///   per vertex (minimum 3 for x, y, z)
    /// * `n_props` - number of properties per vertex (>= 3)
    /// * `tri_indices` - flat u64 array, 3 vertex indices per triangle
    ///
    /// # Errors
    ///
    /// Returns `CsgError::InvalidInput` if `n_props < 3`,
    /// `CsgError::EmptyMesh` if no triangles, or `CsgError::ManifoldStatus`
    /// if the mesh is invalid.
    pub fn from_mesh_f64(
        vert_props: &[f64],
        n_props: usize,
        tri_indices: &[u64],
    ) -> Result<Self, CsgError> {
        if tri_indices.is_empty() {
            return Err(CsgError::EmptyMesh);
        }
        if n_props < 3 {
            return Err(CsgError::InvalidInput(
                "n_props must be >= 3 (x, y, z)".into(),
            ));
        }
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: meshgl is valid. vert_props and tri_indices are valid slices.
        unsafe {
            manifold_meshgl64(
                meshgl,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let manifold = unsafe { manifold_alloc_manifold() };
        // SAFETY: manifold and meshgl are valid handles.
        unsafe { manifold_of_meshgl64(manifold, meshgl) };
        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl64(meshgl) };

        // SAFETY: manifold is valid. Read-only status query.
        let status = unsafe { manifold_status(manifold) };
        if status != ManifoldError::NoError {
            // SAFETY: manifold is valid. Frees the allocation on error path.
            unsafe { manifold_delete_manifold(manifold) };
            return Err(CsgError::ManifoldStatus(status));
        }

        Ok(Self { ptr: manifold })
    }

    /// Create a Manifold from f32 vertex data and u32 triangle indices.
    ///
    /// Uses MeshGL (f32) internally. Prefer [`from_mesh_f64`](Self::from_mesh_f64)
    /// for precision-sensitive work.
    pub fn from_mesh_f32(
        vert_props: &[f32],
        n_props: usize,
        tri_indices: &[u32],
    ) -> Result<Self, CsgError> {
        if tri_indices.is_empty() {
            return Err(CsgError::EmptyMesh);
        }
        if n_props < 3 {
            return Err(CsgError::InvalidInput(
                "n_props must be >= 3 (x, y, z)".into(),
            ));
        }
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl() };
        // SAFETY: meshgl is valid. vert_props and tri_indices are valid slices.
        unsafe {
            manifold_meshgl(
                meshgl,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let manifold = unsafe { manifold_alloc_manifold() };
        // SAFETY: manifold and meshgl are valid handles.
        unsafe { manifold_of_meshgl(manifold, meshgl) };
        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl(meshgl) };

        // SAFETY: manifold is valid. Read-only status query.
        let status = unsafe { manifold_status(manifold) };
        if status != ManifoldError::NoError {
            // SAFETY: manifold is valid. Frees the allocation on error path.
            unsafe { manifold_delete_manifold(manifold) };
            return Err(CsgError::ManifoldStatus(status));
        }

        Ok(Self { ptr: manifold })
    }

    /// Create a smooth manifold from f64 mesh data with per-halfedge smoothness.
    ///
    /// `half_edges` and `smoothness` must have the same length. Each entry
    /// specifies a halfedge index and its smoothness value (0.0 = sharp,
    /// 1.0 = fully smooth).
    ///
    /// # Errors
    ///
    /// Returns `CsgError::InvalidInput` if array lengths don't match or
    /// `CsgError::ManifoldStatus` if the mesh is invalid.
    pub fn smooth_f64(
        vert_props: &[f64],
        n_props: usize,
        tri_indices: &[u64],
        half_edges: &[usize],
        smoothness: &[f64],
    ) -> Result<Self, CsgError> {
        if half_edges.len() != smoothness.len() {
            return Err(CsgError::InvalidInput(
                "half_edges and smoothness must have the same length".into(),
            ));
        }

        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: meshgl valid, slices valid with correct lengths.
        unsafe {
            manifold_meshgl64(
                meshgl,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let manifold = unsafe { manifold_alloc_manifold() };
        // SAFETY: all pointers valid, array lengths match.
        unsafe {
            manifold_smooth64(
                manifold,
                meshgl,
                half_edges.as_ptr(),
                smoothness.as_ptr(),
                half_edges.len(),
            );
        }
        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl64(meshgl) };

        // SAFETY: manifold is valid. Read-only status query.
        let status = unsafe { manifold_status(manifold) };
        if status != ManifoldError::NoError {
            // SAFETY: manifold is valid. Frees the allocation on error path.
            unsafe { manifold_delete_manifold(manifold) };
            return Err(CsgError::ManifoldStatus(status));
        }

        Ok(Self { ptr: manifold })
    }

    /// Create a smooth manifold from f32 mesh data with per-halfedge smoothness.
    ///
    /// See [`smooth_f64`](Self::smooth_f64) for details.
    pub fn smooth_f32(
        vert_props: &[f32],
        n_props: usize,
        tri_indices: &[u32],
        half_edges: &[usize],
        smoothness: &[f64],
    ) -> Result<Self, CsgError> {
        if half_edges.len() != smoothness.len() {
            return Err(CsgError::InvalidInput(
                "half_edges and smoothness must have the same length".into(),
            ));
        }

        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl() };
        // SAFETY: meshgl valid, slices valid with correct lengths.
        unsafe {
            manifold_meshgl(
                meshgl,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let manifold = unsafe { manifold_alloc_manifold() };
        // SAFETY: all pointers valid, array lengths match.
        unsafe {
            manifold_smooth(
                manifold,
                meshgl,
                half_edges.as_ptr(),
                smoothness.as_ptr(),
                half_edges.len(),
            );
        }
        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl(meshgl) };

        // SAFETY: manifold is valid. Read-only status query.
        let status = unsafe { manifold_status(manifold) };
        if status != ManifoldError::NoError {
            // SAFETY: manifold is valid. Frees the allocation on error path.
            unsafe { manifold_delete_manifold(manifold) };
            return Err(CsgError::ManifoldStatus(status));
        }

        Ok(Self { ptr: manifold })
    }

    /// Extract mesh data as f64 vertex properties and u64 triangle indices.
    ///
    /// Returns `(vert_props, n_props, tri_indices)`.
    #[must_use]
    pub fn to_mesh_f64(&self) -> (Vec<f64>, usize, Vec<u64>) {
        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: self.ptr and meshgl are valid handles.
        unsafe { manifold_get_meshgl64(meshgl, self.ptr) };

        // SAFETY: meshgl is valid. Read-only query.
        let n_verts = unsafe { manifold_meshgl64_num_vert(meshgl) };
        // SAFETY: meshgl is valid. Read-only query.
        let n_tris = unsafe { manifold_meshgl64_num_tri(meshgl) };
        // SAFETY: meshgl is valid. Read-only query.
        let n_props = unsafe { manifold_meshgl64_num_prop(meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let vp_len = unsafe { manifold_meshgl64_vert_properties_length(meshgl) };
        let mut vp_buf = vec![0.0f64; vp_len];
        // SAFETY: vp_buf has capacity vp_len.
        unsafe { manifold_meshgl64_vert_properties(vp_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let tri_len = unsafe { manifold_meshgl64_tri_length(meshgl) };
        let mut tri_buf = vec![0u64; tri_len];
        // SAFETY: tri_buf has capacity tri_len.
        unsafe { manifold_meshgl64_tri_verts(tri_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl64(meshgl) };

        debug_assert_eq!(vp_buf.len(), n_verts * n_props);
        debug_assert_eq!(tri_buf.len(), n_tris * 3);

        (vp_buf, n_props, tri_buf)
    }

    /// Extract mesh data as f64 with normals baked into vertex properties.
    ///
    /// The normals are stored starting at property index `normal_idx` (3
    /// consecutive f64 values per vertex: nx, ny, nz). If the mesh doesn't
    /// already have properties at that index, they are added.
    ///
    /// Returns `(vert_props, n_props, tri_indices)`.
    #[must_use]
    pub fn to_mesh_f64_with_normals(&self, normal_idx: i32) -> (Vec<f64>, usize, Vec<u64>) {
        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: self.ptr and meshgl are valid handles.
        unsafe { manifold_get_meshgl64_w_normals(meshgl, self.ptr, normal_idx) };

        // SAFETY: meshgl is valid. Read-only query.
        let n_props = unsafe { manifold_meshgl64_num_prop(meshgl) };
        // SAFETY: meshgl is valid. Returns total element count.
        let vp_len = unsafe { manifold_meshgl64_vert_properties_length(meshgl) };
        let mut vp_buf = vec![0.0f64; vp_len];
        // SAFETY: vp_buf has capacity vp_len.
        unsafe { manifold_meshgl64_vert_properties(vp_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let tri_len = unsafe { manifold_meshgl64_tri_length(meshgl) };
        let mut tri_buf = vec![0u64; tri_len];
        // SAFETY: tri_buf has capacity tri_len.
        unsafe { manifold_meshgl64_tri_verts(tri_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl64(meshgl) };

        (vp_buf, n_props, tri_buf)
    }

    /// Extract mesh data as f32 with normals baked into vertex properties.
    ///
    /// See [`to_mesh_f64_with_normals`](Self::to_mesh_f64_with_normals) for details.
    #[must_use]
    pub fn to_mesh_f32_with_normals(&self, normal_idx: i32) -> (Vec<f32>, usize, Vec<u32>) {
        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl() };
        // SAFETY: self.ptr and meshgl are valid handles.
        unsafe { manifold_get_meshgl_w_normals(meshgl, self.ptr, normal_idx) };

        // SAFETY: meshgl is valid. Read-only query.
        let n_props = unsafe { manifold_meshgl_num_prop(meshgl) };
        // SAFETY: meshgl is valid. Returns total element count.
        let vp_len = unsafe { manifold_meshgl_vert_properties_length(meshgl) };
        let mut vp_buf = vec![0.0f32; vp_len];
        // SAFETY: vp_buf has capacity vp_len.
        unsafe { manifold_meshgl_vert_properties(vp_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let tri_len = unsafe { manifold_meshgl_tri_length(meshgl) };
        let mut tri_buf = vec![0u32; tri_len];
        // SAFETY: tri_buf has capacity tri_len.
        unsafe { manifold_meshgl_tri_verts(tri_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl(meshgl) };

        (vp_buf, n_props, tri_buf)
    }

    /// Extract mesh data as f32 vertex properties and u32 triangle indices.
    ///
    /// Returns `(vert_props, n_props, tri_indices)`.
    #[must_use]
    pub fn to_mesh_f32(&self) -> (Vec<f32>, usize, Vec<u32>) {
        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let meshgl = unsafe { manifold_alloc_meshgl() };
        // SAFETY: self.ptr and meshgl are valid handles.
        unsafe { manifold_get_meshgl(meshgl, self.ptr) };

        // SAFETY: meshgl is valid. Read-only query.
        let n_verts = unsafe { manifold_meshgl_num_vert(meshgl) };
        // SAFETY: meshgl is valid. Read-only query.
        let n_tris = unsafe { manifold_meshgl_num_tri(meshgl) };
        // SAFETY: meshgl is valid. Read-only query.
        let n_props = unsafe { manifold_meshgl_num_prop(meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let vp_len = unsafe { manifold_meshgl_vert_properties_length(meshgl) };
        let mut vp_buf = vec![0.0f32; vp_len];
        // SAFETY: vp_buf has capacity vp_len.
        unsafe { manifold_meshgl_vert_properties(vp_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid. Returns total element count.
        let tri_len = unsafe { manifold_meshgl_tri_length(meshgl) };
        let mut tri_buf = vec![0u32; tri_len];
        // SAFETY: tri_buf has capacity tri_len.
        unsafe { manifold_meshgl_tri_verts(tri_buf.as_mut_ptr(), meshgl) };

        // SAFETY: meshgl is valid and no longer needed.
        unsafe { manifold_delete_meshgl(meshgl) };

        debug_assert_eq!(vp_buf.len(), n_verts * n_props);
        debug_assert_eq!(tri_buf.len(), n_tris * 3);

        (vp_buf, n_props, tri_buf)
    }

    // ── Primitive constructors ──────────────────────────────────────

    /// Axis-aligned box with dimensions `(x, y, z)`.
    ///
    /// If `center` is true, centered at the origin. Otherwise the corner
    /// is at the origin and the box spans `[0, x] × [0, y] × [0, z]`.
    #[must_use]
    pub fn cube(x: f64, y: f64, z: f64, center: bool) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_cube(ptr, x, y, z, i32::from(center)) };
        Self { ptr }
    }

    /// Cylinder along Z axis.
    ///
    /// If `center` is true, centered at the origin. Otherwise the base
    /// is at z=0 and spans `[0, height]`.
    #[must_use]
    pub fn cylinder(
        height: f64,
        radius_low: f64,
        radius_high: f64,
        segments: i32,
        center: bool,
    ) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc.
        unsafe {
            manifold_cylinder(
                ptr,
                height,
                radius_low,
                radius_high,
                segments,
                i32::from(center),
            )
        };
        Self { ptr }
    }

    /// Sphere centred at the origin.
    #[must_use]
    pub fn sphere(radius: f64, segments: i32) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_sphere(ptr, radius, segments) };
        Self { ptr }
    }

    /// Empty manifold (identity for union).
    #[must_use]
    pub fn empty() -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_empty(ptr) };
        Self { ptr }
    }

    // ── Transforms ──────────────────────────────────────────────────

    /// Translate by (x, y, z). Returns a new Manifold.
    #[must_use]
    pub fn translate(&self, x: f64, y: f64, z: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_translate(ptr, self.ptr, x, y, z) };
        Self { ptr }
    }

    /// Scale by (x, y, z). Returns a new Manifold.
    #[must_use]
    pub fn scale(&self, x: f64, y: f64, z: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_scale(ptr, self.ptr, x, y, z) };
        Self { ptr }
    }

    /// Rotate by Euler angles (degrees), applied in z-y'-x" order.
    /// Multiples of 90deg use exact arithmetic internally.
    #[must_use]
    pub fn rotate(&self, x_deg: f64, y_deg: f64, z_deg: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_rotate(ptr, self.ptr, x_deg, y_deg, z_deg) };
        Self { ptr }
    }

    /// Apply a 4x3 affine transformation (column-major).
    ///
    /// The 12 values represent:
    /// ```text
    /// | m[0] m[3] m[6] m[9]  |     col1 = X basis
    /// | m[1] m[4] m[7] m[10] |     col2 = Y basis
    /// | m[2] m[5] m[8] m[11] |     col3 = Z basis
    ///                               col4 = translation
    /// ```
    #[must_use]
    pub fn transform(&self, m: &[f64; 12]) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe {
            manifold_transform(
                ptr, self.ptr, m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10],
                m[11],
            );
        }
        Self { ptr }
    }

    // ── Plane operations ────────────────────────────────────────────

    /// Split into two halves along a plane.
    ///
    /// The plane is `normal · point = offset`.
    /// Returns `(positive_half, negative_half)`.
    #[must_use]
    pub fn split_by_plane(&self, normal: [f64; 3], offset: f64) -> (Self, Self) {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let mem_first = unsafe { manifold_alloc_manifold() };
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let mem_second = unsafe { manifold_alloc_manifold() };
        // SAFETY: self.ptr is valid (invariant), both output handles are fresh.
        let pair = unsafe {
            manifold_split_by_plane(
                mem_first, mem_second, self.ptr, normal[0], normal[1], normal[2], offset,
            )
        };
        (Self { ptr: pair.first }, Self { ptr: pair.second })
    }

    /// Trim to one side of a plane, keeping the positive half.
    ///
    /// **Note:** Upstream issue [#1516](https://github.com/elalish/manifold/issues/1516):
    /// trimmed halves may not reassemble via boolean union due to coincident faces.
    #[must_use]
    pub fn trim_by_plane(&self, normal: [f64; 3], offset: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe {
            manifold_trim_by_plane(ptr, self.ptr, normal[0], normal[1], normal[2], offset);
        }
        Self { ptr }
    }

    // ── Slicing (3D → 2D) ──────────────────────────────────────────

    /// Slice at a given Z height, returning 2D polygon contours.
    ///
    /// Returns a list of polygon rings (each ring is a list of `[x, y]` points).
    #[must_use]
    pub fn slice_at_z(&self, height: f64) -> Vec<Vec<[f64; 2]>> {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: self.ptr is valid (invariant), poly_ptr is valid from alloc.
        unsafe { manifold_slice(poly_ptr, self.ptr, height) };

        let result = read_polygons(poly_ptr);

        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        result
    }

    /// Slice at a given Z height, returning a [`CrossSection`] object.
    ///
    /// The cross-section can be offset, extruded, and boolean'd in 2D.
    #[must_use]
    pub fn slice_to_cross_section(&self, height: f64) -> CrossSection {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: self.ptr is valid (invariant), poly_ptr is valid from alloc.
        unsafe { manifold_slice(poly_ptr, self.ptr, height) };

        // SAFETY: manifold_alloc_cross_section returns a valid handle.
        let cs_ptr = unsafe { manifold_alloc_cross_section() };
        // SAFETY: cs_ptr and poly_ptr are valid.
        unsafe {
            manifold_cross_section_of_polygons(cs_ptr, poly_ptr, ManifoldFillRule::EvenOdd);
        }

        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        CrossSection { ptr: cs_ptr }
    }

    /// Extrude a [`CrossSection`] into a 3D manifold along the Z axis.
    ///
    /// The result spans `z ∈ [0, height]`.
    #[must_use]
    pub fn extrude(cross_section: &CrossSection, height: f64) -> Self {
        // First convert cross-section to polygons (manifold_extrude takes polygons).
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: poly_ptr is valid, cross_section.ptr is valid (invariant).
        unsafe { manifold_cross_section_to_polygons(poly_ptr, cross_section.ptr) };

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, poly_ptr is valid.
        unsafe { manifold_extrude(ptr, poly_ptr, height, 0, 0.0, 1.0, 1.0) };

        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };

        Self { ptr }
    }

    // ── Batch operations ────────────────────────────────────────────

    /// Batch union: combine multiple manifolds in a single operation.
    #[must_use]
    pub fn batch_union(manifolds: &[Self]) -> Self {
        Self::batch_boolean(manifolds, ManifoldOpType::Add)
    }

    /// Batch difference: subtract all subsequent manifolds from the first.
    #[must_use]
    pub fn batch_difference(manifolds: &[Self]) -> Self {
        Self::batch_boolean(manifolds, ManifoldOpType::Subtract)
    }

    fn batch_boolean(manifolds: &[Self], op: ManifoldOpType) -> Self {
        if manifolds.is_empty() {
            return Self::empty();
        }

        // SAFETY: manifold_alloc_manifold_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_manifold_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_manifold_empty_vec(vec_ptr) };

        for m in manifolds {
            // batch_boolean consumes the vec entries, so we need copies.
            // SAFETY: manifold_alloc_manifold returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_manifold() };
            // SAFETY: copy_ptr is valid from alloc, m.ptr is valid (invariant).
            unsafe { manifold_copy(copy_ptr, m.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid manifold.
            unsafe { manifold_manifold_vec_push_back(vec_ptr, copy_ptr) };
            // SAFETY: push_back copies the value; free the temporary allocation.
            unsafe { manifold_delete_manifold(copy_ptr) };
        }

        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let result_ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: result_ptr and vec_ptr are valid.
        unsafe { manifold_batch_boolean(result_ptr, vec_ptr, op) };

        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_manifold_vec(vec_ptr) };

        Self { ptr: result_ptr }
    }

    // ── Queries ─────────────────────────────────────────────────────

    /// Whether the manifold is empty (zero volume).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_is_empty(self.ptr) != 0 }
    }

    /// Enclosed volume.
    #[must_use]
    pub fn volume(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_volume(self.ptr) }
    }

    /// Total surface area.
    #[must_use]
    pub fn surface_area(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_surface_area(self.ptr) }
    }

    /// Number of vertices.
    #[must_use]
    pub fn num_vert(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_num_vert(self.ptr) }
    }

    /// Number of triangles.
    #[must_use]
    pub fn num_tri(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_num_tri(self.ptr) }
    }

    /// Axis-aligned bounding box.
    ///
    /// Returns a [`BoundingBox`] with spatial query methods (containment,
    /// overlap, union, transforms). Returns `None` if the manifold is empty.
    #[must_use]
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        if self.is_empty() {
            return None;
        }
        // SAFETY: manifold_alloc_box returns a valid handle.
        let ptr = unsafe { manifold_alloc_box() };
        // SAFETY: self.ptr is valid (invariant), ptr is a fresh allocation.
        unsafe { manifold_bounding_box(ptr, self.ptr) };
        Some(BoundingBox { ptr })
    }

    // ── Boolean operations ──────────────────────────────────────────

    /// Boolean difference: `self - other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_difference(ptr, self.ptr, other.ptr) };
        // SAFETY: ptr is valid.
        let status = unsafe { manifold_status(ptr) };
        if status != ManifoldError::NoError {
            log::warn!("CSG difference produced error status: {status:?}");
        }
        Self { ptr }
    }

    /// Boolean union: `self + other`.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_union(ptr, self.ptr, other.ptr) };
        // SAFETY: ptr is valid.
        let status = unsafe { manifold_status(ptr) };
        if status != ManifoldError::NoError {
            log::warn!("CSG union produced error status: {status:?}");
        }
        Self { ptr }
    }

    /// Boolean intersection: `self ∩ other`.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_intersection(ptr, self.ptr, other.ptr) };
        // SAFETY: ptr is valid.
        let status = unsafe { manifold_status(ptr) };
        if status != ManifoldError::NoError {
            log::warn!("CSG intersection produced error status: {status:?}");
        }
        Self { ptr }
    }

    /// Generic boolean operation with an explicit operation type.
    ///
    /// Prefer the specific methods ([`union`](Self::union),
    /// [`difference`](Self::difference), [`intersection`](Self::intersection))
    /// or operator overloads (`+`, `-`, `^`) for readability. This method
    /// is useful when the operation type is determined at runtime.
    #[must_use]
    pub fn boolean(&self, other: &Self, op: ManifoldOpType) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: all three pointers are valid.
        unsafe { manifold_boolean(ptr, self.ptr, other.ptr, op) };
        Self { ptr }
    }

    /// Decompose into connected components.
    #[must_use]
    pub fn decompose(&self) -> Vec<Self> {
        // SAFETY: alloc returns a valid vec handle.
        let vec_ptr = unsafe { manifold_alloc_manifold_vec() };
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_decompose(vec_ptr, self.ptr) };

        // SAFETY: vec_ptr is valid.
        let len = unsafe { manifold_manifold_vec_length(vec_ptr) };
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            // SAFETY: alloc returns a valid manifold handle.
            let ptr = unsafe { manifold_alloc_manifold() };
            // SAFETY: ptr and vec_ptr are valid; i is in bounds.
            unsafe { manifold_manifold_vec_get(ptr, vec_ptr, i) };
            result.push(Self { ptr });
        }

        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_manifold_vec(vec_ptr) };
        result
    }

    // ── Convex hull ─────────────────────────────────────────────────

    /// Convex hull of this manifold.
    #[must_use]
    pub fn hull(&self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_hull(ptr, self.ptr) };
        Self { ptr }
    }

    /// Convex hull of multiple manifolds combined.
    #[must_use]
    pub fn batch_hull(manifolds: &[Self]) -> Self {
        if manifolds.is_empty() {
            return Self::empty();
        }
        // SAFETY: manifold_alloc_manifold_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_manifold_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_manifold_empty_vec(vec_ptr) };
        for m in manifolds {
            // SAFETY: manifold_alloc_manifold returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_manifold() };
            // SAFETY: copy_ptr is valid from alloc, m.ptr is valid (invariant).
            unsafe { manifold_copy(copy_ptr, m.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid manifold.
            unsafe { manifold_manifold_vec_push_back(vec_ptr, copy_ptr) };
            // SAFETY: push_back copies the value; free the temporary allocation.
            unsafe { manifold_delete_manifold(copy_ptr) };
        }
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr and vec_ptr are valid.
        unsafe { manifold_batch_hull(ptr, vec_ptr) };
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_manifold_vec(vec_ptr) };
        Self { ptr }
    }

    /// Convex hull of a set of 3D points.
    #[must_use]
    pub fn hull_pts(points: &[[f64; 3]]) -> Self {
        let vec3s: Vec<ManifoldVec3> = points
            .iter()
            .map(|p| ManifoldVec3 {
                x: p[0],
                y: p[1],
                z: p[2],
            })
            .collect();
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, vec3s is a valid slice.
        unsafe { manifold_hull_pts(ptr, vec3s.as_ptr(), vec3s.len()) };
        Self { ptr }
    }

    // ── Mirror ──────────────────────────────────────────────────────

    /// Mirror across a plane through the origin with the given normal.
    #[must_use]
    pub fn mirror(&self, normal: [f64; 3]) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_mirror(ptr, self.ptr, normal[0], normal[1], normal[2]) };
        Self { ptr }
    }

    // ── Refinement ──────────────────────────────────────────────────

    /// Increase the density of the mesh by splitting each edge into `n` pieces.
    #[must_use]
    pub fn refine(&self, n: i32) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_refine(ptr, self.ptr, n) };
        Self { ptr }
    }

    /// Refine until no edge is longer than `length`.
    #[must_use]
    pub fn refine_to_length(&self, length: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_refine_to_length(ptr, self.ptr, length) };
        Self { ptr }
    }

    /// Refine until the deviation from the true surface is less than `tolerance`.
    #[must_use]
    pub fn refine_to_tolerance(&self, tolerance: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_refine_to_tolerance(ptr, self.ptr, tolerance) };
        Self { ptr }
    }

    /// Set the tolerance of the manifold, returning a new manifold.
    #[must_use]
    pub fn set_tolerance(&self, tolerance: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_set_tolerance(ptr, self.ptr, tolerance) };
        Self { ptr }
    }

    /// Simplify the mesh, removing vertices until the error exceeds `tolerance`.
    #[must_use]
    pub fn simplify(&self, tolerance: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_simplify(ptr, self.ptr, tolerance) };
        Self { ptr }
    }

    // ── Smoothing ───────────────────────────────────────────────────

    /// Smooth the manifold by converting sharp edges to smooth curves,
    /// using vertex normals at the given property index.
    #[must_use]
    pub fn smooth_by_normals(&self, normal_idx: i32) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_smooth_by_normals(ptr, self.ptr, normal_idx) };
        Self { ptr }
    }

    /// Smooth out the manifold, making sharp edges smoother.
    ///
    /// `min_sharp_angle` (degrees): edges sharper than this are candidates.
    /// `min_smoothness`: minimum smoothness applied (0-1).
    #[must_use]
    pub fn smooth_out(&self, min_sharp_angle: f64, min_smoothness: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_smooth_out(ptr, self.ptr, min_sharp_angle, min_smoothness) };
        Self { ptr }
    }

    // ── Additional constructors ─────────────────────────────────────

    /// Regular tetrahedron centered at the origin with unit edge length.
    #[must_use]
    pub fn tetrahedron() -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc.
        unsafe { manifold_tetrahedron(ptr) };
        Self { ptr }
    }

    /// Revolve a 2D cross-section around the Y axis to create a solid of revolution.
    #[must_use]
    pub fn revolve(
        cross_section: &CrossSection,
        circular_segments: i32,
        revolve_degrees: f64,
    ) -> Self {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: poly_ptr is valid, cross_section.ptr is valid (invariant).
        unsafe { manifold_cross_section_to_polygons(poly_ptr, cross_section.ptr) };
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, poly_ptr is valid.
        unsafe { manifold_revolve(ptr, poly_ptr, circular_segments, revolve_degrees) };
        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        Self { ptr }
    }

    /// Extrude with full control over slicing, twist, and scale.
    #[must_use]
    pub fn extrude_with_options(
        cross_section: &CrossSection,
        height: f64,
        slices: i32,
        twist_degrees: f64,
        scale_x: f64,
        scale_y: f64,
    ) -> Self {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: poly_ptr is valid, cross_section.ptr is valid (invariant).
        unsafe { manifold_cross_section_to_polygons(poly_ptr, cross_section.ptr) };
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, poly_ptr is valid.
        unsafe {
            manifold_extrude(
                ptr,
                poly_ptr,
                height,
                slices,
                twist_degrees,
                scale_x,
                scale_y,
            )
        };
        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        Self { ptr }
    }

    /// Compose multiple manifolds into a single manifold (without boolean).
    #[must_use]
    pub fn compose(manifolds: &[Self]) -> Self {
        // SAFETY: manifold_alloc_manifold_vec returns a valid handle.
        let vec_ptr = unsafe { manifold_alloc_manifold_vec() };
        // SAFETY: vec_ptr is valid from alloc.
        unsafe { manifold_manifold_empty_vec(vec_ptr) };
        for m in manifolds {
            // SAFETY: manifold_alloc_manifold returns a valid handle.
            let copy_ptr = unsafe { manifold_alloc_manifold() };
            // SAFETY: copy_ptr is valid from alloc, m.ptr is valid (invariant).
            unsafe { manifold_copy(copy_ptr, m.ptr) };
            // SAFETY: vec_ptr is valid, copy_ptr is a valid manifold.
            unsafe { manifold_manifold_vec_push_back(vec_ptr, copy_ptr) };
            // SAFETY: push_back copies the value; free the temporary allocation.
            unsafe { manifold_delete_manifold(copy_ptr) };
        }
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr and vec_ptr are valid.
        unsafe { manifold_compose(ptr, vec_ptr) };
        // SAFETY: vec_ptr is valid and no longer needed.
        unsafe { manifold_delete_manifold_vec(vec_ptr) };
        Self { ptr }
    }

    /// Split by another manifold (instead of a plane).
    #[must_use]
    pub fn split(&self, cutter: &Self) -> (Self, Self) {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let mem_first = unsafe { manifold_alloc_manifold() };
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let mem_second = unsafe { manifold_alloc_manifold() };
        // SAFETY: self.ptr and cutter.ptr are valid (invariant), both output handles are fresh.
        let pair = unsafe { manifold_split(mem_first, mem_second, self.ptr, cutter.ptr) };
        (Self { ptr: pair.first }, Self { ptr: pair.second })
    }

    /// Minkowski sum of two manifolds.
    #[must_use]
    pub fn minkowski_sum(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr and other.ptr are valid (invariant).
        unsafe { manifold_minkowski_sum(ptr, self.ptr, other.ptr) };
        Self { ptr }
    }

    /// Minkowski difference of two manifolds.
    #[must_use]
    pub fn minkowski_difference(&self, other: &Self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr and other.ptr are valid (invariant).
        unsafe { manifold_minkowski_difference(ptr, self.ptr, other.ptr) };
        Self { ptr }
    }

    /// Project the manifold onto the XY plane, returning 2D polygons.
    #[must_use]
    pub fn project(&self) -> Vec<Vec<[f64; 2]>> {
        // SAFETY: manifold_alloc_polygons returns a valid handle.
        let poly_ptr = unsafe { manifold_alloc_polygons() };
        // SAFETY: self.ptr is valid (invariant), poly_ptr is valid from alloc.
        unsafe { manifold_project(poly_ptr, self.ptr) };
        let result = read_polygons(poly_ptr);
        // SAFETY: poly_ptr is valid and no longer needed.
        unsafe { manifold_delete_polygons(poly_ptr) };
        result
    }

    // ── Additional queries ──────────────────────────────────────────

    /// Number of edges.
    #[must_use]
    pub fn num_edge(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_num_edge(self.ptr) }
    }

    /// Number of properties per vertex.
    #[must_use]
    pub fn num_prop(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_num_prop(self.ptr) }
    }

    /// Precision (epsilon) of the manifold.
    #[must_use]
    pub fn epsilon(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_epsilon(self.ptr) }
    }

    /// Tolerance of the manifold (public API, distinct from the `epsilon` testing hook).
    #[must_use]
    pub fn get_tolerance(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_get_tolerance(self.ptr) }
    }

    /// Number of property vertices (may differ from `num_vert` when vertices
    /// are split to accommodate different property values).
    #[must_use]
    pub fn num_prop_vert(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_num_prop_vert(self.ptr) }
    }

    /// Genus of the manifold (topological measure: 0 for sphere, 1 for torus, etc).
    #[must_use]
    pub fn genus(&self) -> i32 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_genus(self.ptr) }
    }

    /// Original ID of this manifold (for tracking through operations).
    #[must_use]
    pub fn original_id(&self) -> i32 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_original_id(self.ptr) }
    }

    /// Mark this manifold as an original, assigning it a unique ID for
    /// tracking through boolean operations.
    ///
    /// Use [`original_id`](Self::original_id) to retrieve the assigned ID,
    /// and [`reserve_ids`] to pre-allocate ID ranges.
    #[must_use]
    pub fn as_original(&self) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_as_original(ptr, self.ptr) };
        Self { ptr }
    }

    /// Minimum gap between this manifold and another, searching up to `search_length`.
    #[must_use]
    pub fn min_gap(&self, other: &Self, search_length: f64) -> f64 {
        // SAFETY: self.ptr and other.ptr are valid (invariant).
        unsafe { manifold_min_gap(self.ptr, other.ptr, search_length) }
    }

    /// Calculate normals and store them as vertex properties at `normal_idx`.
    ///
    /// Edges sharper than `min_sharp_angle` (degrees) get sharp normals.
    #[must_use]
    pub fn calculate_normals(&self, normal_idx: i32, min_sharp_angle: f64) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_calculate_normals(ptr, self.ptr, normal_idx, min_sharp_angle) };
        Self { ptr }
    }

    /// Calculate Gaussian and mean curvature and store as vertex properties.
    #[must_use]
    pub fn calculate_curvature(&self, gaussian_idx: i32, mean_idx: i32) -> Self {
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        unsafe { manifold_calculate_curvature(ptr, self.ptr, gaussian_idx, mean_idx) };
        Self { ptr }
    }

    // ── Warp ────────────────────────────────────────────────────────

    /// Apply a warp function to deform each vertex.
    ///
    /// The closure receives `(x, y, z)` and returns `[x', y', z']`.
    #[must_use]
    pub fn warp<F>(&self, f: F) -> Self
    where
        F: FnMut(f64, f64, f64) -> [f64; 3],
    {
        unsafe extern "C" fn trampoline<F>(
            x: f64,
            y: f64,
            z: f64,
            ctx: *mut std::ffi::c_void,
        ) -> ManifoldVec3
        where
            F: FnMut(f64, f64, f64) -> [f64; 3],
        {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut F below and is valid for
                // the duration of the manifold_warp call.
                let f = unsafe { &mut *(ctx as *mut F) };
                f(x, y, z)
            }));
            match result {
                Ok([rx, ry, rz]) => ManifoldVec3 {
                    x: rx,
                    y: ry,
                    z: rz,
                },
                // Return the original point on panic to avoid UB from unwinding through C.
                Err(_) => ManifoldVec3 { x, y, z },
            }
        }

        let mut closure = f;
        let ctx = &mut closure as *mut F as *mut std::ffi::c_void;
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr is valid from alloc, self.ptr is valid (invariant).
        // The trampoline and ctx are valid for the duration of this call.
        unsafe { manifold_warp(ptr, self.ptr, Some(trampoline::<F>), ctx) };
        Self { ptr }
    }

    // ── Set properties ──────────────────────────────────────────────

    /// Set custom vertex properties using a callback.
    ///
    /// The closure receives `(new_props, position, old_props)` where:
    /// - `new_props`: mutable slice of `num_prop` f64s to write
    /// - `position`: the vertex position `[x, y, z]`
    /// - `old_props`: the existing properties (length = current `self.num_prop()`)
    #[must_use]
    pub fn set_properties<F>(&self, num_prop: usize, mut f: F) -> Self
    where
        F: FnMut(&mut [f64], [f64; 3], &[f64]),
    {
        assert!(
            num_prop <= i32::MAX as usize,
            "num_prop must fit in i32 for the C API"
        );
        let old_num_prop = self.num_prop();
        let new_num_prop = num_prop;

        struct Context<'a, F> {
            f: &'a mut F,
            old_num_prop: usize,
            new_num_prop: usize,
        }

        unsafe extern "C" fn trampoline<F>(
            new_prop: *mut f64,
            position: ManifoldVec3,
            old_prop: *const f64,
            ctx: *mut std::ffi::c_void,
        ) where
            F: FnMut(&mut [f64], [f64; 3], &[f64]),
        {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut Context below and is valid
                // for the duration of the manifold_set_properties call.
                let ctx = unsafe { &mut *(ctx as *mut Context<'_, F>) };
                let pos = [position.x, position.y, position.z];
                // SAFETY: new_prop has ctx.new_num_prop elements (guaranteed by C API).
                let new_slice =
                    unsafe { std::slice::from_raw_parts_mut(new_prop, ctx.new_num_prop) };
                // Zero the output buffer so panics in the closure don't leave
                // uninitialized memory in the manifold's property data.
                new_slice.fill(0.0);
                // SAFETY: old_prop may be null if the manifold has no custom properties.
                // When non-null, it has ctx.old_num_prop elements (guaranteed by C API).
                let old_slice = if old_prop.is_null() {
                    &[]
                } else {
                    // SAFETY: old_prop is non-null and has ctx.old_num_prop elements.
                    unsafe { std::slice::from_raw_parts(old_prop, ctx.old_num_prop) }
                };
                (ctx.f)(new_slice, pos, old_slice);
            }));
        }

        let mut ctx = Context {
            f: &mut f,
            old_num_prop,
            new_num_prop,
        };
        let ctx_ptr = &mut ctx as *mut Context<'_, F> as *mut std::ffi::c_void;
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr valid, self.ptr valid (invariant), trampoline+ctx valid for call duration.
        // SAFETY: num_prop fits in c_int (checked by assert above).
        unsafe {
            manifold_set_properties(
                ptr,
                self.ptr,
                num_prop as std::ffi::c_int,
                Some(trampoline::<F>),
                ctx_ptr,
            )
        };
        Self { ptr }
    }

    // ── OBJ I/O ─────────────────────────────────────────────────────

    /// Parse a Manifold from Wavefront OBJ string content.
    ///
    /// The input should be the content of an .obj file (not a filename).
    pub fn from_obj(obj_content: &str) -> Result<Self, CsgError> {
        let c_str = std::ffi::CString::new(obj_content)
            .map_err(|_| CsgError::InvalidInput("OBJ content contains null byte".into()))?;
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr valid from alloc, c_str.as_ptr() is a valid null-terminated string.
        unsafe { manifold_read_obj(ptr, c_str.as_ptr()) };
        // SAFETY: ptr is valid. Read-only status query.
        let status = unsafe { manifold_status(ptr) };
        if status != ManifoldError::NoError {
            // SAFETY: ptr is valid. Frees on error path.
            unsafe { manifold_delete_manifold(ptr) };
            return Err(CsgError::ManifoldStatus(status));
        }
        Ok(Self { ptr })
    }

    /// Export this Manifold as a Wavefront OBJ string.
    #[must_use]
    pub fn to_obj(&self) -> String {
        let mut result = String::new();

        unsafe extern "C" fn callback(data: *mut std::ffi::c_char, ctx: *mut std::ffi::c_void) {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut String and is valid for the call.
                let result = unsafe { &mut *(ctx as *mut String) };
                // SAFETY: data is a null-terminated C string provided by manifold3d.
                let c_str = unsafe { std::ffi::CStr::from_ptr(data) };
                *result = c_str.to_string_lossy().into_owned();
            }));
        }

        let ctx = &mut result as *mut String as *mut std::ffi::c_void;
        // SAFETY: self.ptr is valid (invariant), callback and ctx are valid for the call.
        unsafe { manifold_write_obj(self.ptr, Some(callback), ctx) };
        result
    }

    // ── Level set (SDF) ─────────────────────────────────────────────

    /// Construct a manifold from a signed distance function (SDF).
    ///
    /// The SDF closure receives `(x, y, z)` and returns the signed distance.
    /// The manifold surface is at `level` (typically 0.0).
    ///
    /// * `bounds` - bounding box `([min_x, min_y, min_z], [max_x, max_y, max_z])`
    /// * `edge_length` - approximate edge length of the output mesh
    /// * `level` - the isosurface level (typically 0.0)
    /// * `tolerance` - tolerance for surface accuracy
    #[must_use]
    pub fn from_sdf<F>(
        f: F,
        bounds: ([f64; 3], [f64; 3]),
        edge_length: f64,
        level: f64,
        tolerance: f64,
    ) -> Self
    where
        F: Fn(f64, f64, f64) -> f64 + Sync,
    {
        unsafe extern "C" fn trampoline<F>(
            x: f64,
            y: f64,
            z: f64,
            ctx: *mut std::ffi::c_void,
        ) -> f64
        where
            F: Fn(f64, f64, f64) -> f64 + Sync,
        {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx points to an F that is Sync, so shared access
                // from multiple C threads is sound.
                let f = unsafe { &*(ctx as *const F) };
                f(x, y, z)
            }));
            // Return large positive distance on panic (outside surface).
            result.unwrap_or(f64::MAX)
        }

        let ctx = &f as *const F as *mut std::ffi::c_void;
        // SAFETY: manifold_alloc_box returns a valid handle.
        let box_ptr = unsafe { manifold_alloc_box() };
        // SAFETY: box_ptr is valid from alloc.
        unsafe {
            manifold_box(
                box_ptr,
                bounds.0[0],
                bounds.0[1],
                bounds.0[2],
                bounds.1[0],
                bounds.1[1],
                bounds.1[2],
            );
        }
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr valid, box_ptr valid, trampoline+ctx valid for call duration.
        unsafe {
            manifold_level_set(
                ptr,
                Some(trampoline::<F>),
                box_ptr,
                edge_length,
                level,
                tolerance,
                ctx,
            )
        };
        // SAFETY: box_ptr is valid and no longer needed.
        unsafe { manifold_delete_box(box_ptr) };
        Self { ptr }
    }

    /// Construct a manifold from a signed distance function (SDF), using
    /// sequential (single-threaded) execution.
    ///
    /// Same as [`from_sdf`](Self::from_sdf) but forces sequential evaluation.
    /// Use this when calling from a runtime that prevents parallel execution of
    /// closures (e.g., Python or Ruby runtimes with a GIL).
    #[must_use]
    pub fn from_sdf_seq<F>(
        mut f: F,
        bounds: ([f64; 3], [f64; 3]),
        edge_length: f64,
        level: f64,
        tolerance: f64,
    ) -> Self
    where
        F: FnMut(f64, f64, f64) -> f64,
    {
        unsafe extern "C" fn trampoline<F>(
            x: f64,
            y: f64,
            z: f64,
            ctx: *mut std::ffi::c_void,
        ) -> f64
        where
            F: FnMut(f64, f64, f64) -> f64,
        {
            // Catch panics to prevent UB from unwinding through C stack frames.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: ctx was created from a &mut F and is valid for the call duration.
                let f = unsafe { &mut *(ctx as *mut F) };
                f(x, y, z)
            }));
            result.unwrap_or(f64::MAX)
        }

        let ctx = &mut f as *mut F as *mut std::ffi::c_void;
        // SAFETY: manifold_alloc_box returns a valid handle.
        let box_ptr = unsafe { manifold_alloc_box() };
        // SAFETY: box_ptr is valid from alloc.
        unsafe {
            manifold_box(
                box_ptr,
                bounds.0[0],
                bounds.0[1],
                bounds.0[2],
                bounds.1[0],
                bounds.1[1],
                bounds.1[2],
            );
        }
        // SAFETY: manifold_alloc_manifold returns a valid handle.
        let ptr = unsafe { manifold_alloc_manifold() };
        // SAFETY: ptr valid, box_ptr valid, trampoline+ctx valid for call duration.
        unsafe {
            manifold_level_set_seq(
                ptr,
                Some(trampoline::<F>),
                box_ptr,
                edge_length,
                level,
                tolerance,
                ctx,
            )
        };
        // SAFETY: box_ptr is valid and no longer needed.
        unsafe { manifold_delete_box(box_ptr) };
        Self { ptr }
    }
}

// ── Operator overloads ──────────────────────────────────────────────────

/// `a - b` → Boolean difference.
impl ops::Sub for &Manifold {
    type Output = Manifold;
    fn sub(self, rhs: &Manifold) -> Manifold {
        self.difference(rhs)
    }
}

/// `a + b` → Boolean union.
impl ops::Add for &Manifold {
    type Output = Manifold;
    fn add(self, rhs: &Manifold) -> Manifold {
        self.union(rhs)
    }
}

/// `a ^ b` → Boolean intersection.
impl ops::BitXor for &Manifold {
    type Output = Manifold;
    fn bitxor(self, rhs: &Manifold) -> Manifold {
        self.intersection(rhs)
    }
}

// ── nalgebra convenience methods ─────────────────────────────────────────

#[cfg(feature = "nalgebra")]
impl Manifold {
    /// Apply a 3x3 rotation/scale matrix plus translation.
    ///
    /// Convenience method for users of the `nalgebra` crate. Equivalent to
    /// [`transform`](Self::transform) but accepts nalgebra types directly.
    #[must_use]
    pub fn transform_nalgebra(
        &self,
        matrix: &nalgebra::Matrix3<f64>,
        translation: &nalgebra::Vector3<f64>,
    ) -> Self {
        self.transform(&[
            matrix[(0, 0)],
            matrix[(1, 0)],
            matrix[(2, 0)],
            matrix[(0, 1)],
            matrix[(1, 1)],
            matrix[(2, 1)],
            matrix[(0, 2)],
            matrix[(1, 2)],
            matrix[(2, 2)],
            translation.x,
            translation.y,
            translation.z,
        ])
    }

    /// Split into two halves along a plane (nalgebra types).
    #[must_use]
    pub fn split_by_plane_nalgebra(
        &self,
        normal: &nalgebra::Vector3<f64>,
        offset: f64,
    ) -> (Self, Self) {
        self.split_by_plane([normal.x, normal.y, normal.z], offset)
    }

    /// Trim to the positive side of a plane (nalgebra types).
    #[must_use]
    pub fn trim_by_plane_nalgebra(&self, normal: &nalgebra::Vector3<f64>, offset: f64) -> Self {
        self.trim_by_plane([normal.x, normal.y, normal.z], offset)
    }

    /// Mirror across a plane with the given normal (nalgebra types).
    #[must_use]
    pub fn mirror_nalgebra(&self, normal: &nalgebra::Vector3<f64>) -> Self {
        self.mirror([normal.x, normal.y, normal.z])
    }

    /// Bounding box as nalgebra points.
    #[must_use]
    pub fn bounding_box_nalgebra(&self) -> Option<(nalgebra::Point3<f64>, nalgebra::Point3<f64>)> {
        self.bounding_box().map(|bb| {
            let min = bb.min();
            let max = bb.max();
            (
                nalgebra::Point3::new(min[0], min[1], min[2]),
                nalgebra::Point3::new(max[0], max[1], max[2]),
            )
        })
    }

    /// Create a Manifold from nalgebra Point3 vertices and face indices.
    pub fn from_vertices_and_faces(
        vertices: &[nalgebra::Point3<f64>],
        faces: &[[u32; 3]],
    ) -> Result<Self, CsgError> {
        let vert_props: Vec<f64> = vertices.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
        let tri_indices: Vec<u64> = faces
            .iter()
            .flat_map(|&[i0, i1, i2]| [u64::from(i0), u64::from(i1), u64::from(i2)])
            .collect();
        Self::from_mesh_f64(&vert_props, 3, &tri_indices)
    }

    /// Extract vertices as nalgebra Point3 and faces as `[u32; 3]`.
    ///
    /// # Panics
    ///
    /// Panics if any vertex index exceeds `u32::MAX` (meshes with over 4
    /// billion vertices). Use [`to_mesh_f64`](Self::to_mesh_f64) for u64 indices.
    #[must_use]
    pub fn to_vertices_and_faces(&self) -> (Vec<nalgebra::Point3<f64>>, Vec<[u32; 3]>) {
        let (verts, n_props, indices) = self.to_mesh_f64();
        let vertices: Vec<nalgebra::Point3<f64>> = verts
            .chunks(n_props)
            .map(|c| nalgebra::Point3::new(c[0], c[1], c[2]))
            .collect();
        let faces: Vec<[u32; 3]> = indices
            .chunks(3)
            .map(|c| {
                [
                    u32::try_from(c[0]).expect("vertex index exceeds u32"),
                    u32::try_from(c[1]).expect("vertex index exceeds u32"),
                    u32::try_from(c[2]).expect("vertex index exceeds u32"),
                ]
            })
            .collect();
        (vertices, faces)
    }
}

// ── Quality globals ─────────────────────────────────────────────────────

/// Guard for global quality setters. The C API's global state is not
/// thread-safe, so we serialize access from the Rust side.
static QUALITY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Set the minimum circular angle (degrees) for tessellation.
///
/// This modifies global state shared by all manifold operations in the
/// current process.
pub fn set_min_circular_angle(degrees: f64) {
    let _guard = QUALITY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by QUALITY_LOCK. No pointer invariants.
    unsafe { manifold_set_min_circular_angle(degrees) };
}

/// Set the minimum circular edge length for tessellation.
///
/// This modifies global state shared by all manifold operations in the
/// current process.
pub fn set_min_circular_edge_length(length: f64) {
    let _guard = QUALITY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by QUALITY_LOCK. No pointer invariants.
    unsafe { manifold_set_min_circular_edge_length(length) };
}

/// Set the number of circular segments for tessellation.
///
/// This modifies global state shared by all manifold operations in the
/// current process.
pub fn set_circular_segments(number: i32) {
    let _guard = QUALITY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by QUALITY_LOCK. No pointer invariants.
    unsafe { manifold_set_circular_segments(number) };
}

/// Reset circular tessellation parameters to defaults.
///
/// This modifies global state shared by all manifold operations in the
/// current process.
pub fn reset_to_circular_defaults() {
    let _guard = QUALITY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by QUALITY_LOCK. No pointer invariants.
    unsafe { manifold_reset_to_circular_defaults() };
}

/// Get the number of circular segments for a given radius.
#[must_use]
pub fn get_circular_segments(radius: f64) -> i32 {
    // SAFETY: Pure query with no pointer invariants.
    unsafe { manifold_get_circular_segments(radius) }
}

/// Reserve a block of `n` original IDs for use with manifold tracking.
#[must_use]
pub fn reserve_ids(n: u32) -> u32 {
    // SAFETY: Pure global state operation.
    unsafe { manifold_reserve_ids(n) }
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Read polygon data from a ManifoldPolygons handle into Vec<Vec<[f64; 2]>>.
pub(crate) fn read_polygons(poly_ptr: *const ManifoldPolygons) -> Vec<Vec<[f64; 2]>> {
    // SAFETY: poly_ptr is valid. Read-only query.
    let n_polys = unsafe { manifold_polygons_length(poly_ptr) };
    let mut result = Vec::with_capacity(n_polys);

    for i in 0..n_polys {
        // SAFETY: poly_ptr is valid, i is in bounds.
        let n_pts = unsafe { manifold_polygons_simple_length(poly_ptr, i) };
        let mut ring = Vec::with_capacity(n_pts);
        for j in 0..n_pts {
            // SAFETY: poly_ptr is valid, i and j are in bounds.
            let pt = unsafe { manifold_polygons_get_point(poly_ptr, i, j) };
            ring.push([pt.x, pt.y]);
        }
        result.push(ring);
    }

    result
}
