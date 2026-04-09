# API Coverage

This document maps every function in the manifold3d v3.4.1 C API to its status
in `manifold-csg-sys` (raw FFI) and `manifold-csg` (safe wrapper).

All 256 C API functions are bound in `manifold-csg-sys`. The safe crate wraps the
most commonly needed operations; infrastructure functions (allocation, vectors,
polygon helpers) are used internally and don't need direct safe wrappers.

**Legend:**
- **Wrapped** — has a safe public method
- **Internal** — used internally by safe wrappers (allocation, Drop, helpers)
- **Not wrapped** — bound in sys but no safe wrapper yet

---

## Manifold — Construction

| C API function | Safe wrapper |
|---|---|
| `manifold_cube` | [`Manifold::cube`](crates/manifold-csg/src/manifold.rs#L276) |
| `manifold_cylinder` | [`Manifold::cylinder`](crates/manifold-csg/src/manifold.rs#L289) |
| `manifold_sphere` | [`Manifold::sphere`](crates/manifold-csg/src/manifold.rs#L305) |
| `manifold_tetrahedron` | [`Manifold::tetrahedron`](crates/manifold-csg/src/manifold.rs#L786) |
| `manifold_empty` | [`Manifold::empty`](crates/manifold-csg/src/manifold.rs#L315) |
| `manifold_of_meshgl` | [`Manifold::from_mesh_f32`](crates/manifold-csg/src/manifold.rs#L147) |
| `manifold_of_meshgl64` | [`Manifold::from_mesh_f64`](crates/manifold-csg/src/manifold.rs#L97) |
| `manifold_extrude` | [`Manifold::extrude`](crates/manifold-csg/src/manifold.rs#L467), [`extrude_with_options`](crates/manifold-csg/src/manifold.rs#L812) |
| `manifold_revolve` | [`Manifold::revolve`](crates/manifold-csg/src/manifold.rs#L796) |
| `manifold_compose` | [`Manifold::compose`](crates/manifold-csg/src/manifold.rs#L835) |
| `manifold_copy` | [`Clone` impl](crates/manifold-csg/src/manifold.rs#L56) |
| `manifold_level_set` | [`Manifold::from_sdf`](crates/manifold-csg/src/manifold.rs#L1114) |
| `manifold_read_obj` | [`Manifold::from_obj`](crates/manifold-csg/src/manifold.rs#L1063) |
| `manifold_smooth` | Not wrapped |
| `manifold_smooth64` | Not wrapped |
| `manifold_level_set_seq` | [`Manifold::from_sdf_seq`](crates/manifold-csg/src/manifold.rs) |

## Manifold — Boolean Operations

| C API function | Safe wrapper |
|---|---|
| `manifold_union` | [`Manifold::union`](crates/manifold-csg/src/manifold.rs#L607), `&a + &b` |
| `manifold_difference` | [`Manifold::difference`](crates/manifold-csg/src/manifold.rs#L592), `&a - &b` |
| `manifold_intersection` | [`Manifold::intersection`](crates/manifold-csg/src/manifold.rs#L622), `&a ^ &b` |
| `manifold_boolean` | Not wrapped |
| `manifold_batch_boolean` | [`Manifold::batch_union`](crates/manifold-csg/src/manifold.rs#L489), [`batch_difference`](crates/manifold-csg/src/manifold.rs#L495) |
| `manifold_batch_hull` | [`Manifold::batch_hull`](crates/manifold-csg/src/manifold.rs#L673) |
| `manifold_split` | [`Manifold::split`](crates/manifold-csg/src/manifold.rs#L859) |
| `manifold_split_by_plane` | [`Manifold::split_by_plane`](crates/manifold-csg/src/manifold.rs#L389) |

## Manifold — Transforms

| C API function | Safe wrapper |
|---|---|
| `manifold_translate` | [`Manifold::translate`](crates/manifold-csg/src/manifold.rs#L327) |
| `manifold_rotate` | [`Manifold::rotate`](crates/manifold-csg/src/manifold.rs#L348) |
| `manifold_scale` | [`Manifold::scale`](crates/manifold-csg/src/manifold.rs#L337) |
| `manifold_transform` | [`Manifold::transform`](crates/manifold-csg/src/manifold.rs#L366) |
| `manifold_mirror` | [`Manifold::mirror`](crates/manifold-csg/src/manifold.rs#L716) |
| `manifold_warp` | [`Manifold::warp`](crates/manifold-csg/src/manifold.rs#L974) |
| `manifold_refine` | [`Manifold::refine`](crates/manifold-csg/src/manifold.rs#L728) |
| `manifold_refine_to_length` | [`Manifold::refine_to_length`](crates/manifold-csg/src/manifold.rs#L738) |
| `manifold_refine_to_tolerance` | [`Manifold::refine_to_tolerance`](crates/manifold-csg/src/manifold.rs#L748) |
| `manifold_smooth_by_normals` | [`Manifold::smooth_by_normals`](crates/manifold-csg/src/manifold.rs#L761) |
| `manifold_smooth_out` | [`Manifold::smooth_out`](crates/manifold-csg/src/manifold.rs#L774) |
| `manifold_calculate_normals` | [`Manifold::calculate_normals`](crates/manifold-csg/src/manifold.rs#L950) |
| `manifold_calculate_curvature` | [`Manifold::calculate_curvature`](crates/manifold-csg/src/manifold.rs#L960) |
| `manifold_set_properties` | [`Manifold::set_properties`](crates/manifold-csg/src/manifold.rs#L1011) |
| `manifold_trim_by_plane` | [`Manifold::trim_by_plane`](crates/manifold-csg/src/manifold.rs#L409) |

## Manifold — Queries

| C API function | Safe wrapper |
|---|---|
| `manifold_is_empty` | [`Manifold::is_empty`](crates/manifold-csg/src/manifold.rs#L534) |
| `manifold_volume` | [`Manifold::volume`](crates/manifold-csg/src/manifold.rs#L541) |
| `manifold_surface_area` | [`Manifold::surface_area`](crates/manifold-csg/src/manifold.rs#L548) |
| `manifold_num_vert` | [`Manifold::num_vert`](crates/manifold-csg/src/manifold.rs#L555) |
| `manifold_num_tri` | [`Manifold::num_tri`](crates/manifold-csg/src/manifold.rs#L562) |
| `manifold_num_edge` | [`Manifold::num_edge`](crates/manifold-csg/src/manifold.rs#L906) |
| `manifold_num_prop` | [`Manifold::num_prop`](crates/manifold-csg/src/manifold.rs#L912) |
| `manifold_epsilon` | [`Manifold::epsilon`](crates/manifold-csg/src/manifold.rs#L920) |
| `manifold_genus` | [`Manifold::genus`](crates/manifold-csg/src/manifold.rs#L927) |
| `manifold_bounding_box` | [`Manifold::bounding_box`](crates/manifold-csg/src/manifold.rs#L571) |
| `manifold_original_id` | [`Manifold::original_id`](crates/manifold-csg/src/manifold.rs#L934) |
| `manifold_min_gap` | [`Manifold::min_gap`](crates/manifold-csg/src/manifold.rs#L941) |
| `manifold_status` | Internal |
| `manifold_as_original` | [`Manifold::as_original`](crates/manifold-csg/src/manifold.rs) |
| `manifold_manifold_size` | Not wrapped |
| `manifold_manifold_pair_size` | Not wrapped |

## Manifold — Hull, Decomposition & Mesh Extraction

| C API function | Safe wrapper |
|---|---|
| `manifold_hull` | [`Manifold::hull`](crates/manifold-csg/src/manifold.rs#L663) |
| `manifold_hull_pts` | [`Manifold::hull_pts`](crates/manifold-csg/src/manifold.rs#L700) |
| `manifold_decompose` | [`Manifold::decompose`](crates/manifold-csg/src/manifold.rs#L637) |
| `manifold_slice` | [`Manifold::slice_at_z`](crates/manifold-csg/src/manifold.rs#L428), [`slice_to_cross_section`](crates/manifold-csg/src/manifold.rs#L445) |
| `manifold_project` | [`Manifold::project`](crates/manifold-csg/src/manifold.rs#L891) |
| `manifold_minkowski_sum` | [`Manifold::minkowski_sum`](crates/manifold-csg/src/manifold.rs#L871) |
| `manifold_minkowski_difference` | [`Manifold::minkowski_difference`](crates/manifold-csg/src/manifold.rs#L881) |
| `manifold_get_meshgl` | [`Manifold::to_mesh_f32`](crates/manifold-csg/src/manifold.rs#L235) |
| `manifold_get_meshgl64` | [`Manifold::to_mesh_f64`](crates/manifold-csg/src/manifold.rs#L197) |
| `manifold_write_obj` | [`Manifold::to_obj`](crates/manifold-csg/src/manifold.rs#L1082) |
| `manifold_get_meshgl_w_normals` | [`Manifold::to_mesh_f32_with_normals`](crates/manifold-csg/src/manifold.rs) |
| `manifold_get_meshgl64_w_normals` | [`Manifold::to_mesh_f64_with_normals`](crates/manifold-csg/src/manifold.rs) |

## CrossSection — Construction & Booleans

| C API function | Safe wrapper |
|---|---|
| `manifold_cross_section_empty` | [`CrossSection::empty`](crates/manifold-csg/src/cross_section.rs#L104) |
| `manifold_cross_section_square` | [`CrossSection::square`](crates/manifold-csg/src/cross_section.rs#L114) |
| `manifold_cross_section_circle` | [`CrossSection::circle`](crates/manifold-csg/src/cross_section.rs#L124) |
| `manifold_cross_section_of_polygons` | [`CrossSection::from_polygons`](crates/manifold-csg/src/cross_section.rs#L137) |
| `manifold_cross_section_copy` | [`Clone` impl](crates/manifold-csg/src/cross_section.rs#L79) |
| `manifold_cross_section_union` | [`CrossSection::union`](crates/manifold-csg/src/cross_section.rs#L166), `&a + &b` |
| `manifold_cross_section_difference` | [`CrossSection::difference`](crates/manifold-csg/src/cross_section.rs#L176), `&a - &b` |
| `manifold_cross_section_intersection` | [`CrossSection::intersection`](crates/manifold-csg/src/cross_section.rs#L186), `&a ^ &b` |
| `manifold_cross_section_boolean` | Not wrapped |
| `manifold_cross_section_batch_boolean` | [`CrossSection::batch_boolean`](crates/manifold-csg/src/cross_section.rs#L349), [`batch_union`](crates/manifold-csg/src/cross_section.rs#L376) |
| `manifold_cross_section_batch_hull` | [`CrossSection::batch_hull`](crates/manifold-csg/src/cross_section.rs#L382) |
| `manifold_cross_section_hull` | [`CrossSection::hull`](crates/manifold-csg/src/cross_section.rs#L234) |
| `manifold_cross_section_compose` | [`CrossSection::compose`](crates/manifold-csg/src/cross_section.rs#L409) |
| `manifold_cross_section_of_simple_polygon` | Not wrapped |
| `manifold_cross_section_hull_polygons` | Not wrapped |
| `manifold_cross_section_hull_simple_polygon` | Not wrapped |
| `manifold_cross_section_decompose` | [`CrossSection::decompose`](crates/manifold-csg/src/cross_section.rs) |

## CrossSection — Transforms & Queries

| C API function | Safe wrapper |
|---|---|
| `manifold_cross_section_translate` | [`CrossSection::translate`](crates/manifold-csg/src/cross_section.rs#L246) |
| `manifold_cross_section_rotate` | [`CrossSection::rotate`](crates/manifold-csg/src/cross_section.rs#L256) |
| `manifold_cross_section_scale` | [`CrossSection::scale`](crates/manifold-csg/src/cross_section.rs#L266) |
| `manifold_cross_section_mirror` | [`CrossSection::mirror`](crates/manifold-csg/src/cross_section.rs#L276) |
| `manifold_cross_section_offset` | [`CrossSection::offset`](crates/manifold-csg/src/cross_section.rs#L207) |
| `manifold_cross_section_simplify` | [`CrossSection::simplify`](crates/manifold-csg/src/cross_section.rs#L339) |
| `manifold_cross_section_warp_context` | [`CrossSection::warp`](crates/manifold-csg/src/cross_section.rs#L447) |
| `manifold_cross_section_area` | [`CrossSection::area`](crates/manifold-csg/src/cross_section.rs#L288) |
| `manifold_cross_section_num_vert` | [`CrossSection::num_vert`](crates/manifold-csg/src/cross_section.rs#L295) |
| `manifold_cross_section_num_contour` | [`CrossSection::num_contour`](crates/manifold-csg/src/cross_section.rs#L302) |
| `manifold_cross_section_is_empty` | [`CrossSection::is_empty`](crates/manifold-csg/src/cross_section.rs#L309) |
| `manifold_cross_section_bounds` | [`CrossSection::bounds`](crates/manifold-csg/src/cross_section.rs#L316) |
| `manifold_cross_section_to_polygons` | [`CrossSection::to_polygons`](crates/manifold-csg/src/cross_section.rs#L479) |
| `manifold_cross_section_transform` | [`CrossSection::transform`](crates/manifold-csg/src/cross_section.rs) |
| `manifold_cross_section_size` | Not wrapped |

## MeshGL (f32) & MeshGL64 (f64)

| C API function | Safe wrapper |
|---|---|
| `manifold_meshgl` | [`MeshGL::new`](crates/manifold-csg/src/mesh.rs#L35) |
| `manifold_meshgl_num_vert` | [`MeshGL::num_vert`](crates/manifold-csg/src/mesh.rs#L51) |
| `manifold_meshgl_num_tri` | [`MeshGL::num_tri`](crates/manifold-csg/src/mesh.rs#L58) |
| `manifold_meshgl_num_prop` | [`MeshGL::num_prop`](crates/manifold-csg/src/mesh.rs#L65) |
| `manifold_meshgl_vert_properties` | [`MeshGL::vert_properties`](crates/manifold-csg/src/mesh.rs#L72) |
| `manifold_meshgl_tri_verts` | [`MeshGL::tri_verts`](crates/manifold-csg/src/mesh.rs#L83) |
| `manifold_meshgl64` | [`MeshGL64::new`](crates/manifold-csg/src/mesh.rs#L120) |
| `manifold_meshgl64_num_vert` | [`MeshGL64::num_vert`](crates/manifold-csg/src/mesh.rs#L136) |
| `manifold_meshgl64_num_tri` | [`MeshGL64::num_tri`](crates/manifold-csg/src/mesh.rs#L143) |
| `manifold_meshgl64_num_prop` | [`MeshGL64::num_prop`](crates/manifold-csg/src/mesh.rs#L150) |
| `manifold_meshgl64_vert_properties` | [`MeshGL64::vert_properties`](crates/manifold-csg/src/mesh.rs#L157) |
| `manifold_meshgl64_tri_verts` | [`MeshGL64::tri_verts`](crates/manifold-csg/src/mesh.rs#L168) |
| `manifold_meshgl_vert_properties_length` | Internal |
| `manifold_meshgl_tri_length` | Internal |
| `manifold_meshgl64_vert_properties_length` | Internal |
| `manifold_meshgl64_tri_length` | Internal |
| `manifold_meshgl_copy` | Not wrapped |
| `manifold_meshgl64_copy` | Not wrapped |
| `manifold_meshgl_w_options` | Not wrapped |
| `manifold_meshgl64_w_options` | Not wrapped |
| `manifold_meshgl_w_tangents` | Not wrapped |
| `manifold_meshgl64_w_tangents` | Not wrapped |
| `manifold_meshgl_halfedge_tangent` | Not wrapped |
| `manifold_meshgl64_halfedge_tangent` | Not wrapped |
| `manifold_meshgl_tangent_length` | Not wrapped |
| `manifold_meshgl64_tangent_length` | Not wrapped |
| `manifold_meshgl_merge` | [`MeshGL::merge`](crates/manifold-csg/src/mesh.rs) |
| `manifold_meshgl64_merge` | [`MeshGL64::merge`](crates/manifold-csg/src/mesh.rs) |
| `manifold_meshgl_merge_from_vert` | Not wrapped |
| `manifold_meshgl64_merge_from_vert` | Not wrapped |
| `manifold_meshgl_merge_to_vert` | Not wrapped |
| `manifold_meshgl64_merge_to_vert` | Not wrapped |
| `manifold_meshgl_merge_length` | Not wrapped |
| `manifold_meshgl64_merge_length` | Not wrapped |
| `manifold_meshgl_run_index` | Not wrapped |
| `manifold_meshgl64_run_index` | Not wrapped |
| `manifold_meshgl_run_index_length` | Not wrapped |
| `manifold_meshgl64_run_index_length` | Not wrapped |
| `manifold_meshgl_run_original_id` | Not wrapped |
| `manifold_meshgl64_run_original_id` | Not wrapped |
| `manifold_meshgl_run_original_id_length` | Not wrapped |
| `manifold_meshgl64_run_original_id_length` | Not wrapped |
| `manifold_meshgl_run_transform` | Not wrapped |
| `manifold_meshgl64_run_transform` | Not wrapped |
| `manifold_meshgl_run_transform_length` | Not wrapped |
| `manifold_meshgl64_run_transform_length` | Not wrapped |
| `manifold_meshgl_face_id` | Not wrapped |
| `manifold_meshgl64_face_id` | Not wrapped |
| `manifold_meshgl_face_id_length` | Not wrapped |
| `manifold_meshgl64_face_id_length` | Not wrapped |
| `manifold_meshgl_size` | Not wrapped |
| `manifold_meshgl64_size` | Not wrapped |
| `manifold_meshgl64_read_obj` | Not wrapped |
| `manifold_meshgl64_write_obj` | Not wrapped |

## Triangulation

| C API function | Safe wrapper |
|---|---|
| `manifold_triangulate` | [`triangulate_polygons`](crates/manifold-csg/src/triangulation.rs#L22) |
| `manifold_triangulation_num_tri` | Internal |
| `manifold_triangulation_tri_verts` | Internal |
| `manifold_triangulation_size` | Not used |

## Quality Globals

| C API function | Safe wrapper |
|---|---|
| `manifold_set_min_circular_angle` | [`set_min_circular_angle`](crates/manifold-csg/src/manifold.rs#L1288) |
| `manifold_set_min_circular_edge_length` | [`set_min_circular_edge_length`](crates/manifold-csg/src/manifold.rs#L1296) |
| `manifold_set_circular_segments` | [`set_circular_segments`](crates/manifold-csg/src/manifold.rs#L1304) |
| `manifold_reset_to_circular_defaults` | [`reset_to_circular_defaults`](crates/manifold-csg/src/manifold.rs#L1312) |
| `manifold_get_circular_segments` | [`get_circular_segments`](crates/manifold-csg/src/manifold.rs#L1319) |
| `manifold_reserve_ids` | [`reserve_ids`](crates/manifold-csg/src/manifold.rs#L1326) |

## Box3D Operations (`BoundingBox`)

Fully wrapped via the `BoundingBox` type in `crates/manifold-csg/src/bounding_box.rs`.

| C API function | Safe wrapper |
|---|---|
| `manifold_box` | [`BoundingBox::new`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_min` | [`BoundingBox::min`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_max` | [`BoundingBox::max`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_center` | [`BoundingBox::center`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_dimensions` | [`BoundingBox::dimensions`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_scale` | [`BoundingBox::scale`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_is_finite` | [`BoundingBox::is_finite`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_contains_pt` | [`BoundingBox::contains_point`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_contains_box` | [`BoundingBox::contains_box`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_does_overlap_pt` | [`BoundingBox::overlaps_point`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_does_overlap_box` | [`BoundingBox::overlaps_box`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_include_pt` | [`BoundingBox::include_point`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_union` | [`BoundingBox::union`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_transform` | [`BoundingBox::transform`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_translate` | [`BoundingBox::translate`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_mul` | [`BoundingBox::mul`](crates/manifold-csg/src/bounding_box.rs) |
| `manifold_box_size` | Not wrapped |

## Rect2D Operations (`Rect`)

Fully wrapped via the `Rect` type in `crates/manifold-csg/src/rect.rs`.

| C API function | Safe wrapper |
|---|---|
| `manifold_rect` | [`Rect::new`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_min` | [`Rect::min`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_max` | [`Rect::max`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_center` | [`Rect::center`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_dimensions` | [`Rect::dimensions`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_scale` | [`Rect::scale`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_is_empty` | [`Rect::is_empty`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_is_finite` | [`Rect::is_finite`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_contains_pt` | [`Rect::contains_point`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_contains_rect` | [`Rect::contains_rect`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_does_overlap_rect` | [`Rect::overlaps_rect`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_include_pt` | [`Rect::include_point`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_union` | [`Rect::union`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_transform` | [`Rect::transform`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_translate` | [`Rect::translate`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_mul` | [`Rect::mul`](crates/manifold-csg/src/rect.rs) |
| `manifold_rect_size` | Not wrapped |

## Polygon Helpers (Internal)

Used internally by `CrossSection::from_polygons`, `slice_at_z`, `triangulate_polygons`, etc.

| C API function |
|---|---|
| `manifold_polygons` | Internal |
| `manifold_polygons_length` | Internal |
| `manifold_polygons_simple_length` | Internal |
| `manifold_polygons_get_point` | Internal |
| `manifold_polygons_get_simple` | Not used |
| `manifold_polygons_size` | Not used |
| `manifold_simple_polygon` | Internal |
| `manifold_simple_polygon_get_point` | Not used |
| `manifold_simple_polygon_length` | Not used |
| `manifold_simple_polygon_size` | Not used |

## Vector Containers (Internal)

Used internally by batch operations, decompose, etc.

| C API function |
|---|---|
| `manifold_manifold_vec` | Internal |
| `manifold_manifold_empty_vec` | Internal |
| `manifold_manifold_vec_get` | Internal |
| `manifold_manifold_vec_length` | Internal |
| `manifold_manifold_vec_push_back` | Internal |
| `manifold_manifold_vec_reserve` | Not used |
| `manifold_manifold_vec_set` | Not used |
| `manifold_manifold_vec_size` | Not used |
| `manifold_cross_section_vec` | Internal |
| `manifold_cross_section_empty_vec` | Internal |
| `manifold_cross_section_vec_get` | Internal |
| `manifold_cross_section_vec_length` | Internal |
| `manifold_cross_section_vec_push_back` | Internal |
| `manifold_cross_section_vec_reserve` | Not used |
| `manifold_cross_section_vec_set` | Not used |
| `manifold_cross_section_vec_size` | Not used |

## Allocation & Deallocation (Internal)

Every `manifold_alloc_*` and `manifold_delete_*`/`manifold_destruct_*` function
is bound in `manifold-csg-sys` and used internally by the safe wrappers' constructors
and `Drop` implementations.

| Function group | Count |
|---|---|
| `manifold_alloc_*` | 12 |
| `manifold_delete_*` | 12 |
| `manifold_destruct_*` | 12 |

---

## Summary

| Category | Wrapped | Internal |
|---|---|---|
| Manifold construction | 15 | 0 |
| Manifold booleans | 7 | 0 |
| Manifold transforms | 15 | 0 |
| Manifold queries | 13 | 0 |
| Manifold hull/decompose/mesh | 10 | 0 |
| CrossSection construction & booleans | 13 | 0 |
| CrossSection transforms & queries | 15 | 0 |
| MeshGL/MeshGL64 | 18 | 4 |
| Triangulation | 1 | 2 |
| Quality globals | 6 | 0 |
| Box3D (BoundingBox) | 16 | 0 |
| Rect2D (Rect) | 16 | 0 |
| Polygon helpers | 0 | 7 |
| Vector containers | 0 | 10 |
| Alloc/delete/destruct | 0 | 24 |
| **Total** | **145** | **47** |

The remaining unwrapped functions are primarily:
- MeshGL advanced accessors (run tables, face IDs, tangents) — 30 functions
- Allocation infrastructure (`destruct_*` variants, unused vec ops) — 18 functions
- Specialized variants (smooth constructors) — 4 functions
- Internal size queries — 10 functions

All operations commonly needed for CSG workflows (primitives, booleans, transforms,
mesh I/O, extrusion, hull, slicing, SDF, spatial queries) have safe wrappers.
