//! Raw FFI bindings to the [manifold3d](https://github.com/elalish/manifold) C API.
//!
//! These are low-level, unsafe bindings. Users should prefer the safe wrappers
//! in the [`manifold-csg`](https://crates.io/crates/manifold-csg) crate.
//!
//! # Overview
//!
//! manifold3d is a geometry kernel for constructive solid geometry (CSG)
//! operations. It provides:
//!
//! - **3D Boolean operations**: union, difference, intersection of solid meshes
//! - **2D cross-section operations**: offset, boolean, hull for 2D regions
//! - **Mesh construction**: from vertices/indices, primitives (cube, sphere, cylinder)
//! - **Extrusion**: 2D cross-sections to 3D solids
//! - **Queries**: volume, surface area, bounding box, vertex/face counts

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_void};
use std::os::raw::c_int;

// ── Opaque handle types ─────────────────────────────────────────────────

/// Opaque handle to a manifold3d Manifold object (3D solid).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldManifold {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `ManifoldVec` (vector of Manifold objects).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldManifoldVec {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `Polygons` object (2D polygon set).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldPolygons {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `SimplePolygon` object (single polygon ring).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldSimplePolygon {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `RayHitVec` (vector of ray hit results).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldRayHitVec {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `ExecutionContext` — observes progress
/// and allows cooperative cancellation of long-running boolean evaluations.
/// The C API documents it as safe to read/write from any thread.
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldExecutionContext {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `Triangulation` result.
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldTriangulation {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `MeshGL` object (f32 vertices, u32 indices).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldMeshGL {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `MeshGL64` object (f64 vertices, u64 indices).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldMeshGL64 {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `Box` (3D axis-aligned bounding box).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldBox {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `CrossSection` object (2D region).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldCrossSection {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `CrossSectionVec` (vector of CrossSection objects).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldCrossSectionVec {
    _private: [u8; 0],
}

/// Opaque handle to a manifold3d `Rect` (2D axis-aligned bounding box).
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldRect {
    _private: [u8; 0],
}

// ── Value types ─────────────────────────────────────────────────────────

/// Pair of manifolds returned by `manifold_split` / `manifold_split_by_plane`.
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldManifoldPair {
    pub first: *mut ManifoldManifold,
    pub second: *mut ManifoldManifold,
}

/// 2D vector used by manifold3d polygon API.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldVec2 {
    pub x: f64,
    pub y: f64,
}

/// 3D vector returned by manifold3d C API.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// 3D integer vector.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldIVec3 {
    pub x: c_int,
    pub y: c_int,
    pub z: c_int,
}

/// 4D vector (e.g. for tangents with weight).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldVec4 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

/// Surface area and volume properties returned by manifold queries.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldProperties {
    pub surface_area: f64,
    pub volume: f64,
}

/// Result of a ray-manifold intersection test.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ManifoldRayHit {
    pub face_id: u64,
    pub distance: f64,
    pub position: ManifoldVec3,
    pub normal: ManifoldVec3,
    pub crossing: c_int,
}

/// Options for constructing a `MeshGL` with additional metadata.
///
/// The upstream C header declares these input-only pointers as mutable. The
/// Rust binding uses const pointers for the same ABI so safe wrappers can pass
/// shared slices without casting away constness.
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldMeshGLOptions {
    pub run_indices: *const u32,
    pub run_indices_length: usize,
    pub run_original_ids: *const u32,
    pub run_original_ids_length: usize,
    pub merge_from_vert: *const u32,
    pub merge_to_vert: *const u32,
    pub merge_verts_length: usize,
    pub halfedge_tangents: *const f32,
}

/// Options for constructing a `MeshGL64` with additional metadata.
///
/// The upstream C header declares these input-only pointers as mutable. The
/// Rust binding uses const pointers for the same ABI so safe wrappers can pass
/// shared slices without casting away constness.
#[repr(C)]
#[derive(Debug)]
pub struct ManifoldMeshGL64Options {
    pub run_indices: *const u64,
    pub run_indices_length: usize,
    pub run_original_ids: *const u32,
    pub run_original_ids_length: usize,
    pub merge_from_vert: *const u64,
    pub merge_to_vert: *const u64,
    pub merge_verts_length: usize,
    pub halfedge_tangents: *const f64,
}

// ── Enums ───────────────────────────────────────────────────────────────

/// Boolean operation type for `manifold_batch_boolean`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldOpType {
    Add = 0,
    Subtract = 1,
    Intersect = 2,
}

/// Join type for CrossSection offset operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldJoinType {
    Square = 0,
    Round = 1,
    Miter = 2,
    Bevel = 3,
}

/// Error codes from manifold3d status check.
#[repr(C)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldError {
    NoError = 0,
    NonFiniteVertex = 1,
    NotManifold = 2,
    VertexOutOfBounds = 3,
    PropertiesWrongLength = 4,
    MissingPositionProperties = 5,
    MergeVectorsDifferentLengths = 6,
    MergeIndexOutOfBounds = 7,
    TransformWrongLength = 8,
    RunIndexWrongLength = 9,
    FaceIdWrongLength = 10,
    InvalidConstruction = 11,
    ResultTooLarge = 12,
    InvalidTangents = 13,
    Cancelled = 14,
}

impl ManifoldError {
    /// Returns true when the status is [`ManifoldError::NoError`].
    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::NoError)
    }

    /// Returns true when the status represents an error.
    #[must_use]
    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }
}

// ── Function pointer types ─────────────────────────────────────────────

/// SDF callback: `fn(x, y, z, ctx) -> distance`
pub type ManifoldSdf = Option<unsafe extern "C" fn(f64, f64, f64, *mut std::ffi::c_void) -> f64>;

// ── C API declarations ──────────────────────────────────────────────────

