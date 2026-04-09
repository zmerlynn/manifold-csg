//! Safe wrappers for manifold3d mesh data types.
//!
//! [`MeshGL`] wraps f32 mesh data, [`MeshGL64`] wraps f64 mesh data.
//! These are primarily used for constructing [`Manifold`](crate::Manifold)
//! objects and extracting mesh data from them.

use manifold_csg_sys::*;

/// Safe wrapper around a manifold3d MeshGL object (f32 vertices, u32 indices).
pub struct MeshGL {
    ptr: *mut ManifoldMeshGL,
}

// SAFETY: MeshGL owns its heap allocation with no thread-local state.
unsafe impl Send for MeshGL {}

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
        // SAFETY: ptr and self.ptr are valid. With the carry-patch applied,
        // manifold_meshgl_merge always returns ptr (the output buffer).
        unsafe { manifold_meshgl_merge(ptr, self.ptr) };
        Self { ptr }
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
pub struct MeshGL64 {
    ptr: *mut ManifoldMeshGL64,
}

// SAFETY: MeshGL64 owns its heap allocation with no thread-local state.
unsafe impl Send for MeshGL64 {}

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
        // SAFETY: ptr and self.ptr are valid. With the carry-patch applied,
        // manifold_meshgl64_merge always returns ptr (the output buffer).
        unsafe { manifold_meshgl64_merge(ptr, self.ptr) };
        Self { ptr }
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
