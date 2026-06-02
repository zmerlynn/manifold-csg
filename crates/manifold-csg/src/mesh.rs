//! Safe wrappers for manifold3d mesh data types.
//!
//! [`MeshGL`] wraps f32 mesh data, [`MeshGL64`] wraps f64 mesh data.
//! These are primarily used for constructing [`Manifold`](crate::Manifold)
//! objects and extracting mesh data from them.

use manifold_csg_sys::*;

use crate::types::CsgError;

/// Optional metadata for constructing a [`MeshGL`].
///
/// Use this when importing mesh data that already has manifold topology
/// metadata. The current C shim supports run indices/original IDs, merge
/// vectors, and halfedge tangents. Other upstream `MeshGL` fields can be added
/// as builder methods later without changing this type's construction pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshGLOptions<'a> {
    runs: Option<(&'a [u32], &'a [u32])>,
    merge_vertices: Option<(&'a [u32], &'a [u32])>,
    halfedge_tangents: Option<&'a [f32]>,
}

impl<'a> MeshGLOptions<'a> {
    /// Create empty mesh-construction options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            runs: None,
            merge_vertices: None,
            halfedge_tangents: None,
        }
    }

    /// Attach triangle run indices and original IDs.
    ///
    /// `run_indices` indexes into the flat `tri_verts` array and should be the
    /// same length as `run_original_ids`, or one longer with a final sentinel.
    #[must_use]
    pub const fn runs(mut self, run_indices: &'a [u32], run_original_ids: &'a [u32]) -> Self {
        self.runs = Some((run_indices, run_original_ids));
        self
    }

    /// Attach explicit vertex-merge pairs.
    #[must_use]
    pub const fn merge_vertices(mut self, merge_from: &'a [u32], merge_to: &'a [u32]) -> Self {
        self.merge_vertices = Some((merge_from, merge_to));
        self
    }

    /// Attach halfedge tangent data.
    ///
    /// The tangent slice must have `num_tri * 3 * 4` elements.
    #[must_use]
    pub const fn halfedge_tangents(mut self, halfedge_tangents: &'a [f32]) -> Self {
        self.halfedge_tangents = Some(halfedge_tangents);
        self
    }
}

/// Optional metadata for constructing a [`MeshGL64`].
///
/// This is the f64/u64 counterpart to [`MeshGLOptions`].
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshGL64Options<'a> {
    runs: Option<(&'a [u64], &'a [u32])>,
    merge_vertices: Option<(&'a [u64], &'a [u64])>,
    halfedge_tangents: Option<&'a [f64]>,
}

impl<'a> MeshGL64Options<'a> {
    /// Create empty mesh-construction options.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            runs: None,
            merge_vertices: None,
            halfedge_tangents: None,
        }
    }

    /// Attach triangle run indices and original IDs.
    ///
    /// `run_indices` indexes into the flat `tri_verts` array and should be the
    /// same length as `run_original_ids`, or one longer with a final sentinel.
    #[must_use]
    pub const fn runs(mut self, run_indices: &'a [u64], run_original_ids: &'a [u32]) -> Self {
        self.runs = Some((run_indices, run_original_ids));
        self
    }

    /// Attach explicit vertex-merge pairs.
    #[must_use]
    pub const fn merge_vertices(mut self, merge_from: &'a [u64], merge_to: &'a [u64]) -> Self {
        self.merge_vertices = Some((merge_from, merge_to));
        self
    }

    /// Attach halfedge tangent data.
    ///
    /// The tangent slice must have `num_tri * 3 * 4` elements.
    #[must_use]
    pub const fn halfedge_tangents(mut self, halfedge_tangents: &'a [f64]) -> Self {
        self.halfedge_tangents = Some(halfedge_tangents);
        self
    }
}

/// Safe wrapper around a manifold3d MeshGL object (f32 vertices, u32 indices).
///
/// See the [upstream `MeshGL` docs](https://elalish.github.io/manifold/docs/html/structmanifold_1_1_mesh_g_l_p.html)
/// for field semantics (run indices, merge vectors, tangents, etc.).
pub struct MeshGL {
    pub(crate) ptr: *mut ManifoldMeshGL,
}

