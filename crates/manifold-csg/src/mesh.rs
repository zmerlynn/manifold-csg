//! Safe wrappers for manifold3d mesh data types.
//!
//! [`MeshGL`] wraps f32 mesh data, [`MeshGL64`] wraps f64 mesh data.
//! These are primarily used for constructing [`Manifold`](crate::Manifold)
//! objects and extracting mesh data from them.

use manifold_csg_sys::*;

/// Safe wrapper around a manifold3d MeshGL object (f32 vertices, u32 indices).
///
/// See the [upstream `MeshGL` docs](https://elalish.github.io/manifold/docs/html/structmanifold_1_1_mesh_g_l_p.html)
/// for field semantics (run indices, merge vectors, tangents, etc.).
pub struct MeshGL {
    ptr: *mut ManifoldMeshGL,
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
    /// # Panics
    ///
    /// Panics if `n_props < 3`, if `vert_props.len()` is not divisible by
    /// `n_props`, or if `tri_indices.len()` is not divisible by 3.
    #[must_use]
    pub fn new(vert_props: &[f32], n_props: usize, tri_indices: &[u32]) -> Self {
        assert!(n_props >= 3, "n_props must be >= 3");
        assert!(
            vert_props.len() % n_props == 0,
            "vert_props length must be divisible by n_props"
        );
        assert!(
            tri_indices.len() % 3 == 0,
            "tri_indices length must be divisible by 3"
        );
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
        Self { ptr }
    }

    /// Create a MeshGL with halfedge tangent data.
    ///
    /// `halfedge_tangent` must have `num_tri * 3 * 4` elements (4 floats per
    /// halfedge, 3 halfedges per triangle).
    ///
    /// # Panics
    ///
    /// Same as [`new`](Self::new).
    #[must_use]
    pub fn new_with_tangents(
        vert_props: &[f32],
        n_props: usize,
        tri_indices: &[u32],
        halfedge_tangent: &[f32],
    ) -> Self {
        assert!(n_props >= 3, "n_props must be >= 3");
        assert!(vert_props.len() % n_props == 0);
        assert!(tri_indices.len() % 3 == 0);
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

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
        Self { ptr }
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

    /// Update normals based on run transforms and backside flags, then clear
    /// those fields to avoid double-applying on round-trip.
    ///
    /// `normal_idx` specifies the first of three consecutive property channels
    /// forming the (x, y, z) normals. Must be >= 3 and `num_prop` must be at
    /// least `normal_idx + 3`.
    pub fn update_normals(&mut self, normal_idx: i32) {
        // SAFETY: self.ptr is valid (invariant), mutation is exclusive via &mut self.
        unsafe { manifold_meshgl_update_normals(self.ptr, normal_idx) };
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
    ptr: *mut ManifoldMeshGL64,
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
    /// # Panics
    ///
    /// Panics if `n_props < 3`, if `vert_props.len()` is not divisible by
    /// `n_props`, or if `tri_indices.len()` is not divisible by 3.
    #[must_use]
    pub fn new(vert_props: &[f64], n_props: usize, tri_indices: &[u64]) -> Self {
        assert!(n_props >= 3, "n_props must be >= 3");
        assert!(
            vert_props.len() % n_props == 0,
            "vert_props length must be divisible by n_props"
        );
        assert!(
            tri_indices.len() % 3 == 0,
            "tri_indices length must be divisible by 3"
        );
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
        Self { ptr }
    }

    /// Create a MeshGL64 with halfedge tangent data.
    ///
    /// See [`MeshGL::new_with_tangents`] for details.
    #[must_use]
    pub fn new_with_tangents(
        vert_props: &[f64],
        n_props: usize,
        tri_indices: &[u64],
        halfedge_tangent: &[f64],
    ) -> Self {
        assert!(n_props >= 3, "n_props must be >= 3");
        assert!(vert_props.len() % n_props == 0);
        assert!(tri_indices.len() % 3 == 0);
        let n_verts = vert_props.len() / n_props;
        let n_tris = tri_indices.len() / 3;

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
        Self { ptr }
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

    /// Update normals based on run transforms and backside flags, then clear
    /// those fields to avoid double-applying on round-trip.
    ///
    /// `normal_idx` specifies the first of three consecutive property channels
    /// forming the (x, y, z) normals. Must be >= 3 and `num_prop` must be at
    /// least `normal_idx + 3`.
    pub fn update_normals(&mut self, normal_idx: i32) {
        // SAFETY: self.ptr is valid (invariant), mutation is exclusive via &mut self.
        unsafe { manifold_meshgl64_update_normals(self.ptr, normal_idx) };
    }

    /// Read a MeshGL64 from a Wavefront OBJ string.
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
