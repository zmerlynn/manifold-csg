//! Constrained Delaunay triangulation of 2D polygons.

use manifold_csg_sys::*;

use crate::cross_section::build_polygons_ffi;

/// Triangulate a set of 2D polygon rings using manifold3d's constrained
/// Delaunay triangulator.
///
/// Each ring in `polygons` is a list of `[x, y]` points forming a closed
/// loop. The first ring is the outer boundary; subsequent rings are holes.
///
/// # Arguments
///
/// * `polygons` - polygon rings; the first ring is the outer boundary,
///   subsequent rings are holes
/// * `epsilon` - tolerance for degenerate triangle detection. Triangles with
///   area smaller than this are considered degenerate. A typical value is `1e-6`.
///
/// Returns triangle indices into the flattened vertex list (all rings
/// concatenated in order). Returns `None` if the input is degenerate.
pub fn triangulate_polygons(polygons: &[Vec<[f64; 2]>], epsilon: f64) -> Option<Vec<[u32; 3]>> {
    if polygons.is_empty() {
        return None;
    }

    let (polys_ptr, simple_ptrs) = build_polygons_ffi(polygons);

    // SAFETY: manifold_alloc_triangulation returns a valid handle.
    let tri_ptr = unsafe { manifold_alloc_triangulation() };
    // SAFETY: tri_ptr is valid, polys_ptr is valid from construction.
    unsafe { manifold_triangulate(tri_ptr, polys_ptr, epsilon) };

    // SAFETY: tri_ptr is valid. Read-only size query.
    let n_tris = unsafe { manifold_triangulation_num_tri(tri_ptr) };

    let result = if n_tris > 0 {
        let mut indices = vec![0i32; n_tris * 3];
        // SAFETY: indices has capacity for n_tris * 3, tri_ptr is valid.
        unsafe { manifold_triangulation_tri_verts(indices.as_mut_ptr(), tri_ptr) };

        let triangles: Vec<[u32; 3]> = indices
            .chunks(3)
            .map(|c| {
                // The C API returns non-negative indices into the vertex array.
                // Validate to catch any upstream bugs rather than silently wrapping.
                debug_assert!(c[0] >= 0 && c[1] >= 0 && c[2] >= 0, "negative triangle index from C API");
                [c[0] as u32, c[1] as u32, c[2] as u32]
            })
            .collect();
        Some(triangles)
    } else {
        None
    };

    // Clean up all C-side allocations.
    // SAFETY: tri_ptr is valid and no longer needed.
    unsafe { manifold_delete_triangulation(tri_ptr) };
    // SAFETY: polys_ptr is valid and no longer needed.
    unsafe { manifold_delete_polygons(polys_ptr) };
    for sp in simple_ptrs {
        // SAFETY: sp is valid and no longer needed.
        unsafe { manifold_delete_simple_polygon(sp) };
    }

    result
}