// SAFETY: MeshGL owns its heap allocation with no thread-local state.
unsafe impl Send for MeshGL {}

// SAFETY: MeshGL is a pure data container (vertex arrays, index arrays) with no
// lazy evaluation or mutable internal state. Concurrent read access is safe.
unsafe impl Sync for MeshGL {}

impl Drop for MeshGL {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_meshgl.
            unsafe { manifold_delete_meshgl(self.ptr) };
        }
    }
}

impl MeshGL {
    /// Create a MeshGL from f32 vertex properties and u32 triangle indices.
    ///
    /// `vert_props` is a flat array with `n_props` values per vertex
    /// (minimum 3 for x, y, z). `tri_indices` has 3 values per triangle.
    ///
    /// Empty vertex and triangle buffers are valid.
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if `n_props < 3`, `vert_props` is
    /// not divisible by `n_props`, or `tri_indices` is not divisible by 3.
    pub fn new(vert_props: &[f32], n_props: usize, tri_indices: &[u32]) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl() };
        // SAFETY: ptr is valid, slices are valid with correct lengths.
        unsafe {
            manifold_meshgl(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }
        Ok(Self { ptr })
    }

    /// Create a MeshGL with halfedge tangent data.
    ///
    /// `halfedge_tangent` must have `num_tri * 3 * 4` elements (4 floats per
    /// halfedge, 3 halfedges per triangle).
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if the mesh buffers are malformed or
    /// the tangent buffer length is not `num_tri * 3 * 4`.
    pub fn new_with_tangents(
        vert_props: &[f32],
        n_props: usize,
        tri_indices: &[u32],
        halfedge_tangent: &[f32],
    ) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;
        validate_tangent_len(halfedge_tangent.len(), n_tris)?;

        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl() };
        // SAFETY: ptr valid, all slices valid with correct lengths.
        unsafe {
            manifold_meshgl_w_tangents(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
                halfedge_tangent.as_ptr(),
            );
        }
        Ok(Self { ptr })
    }

    /// Create a MeshGL with optional run, merge, and tangent metadata.
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if the mesh buffers or metadata
    /// buffers are malformed.
    pub fn new_with_options(
        vert_props: &[f32],
        n_props: usize,
        tri_indices: &[u32],
        options: MeshGLOptions<'_>,
    ) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;
        validate_meshgl_options(n_tris, tri_indices.len(), options)?;

        let (run_indices, run_original_ids) = options.runs.unwrap_or((&[], &[]));
        let (merge_from, merge_to) = options.merge_vertices.unwrap_or((&[], &[]));
        let halfedge_tangents = options.halfedge_tangents.unwrap_or(&[]);
        let sys_options = ManifoldMeshGLOptions {
            run_indices: slice_ptr(run_indices),
            run_indices_length: run_indices.len(),
            run_original_ids: slice_ptr(run_original_ids),
            run_original_ids_length: run_original_ids.len(),
            merge_from_vert: slice_ptr(merge_from),
            merge_to_vert: slice_ptr(merge_to),
            merge_verts_length: merge_from.len(),
            halfedge_tangents: slice_ptr(halfedge_tangents),
        };

        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl() };
        // SAFETY: ptr is valid. All slices outlive the call and were validated.
        unsafe {
            manifold_meshgl_w_options(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
                &sys_options,
            );
        }
        Ok(Self { ptr })
    }

    /// Number of vertices.
    #[must_use]
    pub fn num_vert(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl_num_vert(self.ptr) }
    }

    /// Number of triangles.
    #[must_use]
    pub fn num_tri(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl_num_tri(self.ptr) }
    }

    /// Number of properties per vertex.
    #[must_use]
    pub fn num_prop(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl_num_prop(self.ptr) }
    }

    /// Copy vertex properties out as a flat f32 array.
    #[must_use]
    pub fn vert_properties(&self) -> Vec<f32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_vert_properties_length(self.ptr) };
        let mut buf = vec![0.0f32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_vert_properties(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy triangle indices out as a flat u32 array.
    #[must_use]
    pub fn tri_verts(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_tri_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_tri_verts(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Merge coincident vertices, returning a new mesh.
    ///
    /// Processes the mesh's merge vectors to weld vertices that share
    /// the same position. Returns a new mesh (the original is unchanged).
    #[must_use]
    pub fn merge(&self) -> Self {
        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_meshgl_merge(ptr, self.ptr) };
        Self { ptr }
    }

    /// Copy merge-from vertex indices out as a flat u32 array.
    #[must_use]
    pub fn merge_from_vert(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_merge_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_merge_from_vert(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy merge-to vertex indices out as a flat u32 array.
    #[must_use]
    pub fn merge_to_vert(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_merge_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_merge_to_vert(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run indices out as a flat u32 array.
    #[must_use]
    pub fn run_index(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_run_index_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_run_index(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run original IDs out as a flat u32 array.
    #[must_use]
    pub fn run_original_id(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_run_original_id_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_run_original_id(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run transforms out as a flat f32 array (4x3 matrices, 12 floats each).
    #[must_use]
    pub fn run_transform(&self) -> Vec<f32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_run_transform_length(self.ptr) };
        let mut buf = vec![0.0f32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_run_transform(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy face IDs out as a flat u32 array.
    #[must_use]
    pub fn face_id(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_face_id_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_face_id(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy halfedge tangents out as a flat f32 array (4 floats per halfedge).
    #[must_use]
    pub fn halfedge_tangent(&self) -> Vec<f32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_tangent_length(self.ptr) };
        let mut buf = vec![0.0f32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_halfedge_tangent(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Tolerance used for merging and vertex welding.
    #[must_use]
    pub fn tolerance(&self) -> f32 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl_tolerance(self.ptr) }
    }

    /// Number of triangle runs.
    #[must_use]
    pub fn num_run(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl_num_run(self.ptr) }
    }

    /// Copy run flags out as a u8 array (one per triangle run).
    #[must_use]
    pub fn run_flags(&self) -> Vec<u8> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl_run_flags_length(self.ptr) };
        let mut buf = vec![0u8; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl_run_flags(buf.as_mut_ptr(), self.ptr) };
        buf
    }
}

impl Clone for MeshGL {
    fn clone(&self) -> Self {
        // SAFETY: manifold_alloc_meshgl returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_meshgl_copy(ptr, self.ptr) };
        Self { ptr }
    }
}

/// Safe wrapper around a manifold3d MeshGL64 object (f64 vertices, u64 indices).
///
/// This is the high-precision variant — use this when sub-mm features matter
/// at large coordinates (e.g., 0.6mm indents at z=128mm).
///
/// See the [upstream `MeshGL` docs](https://elalish.github.io/manifold/docs/html/structmanifold_1_1_mesh_g_l_p.html)
/// for field semantics (run indices, merge vectors, tangents, etc.).
pub struct MeshGL64 {
    pub(crate) ptr: *mut ManifoldMeshGL64,
}

// SAFETY: MeshGL64 owns its heap allocation with no thread-local state.
unsafe impl Send for MeshGL64 {}

// SAFETY: MeshGL64 is a pure data container (vertex arrays, index arrays) with
// no lazy evaluation or mutable internal state. Concurrent read access is safe.
unsafe impl Sync for MeshGL64 {}

impl Drop for MeshGL64 {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr was allocated by manifold_alloc_meshgl64.
            unsafe { manifold_delete_meshgl64(self.ptr) };
        }
    }
}

impl MeshGL64 {
    /// Create a MeshGL64 from f64 vertex properties and u64 triangle indices.
    ///
    /// `vert_props` is a flat array with `n_props` values per vertex
    /// (minimum 3 for x, y, z). `tri_indices` has 3 values per triangle.
    /// Empty vertex and triangle buffers are valid.
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if `n_props < 3`, `vert_props` is
    /// not divisible by `n_props`, or `tri_indices` is not divisible by 3.
    pub fn new(vert_props: &[f64], n_props: usize, tri_indices: &[u64]) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr is valid, slices are valid with correct lengths.
        unsafe {
            manifold_meshgl64(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
            );
        }
        Ok(Self { ptr })
    }

    /// Create a MeshGL64 with halfedge tangent data.
    ///
    /// See [`MeshGL::new_with_tangents`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if the mesh buffers are malformed or
    /// the tangent buffer length is not `num_tri * 3 * 4`.
    pub fn new_with_tangents(
        vert_props: &[f64],
        n_props: usize,
        tri_indices: &[u64],
        halfedge_tangent: &[f64],
    ) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;
        validate_tangent_len(halfedge_tangent.len(), n_tris)?;

        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr valid, all slices valid with correct lengths.
        unsafe {
            manifold_meshgl64_w_tangents(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
                halfedge_tangent.as_ptr(),
            );
        }
        Ok(Self { ptr })
    }

    /// Create a MeshGL64 with optional run, merge, and tangent metadata.
    ///
    /// # Errors
    ///
    /// Returns [`CsgError::InvalidInput`] if the mesh buffers or metadata
    /// buffers are malformed.
    pub fn new_with_options(
        vert_props: &[f64],
        n_props: usize,
        tri_indices: &[u64],
        options: MeshGL64Options<'_>,
    ) -> Result<Self, CsgError> {
        validate_mesh_shape(vert_props.len(), n_props, tri_indices.len())?;
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;
        validate_meshgl64_options(n_tris, tri_indices.len(), options)?;

        let (run_indices, run_original_ids) = options.runs.unwrap_or((&[], &[]));
        let (merge_from, merge_to) = options.merge_vertices.unwrap_or((&[], &[]));
        let halfedge_tangents = options.halfedge_tangents.unwrap_or(&[]);
        let sys_options = ManifoldMeshGL64Options {
            run_indices: slice_ptr(run_indices),
            run_indices_length: run_indices.len(),
            run_original_ids: slice_ptr(run_original_ids),
            run_original_ids_length: run_original_ids.len(),
            merge_from_vert: slice_ptr(merge_from),
            merge_to_vert: slice_ptr(merge_to),
            merge_verts_length: merge_from.len(),
            halfedge_tangents: slice_ptr(halfedge_tangents),
        };

        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr is valid. All slices outlive the call and were validated.
        unsafe {
            manifold_meshgl64_w_options(
                ptr,
                vert_props.as_ptr(),
                n_verts,
                n_props,
                tri_indices.as_ptr(),
                n_tris,
                &sys_options,
            );
        }
        Ok(Self { ptr })
    }

    /// Number of vertices.
    #[must_use]
    pub fn num_vert(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl64_num_vert(self.ptr) }
    }

    /// Number of triangles.
    #[must_use]
    pub fn num_tri(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl64_num_tri(self.ptr) }
    }

    /// Number of properties per vertex.
    #[must_use]
    pub fn num_prop(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl64_num_prop(self.ptr) }
    }

    /// Copy vertex properties out as a flat f64 array.
    #[must_use]
    pub fn vert_properties(&self) -> Vec<f64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_vert_properties_length(self.ptr) };
        let mut buf = vec![0.0f64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_vert_properties(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy triangle indices out as a flat u64 array.
    #[must_use]
    pub fn tri_verts(&self) -> Vec<u64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_tri_length(self.ptr) };
        let mut buf = vec![0u64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_tri_verts(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Merge coincident vertices, returning a new mesh.
    ///
    /// Processes the mesh's merge vectors to weld vertices that share
    /// the same position. Returns a new mesh (the original is unchanged).
    #[must_use]
    pub fn merge(&self) -> Self {
        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_meshgl64_merge(ptr, self.ptr) };
        Self { ptr }
    }

    /// Copy merge-from vertex indices out as a flat u64 array.
    #[must_use]
    pub fn merge_from_vert(&self) -> Vec<u64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_merge_length(self.ptr) };
        let mut buf = vec![0u64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_merge_from_vert(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy merge-to vertex indices out as a flat u64 array.
    #[must_use]
    pub fn merge_to_vert(&self) -> Vec<u64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_merge_length(self.ptr) };
        let mut buf = vec![0u64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_merge_to_vert(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run indices out as a flat u64 array.
    #[must_use]
    pub fn run_index(&self) -> Vec<u64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_run_index_length(self.ptr) };
        let mut buf = vec![0u64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_run_index(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run original IDs out as a flat u32 array.
    #[must_use]
    pub fn run_original_id(&self) -> Vec<u32> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_run_original_id_length(self.ptr) };
        let mut buf = vec![0u32; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_run_original_id(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy run transforms out as a flat f64 array (4x3 matrices, 12 doubles each).
    #[must_use]
    pub fn run_transform(&self) -> Vec<f64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_run_transform_length(self.ptr) };
        let mut buf = vec![0.0f64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_run_transform(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy face IDs out as a flat u64 array.
    #[must_use]
    pub fn face_id(&self) -> Vec<u64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_face_id_length(self.ptr) };
        let mut buf = vec![0u64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_face_id(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Copy halfedge tangents out as a flat f64 array (4 doubles per halfedge).
    #[must_use]
    pub fn halfedge_tangent(&self) -> Vec<f64> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_tangent_length(self.ptr) };
        let mut buf = vec![0.0f64; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_halfedge_tangent(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Tolerance used for merging and vertex welding.
    #[must_use]
    pub fn tolerance(&self) -> f64 {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl64_tolerance(self.ptr) }
    }

    /// Number of triangle runs.
    #[must_use]
    pub fn num_run(&self) -> usize {
        // SAFETY: self.ptr is valid (invariant).
        unsafe { manifold_meshgl64_num_run(self.ptr) }
    }

    /// Copy run flags out as a u8 array (one per triangle run).
    #[must_use]
    pub fn run_flags(&self) -> Vec<u8> {
        // SAFETY: self.ptr is valid (invariant).
        let len = unsafe { manifold_meshgl64_run_flags_length(self.ptr) };
        let mut buf = vec![0u8; len];
        // SAFETY: buf has capacity len, self.ptr is valid.
        unsafe { manifold_meshgl64_run_flags(buf.as_mut_ptr(), self.ptr) };
        buf
    }

    /// Read a MeshGL64 from a Wavefront OBJ string.
    ///
    /// Unavailable on `wasm32-unknown-unknown` (manifold's iostream-based
    /// OBJ paths are excluded from the freestanding wasm build).
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn from_obj(obj_content: &str) -> Result<Self, crate::types::CsgError> {
        let c_str = std::ffi::CString::new(obj_content).map_err(|_| {
            crate::types::CsgError::InvalidInput("OBJ content contains null byte".into())
        })?;
        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr valid from alloc, c_str.as_ptr() is a valid null-terminated string.
        unsafe { manifold_meshgl64_read_obj(ptr, c_str.as_ptr()) };
        Ok(Self { ptr })
    }

    /// Export this mesh as a Wavefront OBJ string.
    ///
    /// Unavailable on `wasm32-unknown-unknown` (see [`from_obj`](Self::from_obj)).
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
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
        unsafe { manifold_meshgl64_write_obj(self.ptr, Some(callback), ctx) };
        result
    }
}

impl Clone for MeshGL64 {
    fn clone(&self) -> Self {
        // SAFETY: manifold_alloc_meshgl64 returns a valid handle.
        let ptr = unsafe { manifold_alloc_meshgl64() };
        // SAFETY: ptr and self.ptr are valid.
        unsafe { manifold_meshgl64_copy(ptr, self.ptr) };
        Self { ptr }
    }
}

fn validate_mesh_shape(
    vert_props_len: usize,
    n_props: usize,
    tri_indices_len: usize,
) -> Result<(), CsgError> {
    if n_props < 3 {
        return Err(CsgError::InvalidInput(
            "n_props must be >= 3 (x, y, z)".into(),
        ));
    }
    if vert_props_len % n_props != 0 {
        return Err(CsgError::InvalidInput(
            "vert_props length must be divisible by n_props".into(),
        ));
    }
    if tri_indices_len % 3 != 0 {
        return Err(CsgError::InvalidInput(
            "tri_indices length must be divisible by 3".into(),
        ));
    }
    Ok(())
}

fn validate_tangent_len(tangent_len: usize, n_tris: usize) -> Result<(), CsgError> {
    let expected = n_tris
        .checked_mul(3)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| CsgError::InvalidInput("tangent length overflow".into()))?;
    if tangent_len != expected {
        return Err(CsgError::InvalidInput(format!(
            "halfedge_tangent length must be num_tri * 3 * 4 ({expected}), got {tangent_len}"
        )));
    }
    Ok(())
}

fn validate_meshgl_options(
    n_tris: usize,
    tri_indices_len: usize,
    options: MeshGLOptions<'_>,
) -> Result<(), CsgError> {
    if let Some(tangents) = options.halfedge_tangents {
        validate_tangent_len(tangents.len(), n_tris)?;
    }
    if let Some((run_indices, run_original_ids)) = options.runs {
        validate_run_metadata(run_indices, run_original_ids.len(), tri_indices_len)?;
    }
    if let Some((merge_from, merge_to)) = options.merge_vertices {
        validate_merge_metadata(merge_from.len(), merge_to.len())?;
    }
    Ok(())
}

fn validate_meshgl64_options(
    n_tris: usize,
    tri_indices_len: usize,
    options: MeshGL64Options<'_>,
) -> Result<(), CsgError> {
    if let Some(tangents) = options.halfedge_tangents {
        validate_tangent_len(tangents.len(), n_tris)?;
    }
    if let Some((run_indices, run_original_ids)) = options.runs {
        validate_run_metadata(run_indices, run_original_ids.len(), tri_indices_len)?;
    }
    if let Some((merge_from, merge_to)) = options.merge_vertices {
        validate_merge_metadata(merge_from.len(), merge_to.len())?;
    }
    Ok(())
}

fn validate_run_metadata<I>(
    run_indices: &[I],
    run_original_ids_len: usize,
    tri_indices_len: usize,
) -> Result<(), CsgError>
where
    I: Copy + Into<u64>,
{
    if run_original_ids_len == 0 {
        return Err(CsgError::InvalidInput(
            "run metadata requires at least one run_original_id".into(),
        ));
    }
    if run_indices.len() != run_original_ids_len && run_indices.len() != run_original_ids_len + 1 {
        return Err(CsgError::InvalidInput(
            "run_indices length must equal run_original_ids length, or be one longer".into(),
        ));
    }
    let Some(first) = run_indices.first() else {
        return Err(CsgError::InvalidInput(
            "run metadata requires at least one run_index".into(),
        ));
    };
    if (*first).into() != 0 {
        return Err(CsgError::InvalidInput("run_indices must start at 0".into()));
    }
    let mut previous = (*first).into();
    for &index in run_indices {
        let index = index.into();
        if index % 3 != 0 {
            return Err(CsgError::InvalidInput(
                "run_indices values must be divisible by 3".into(),
            ));
        }
        if index > tri_indices_len as u64 {
            return Err(CsgError::InvalidInput(
                "run_indices values must not exceed tri_indices length".into(),
            ));
        }
        if index < previous {
            return Err(CsgError::InvalidInput("run_indices must be sorted".into()));
        }
        previous = index;
    }
    if run_indices.len() == run_original_ids_len + 1
        && run_indices.last().copied().map(Into::into) != Some(tri_indices_len as u64)
    {
        return Err(CsgError::InvalidInput(
            "run_indices sentinel must equal tri_indices length".into(),
        ));
    }
    Ok(())
}

fn validate_merge_metadata(merge_from_len: usize, merge_to_len: usize) -> Result<(), CsgError> {
    if merge_from_len != merge_to_len {
        return Err(CsgError::InvalidInput(
            "merge_from and merge_to must have the same length".into(),
        ));
    }
    Ok(())
}

fn slice_ptr<T>(slice: &[T]) -> *const T {
    if slice.is_empty() {
        std::ptr::null()
    } else {
        slice.as_ptr()
    }
}
