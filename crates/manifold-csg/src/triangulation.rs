//! Constrained Delaunay triangulation of 2D polygons.

use crate::owned;
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

        // SAFETY: polys.ptr() is valid from construction, and the call returns a new handle.
        let tri_ptr = owned(unsafe { manifold_triangulate(polys.ptr(), epsilon) });

        // SAFETY: tri_ptr is valid. Read-only size query.
        let n_tris = unsafe { manifold_triangulation_num_tri(tri_ptr) };

        let indices = if n_tris > 0 {
            // SAFETY: tri_ptr is valid. The result points at its n_tris * 3
            // indices and stays valid until tri_ptr is deleted below, after
            // the copy.
            let verts = unsafe { manifold_triangulation_tri_verts(tri_ptr) };
            // SAFETY: as above, verts addresses n_tris * 3 initialized ints.
            Some(unsafe { std::slice::from_raw_parts(verts, n_tris * 3) }.to_vec())
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
