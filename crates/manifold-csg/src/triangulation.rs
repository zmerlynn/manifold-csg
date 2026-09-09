//! Constrained Delaunay triangulation of 2D polygons.

use manifold_csg_sys::*;

use crate::cross_section::PolygonsFfi;

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

    // Everything C-allocated is released before the validation below runs, so
    // the assert cannot leak. `polys` frees itself at the end of this scope.
    let indices = {
        let polys = PolygonsFfi::new(polygons);

        // SAFETY: manifold_alloc_triangulation returns a valid handle.
        let tri_ptr = unsafe { manifold_alloc_triangulation() };
        // SAFETY: tri_ptr is valid, polys.ptr() is valid from construction.
        unsafe { manifold_triangulate(tri_ptr, polys.ptr(), epsilon) };

        // SAFETY: tri_ptr is valid. Read-only size query.
        let n_tris = unsafe { manifold_triangulation_num_tri(tri_ptr) };

        let indices = if n_tris > 0 {
            let mut buf = vec![0i32; n_tris * 3];
            // SAFETY: buf has capacity for n_tris * 3, tri_ptr is valid.
            unsafe { manifold_triangulation_tri_verts(buf.as_mut_ptr(), tri_ptr) };
            Some(buf)
        } else {
            None
        };

        // SAFETY: tri_ptr is valid and no longer needed.
        unsafe { manifold_delete_triangulation(tri_ptr) };

        indices
    }?;

    Some(
        indices
            .chunks(3)
            .map(|c| {
                // The C API returns non-negative indices into the vertex array.
                // Validate to catch any upstream bugs rather than silently wrapping.
                assert!(
                    c[0] >= 0 && c[1] >= 0 && c[2] >= 0,
                    "negative triangle index from C API"
                );
                [c[0] as u32, c[1] as u32, c[2] as u32]
            })
            .collect(),
    )
}