unsafe extern "C" {
    // ── Deletion ───────────────────────────────────────────────────────

    pub fn manifold_delete_manifold(m: *mut ManifoldManifold);
    pub fn manifold_delete_manifold_vec(ms: *mut ManifoldManifoldVec);
    pub fn manifold_delete_cross_section(cs: *mut ManifoldCrossSection);
    pub fn manifold_delete_cross_section_vec(csv: *mut ManifoldCrossSectionVec);
    pub fn manifold_delete_simple_polygon(p: *mut ManifoldSimplePolygon);
    pub fn manifold_delete_polygons(p: *mut ManifoldPolygons);
    pub fn manifold_delete_meshgl(m: *mut ManifoldMeshGL);
    pub fn manifold_delete_meshgl64(m: *mut ManifoldMeshGL64);
    pub fn manifold_delete_box(b: *mut ManifoldBox);
    pub fn manifold_delete_rect(b: *mut ManifoldRect);
    pub fn manifold_delete_triangulation(m: *mut ManifoldTriangulation);
    pub fn manifold_delete_ray_hit_vec(v: *mut ManifoldRayHitVec);
    pub fn manifold_delete_execution_context(ctx: *mut ManifoldExecutionContext);

    // ── Polygons ───────────────────────────────────────────────────────

    /// Create a simple polygon from an array of 2D points.
    pub fn manifold_simple_polygon(
        ps: *const ManifoldVec2,
        length: usize,
    ) -> *mut ManifoldSimplePolygon;

    /// Create a polygon set from an array of simple polygon pointers.
    pub fn manifold_polygons(
        ps: *const *const ManifoldSimplePolygon,
        length: usize,
    ) -> *mut ManifoldPolygons;

    /// Get the number of points in a simple polygon.
    pub fn manifold_simple_polygon_length(p: *const ManifoldSimplePolygon) -> usize;

    /// Get the number of simple polygons in a polygon set.
    pub fn manifold_polygons_length(ps: *const ManifoldPolygons) -> usize;

    /// Get the number of points in a specific simple polygon within a polygon set.
    pub fn manifold_polygons_simple_length(ps: *const ManifoldPolygons, idx: usize) -> usize;

    /// Get a point from a simple polygon by index.
    pub fn manifold_simple_polygon_get_point(
        p: *const ManifoldSimplePolygon,
        idx: usize,
    ) -> ManifoldVec2;

    /// Extract a simple polygon from a polygon set by index.
    pub fn manifold_polygons_get_simple(
        ps: *const ManifoldPolygons,
        idx: usize,
    ) -> *mut ManifoldSimplePolygon;

    /// Get a point from a polygon set by simple polygon index and point index.
    pub fn manifold_polygons_get_point(
        ps: *const ManifoldPolygons,
        simple_idx: usize,
        pt_idx: usize,
    ) -> ManifoldVec2;

    // ── MeshGL construction (f32) ───────────────────────────────────────

    /// Create a `MeshGL` from vertex properties and triangle indices.
    /// `vert_props`: flat f32 array [x,y,z,...] with `n_props` values per vertex.
    /// `tri_verts`: flat u32 array, 3 indices per triangle.
    pub fn manifold_meshgl(
        vert_props: *const f32,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u32,
        n_tris: usize,
    ) -> *mut ManifoldMeshGL;

    /// Create a `MeshGL` with halfedge tangents.
    pub fn manifold_meshgl_w_tangents(
        vert_props: *const f32,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u32,
        n_tris: usize,
        halfedge_tangent: *const f32,
    ) -> *mut ManifoldMeshGL;

    /// Create a `MeshGL` with full options (run indices, merge, tangents).
    pub fn manifold_meshgl_w_options(
        vert_props: *const f32,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u32,
        n_tris: usize,
        options: *const ManifoldMeshGLOptions,
    ) -> *mut ManifoldMeshGL;

    /// Create a Manifold from a `MeshGL`.
    pub fn manifold_of_meshgl(mesh: *const ManifoldMeshGL) -> *mut ManifoldManifold;

    /// Extract `MeshGL` from a Manifold.
    pub fn manifold_get_meshgl(m: *const ManifoldManifold) -> *mut ManifoldMeshGL;

    /// Extract `MeshGL` from a Manifold with normals at the given property index.
    pub fn manifold_get_meshgl_w_normals(
        m: *const ManifoldManifold,
        normal_idx: i32,
    ) -> *mut ManifoldMeshGL;

    /// Copy a `MeshGL`.
    pub fn manifold_meshgl_copy(m: *const ManifoldMeshGL) -> *mut ManifoldMeshGL;

    /// Merge coincident vertices in a `MeshGL`, returning a new merged mesh.
    pub fn manifold_meshgl_merge(m: *const ManifoldMeshGL, tolerance: f32) -> *mut ManifoldMeshGL;

    // ── MeshGL data access ──────────────────────────────────────────────

    pub fn manifold_meshgl_num_prop(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_num_vert(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_num_tri(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_vert_properties_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_tri_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_merge_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_run_index_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_run_original_id_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_run_transform_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_face_id_length(m: *const ManifoldMeshGL) -> usize;
    pub fn manifold_meshgl_tangent_length(m: *const ManifoldMeshGL) -> usize;

    /// Pointer to the mesh's vertex properties, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_vert_properties(m: *const ManifoldMeshGL) -> *const f32;

    /// Pointer to the mesh's triangle indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_tri_verts(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's merge-from vertex indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_merge_from_vert(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's merge-to vertex indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_merge_to_vert(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's run indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_run_index(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's run original IDs, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_run_original_id(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's run transforms, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_run_transform(m: *const ManifoldMeshGL) -> *const f32;

    /// Pointer to the mesh's face IDs, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_face_id(m: *const ManifoldMeshGL) -> *const u32;

    /// Pointer to the mesh's halfedge tangents, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_halfedge_tangent(m: *const ManifoldMeshGL) -> *const f32;

    /// Get the number of run flags.
    pub fn manifold_meshgl_run_flags_length(m: *const ManifoldMeshGL) -> usize;

    /// Pointer to the mesh's run flags, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl_run_flags(m: *const ManifoldMeshGL) -> *const u8;

    /// Get the number of triangle runs.
    pub fn manifold_meshgl_num_run(m: *const ManifoldMeshGL) -> usize;

    /// Non-zero when run `run` is a mirrored (backside) instance, i.e. bit 0
    /// of its run flag.
    pub fn manifold_meshgl_backside(m: *const ManifoldMeshGL, run: usize) -> c_int;

    /// Non-zero when property slots 3, 4, 5 of run `run` carry world-frame
    /// vertex normals, i.e. bit 1 of its run flag.
    pub fn manifold_meshgl_has_normals(m: *const ManifoldMeshGL, run: usize) -> c_int;

    // ── MeshGL64 construction (f64 vertices, u64 indices) ───────────────

    /// Create a `MeshGL64` from f64 vertex properties and u64 triangle indices.
    pub fn manifold_meshgl64(
        vert_props: *const f64,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u64,
        n_tris: usize,
    ) -> *mut ManifoldMeshGL64;

    /// Create a `MeshGL64` with halfedge tangents.
    pub fn manifold_meshgl64_w_tangents(
        vert_props: *const f64,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u64,
        n_tris: usize,
        halfedge_tangent: *const f64,
    ) -> *mut ManifoldMeshGL64;

    /// Create a `MeshGL64` with full options (run indices, merge, tangents).
    pub fn manifold_meshgl64_w_options(
        vert_props: *const f64,
        n_verts: usize,
        n_props: usize,
        tri_verts: *const u64,
        n_tris: usize,
        options: *const ManifoldMeshGL64Options,
    ) -> *mut ManifoldMeshGL64;

    /// Create a Manifold from a `MeshGL64`.
    pub fn manifold_of_meshgl64(mesh: *const ManifoldMeshGL64) -> *mut ManifoldManifold;

    /// Extract `MeshGL64` from a Manifold.
    pub fn manifold_get_meshgl64(m: *const ManifoldManifold) -> *mut ManifoldMeshGL64;

    /// Extract `MeshGL64` from a Manifold with normals at the given property index.
    pub fn manifold_get_meshgl64_w_normals(
        m: *const ManifoldManifold,
        normal_idx: i32,
    ) -> *mut ManifoldMeshGL64;

    /// Copy a `MeshGL64`.
    pub fn manifold_meshgl64_copy(m: *const ManifoldMeshGL64) -> *mut ManifoldMeshGL64;

    /// Merge coincident vertices in a `MeshGL64`, returning a new merged mesh.
    pub fn manifold_meshgl64_merge(
        m: *const ManifoldMeshGL64,
        tolerance: f64,
    ) -> *mut ManifoldMeshGL64;

    // ── MeshGL64 data access ────────────────────────────────────────────

    pub fn manifold_meshgl64_num_prop(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_num_vert(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_num_tri(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_vert_properties_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_tri_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_merge_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_run_index_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_run_original_id_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_run_transform_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_face_id_length(m: *const ManifoldMeshGL64) -> usize;
    pub fn manifold_meshgl64_tangent_length(m: *const ManifoldMeshGL64) -> usize;

    /// Pointer to the mesh's f64 vertex properties, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_vert_properties(m: *const ManifoldMeshGL64) -> *const f64;

    /// Pointer to the mesh's u64 triangle indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_tri_verts(m: *const ManifoldMeshGL64) -> *const u64;

    /// Pointer to the mesh's merge-from vertex indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_merge_from_vert(m: *const ManifoldMeshGL64) -> *const u64;

    /// Pointer to the mesh's merge-to vertex indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_merge_to_vert(m: *const ManifoldMeshGL64) -> *const u64;

    /// Pointer to the mesh's run indices, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_run_index(m: *const ManifoldMeshGL64) -> *const u64;

    /// Pointer to the mesh's run original IDs, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_run_original_id(m: *const ManifoldMeshGL64) -> *const u32;

    /// Pointer to the mesh's run transforms, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_run_transform(m: *const ManifoldMeshGL64) -> *const f64;

    /// Pointer to the mesh's face IDs, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_face_id(m: *const ManifoldMeshGL64) -> *const u64;

    /// Pointer to the mesh's halfedge tangents, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_halfedge_tangent(m: *const ManifoldMeshGL64) -> *const f64;

    /// Get the number of run flags.
    pub fn manifold_meshgl64_run_flags_length(m: *const ManifoldMeshGL64) -> usize;

    /// Pointer to the mesh's run flags, borrowed from `m` and valid only
    /// while `m` is alive and unmodified. The length comes from the matching
    /// `_length` function.
    pub fn manifold_meshgl64_run_flags(m: *const ManifoldMeshGL64) -> *const u8;

    /// Get the number of triangle runs.
    pub fn manifold_meshgl64_num_run(m: *const ManifoldMeshGL64) -> usize;

    /// Non-zero when run `run` is a mirrored (backside) instance, i.e. bit 0
    /// of its run flag.
    pub fn manifold_meshgl64_backside(m: *const ManifoldMeshGL64, run: usize) -> c_int;

    /// Non-zero when property slots 3, 4, 5 of run `run` carry world-frame
    /// vertex normals, i.e. bit 1 of its run flag.
    pub fn manifold_meshgl64_has_normals(m: *const ManifoldMeshGL64, run: usize) -> c_int;

    // ── SDF (level set) ────────────────────────────────────────────────

    /// Create a manifold from a signed distance function.
    ///
    /// By default, the execution policy (sequential or parallel) will be chosen
    /// automatically depending on the size of the job and whether Manifold has
    /// been compiled with a PAR backend.
    pub fn manifold_level_set(
        sdf: ManifoldSdf,
        bounds: *const ManifoldBox,
        edge_length: f64,
        level: f64,
        tolerance: f64,
        ctx: *mut c_void,
    ) -> *mut ManifoldManifold;

    /// Create a manifold from a signed distance function (sequential execution).
    ///
    /// Use this if you are calling from a language runtime that has a lock
    /// preventing parallel execution of closures.
    pub fn manifold_level_set_seq(
        sdf: ManifoldSdf,
        bounds: *const ManifoldBox,
        edge_length: f64,
        level: f64,
        tolerance: f64,
        ctx: *mut c_void,
    ) -> *mut ManifoldManifold;

    // ── Manifold vectors ───────────────────────────────────────────────

    pub fn manifold_manifold_empty_vec() -> *mut ManifoldManifoldVec;

    pub fn manifold_manifold_vec(sz: usize) -> *mut ManifoldManifoldVec;

    pub fn manifold_manifold_vec_reserve(ms: *mut ManifoldManifoldVec, sz: usize);

    pub fn manifold_manifold_vec_length(ms: *const ManifoldManifoldVec) -> usize;

    pub fn manifold_manifold_vec_get(
        ms: *const ManifoldManifoldVec,
        idx: usize,
    ) -> *mut ManifoldManifold;

    pub fn manifold_manifold_vec_set(
        ms: *mut ManifoldManifoldVec,
        idx: usize,
        m: *const ManifoldManifold,
    );

    pub fn manifold_manifold_vec_push_back(
        ms: *mut ManifoldManifoldVec,
        m: *const ManifoldManifold,
    );

    // ── Manifold boolean operations ────────────────────────────────────

    /// Generic boolean operation between two manifolds.
    pub fn manifold_boolean(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
        op: ManifoldOpType,
    ) -> *mut ManifoldManifold;

    /// Batch boolean: apply `op` across all manifolds in the vector.
    pub fn manifold_batch_boolean(
        ms: *const ManifoldManifoldVec,
        op: ManifoldOpType,
    ) -> *mut ManifoldManifold;

    pub fn manifold_union(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> *mut ManifoldManifold;

    pub fn manifold_difference(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> *mut ManifoldManifold;

    pub fn manifold_intersection(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> *mut ManifoldManifold;

    /// Split manifold `a` by manifold `b` into two parts.
    pub fn manifold_split(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> ManifoldManifoldPair;

    /// Minkowski sum of two manifolds.
    pub fn manifold_minkowski_sum(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> *mut ManifoldManifold;

    /// Minkowski difference of two manifolds.
    pub fn manifold_minkowski_difference(
        a: *const ManifoldManifold,
        b: *const ManifoldManifold,
    ) -> *mut ManifoldManifold;

    // ── Plane operations ────────────────────────────────────────────────

    /// Split a manifold into two halves along a plane.
    ///
    /// The plane is `nx*x + ny*y + nz*z = offset`.
    /// Returns a pair: `first` is on the positive side, `second` on negative.
    pub fn manifold_split_by_plane(
        m: *const ManifoldManifold,
        normal_x: f64,
        normal_y: f64,
        normal_z: f64,
        offset: f64,
    ) -> ManifoldManifoldPair;

    /// Trim to the positive side of a plane.
    pub fn manifold_trim_by_plane(
        m: *const ManifoldManifold,
        normal_x: f64,
        normal_y: f64,
        normal_z: f64,
        offset: f64,
    ) -> *mut ManifoldManifold;

    // ── 3D to 2D ────────────────────────────────────────────────────────

    /// Slice a manifold at a given Z height, returning 2D polygons.
    pub fn manifold_slice(m: *const ManifoldManifold, height: f64) -> *mut ManifoldPolygons;

    /// Project a manifold onto the XY plane, returning 2D polygons.
    pub fn manifold_project(m: *const ManifoldManifold) -> *mut ManifoldPolygons;

    // ── Convex hulls ────────────────────────────────────────────────────

    /// Compute the convex hull of a manifold.
    pub fn manifold_hull(m: *const ManifoldManifold) -> *mut ManifoldManifold;

    /// Compute the convex hull of a set of manifolds.
    pub fn manifold_batch_hull(ms: *const ManifoldManifoldVec) -> *mut ManifoldManifold;

    /// Compute the convex hull of a set of 3D points.
    pub fn manifold_hull_pts(ps: *const ManifoldVec3, length: usize) -> *mut ManifoldManifold;

    // ── Transforms ──────────────────────────────────────────────────────

    pub fn manifold_translate(
        m: *const ManifoldManifold,
        x: f64,
        y: f64,
        z: f64,
    ) -> *mut ManifoldManifold;

    /// Rotate by Euler angles (degrees), applied in z-y'-x" order.
    pub fn manifold_rotate(
        m: *const ManifoldManifold,
        x: f64,
        y: f64,
        z: f64,
    ) -> *mut ManifoldManifold;

    pub fn manifold_scale(
        m: *const ManifoldManifold,
        x: f64,
        y: f64,
        z: f64,
    ) -> *mut ManifoldManifold;

    /// Apply a 4x3 affine transformation matrix (column-major).
    ///
    /// ```text
    /// | x1 x2 x3 x4 |     col1 = (x1, y1, z1) -- X basis
    /// | y1 y2 y3 y4 |     col2 = (x2, y2, z2) -- Y basis
    /// | z1 z2 z3 z4 |     col3 = (x3, y3, z3) -- Z basis
    ///                      col4 = (x4, y4, z4) -- translation
    /// ```
    pub fn manifold_transform(
        m: *const ManifoldManifold,
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
        x3: f64,
        y3: f64,
        z3: f64,
        x4: f64,
        y4: f64,
        z4: f64,
    ) -> *mut ManifoldManifold;

    /// Mirror a manifold across the plane defined by normal (nx, ny, nz).
    pub fn manifold_mirror(
        m: *const ManifoldManifold,
        nx: f64,
        ny: f64,
        nz: f64,
    ) -> *mut ManifoldManifold;

    /// Warp a manifold by applying a function to each vertex.
    pub fn manifold_warp(
        m: *const ManifoldManifold,
        fun: Option<unsafe extern "C" fn(f64, f64, f64, *mut std::ffi::c_void) -> ManifoldVec3>,
        ctx: *mut std::ffi::c_void,
    ) -> *mut ManifoldManifold;

    /// Smooth a manifold by converting sharp edges to smooth curves using
    /// vertex normals at the given property index.
    pub fn manifold_smooth_by_normals(
        m: *const ManifoldManifold,
        normal_idx: c_int,
    ) -> *mut ManifoldManifold;

    /// Increase the density of the mesh by splitting every edge into
    /// `refine` pieces.
    pub fn manifold_refine(m: *const ManifoldManifold, refine: c_int) -> *mut ManifoldManifold;

    /// Refine the mesh so that no edge is longer than `length`.
    pub fn manifold_refine_to_length(
        m: *const ManifoldManifold,
        length: f64,
    ) -> *mut ManifoldManifold;

    /// Refine the mesh to a given tolerance.
    pub fn manifold_refine_to_tolerance(
        m: *const ManifoldManifold,
        tolerance: f64,
    ) -> *mut ManifoldManifold;

    /// Simplify the mesh, removing vertices until the error exceeds `tolerance`.
    pub fn manifold_simplify(m: *const ManifoldManifold, tolerance: f64) -> *mut ManifoldManifold;

    // ── Shapes / Constructors ───────────────────────────────────────────

    pub fn manifold_empty() -> *mut ManifoldManifold;

    pub fn manifold_copy(m: *const ManifoldManifold) -> *mut ManifoldManifold;

    /// Construct a regular tetrahedron.
    pub fn manifold_tetrahedron() -> *mut ManifoldManifold;

    pub fn manifold_cube(x: f64, y: f64, z: f64, center: c_int) -> *mut ManifoldManifold;

    pub fn manifold_cylinder(
        height: f64,
        radius_low: f64,
        radius_high: f64,
        circular_segments: c_int,
        center: c_int,
    ) -> *mut ManifoldManifold;

    pub fn manifold_sphere(radius: f64, circular_segments: c_int) -> *mut ManifoldManifold;

    /// Extrude a 2D polygon set into a 3D manifold along the Z axis.
    ///
    /// The resulting solid spans `z in [0, height]`.
    pub fn manifold_extrude(
        cs: *const ManifoldPolygons,
        height: f64,
        slices: c_int,
        twist_degrees: f64,
        scale_x: f64,
        scale_y: f64,
    ) -> *mut ManifoldManifold;

    /// Revolve a 2D polygon set around the Y axis to create a 3D manifold.
    pub fn manifold_revolve(
        cs: *const ManifoldPolygons,
        circular_segments: c_int,
        revolve_degrees: f64,
    ) -> *mut ManifoldManifold;

    /// Compose multiple manifolds into a single compound manifold.
    pub fn manifold_compose(ms: *const ManifoldManifoldVec) -> *mut ManifoldManifold;

    /// Decompose a manifold into its connected components.
    pub fn manifold_decompose(m: *const ManifoldManifold) -> *mut ManifoldManifoldVec;

    /// Mark this manifold as the original, assigning it a unique ID for
    /// tracking through boolean operations.
    pub fn manifold_as_original(m: *const ManifoldManifold, id: c_int) -> *mut ManifoldManifold;

    // ── Manifold info / queries ─────────────────────────────────────────

    pub fn manifold_is_empty(m: *const ManifoldManifold) -> c_int;
    pub fn manifold_status(m: *const ManifoldManifold) -> ManifoldError;
    /// Returns a copy of `m` with `ctx` attached. The attachment is
    /// consumed by the next eager op (`manifold_status`, `manifold_refine*`).
    /// Deferred ops (booleans, transforms, batch) ignore any attached
    /// context and produce results with no attached context.
    pub fn manifold_with_context(
        m: *const ManifoldManifold,
        ctx: *const ManifoldExecutionContext,
    ) -> *mut ManifoldManifold;
    pub fn manifold_num_vert(m: *const ManifoldManifold) -> usize;
    pub fn manifold_num_edge(m: *const ManifoldManifold) -> usize;
    pub fn manifold_num_tri(m: *const ManifoldManifold) -> usize;
    pub fn manifold_num_prop(m: *const ManifoldManifold) -> usize;
    pub fn manifold_volume(m: *const ManifoldManifold) -> f64;
    pub fn manifold_surface_area(m: *const ManifoldManifold) -> f64;
    pub fn manifold_epsilon(m: *const ManifoldManifold) -> f64;
    pub fn manifold_num_prop_vert(m: *const ManifoldManifold) -> usize;
    pub fn manifold_genus(m: *const ManifoldManifold) -> c_int;
    pub fn manifold_original_id(m: *const ManifoldManifold) -> c_int;

    /// Winding number of the point `(x, y, z)` with respect to `m`: the
    /// number of times the surface wraps around it. 0 is outside, 1 inside a
    /// single solid.
    pub fn manifold_winding_number(m: *const ManifoldManifold, x: f64, y: f64, z: f64) -> c_int;

    /// Get the number of circular segments for a given radius.
    pub fn manifold_get_circular_segments(radius: f64) -> c_int;

    /// Reserve a block of unique IDs for use with `manifold_as_original`.
    pub fn manifold_reserve_ids(n: u32) -> u32;

    /// Set custom properties on each vertex of a manifold.
    ///
    /// The callback receives: `(new_prop, position, old_prop, ctx)`.
    pub fn manifold_set_properties(
        m: *const ManifoldManifold,
        num_prop: c_int,
        fun: Option<
            unsafe extern "C" fn(*mut f64, ManifoldVec3, *const f64, *mut std::ffi::c_void),
        >,
        ctx: *mut std::ffi::c_void,
    ) -> *mut ManifoldManifold;

    /// Calculate Gaussian and mean curvature and store at the given property indices.
    pub fn manifold_calculate_curvature(
        m: *const ManifoldManifold,
        gaussian_idx: c_int,
        mean_idx: c_int,
    ) -> *mut ManifoldManifold;

    /// Compute the minimum gap between two manifolds within `search_length`.
    pub fn manifold_min_gap(
        m: *const ManifoldManifold,
        other: *const ManifoldManifold,
        search_length: f64,
    ) -> f64;

    /// Calculate vertex normals and store at the given property index.
    pub fn manifold_calculate_normals(
        m: *const ManifoldManifold,
        normal_idx: c_int,
        min_sharp_angle: f64,
    ) -> *mut ManifoldManifold;

    // ── Ray casting ─────────────────────────────────────────────────────

    /// Cast a ray from `origin` to `end` against a manifold, returning all hits.
    pub fn manifold_ray_cast(
        m: *const ManifoldManifold,
        origin_x: f64,
        origin_y: f64,
        origin_z: f64,
        end_x: f64,
        end_y: f64,
        end_z: f64,
    ) -> *mut ManifoldRayHitVec;

    /// Number of hits in a ray hit vector.
    pub fn manifold_ray_hit_vec_length(v: *const ManifoldRayHitVec) -> usize;

    /// Get a hit by index from a ray hit vector.
    pub fn manifold_ray_hit_vec_get(v: *const ManifoldRayHitVec, idx: usize) -> ManifoldRayHit;

    // ── Execution context (cancel + progress) ───────────────────────────

    /// Create a new `ExecutionContext`. Free it with
    /// [`manifold_delete_execution_context`]. Attach it to a manifold via
    /// [`manifold_with_context`]. Subsequent eager operations on the result
    /// observe the context.
    pub fn manifold_execution_context() -> *mut ManifoldExecutionContext;

    /// Request cancellation of any boolean evaluation observing this context.
    /// Sticky: once cancelled, the context stays cancelled.
    pub fn manifold_execution_context_cancel(ctx: *mut ManifoldExecutionContext);

    /// Returns nonzero if the context has been cancelled.
    pub fn manifold_execution_context_cancelled(ctx: *const ManifoldExecutionContext) -> c_int;

    /// Progress in [0.0, 1.0] for an in-flight evaluation observing this context.
    pub fn manifold_execution_context_progress(ctx: *const ManifoldExecutionContext) -> f64;

    // -- Execution-context static factories ------------------------------
    //
    // ctx-aware variants of `manifold_level_set` / `manifold_of_meshgl` /
    // `manifold_smooth`. These have no source manifold to attach a context
    // to via `manifold_with_context`, so they run on the
    // `ExecutionContext` directly to report progress / observe
    // cancellation. Added upstream in v3.5.1.

    /// ctx-aware [`manifold_level_set`]. `sdf_context` is the SDF callback's user-data.
    pub fn manifold_execution_context_level_set(
        ec: *mut ManifoldExecutionContext,
        sdf: ManifoldSdf,
        bounds: *const ManifoldBox,
        edge_length: f64,
        level: f64,
        tolerance: f64,
        sdf_context: *mut c_void,
    ) -> *mut ManifoldManifold;

    /// ctx-aware [`manifold_level_set_seq`] (sequential execution).
    pub fn manifold_execution_context_level_set_seq(
        ec: *mut ManifoldExecutionContext,
        sdf: ManifoldSdf,
        bounds: *const ManifoldBox,
        edge_length: f64,
        level: f64,
        tolerance: f64,
        sdf_context: *mut c_void,
    ) -> *mut ManifoldManifold;

    /// ctx-aware [`manifold_of_meshgl`].
    pub fn manifold_execution_context_of_meshgl(
        ec: *mut ManifoldExecutionContext,
        mesh: *const ManifoldMeshGL,
    ) -> *mut ManifoldManifold;

    /// ctx-aware [`manifold_of_meshgl64`].
    pub fn manifold_execution_context_of_meshgl64(
        ec: *mut ManifoldExecutionContext,
        mesh: *const ManifoldMeshGL64,
    ) -> *mut ManifoldManifold;

    // ── Bounding box ────────────────────────────────────────────────────

    pub fn manifold_bounding_box(m: *const ManifoldManifold) -> *mut ManifoldBox;

    pub fn manifold_box(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> *mut ManifoldBox;

    pub fn manifold_box_min(b: *const ManifoldBox) -> ManifoldVec3;
    pub fn manifold_box_max(b: *const ManifoldBox) -> ManifoldVec3;
    pub fn manifold_box_dimensions(b: *const ManifoldBox) -> ManifoldVec3;
    pub fn manifold_box_center(b: *const ManifoldBox) -> ManifoldVec3;
    pub fn manifold_box_scale(b: *const ManifoldBox) -> f64;

    pub fn manifold_box_contains_pt(b: *const ManifoldBox, x: f64, y: f64, z: f64) -> c_int;

    pub fn manifold_box_contains_box(a: *const ManifoldBox, b: *const ManifoldBox) -> c_int;

    pub fn manifold_box_include_pt(b: *mut ManifoldBox, x: f64, y: f64, z: f64);

    pub fn manifold_box_union(a: *const ManifoldBox, b: *const ManifoldBox) -> *mut ManifoldBox;

    pub fn manifold_box_transform(
        b: *const ManifoldBox,
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
        x3: f64,
        y3: f64,
        z3: f64,
        x4: f64,
        y4: f64,
        z4: f64,
    ) -> *mut ManifoldBox;

    pub fn manifold_box_translate(
        b: *const ManifoldBox,
        x: f64,
        y: f64,
        z: f64,
    ) -> *mut ManifoldBox;

    pub fn manifold_box_mul(b: *const ManifoldBox, x: f64, y: f64, z: f64) -> *mut ManifoldBox;

    pub fn manifold_box_does_overlap_pt(b: *const ManifoldBox, x: f64, y: f64, z: f64) -> c_int;

    pub fn manifold_box_does_overlap_box(a: *const ManifoldBox, b: *const ManifoldBox) -> c_int;

    pub fn manifold_box_is_finite(b: *const ManifoldBox) -> c_int;

    // ── CrossSection constructors ───────────────────────────────────────

    pub fn manifold_cross_section_empty() -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_copy(
        cs: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from one contour, filled with the Positive rule (inside
    /// where the winding number is greater than zero).
    pub fn manifold_cross_section_of_simple_polygon(
        p: *const ManifoldSimplePolygon,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from contours, filled with the Positive rule.
    pub fn manifold_cross_section_of_polygons(
        p: *const ManifoldPolygons,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from one contour, filled with the even-odd rule.
    pub fn manifold_cross_section_even_odd_simple_polygon(
        p: *const ManifoldSimplePolygon,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from contours, filled with the even-odd rule.
    pub fn manifold_cross_section_even_odd_polygons(
        p: *const ManifoldPolygons,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from one contour, filled with the non-zero rule.
    pub fn manifold_cross_section_non_zero_simple_polygon(
        p: *const ManifoldSimplePolygon,
    ) -> *mut ManifoldCrossSection;

    /// Cross-section from contours, filled with the non-zero rule.
    pub fn manifold_cross_section_non_zero_polygons(
        p: *const ManifoldPolygons,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_square(
        x: f64,
        y: f64,
        center: c_int,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_circle(
        radius: f64,
        circular_segments: c_int,
    ) -> *mut ManifoldCrossSection;

    // ── CrossSection decompose ──────────────────────────────────────────

    pub fn manifold_cross_section_decompose(
        cs: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSectionVec;

    // ── CrossSection vectors ────────────────────────────────────────────

    pub fn manifold_cross_section_empty_vec() -> *mut ManifoldCrossSectionVec;

    pub fn manifold_cross_section_vec(sz: usize) -> *mut ManifoldCrossSectionVec;

    pub fn manifold_cross_section_vec_reserve(csv: *mut ManifoldCrossSectionVec, sz: usize);

    pub fn manifold_cross_section_vec_length(csv: *const ManifoldCrossSectionVec) -> usize;

    pub fn manifold_cross_section_vec_get(
        csv: *const ManifoldCrossSectionVec,
        idx: usize,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_vec_set(
        csv: *mut ManifoldCrossSectionVec,
        idx: usize,
        cs: *const ManifoldCrossSection,
    );

    pub fn manifold_cross_section_vec_push_back(
        csv: *mut ManifoldCrossSectionVec,
        cs: *const ManifoldCrossSection,
    );

    // ── CrossSection booleans ───────────────────────────────────────────

    /// Generic boolean operation between two cross-sections.
    pub fn manifold_cross_section_boolean(
        a: *const ManifoldCrossSection,
        b: *const ManifoldCrossSection,
        op: ManifoldOpType,
    ) -> *mut ManifoldCrossSection;

    /// Batch boolean: apply `op` across all cross-sections in the vector.
    pub fn manifold_cross_section_batch_boolean(
        csv: *const ManifoldCrossSectionVec,
        op: ManifoldOpType,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_union(
        a: *const ManifoldCrossSection,
        b: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_difference(
        a: *const ManifoldCrossSection,
        b: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_intersection(
        a: *const ManifoldCrossSection,
        b: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSection;

    // ── CrossSection convex hulls ───────────────────────────────────────

    pub fn manifold_cross_section_hull(
        cs: *const ManifoldCrossSection,
    ) -> *mut ManifoldCrossSection;

    /// Compute the convex hull of a set of cross-sections.
    pub fn manifold_cross_section_batch_hull(
        css: *const ManifoldCrossSectionVec,
    ) -> *mut ManifoldCrossSection;

    /// Compute the convex hull of a simple polygon.
    pub fn manifold_cross_section_hull_simple_polygon(
        ps: *const ManifoldSimplePolygon,
    ) -> *mut ManifoldCrossSection;

    /// Compute the convex hull of a polygon set.
    pub fn manifold_cross_section_hull_polygons(
        ps: *const ManifoldPolygons,
    ) -> *mut ManifoldCrossSection;

    // ── CrossSection transforms ─────────────────────────────────────────

    pub fn manifold_cross_section_translate(
        cs: *const ManifoldCrossSection,
        x: f64,
        y: f64,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_rotate(
        cs: *const ManifoldCrossSection,
        deg: f64,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_scale(
        cs: *const ManifoldCrossSection,
        x: f64,
        y: f64,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_mirror(
        cs: *const ManifoldCrossSection,
        ax_x: f64,
        ax_y: f64,
    ) -> *mut ManifoldCrossSection;

    pub fn manifold_cross_section_transform(
        cs: *const ManifoldCrossSection,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    ) -> *mut ManifoldCrossSection;

    /// Warp a cross-section by applying a function to each vertex.
    pub fn manifold_cross_section_warp_context(
        cs: *const ManifoldCrossSection,
        fun: Option<unsafe extern "C" fn(f64, f64, *mut std::ffi::c_void) -> ManifoldVec2>,
        ctx: *mut std::ffi::c_void,
    ) -> *mut ManifoldCrossSection;

    /// Simplify the contours of a cross-section to within `tolerance`.
    pub fn manifold_cross_section_simplify(
        cs: *const ManifoldCrossSection,
        tolerance: f64,
    ) -> *mut ManifoldCrossSection;

    // ── CrossSection offset ─────────────────────────────────────────────

    /// Offset (inflate/deflate) a cross-section by `delta`.
    ///
    /// `jt` selects the join type.
    /// `miter_limit` is used when `jt` is Miter.
    /// `circular_segments` controls round join resolution.
    pub fn manifold_cross_section_offset(
        cs: *const ManifoldCrossSection,
        delta: f64,
        jt: ManifoldJoinType,
        miter_limit: f64,
        circular_segments: c_int,
    ) -> *mut ManifoldCrossSection;

    // ── CrossSection queries ────────────────────────────────────────────

    pub fn manifold_cross_section_area(cs: *const ManifoldCrossSection) -> f64;
    pub fn manifold_cross_section_num_vert(cs: *const ManifoldCrossSection) -> usize;
    pub fn manifold_cross_section_num_contour(cs: *const ManifoldCrossSection) -> usize;
    pub fn manifold_cross_section_is_empty(cs: *const ManifoldCrossSection) -> c_int;

    pub fn manifold_cross_section_bounds(cs: *const ManifoldCrossSection) -> *mut ManifoldRect;

    // ── CrossSection extraction ─────────────────────────────────────────

    pub fn manifold_cross_section_to_polygons(
        cs: *const ManifoldCrossSection,
    ) -> *mut ManifoldPolygons;

    // ── Rectangle ───────────────────────────────────────────────────────

    pub fn manifold_rect(x1: f64, y1: f64, x2: f64, y2: f64) -> *mut ManifoldRect;

    pub fn manifold_rect_min(r: *const ManifoldRect) -> ManifoldVec2;
    pub fn manifold_rect_max(r: *const ManifoldRect) -> ManifoldVec2;
    pub fn manifold_rect_dimensions(r: *const ManifoldRect) -> ManifoldVec2;
    pub fn manifold_rect_center(r: *const ManifoldRect) -> ManifoldVec2;
    pub fn manifold_rect_scale(r: *const ManifoldRect) -> f64;

    pub fn manifold_rect_contains_pt(r: *const ManifoldRect, x: f64, y: f64) -> c_int;

    pub fn manifold_rect_contains_rect(a: *const ManifoldRect, b: *const ManifoldRect) -> c_int;

    pub fn manifold_rect_include_pt(r: *mut ManifoldRect, x: f64, y: f64);

    pub fn manifold_rect_union(a: *const ManifoldRect, b: *const ManifoldRect)
    -> *mut ManifoldRect;

    pub fn manifold_rect_transform(
        r: *const ManifoldRect,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    ) -> *mut ManifoldRect;

    pub fn manifold_rect_translate(r: *const ManifoldRect, x: f64, y: f64) -> *mut ManifoldRect;

    pub fn manifold_rect_mul(r: *const ManifoldRect, x: f64, y: f64) -> *mut ManifoldRect;

    pub fn manifold_rect_does_overlap_rect(a: *const ManifoldRect, r: *const ManifoldRect)
    -> c_int;

    pub fn manifold_rect_is_empty(r: *const ManifoldRect) -> c_int;
    pub fn manifold_rect_is_finite(r: *const ManifoldRect) -> c_int;

    // ── Triangulation ───────────────────────────────────────────────────

    /// Triangulate a polygon set, returning triangle indices.
    pub fn manifold_triangulate(
        ps: *const ManifoldPolygons,
        epsilon: f64,
    ) -> *mut ManifoldTriangulation;

    pub fn manifold_triangulation_num_tri(m: *const ManifoldTriangulation) -> usize;

    /// Triangle vertex indices, 3 consecutive values per triangle. The buffer
    /// is borrowed from `m` and is only valid for the lifetime of `m`.
    pub fn manifold_triangulation_tri_verts(m: *const ManifoldTriangulation) -> *const c_int;

    // ── Static quality globals ──────────────────────────────────────────

    pub fn manifold_set_min_circular_angle(degrees: f64);
    pub fn manifold_set_min_circular_edge_length(length: f64);
    pub fn manifold_set_circular_segments(number: c_int);
    pub fn manifold_reset_to_circular_defaults();

    /// Set the global relative precision used to decide when geometry is
    /// coplanar or collinear. Replaces the per-manifold tolerance upstream
    /// removed.
    pub fn manifold_set_relative_precision(precision: f64);
    /// The global relative precision.
    pub fn manifold_get_relative_precision() -> f64;

    // ── Version ─────────────────────────────────────────────────────────

    /// Write the linked manifold's version as `[major, minor, patch]`.
    /// `version` must point at 3 writable ints.
    pub fn manifold_version(version: *mut c_int);

    // ── OBJ I/O ─────────────────────────────────────────────────────────
    //
    // These four functions live in manifold's iostream-using path, gated by
    // `MANIFOLD_NO_IOSTREAM` upstream-side. On wasm32-unknown-unknown we
    // build with that macro defined (the C++ symbols don't exist), so the
    // FFI declarations are also elided. Consumers reaching for OBJ I/O on
    // that target should use the binary `MeshGL64` API instead.

    /// Import a manifold from a Wavefront obj file.
    ///
    /// The `obj_file` parameter is the content of the obj file (not the filename),
    /// and should be null-terminated.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn manifold_read_obj(obj_file: *const c_char) -> *mut ManifoldManifold;

    /// Import a MeshGL64 from a Wavefront obj file.
    ///
    /// The `obj_file` parameter is the content of the obj file (not the filename),
    /// and should be null-terminated.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn manifold_meshgl64_read_obj(obj_file: *const c_char) -> *mut ManifoldMeshGL64;

    /// Export a manifold to a Wavefront obj string via callback.
    ///
    /// The callback receives a temporary null-terminated string buffer containing
    /// the obj content, plus a user-provided context pointer. The buffer is freed
    /// automatically after the callback returns.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn manifold_write_obj(
        manifold: *const ManifoldManifold,
        callback: Option<unsafe extern "C" fn(*mut std::ffi::c_char, *mut std::ffi::c_void)>,
        args: *mut std::ffi::c_void,
    );

    /// Export a MeshGL64 to a Wavefront obj string via callback.
    ///
    /// The callback receives a temporary null-terminated string buffer containing
    /// the obj content, plus a user-provided context pointer. The buffer is freed
    /// automatically after the callback returns.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn manifold_meshgl64_write_obj(
        mesh: *const ManifoldMeshGL64,
        callback: Option<unsafe extern "C" fn(*mut std::ffi::c_char, *mut std::ffi::c_void)>,
        args: *mut std::ffi::c_void,
    );
}
