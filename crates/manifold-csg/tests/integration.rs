use approx::assert_relative_eq;
use manifold_csg::{
    BoundingBox, CrossSection, FillRule, JoinType, Manifold, MeshGL, MeshGL64, OpType, Rect,
    get_circular_segments, reserve_ids, reset_to_circular_defaults, set_circular_segments,
    set_min_circular_angle, set_min_circular_edge_length, triangulate_polygons,
};

// ── Primitive tests ─────────────────────────────────────────────────────

#[test]
fn cube_volume() {
    let c = Manifold::cube(2.0, 3.0, 4.0, true);
    assert_relative_eq!(c.volume(), 24.0, epsilon = 0.01);
}

#[test]
fn cube_not_empty() {
    let c = Manifold::cube(1.0, 1.0, 1.0, true);
    assert!(!c.is_empty());
}

#[test]
fn empty_is_empty() {
    let e = Manifold::empty();
    assert!(e.is_empty());
}

#[test]
fn sphere_volume() {
    let r = 10.0;
    let s = Manifold::sphere(r, 64);
    let expected = (4.0 / 3.0) * std::f64::consts::PI * r * r * r;
    assert_relative_eq!(s.volume(), expected, epsilon = expected * 0.02);
}

#[test]
fn cylinder_volume() {
    let r = 10.0;
    let h = 20.0;
    let c = Manifold::cylinder(h, r, r, 64, false);
    let expected = std::f64::consts::PI * r * r * h;
    assert_relative_eq!(c.volume(), expected, epsilon = expected * 0.02);
}

#[test]
fn frustum_cylinder_volume() {
    let r = 5.0;
    let h = 10.0;
    let c = Manifold::cylinder(h, r, r, 64, false);
    let expected = std::f64::consts::PI * r * r * h;
    assert_relative_eq!(c.volume(), expected, epsilon = expected * 0.02);
}

// ── Boolean operation tests ─────────────────────────────────────────────

#[test]
fn difference_reduces_volume() {
    let big = Manifold::cube(10.0, 10.0, 10.0, true);
    let small = Manifold::cube(4.0, 4.0, 4.0, true);
    let result = &big - &small;
    assert_relative_eq!(result.volume(), 936.0, epsilon = 1.0);
}

#[test]
fn union_of_same_object() {
    let a = Manifold::cube(5.0, 5.0, 5.0, true);
    let b = Manifold::cube(5.0, 5.0, 5.0, true);
    let result = &a + &b;
    assert_relative_eq!(result.volume(), 125.0, epsilon = 1.0);
}

#[test]
fn intersection_of_overlapping_cubes() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true).translate(3.0, 0.0, 0.0);
    let result = &a ^ &b;
    assert_relative_eq!(result.volume(), 700.0, epsilon = 5.0);
}

#[test]
fn difference_with_non_overlapping_is_noop() {
    let a = Manifold::cube(2.0, 2.0, 2.0, true);
    let b = Manifold::cube(2.0, 2.0, 2.0, true).translate(100.0, 0.0, 0.0);
    let result = &a - &b;
    assert_relative_eq!(result.volume(), 8.0, epsilon = 0.1);
}

#[test]
fn subtract_cylinder_from_cube() {
    let cube = Manifold::cube(20.0, 20.0, 20.0, true);
    let hole = Manifold::cylinder(30.0, 5.0, 5.0, 32, false).translate(0.0, 0.0, -15.0);
    let result = &cube - &hole;
    let expected = 8000.0 - std::f64::consts::PI * 25.0 * 20.0;
    assert_relative_eq!(result.volume(), expected, epsilon = expected * 0.05);
}

// ── Transform tests ─────────────────────────────────────────────────────

#[test]
fn translate_preserves_volume() {
    let c = Manifold::cube(2.0, 3.0, 4.0, true);
    let moved = c.translate(100.0, 200.0, 300.0);
    assert_relative_eq!(moved.volume(), 24.0, epsilon = 0.01);
}

#[test]
fn scale_changes_volume() {
    let c = Manifold::cube(1.0, 1.0, 1.0, true);
    let scaled = c.scale(2.0, 3.0, 4.0);
    assert_relative_eq!(scaled.volume(), 24.0, epsilon = 0.01);
}

#[test]
fn transform_identity_preserves_volume() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    #[rustfmt::skip]
    let identity = [
        1.0, 0.0, 0.0,  // col1: X basis
        0.0, 1.0, 0.0,  // col2: Y basis
        0.0, 0.0, 1.0,  // col3: Z basis
        0.0, 0.0, 0.0,  // col4: translation
    ];
    let result = cube.transform(&identity);
    assert_relative_eq!(result.volume(), 24.0, epsilon = 0.01);
}

#[test]
fn transform_with_rotation() {
    // 90 degree rotation about Z axis should preserve volume.
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    #[rustfmt::skip]
    let rot_z_90 = [
        0.0,  1.0, 0.0,  // col1
       -1.0,  0.0, 0.0,  // col2
        0.0,  0.0, 1.0,  // col3
        0.0,  0.0, 0.0,  // col4
    ];
    let result = cube.transform(&rot_z_90);
    assert_relative_eq!(result.volume(), 24.0, epsilon = 0.01);
}

#[test]
fn transform_with_translation() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    #[rustfmt::skip]
    let m = [
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
        100.0, 200.0, 300.0,
    ];
    let result = cube.transform(&m);
    assert_relative_eq!(result.volume(), 24.0, epsilon = 0.01);
    let bb = result.bounding_box().unwrap();
    let bb_min = bb.min();
    assert_relative_eq!(bb_min[0], 99.0, epsilon = 0.1);
    assert_relative_eq!(bb_min[1], 198.5, epsilon = 0.1);
}

// ── Mesh round-trip tests ───────────────────────────────────────────────

#[test]
fn mesh_f64_round_trip() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    assert_relative_eq!(cube.volume(), 24.0, epsilon = 0.1);

    let (verts, n_props, indices) = cube.to_mesh_f64();
    assert!(n_props >= 3);
    assert!(!verts.is_empty());
    assert!(!indices.is_empty());

    let rebuilt = Manifold::from_mesh_f64(&verts, n_props, &indices).unwrap();
    assert_relative_eq!(rebuilt.volume(), 24.0, epsilon = 0.5);
}

#[test]
fn mesh_f32_round_trip() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let rebuilt = Manifold::from_mesh_f32(&verts, n_props, &indices).unwrap();
    assert_relative_eq!(rebuilt.volume(), 24.0, epsilon = 0.5);
}

#[test]
fn csg_result_round_trip() {
    let big = Manifold::cube(10.0, 10.0, 10.0, true);
    let small = Manifold::cube(4.0, 4.0, 4.0, true);
    let result = &big - &small;
    let (verts, n_props, indices) = result.to_mesh_f64();
    let rebuilt = Manifold::from_mesh_f64(&verts, n_props, &indices).unwrap();
    assert_relative_eq!(rebuilt.volume(), 936.0, epsilon = 5.0);
}

#[test]
fn empty_mesh_returns_error() {
    let result = Manifold::from_mesh_f64(&[], 3, &[]);
    assert!(result.is_err());
}

// ── Chained operations ──────────────────────────────────────────────────

#[test]
fn chained_booleans_work() {
    let base = Manifold::cube(20.0, 20.0, 20.0, true);
    let hole1 = Manifold::cylinder(30.0, 3.0, 3.0, 32, false).translate(0.0, 0.0, -15.0);
    let hole2 = Manifold::cylinder(30.0, 3.0, 3.0, 32, false).translate(5.0, 0.0, -15.0);
    let result = &(&base - &hole1) - &hole2;
    let vol = result.volume();
    assert!(vol < 8000.0, "volume {vol} should be < 8000");
    assert!(vol > 6000.0, "volume {vol} should be > 6000");
}

#[test]
fn five_chained_subtractions() {
    let mut result = Manifold::cube(30.0, 30.0, 30.0, true);
    for i in 0..5 {
        let hole = Manifold::sphere(2.0, 16).translate(f64::from(i) * 5.0 - 10.0, 0.0, 0.0);
        result = &result - &hole;
    }
    assert!(result.volume() > 0.0);
    assert!(result.volume() < 27000.0);
}

// ── Split by plane tests ────────────────────────────────────────────────

#[test]
fn split_by_plane_halves_cube() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let (pos, neg) = cube.split_by_plane([1.0, 0.0, 0.0], 0.0);
    assert_relative_eq!(pos.volume(), 500.0, epsilon = 5.0);
    assert_relative_eq!(neg.volume(), 500.0, epsilon = 5.0);
}

#[test]
fn split_by_plane_off_center() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let (pos, neg) = cube.split_by_plane([1.0, 0.0, 0.0], 2.0);
    assert_relative_eq!(pos.volume(), 300.0, epsilon = 10.0);
    assert_relative_eq!(neg.volume(), 700.0, epsilon = 10.0);
}

#[test]
fn split_by_plane_z_axis() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let (pos, neg) = cube.split_by_plane([0.0, 0.0, 1.0], 0.0);
    assert_relative_eq!(pos.volume(), 500.0, epsilon = 5.0);
    assert_relative_eq!(neg.volume(), 500.0, epsilon = 5.0);
}

// ── Trim by plane tests ────────────────────────────────────────────────

#[test]
fn trim_by_plane_keeps_positive_half() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let trimmed = cube.trim_by_plane([1.0, 0.0, 0.0], 0.0);
    assert_relative_eq!(trimmed.volume(), 500.0, epsilon = 5.0);
}

#[test]
fn trim_by_plane_z_removes_top() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let trimmed = cube.trim_by_plane([0.0, 0.0, -1.0], 0.0);
    assert_relative_eq!(trimmed.volume(), 500.0, epsilon = 5.0);
}

// ── Batch boolean tests ─────────────────────────────────────────────────

#[test]
fn batch_union_multiple_cubes() {
    let cubes: Vec<Manifold> = (0..5)
        .map(|i| Manifold::cube(2.0, 2.0, 2.0, true).translate(f64::from(i) * 10.0, 0.0, 0.0))
        .collect();
    let result = Manifold::batch_union(&cubes);
    assert_relative_eq!(result.volume(), 40.0, epsilon = 1.0);
}

#[test]
fn batch_difference_drills_holes() {
    let base = Manifold::cube(30.0, 30.0, 30.0, true);
    let hole = Manifold::sphere(2.0, 16);
    let mut parts = vec![base];
    for i in 0..3 {
        parts.push(hole.translate(f64::from(i) * 8.0 - 8.0, 0.0, 0.0));
    }
    let result = Manifold::batch_difference(&parts);
    assert!(result.volume() < 27000.0);
    assert!(result.volume() > 26000.0);
}

#[test]
fn batch_union_empty_input() {
    let result = Manifold::batch_union(&[]);
    assert!(result.is_empty());
}

// ── Slice at Z tests ────────────────────────────────────────────────────

#[test]
fn slice_cube_at_midpoint() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let contours = cube.slice_at_z(0.0);
    assert_eq!(contours.len(), 1);
    assert!(contours[0].len() >= 4);
}

#[test]
fn slice_outside_returns_empty() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let contours = cube.slice_at_z(100.0);
    assert!(contours.is_empty());
}

// ── Decompose tests ─────────────────────────────────────────────────────

#[test]
fn decompose_single_body() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let parts = cube.decompose();
    assert_eq!(parts.len(), 1);
    assert_relative_eq!(parts[0].volume(), 1000.0, epsilon = 1.0);
}

#[test]
fn decompose_two_bodies() {
    let a = Manifold::cube(5.0, 5.0, 5.0, true).translate(-10.0, 0.0, 0.0);
    let b = Manifold::cube(5.0, 5.0, 5.0, true).translate(10.0, 0.0, 0.0);
    let combined = &a + &b;
    let parts = combined.decompose();
    assert_eq!(parts.len(), 2);
}

// ── Triangulate tests ───────────────────────────────────────────────────

#[test]
fn triangulate_simple_square() {
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]];
    let tris = triangulate_polygons(&square, 1e-6);
    assert!(tris.is_some());
    assert_eq!(tris.unwrap().len(), 2);
}

#[test]
fn triangulate_l_shape() {
    let l_shape = vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 5.0],
        [5.0, 5.0],
        [5.0, 10.0],
        [0.0, 10.0],
    ]];
    let tris = triangulate_polygons(&l_shape, 1e-6);
    assert!(tris.is_some());
    assert_eq!(tris.unwrap().len(), 4);
}

#[test]
fn triangulate_empty_returns_none() {
    let result = triangulate_polygons(&[], 1e-6);
    assert!(result.is_none());
}

// ── CrossSection tests ──────────────────────────────────────────────────

#[test]
fn cross_section_empty_is_empty() {
    let cs = CrossSection::empty();
    assert!(cs.is_empty());
    assert_relative_eq!(cs.area(), 0.0, epsilon = 1e-6);
}

#[test]
fn cross_section_square_area() {
    let cs = CrossSection::square(10.0, 5.0, true);
    assert!(!cs.is_empty());
    assert_relative_eq!(cs.area(), 50.0, epsilon = 0.1);
}

#[test]
fn cross_section_circle_area() {
    let r = 10.0;
    let cs = CrossSection::circle(r, 64);
    let expected = std::f64::consts::PI * r * r;
    assert_relative_eq!(cs.area(), expected, epsilon = expected * 0.02);
}

#[test]
fn cross_section_from_polygons() {
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]];
    let cs = CrossSection::from_polygons(&square);
    assert!(!cs.is_empty());
    assert_relative_eq!(cs.area(), 100.0, epsilon = 1.0);
}

#[test]
fn cross_section_union() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(10.0, 10.0, true).translate(5.0, 0.0);
    let result = &a + &b;
    assert_relative_eq!(result.area(), 150.0, epsilon = 1.0);
}

#[test]
fn cross_section_difference() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(4.0, 4.0, true);
    let result = &a - &b;
    assert_relative_eq!(result.area(), 84.0, epsilon = 1.0);
}

#[test]
fn cross_section_intersection() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(10.0, 10.0, true).translate(3.0, 0.0);
    let result = &a ^ &b;
    assert_relative_eq!(result.area(), 70.0, epsilon = 1.0);
}

#[test]
fn cross_section_offset_grow() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let grown = cs.offset(1.0, JoinType::Square, 2.0, 0);
    assert_relative_eq!(grown.area(), 144.0, epsilon = 2.0);
}

#[test]
fn cross_section_offset_shrink() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let shrunk = cs.offset(-1.0, JoinType::Square, 2.0, 0);
    assert_relative_eq!(shrunk.area(), 64.0, epsilon = 2.0);
}

#[test]
fn cross_section_offset_round() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let grown = cs.offset(2.0, JoinType::Round, 2.0, 32);
    assert!(grown.area() > 185.0, "got {}", grown.area());
    assert!(grown.area() < 250.0, "got {}", grown.area());
}

#[test]
fn cross_section_bounds() {
    let cs = CrossSection::square(10.0, 6.0, false);
    let r = cs.bounds();
    let lo = r.min();
    let hi = r.max();
    assert_relative_eq!(lo[0], 0.0, epsilon = 0.1);
    assert_relative_eq!(lo[1], 0.0, epsilon = 0.1);
    assert_relative_eq!(hi[0], 10.0, epsilon = 0.1);
    assert_relative_eq!(hi[1], 6.0, epsilon = 0.1);
}

#[test]
fn cross_section_to_polygons_roundtrip() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let polys = cs.to_polygons();
    assert!(!polys.is_empty());
    let cs2 = CrossSection::from_polygons(&polys);
    assert_relative_eq!(cs2.area(), cs.area(), epsilon = 0.1);
}

#[test]
fn cross_section_translate() {
    let cs = CrossSection::square(4.0, 4.0, false);
    let moved = cs.translate(10.0, 20.0);
    let r = moved.bounds();
    let lo = r.min();
    assert_relative_eq!(lo[0], 10.0, epsilon = 0.1);
    assert_relative_eq!(lo[1], 20.0, epsilon = 0.1);
}

#[test]
fn cross_section_scale() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let scaled = cs.scale(2.0, 3.0);
    assert_relative_eq!(scaled.area(), 600.0, epsilon = 1.0);
}

#[test]
fn cross_section_clone() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let cloned = cs.clone();
    assert_relative_eq!(cloned.area(), cs.area(), epsilon = 0.01);
}

#[test]
fn cross_section_hull() {
    let l_shape = vec![vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 5.0],
        [5.0, 5.0],
        [5.0, 10.0],
        [0.0, 10.0],
    ]];
    let cs = CrossSection::from_polygons(&l_shape);
    let hull = cs.hull();
    assert!(hull.area() >= cs.area() - 0.1);
    assert!(hull.area() <= 100.5);
}

// ── Extrude tests ───────────────────────────────────────────────────────

#[test]
fn extrude_square_to_cube() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let solid = Manifold::extrude(&cs, 10.0);
    assert_relative_eq!(solid.volume(), 1000.0, epsilon = 5.0);
}

#[test]
fn extrude_circle_to_cylinder() {
    let r = 5.0;
    let h = 20.0;
    let cs = CrossSection::circle(r, 64);
    let solid = Manifold::extrude(&cs, h);
    let expected = std::f64::consts::PI * r * r * h;
    assert_relative_eq!(solid.volume(), expected, epsilon = expected * 0.02);
}

#[test]
fn slice_to_cross_section_roundtrip() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let cs = cube.slice_to_cross_section(0.0);
    assert!(!cs.is_empty());
    assert_relative_eq!(cs.area(), 100.0, epsilon = 1.0);

    let solid = Manifold::extrude(&cs, 10.0);
    assert_relative_eq!(solid.volume(), 1000.0, epsilon = 5.0);
}

#[test]
fn slice_offset_extrude_pipeline() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let cs = cube.slice_to_cross_section(0.0);
    let grown = cs.offset(2.0, JoinType::Square, 2.0, 0);
    let solid = Manifold::extrude(&grown, 10.0);
    assert_relative_eq!(solid.volume(), 1960.0, epsilon = 100.0);
}

#[test]
fn extrude_empty_cross_section() {
    let cs = CrossSection::empty();
    let solid = Manifold::extrude(&cs, 10.0);
    assert!(solid.is_empty());
}

// ── Send safety test ────────────────────────────────────────────────────

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn manifold_is_send() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let handle = std::thread::spawn(move || cube.volume());
    let vol = handle.join().unwrap();
    assert_relative_eq!(vol, 1000.0, epsilon = 1.0);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn cross_section_is_send() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let handle = std::thread::spawn(move || cs.area());
    let area = handle.join().unwrap();
    assert_relative_eq!(area, 100.0, epsilon = 1.0);
}

// ── f64 precision test ──────────────────────────────────────────────────

#[test]
fn f64_precision_at_large_coordinates() {
    // Verify sub-mm features survive round-trip at large coordinates.
    // This is the key advantage of MeshGL64 over MeshGL (f32).
    let cube = Manifold::cube(0.5, 0.5, 0.5, true).translate(1000.0, 1000.0, 1000.0);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let rebuilt = Manifold::from_mesh_f64(&verts, n_props, &indices).unwrap();
    // 0.5^3 = 0.125 mm^3 — should survive f64 round-trip.
    assert_relative_eq!(rebuilt.volume(), 0.125, epsilon = 0.01);
}

// ── Bounding box tests ──────────────────────────────────────────────────

#[test]
fn bounding_box_cube() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let bb = cube.bounding_box().unwrap();
    let min = bb.min();
    let max = bb.max();
    assert_relative_eq!(min[0], -5.0, epsilon = 0.1);
    assert_relative_eq!(max[0], 5.0, epsilon = 0.1);
}

#[test]
fn bounding_box_empty_is_none() {
    let e = Manifold::empty();
    assert!(e.bounding_box().is_none());
}

// ── Clone test ──────────────────────────────────────────────────────────

#[test]
fn manifold_clone() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let cloned = cube.clone();
    assert_relative_eq!(cloned.volume(), 1000.0, epsilon = 1.0);
    // Original should still work after clone.
    assert_relative_eq!(cube.volume(), 1000.0, epsilon = 1.0);
}

// ── MeshGL types tests ──────────────────────────────────────────────────

#[test]
fn meshgl_basic() {
    use manifold_csg::MeshGL;
    // A single triangle.
    let verts: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let indices: Vec<u32> = vec![0, 1, 2];
    let mesh = MeshGL::new(&verts, 3, &indices);
    assert_eq!(mesh.num_vert(), 3);
    assert_eq!(mesh.num_tri(), 1);
    assert_eq!(mesh.num_prop(), 3);
}

#[test]
fn meshgl64_basic() {
    use manifold_csg::MeshGL64;
    let verts: Vec<f64> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let indices: Vec<u64> = vec![0, 1, 2];
    let mesh = MeshGL64::new(&verts, 3, &indices);
    assert_eq!(mesh.num_vert(), 3);
    assert_eq!(mesh.num_tri(), 1);
    assert_eq!(mesh.num_prop(), 3);
}

// ── Hull tests ──────────────────────────────────────────────────────

#[test]
fn manifold_hull() {
    // Hull of a cube is the cube itself (already convex).
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let hull = cube.hull();
    assert_relative_eq!(hull.volume(), 1000.0, epsilon = 5.0);
}

#[test]
fn hull_from_points() {
    let points = vec![
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [0.0, 10.0, 0.0],
        [0.0, 0.0, 10.0],
    ];
    let hull = Manifold::hull_pts(&points);
    assert!(!hull.is_empty());
    assert!(hull.volume() > 0.0);
}

// ── Mirror test ─────────────────────────────────────────────────────

#[test]
fn mirror_preserves_volume() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    let mirrored = cube.mirror([1.0, 0.0, 0.0]);
    assert_relative_eq!(mirrored.volume(), 24.0, epsilon = 0.01);
}

// ── Refine test ─────────────────────────────────────────────────────

#[test]
fn refine_increases_mesh_density() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let original_verts = cube.num_vert();
    let refined = cube.refine(2);
    assert!(refined.num_vert() > original_verts);
    assert_relative_eq!(refined.volume(), 1000.0, epsilon = 5.0);
}

// ── Tetrahedron test ────────────────────────────────────────────────

#[test]
fn tetrahedron_has_volume() {
    let tet = Manifold::tetrahedron();
    assert!(!tet.is_empty());
    assert!(tet.volume() > 0.0);
    assert_eq!(tet.num_vert(), 4);
    assert_eq!(tet.num_tri(), 4);
}

// ── Revolve test ────────────────────────────────────────────────────

#[test]
fn revolve_circle_makes_torus_like() {
    // Revolve a small circle offset from the Y axis to make a torus-like shape.
    let cs = CrossSection::circle(2.0, 32).translate(5.0, 0.0);
    let solid = Manifold::revolve(&cs, 32, 360.0);
    assert!(!solid.is_empty());
    assert!(solid.volume() > 0.0);
}

// ── Query tests ─────────────────────────────────────────────────────

#[test]
fn genus_of_sphere_is_zero() {
    let s = Manifold::sphere(10.0, 32);
    assert_eq!(s.genus(), 0);
}

#[test]
fn num_edge_positive() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    assert!(cube.num_edge() > 0);
}

#[test]
fn epsilon_positive() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    assert!(cube.epsilon() >= 0.0);
}

// ── Compose test ────────────────────────────────────────────────────

#[test]
fn compose_preserves_total_volume() {
    let a = Manifold::cube(5.0, 5.0, 5.0, true).translate(-10.0, 0.0, 0.0);
    let b = Manifold::cube(5.0, 5.0, 5.0, true).translate(10.0, 0.0, 0.0);
    let composed = Manifold::compose(&[a, b]);
    assert_relative_eq!(composed.volume(), 250.0, epsilon = 5.0);
}

// ── Split test ──────────────────────────────────────────────────────

#[test]
fn split_by_manifold() {
    let big = Manifold::cube(10.0, 10.0, 10.0, true);
    let cutter = Manifold::cube(10.0, 10.0, 10.0, true).translate(5.0, 0.0, 0.0);
    let (inside, outside) = big.split(&cutter);
    assert!(!inside.is_empty());
    assert!(!outside.is_empty());
}

// ── Project test ────────────────────────────────────────────────────

#[test]
fn project_cube_to_2d() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let polys = cube.project();
    assert!(!polys.is_empty());
}

// ── CrossSection simplify test ──────────────────────────────────────

#[test]
fn cross_section_simplify() {
    let cs = CrossSection::circle(10.0, 128);
    let simplified = cs.simplify(0.5);
    assert!(simplified.num_vert() <= cs.num_vert());
    assert_relative_eq!(simplified.area(), cs.area(), epsilon = 10.0);
}

// ── CrossSection batch tests ────────────────────────────────────────

#[test]
fn cross_section_batch_union() {
    let sections: Vec<CrossSection> = (0..3)
        .map(|i| CrossSection::square(5.0, 5.0, true).translate(f64::from(i) * 10.0, 0.0))
        .collect();
    let result = CrossSection::batch_union(&sections);
    assert!(!result.is_empty());
    assert!(result.area() > 25.0);
}

// ── Smooth out test ─────────────────────────────────────────────────

#[test]
fn smooth_out_preserves_topology() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let smoothed = cube.smooth_out(60.0, 0.5);
    // After smooth_out, volume should be similar (smoothing rounds edges).
    assert!(!smoothed.is_empty());
    // Smoothed cube will have less volume than original due to rounding.
    assert!(smoothed.volume() > 500.0);
}

// ── Min gap test ────────────────────────────────────────────────────

#[test]
fn min_gap_between_separated_cubes() {
    let a = Manifold::cube(4.0, 4.0, 4.0, true).translate(-5.0, 0.0, 0.0);
    let b = Manifold::cube(4.0, 4.0, 4.0, true).translate(5.0, 0.0, 0.0);
    let gap = a.min_gap(&b, 20.0);
    // Cubes are 4x4x4 centered at x=-5 and x=5: edges at x=-3 and x=3, gap=6.
    assert_relative_eq!(gap, 6.0, epsilon = 0.5);
}

// ── Ray casting tests ──────────────────────────────────────────────

#[test]
fn ray_cast_through_cube() {
    // Cube from [0,0,0] to [10,10,10].
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    // Ray along +X through the center of the cube.
    let hits = cube.ray_cast([-5.0, 5.0, 5.0], [15.0, 5.0, 5.0]);
    // Should hit two faces (entry and exit).
    assert_eq!(hits.len(), 2);
    // Entry hit near x=0, exit hit near x=10.
    let mut distances: Vec<f64> = hits.iter().map(|h| h.position[0]).collect();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_relative_eq!(distances[0], 0.0, epsilon = 0.1);
    assert_relative_eq!(distances[1], 10.0, epsilon = 0.1);
}

#[test]
fn ray_cast_miss() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    // Ray that misses entirely.
    let hits = cube.ray_cast([100.0, 100.0, 100.0], [200.0, 200.0, 200.0]);
    assert!(hits.is_empty());
}

#[test]
fn ray_cast_hit_has_normal() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let hits = cube.ray_cast([-5.0, 5.0, 5.0], [15.0, 5.0, 5.0]);
    assert!(!hits.is_empty());
    for hit in &hits {
        // Normal should be unit length.
        let len = (hit.normal[0].powi(2) + hit.normal[1].powi(2) + hit.normal[2].powi(2)).sqrt();
        assert_relative_eq!(len, 1.0, epsilon = 0.01);
    }
}

// ── Minkowski tests ─────────────────────────────────────────────────

#[test]
fn minkowski_sum_increases_volume() {
    let a = Manifold::cube(4.0, 4.0, 4.0, true);
    let b = Manifold::sphere(1.0, 16);
    let result = a.minkowski_sum(&b);
    assert!(result.volume() > a.volume());
}

#[test]
fn minkowski_difference_produces_result() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::sphere(1.0, 16);
    let result = a.minkowski_difference(&b);
    assert!(!result.is_empty());
    assert!(result.volume() < a.volume());
}

// ── Extrude with options test ───────────────────────────────────────

#[test]
fn extrude_with_twist() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let solid = Manifold::extrude_with_options(&cs, 20.0, 10, 90.0, 1.0, 1.0);
    assert!(!solid.is_empty());
    // Twisted extrusion preserves cross-sectional area, so volume is similar.
    assert!(solid.volume() > 1000.0);
}

// ── Calculate normals/curvature tests ───────────────────────────────

#[test]
fn calculate_normals_adds_properties() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let with_normals = cube.calculate_normals(3, 60.0);
    assert!(!with_normals.is_empty());
    assert!(with_normals.num_prop() >= 6);
}

#[test]
fn calculate_curvature_adds_properties() {
    let sphere = Manifold::sphere(10.0, 32);
    let with_curvature = sphere.calculate_curvature(3, 4);
    assert!(!with_curvature.is_empty());
    assert!(with_curvature.num_prop() >= 5);
}

// ── Refine variants tests ───────────────────────────────────────────

#[test]
fn refine_to_length_increases_density() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let original_verts = cube.num_vert();
    let refined = cube.refine_to_length(2.0);
    assert!(refined.num_vert() > original_verts);
    assert_relative_eq!(refined.volume(), 1000.0, epsilon = 5.0);
}

#[test]
fn refine_to_tolerance_increases_density() {
    // Use a coarse sphere so there's room to refine.
    let sphere = Manifold::sphere(10.0, 8);
    let original_verts = sphere.num_vert();
    let refined = sphere.refine_to_tolerance(0.1);
    assert!(refined.num_vert() >= original_verts);
}

// ── Smooth by normals test ──────────────────────────────────────────

#[test]
fn smooth_by_normals_works() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let with_normals = cube.calculate_normals(3, 60.0);
    let smoothed = with_normals.smooth_by_normals(3);
    assert!(!smoothed.is_empty());
}

// ── Additional query tests ──────────────────────────────────────────

#[test]
fn num_prop_on_constructed_manifold() {
    // Primitive manifolds may not have a fixed num_prop until mesh extraction.
    // Build from mesh data where we control n_props.
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    // After calculate_normals at index 3, we know num_prop >= 6.
    let with_normals = cube.calculate_normals(3, 60.0);
    assert!(with_normals.num_prop() >= 6);
}

#[test]
fn original_id_is_non_negative() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    assert!(cube.original_id() >= 0);
}

// ── CrossSection compose test ───────────────────────────────────────

#[test]
fn cross_section_compose() {
    let a = CrossSection::square(5.0, 5.0, true).translate(-10.0, 0.0);
    let b = CrossSection::square(5.0, 5.0, true).translate(10.0, 0.0);
    let composed = CrossSection::compose(&[a, b]);
    assert!(!composed.is_empty());
    assert_relative_eq!(composed.area(), 50.0, epsilon = 1.0);
}

// ── Batch difference test ───────────────────────────────────────────

#[test]
fn batch_difference_specific() {
    let base = Manifold::cube(20.0, 20.0, 20.0, true);
    let hole = Manifold::cylinder(30.0, 3.0, 3.0, 32, false).translate(0.0, 0.0, -15.0);
    let parts = vec![base, hole];
    let result = Manifold::batch_difference(&parts);
    assert!(result.volume() < 8000.0);
    assert!(result.volume() > 7000.0);
}

// ── JoinType::Bevel test ────────────────────────────────────────────

#[test]
fn cross_section_offset_bevel() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let grown = cs.offset(1.0, JoinType::Bevel, 2.0, 0);
    // Beveled corners cut off corners, area should be between square offset (144) and no offset (100).
    assert!(grown.area() > 100.0);
    assert!(grown.area() < 150.0);
}

// ── CrossSection::extrude convenience ───────────────────────────────

#[test]
fn cross_section_extrude_convenience() {
    let cs = CrossSection::square(10.0, 10.0, true);
    let solid = cs.extrude(5.0);
    assert_relative_eq!(solid.volume(), 500.0, epsilon = 5.0);
}

// ── Warp tests ──────────────────────────────────────────────────────

#[test]
fn manifold_warp_translates() {
    let cube = Manifold::cube(4.0, 4.0, 4.0, true);
    // Warp: shift everything +10 in X.
    let warped = cube.warp(|x, y, z| [x + 10.0, y, z]);
    assert_relative_eq!(warped.volume(), 64.0, epsilon = 1.0);
    let bb = warped.bounding_box().unwrap();
    let min = bb.min();
    assert!(min[0] > 7.0, "min x should be shifted: got {}", min[0]);
}

#[test]
fn cross_section_warp_scales() {
    let cs = CrossSection::square(10.0, 10.0, true);
    // Warp: double X coordinates.
    let warped = cs.warp(|x, y| [x * 2.0, y]);
    // Area should roughly double (20x10 = 200).
    assert!(warped.area() > 150.0);
}

// ── Set properties test ─────────────────────────────────────────────

#[test]
fn set_properties_adds_custom_data() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    // Add a 4th property: distance from origin.
    // Note: old_props may be empty for primitive manifolds, so use pos directly.
    let with_props = cube.set_properties(4, |new, pos, _old| {
        new[0] = pos[0];
        new[1] = pos[1];
        new[2] = pos[2];
        new[3] = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
    });
    assert!(!with_props.is_empty());
    assert_eq!(with_props.num_prop(), 4);
}

// ── SDF (level set) test ────────────────────────────────────────────

#[test]
fn from_sdf_creates_manifold() {
    // SDF for a sphere of radius 5 centered at origin.
    let result = Manifold::from_sdf(
        |x, y, z| (x * x + y * y + z * z).sqrt() - 5.0,
        ([-6.0, -6.0, -6.0], [6.0, 6.0, 6.0]),
        1.0,
        0.0,
        0.05,
    );
    assert!(!result.is_empty());
    assert!(result.volume() > 0.0);
    // SDF meshing produces a solid — verify it has vertices and faces.
    assert!(result.num_vert() > 10);
    assert!(result.num_tri() > 10);
}

// ── OBJ I/O tests ───────────────────────────────────────────────────

#[test]
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn obj_round_trip() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let obj_str = cube.to_obj();
    assert!(!obj_str.is_empty());
    assert!(obj_str.contains("v "), "OBJ should contain vertex lines");

    let rebuilt = Manifold::from_obj(&obj_str).unwrap();
    assert!(!rebuilt.is_empty());
    assert_relative_eq!(rebuilt.volume(), 1000.0, epsilon = 50.0);
}

#[test]
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn from_obj_invalid_returns_error() {
    let result = Manifold::from_obj("not valid obj data");
    // Either parses as empty or returns error — both are acceptable.
    if let Ok(m) = result {
        assert!(m.is_empty());
    }
}

// ── Rotate test ─────────────────────────────────────────────────────

#[test]
fn rotate_preserves_volume() {
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    let rotated = cube.rotate(45.0, 0.0, 0.0);
    assert_relative_eq!(rotated.volume(), 24.0, epsilon = 0.1);
}

// ── Surface area test ───────────────────────────────────────────────

#[test]
fn cube_surface_area() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    // Surface area of 10x10x10 cube = 6 * 100 = 600.
    assert_relative_eq!(cube.surface_area(), 600.0, epsilon = 1.0);
}

// ── nalgebra convenience tests ──────────────────────────────────────
// Run with: cargo test --features nalgebra

#[cfg(feature = "nalgebra")]
#[test]
fn nalgebra_transform() {
    use nalgebra::{Matrix3, Vector3};
    let cube = Manifold::cube(2.0, 3.0, 4.0, true);
    let identity = Matrix3::identity();
    let translation = Vector3::new(100.0, 200.0, 300.0);
    let result = cube.transform_nalgebra(&identity, &translation);
    assert_relative_eq!(result.volume(), 24.0, epsilon = 0.01);
    let (bb_min, _) = result.bounding_box_nalgebra().unwrap();
    assert_relative_eq!(bb_min.x, 99.0, epsilon = 0.1);
}

#[cfg(feature = "nalgebra")]
#[test]
fn nalgebra_vertices_round_trip() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let (verts, faces) = cube.to_vertices_and_faces();
    assert!(!verts.is_empty());
    assert!(!faces.is_empty());
    let rebuilt = Manifold::from_vertices_and_faces(&verts, &faces).unwrap();
    assert_relative_eq!(rebuilt.volume(), 1000.0, epsilon = 5.0);
}

#[cfg(feature = "nalgebra")]
#[test]
fn nalgebra_split_by_plane() {
    use nalgebra::Vector3;
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let normal = Vector3::new(1.0, 0.0, 0.0);
    let (pos, neg) = cube.split_by_plane_nalgebra(&normal, 0.0);
    assert_relative_eq!(pos.volume(), 500.0, epsilon = 5.0);
    assert_relative_eq!(neg.volume(), 500.0, epsilon = 5.0);
}

// ── BoundingBox tests ──────────────────────────────────────────────────

#[test]
fn bounding_box_new_and_accessors() {
    let bb = BoundingBox::new([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    assert_eq!(bb.min(), [1.0, 2.0, 3.0]);
    assert_eq!(bb.max(), [4.0, 5.0, 6.0]);
    assert_relative_eq!(bb.dimensions()[0], 3.0, epsilon = 1e-10);
    assert_relative_eq!(bb.dimensions()[1], 3.0, epsilon = 1e-10);
    assert_relative_eq!(bb.dimensions()[2], 3.0, epsilon = 1e-10);
    assert_relative_eq!(bb.center()[0], 2.5, epsilon = 1e-10);
    assert_relative_eq!(bb.center()[1], 3.5, epsilon = 1e-10);
    assert_relative_eq!(bb.center()[2], 4.5, epsilon = 1e-10);
    assert!(bb.scale() > 0.0);
    assert!(bb.is_finite());
    assert!(!bb.is_empty());
}

#[test]
fn bounding_box_contains_point() {
    let bb = BoundingBox::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    assert!(bb.contains_point([5.0, 5.0, 5.0]));
    assert!(!bb.contains_point([15.0, 5.0, 5.0]));
    assert!(!bb.contains_point([-1.0, 5.0, 5.0]));
}

#[test]
fn bounding_box_contains_box() {
    let outer = BoundingBox::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let inner = BoundingBox::new([2.0, 2.0, 2.0], [8.0, 8.0, 8.0]);
    let outside = BoundingBox::new([20.0, 20.0, 20.0], [30.0, 30.0, 30.0]);
    assert!(outer.contains_box(&inner));
    assert!(!inner.contains_box(&outer));
    assert!(!outer.contains_box(&outside));
}

#[test]
fn bounding_box_overlaps() {
    let a = BoundingBox::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let b = BoundingBox::new([5.0, 5.0, 5.0], [15.0, 15.0, 15.0]);
    let c = BoundingBox::new([20.0, 20.0, 20.0], [30.0, 30.0, 30.0]);
    assert!(a.overlaps_box(&b));
    assert!(b.overlaps_box(&a));
    assert!(!a.overlaps_box(&c));
    assert!(a.overlaps_point([5.0, 5.0, 5.0]));
    assert!(!a.overlaps_point([15.0, 5.0, 5.0]));
}

#[test]
fn bounding_box_include_point() {
    let mut bb = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    bb.include_point([5.0, 5.0, 5.0]);
    assert_relative_eq!(bb.max()[0], 5.0, epsilon = 1e-10);
    assert_relative_eq!(bb.max()[1], 5.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_union() {
    let a = BoundingBox::new([0.0, 0.0, 0.0], [5.0, 5.0, 5.0]);
    let b = BoundingBox::new([3.0, 3.0, 3.0], [10.0, 10.0, 10.0]);
    let u = a.union(&b);
    assert_relative_eq!(u.min()[0], 0.0, epsilon = 1e-10);
    assert_relative_eq!(u.max()[0], 10.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_translate() {
    let bb = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let moved = bb.translate([10.0, 20.0, 30.0]);
    assert_relative_eq!(moved.min()[0], 10.0, epsilon = 1e-10);
    assert_relative_eq!(moved.min()[1], 20.0, epsilon = 1e-10);
    assert_relative_eq!(moved.min()[2], 30.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_mul() {
    let bb = BoundingBox::new([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]);
    let scaled = bb.mul([3.0, 3.0, 3.0]);
    assert_relative_eq!(scaled.min()[0], 3.0, epsilon = 1e-10);
    assert_relative_eq!(scaled.max()[0], 6.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_transform_identity() {
    let bb = BoundingBox::new([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
    let result = bb.transform(&identity);
    assert_relative_eq!(result.min()[0], 1.0, epsilon = 1e-10);
    assert_relative_eq!(result.max()[2], 6.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_clone_is_independent() {
    let bb = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let clone = bb.clone();
    // Modifying the original via include_point shouldn't affect the clone.
    drop(bb);
    assert_relative_eq!(clone.max()[0], 1.0, epsilon = 1e-10);
}

#[test]
fn bounding_box_is_empty() {
    let empty = BoundingBox::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    assert!(empty.is_empty());
    let nonempty = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    assert!(!nonempty.is_empty());
}

// ── Rect tests ─────────────────────────────────────────────────────────

#[test]
fn rect_new_and_accessors() {
    let r = Rect::new([1.0, 2.0], [4.0, 6.0]);
    assert_eq!(r.min(), [1.0, 2.0]);
    assert_eq!(r.max(), [4.0, 6.0]);
    assert_relative_eq!(r.dimensions()[0], 3.0, epsilon = 1e-10);
    assert_relative_eq!(r.dimensions()[1], 4.0, epsilon = 1e-10);
    assert_relative_eq!(r.center()[0], 2.5, epsilon = 1e-10);
    assert_relative_eq!(r.center()[1], 4.0, epsilon = 1e-10);
    assert!(r.scale() > 0.0);
    assert!(r.is_finite());
    assert!(!r.is_empty());
}

#[test]
fn rect_contains_point() {
    let r = Rect::new([0.0, 0.0], [10.0, 10.0]);
    assert!(r.contains_point([5.0, 5.0]));
    assert!(!r.contains_point([15.0, 5.0]));
    assert!(!r.contains_point([-1.0, 5.0]));
}

#[test]
fn rect_contains_rect() {
    let outer = Rect::new([0.0, 0.0], [10.0, 10.0]);
    let inner = Rect::new([2.0, 2.0], [8.0, 8.0]);
    assert!(outer.contains_rect(&inner));
    assert!(!inner.contains_rect(&outer));
}

#[test]
fn rect_overlaps() {
    let a = Rect::new([0.0, 0.0], [10.0, 10.0]);
    let b = Rect::new([5.0, 5.0], [15.0, 15.0]);
    let c = Rect::new([20.0, 20.0], [30.0, 30.0]);
    assert!(a.overlaps_rect(&b));
    assert!(!a.overlaps_rect(&c));
}

#[test]
fn rect_include_point() {
    let mut r = Rect::new([0.0, 0.0], [1.0, 1.0]);
    r.include_point([5.0, 5.0]);
    assert_relative_eq!(r.max()[0], 5.0, epsilon = 1e-10);
    assert_relative_eq!(r.max()[1], 5.0, epsilon = 1e-10);
}

#[test]
fn rect_union() {
    let a = Rect::new([0.0, 0.0], [5.0, 5.0]);
    let b = Rect::new([3.0, 3.0], [10.0, 10.0]);
    let u = a.union(&b);
    assert_relative_eq!(u.min()[0], 0.0, epsilon = 1e-10);
    assert_relative_eq!(u.max()[0], 10.0, epsilon = 1e-10);
}

#[test]
fn rect_translate() {
    let r = Rect::new([0.0, 0.0], [1.0, 1.0]);
    let moved = r.translate([10.0, 20.0]);
    assert_relative_eq!(moved.min()[0], 10.0, epsilon = 1e-10);
    assert_relative_eq!(moved.min()[1], 20.0, epsilon = 1e-10);
}

#[test]
fn rect_mul() {
    let r = Rect::new([1.0, 1.0], [2.0, 2.0]);
    let scaled = r.mul([3.0, 3.0]);
    assert_relative_eq!(scaled.min()[0], 3.0, epsilon = 1e-10);
    assert_relative_eq!(scaled.max()[0], 6.0, epsilon = 1e-10);
}

#[test]
fn rect_clone_is_independent() {
    let r = Rect::new([0.0, 0.0], [1.0, 1.0]);
    let clone = r.clone();
    drop(r);
    assert_relative_eq!(clone.max()[0], 1.0, epsilon = 1e-10);
}

#[test]
fn rect_is_empty() {
    let empty = Rect::new([0.0, 0.0], [0.0, 0.0]);
    assert!(empty.is_empty());
    let nonempty = Rect::new([0.0, 0.0], [1.0, 1.0]);
    assert!(!nonempty.is_empty());
}

// ── FillRule tests ─────────────────────────────────────────────────────

#[test]
fn fill_rule_even_odd_default() {
    // A simple square polygon should produce the same result with explicit EvenOdd
    // as with the default from_polygons.
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]];
    let default_cs = CrossSection::from_polygons(&square);
    let explicit_cs = CrossSection::from_polygons_with_fill_rule(&square, FillRule::EvenOdd);
    assert_relative_eq!(default_cs.area(), explicit_cs.area(), epsilon = 0.1);
}

#[test]
fn fill_rule_non_zero() {
    // A self-intersecting polygon (figure-8 shape) should produce different
    // results with EvenOdd vs NonZero fill rules.
    let figure_8 = vec![vec![[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]]];
    let even_odd = CrossSection::from_polygons_with_fill_rule(&figure_8, FillRule::EvenOdd);
    let non_zero = CrossSection::from_polygons_with_fill_rule(&figure_8, FillRule::NonZero);
    // Both should produce valid (non-empty) cross-sections.
    assert!(!even_odd.is_empty() || !non_zero.is_empty());
}

#[test]
fn fill_rule_positive_negative() {
    // Verify Positive and Negative variants don't crash.
    let square = vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]];
    let pos = CrossSection::from_polygons_with_fill_rule(&square, FillRule::Positive);
    let neg = CrossSection::from_polygons_with_fill_rule(&square, FillRule::Negative);
    // At minimum, one of these should be valid for a CCW polygon.
    assert!(pos.area() >= 0.0);
    assert!(neg.area() >= 0.0);
}

// ── CrossSection::transform tests ──────────────────────────────────────

#[test]
fn cross_section_transform_identity() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; // 2D identity
    let result = cs.transform(&identity);
    assert_relative_eq!(result.area(), cs.area(), epsilon = 0.1);
}

#[test]
fn cross_section_transform_translate() {
    let cs = CrossSection::square(4.0, 4.0, false);
    // Translation by (10, 20) via 3x2 matrix
    let m = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
    let moved = cs.transform(&m);
    let r = moved.bounds();
    assert_relative_eq!(r.min()[0], 10.0, epsilon = 0.1);
    assert_relative_eq!(r.min()[1], 20.0, epsilon = 0.1);
}

// ── CrossSection::decompose tests ──────────────────────────────────────

#[test]
fn cross_section_decompose_single() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let parts = cs.decompose();
    assert_eq!(parts.len(), 1);
    assert_relative_eq!(parts[0].area(), cs.area(), epsilon = 0.1);
}

#[test]
fn cross_section_decompose_two_disjoint() {
    let a = CrossSection::square(5.0, 5.0, false);
    let b = CrossSection::square(5.0, 5.0, false).translate(20.0, 0.0);
    let composed = CrossSection::compose(&[a, b]);
    let parts = composed.decompose();
    assert_eq!(parts.len(), 2);
}

// ── Manifold::as_original tests ────────────────────────────────────────

#[test]
fn as_original_assigns_unique_id() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let orig = cube.as_original();
    let id = orig.original_id();
    // as_original should assign a non-negative ID.
    assert!(id >= 0, "expected non-negative id, got {id}");
}

#[test]
fn as_original_preserves_geometry() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let orig = cube.as_original();
    assert_relative_eq!(orig.volume(), cube.volume(), epsilon = 0.01);
    assert_eq!(orig.num_vert(), cube.num_vert());
}

// ── MeshGL/MeshGL64 clone and merge tests ──────────────────────────────

#[test]
fn meshgl_clone_is_independent() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let mesh = MeshGL::new(&verts, n_props, &indices);
    let clone = mesh.clone();
    assert_eq!(mesh.num_vert(), clone.num_vert());
    assert_eq!(mesh.num_tri(), clone.num_tri());
    // Drop original, clone should still be valid.
    drop(mesh);
    assert!(clone.num_vert() > 0);
}

#[test]
fn meshgl64_clone_is_independent() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = MeshGL64::new(&verts, n_props, &indices);
    let clone = mesh.clone();
    assert_eq!(mesh.num_vert(), clone.num_vert());
    drop(mesh);
    assert!(clone.num_vert() > 0);
}

// NOTE: MeshGL::merge() / MeshGL64::merge() are NOT exposed because the C API's
// manifold_meshgl_merge returns a mesh that shares internal buffers with the
// source, causing double-free on drop. This needs further upstream investigation.

// ── Binding-specific: Drop safety ──────────────────────────────────────

#[test]
fn drop_many_manifolds_no_leak() {
    // Create and drop many manifolds to exercise allocation/deallocation.
    for _ in 0..100 {
        let _ = Manifold::cube(1.0, 1.0, 1.0, false);
    }
}

#[test]
fn drop_many_cross_sections_no_leak() {
    for _ in 0..100 {
        let _ = CrossSection::square(1.0, 1.0, false);
    }
}

#[test]
fn drop_many_bounding_boxes_no_leak() {
    for _ in 0..100 {
        let _ = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    }
}

#[test]
fn drop_many_rects_no_leak() {
    for _ in 0..100 {
        let _ = Rect::new([0.0, 0.0], [1.0, 1.0]);
    }
}

// ── Binding-specific: Send across threads ──────────────────────────────

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn manifold_send_across_thread() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let vol = std::thread::spawn(move || cube.volume()).join().unwrap();
    assert_relative_eq!(vol, 1000.0, epsilon = 1.0);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn cross_section_send_across_thread() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let area = std::thread::spawn(move || cs.area()).join().unwrap();
    assert_relative_eq!(area, 100.0, epsilon = 1.0);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn bounding_box_send_across_thread() {
    let bb = BoundingBox::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let center = std::thread::spawn(move || bb.center()).join().unwrap();
    assert_relative_eq!(center[0], 5.0, epsilon = 1e-10);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn rect_send_across_thread() {
    let r = Rect::new([0.0, 0.0], [10.0, 10.0]);
    let center = std::thread::spawn(move || r.center()).join().unwrap();
    assert_relative_eq!(center[0], 5.0, epsilon = 1e-10);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn meshgl64_send_across_thread() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = MeshGL64::new(&verts, n_props, &indices);
    let nv = std::thread::spawn(move || mesh.num_vert()).join().unwrap();
    assert!(nv > 0);
}

// ── Binding-specific: Sync (concurrent shared reads) ───────────────────

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn manifold_sync_concurrent_reads() {
    let cube = std::sync::Arc::new(Manifold::cube(10.0, 10.0, 10.0, true));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let c = std::sync::Arc::clone(&cube);
            std::thread::spawn(move || c.volume())
        })
        .collect();
    for h in handles {
        let vol = h.join().unwrap();
        assert_relative_eq!(vol, 1000.0, epsilon = 1.0);
    }
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn cross_section_sync_concurrent_reads() {
    let cs = std::sync::Arc::new(CrossSection::square(10.0, 10.0, false));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let c = std::sync::Arc::clone(&cs);
            std::thread::spawn(move || c.area())
        })
        .collect();
    for h in handles {
        let area = h.join().unwrap();
        assert_relative_eq!(area, 100.0, epsilon = 1.0);
    }
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn meshgl64_sync_concurrent_reads() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = std::sync::Arc::new(MeshGL64::new(&verts, n_props, &indices));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let m = std::sync::Arc::clone(&mesh);
            std::thread::spawn(move || m.num_vert())
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap() > 0);
    }
}

// ── Binding-specific: FFI data integrity ───────────────────────────────

#[test]
fn mesh_f64_buffer_sizes_consistent() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    // Verify internal consistency.
    assert_eq!(verts.len() % n_props, 0);
    assert_eq!(indices.len() % 3, 0);
    let n_verts = verts.len() / n_props;
    let n_tris = indices.len() / 3;
    assert_eq!(n_verts, cube.num_vert());
    assert_eq!(n_tris, cube.num_tri());
    // All indices should be in range.
    for &idx in &indices {
        assert!(
            (idx as usize) < n_verts,
            "index {idx} out of range for {n_verts} verts"
        );
    }
}

#[test]
fn mesh_f32_buffer_sizes_consistent() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    assert_eq!(verts.len() % n_props, 0);
    assert_eq!(indices.len() % 3, 0);
    let n_verts = verts.len() / n_props;
    for &idx in &indices {
        assert!(
            (idx as usize) < n_verts,
            "index {idx} out of range for {n_verts} verts"
        );
    }
}

#[test]
fn bounding_box_from_manifold_matches_mesh_extents() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let bb = cube.bounding_box().unwrap();
    let (verts, n_props, _) = cube.to_mesh_f64();
    // Every vertex position should be within the bounding box.
    for chunk in verts.chunks(n_props) {
        let (x, y, z) = (chunk[0], chunk[1], chunk[2]);
        assert!(
            bb.contains_point([x, y, z]),
            "vertex ({x}, {y}, {z}) not in bbox {:?}..{:?}",
            bb.min(),
            bb.max()
        );
    }
}

// ── Binding-specific: edge cases at FFI boundary ───────────────────────

#[test]
fn empty_manifold_operations_dont_crash() {
    let e = Manifold::empty();
    assert!(e.is_empty());
    assert_eq!(e.num_vert(), 0);
    assert_eq!(e.num_tri(), 0);
    assert_relative_eq!(e.volume(), 0.0, epsilon = 1e-10);
    assert!(e.bounding_box().is_none());
    // Boolean with empty should return the other operand.
    let cube = Manifold::cube(1.0, 1.0, 1.0, false);
    let u = cube.union(&e);
    assert_relative_eq!(u.volume(), 1.0, epsilon = 0.01);
}

#[test]
fn empty_cross_section_operations_dont_crash() {
    let e = CrossSection::empty();
    assert!(e.is_empty());
    assert_eq!(e.num_vert(), 0);
    assert_relative_eq!(e.area(), 0.0, epsilon = 1e-10);
    let polys = e.to_polygons();
    assert!(polys.is_empty());
    // decompose() may return an empty vector or a single empty component.
    let _parts = e.decompose();
}

#[test]
fn empty_polygons_from_polygons() {
    let cs = CrossSection::from_polygons(&[]);
    assert!(cs.is_empty());
}

#[test]
fn manifold_clone_is_independent() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let clone = cube.clone();
    drop(cube);
    // Clone should still be fully functional after original is dropped.
    assert_relative_eq!(clone.volume(), 1000.0, epsilon = 1.0);
    assert_eq!(clone.num_vert(), 8);
}

#[test]
fn cross_section_clone_is_independent() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let clone = cs.clone();
    drop(cs);
    assert_relative_eq!(clone.area(), 100.0, epsilon = 1.0);
}

// ── Binding-specific: callback catch_unwind ────────────────────────────

#[test]
fn warp_callback_doesnt_crash_on_identity() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, true);
    let warped = cube.warp(|x, y, z| [x, y, z]);
    assert_relative_eq!(warped.volume(), cube.volume(), epsilon = 1.0);
}

#[test]
fn cross_section_warp_identity() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let warped = cs.warp(|x, y| [x, y]);
    assert_relative_eq!(warped.area(), cs.area(), epsilon = 1.0);
}

// ── Binding-specific: bounds() returns rich Rect ───────────────────────

#[test]
fn cross_section_bounds_returns_rect_with_methods() {
    let cs = CrossSection::square(10.0, 6.0, false);
    let r = cs.bounds();
    // Verify accessors beyond min/max work.
    assert_relative_eq!(r.dimensions()[0], 10.0, epsilon = 0.1);
    assert_relative_eq!(r.dimensions()[1], 6.0, epsilon = 0.1);
    assert_relative_eq!(r.center()[0], 5.0, epsilon = 0.1);
    assert_relative_eq!(r.center()[1], 3.0, epsilon = 0.1);
    assert!(!r.is_empty());
    assert!(r.is_finite());
    assert!(r.contains_point([5.0, 3.0]));
    assert!(!r.contains_point([15.0, 3.0]));
}

// ── Binding-specific: batch operations clean up properly ───────────────

#[test]
fn batch_union_many_doesnt_leak() {
    let parts: Vec<_> = (0..50)
        .map(|i| Manifold::sphere(1.0, 8).translate(i as f64 * 3.0, 0.0, 0.0))
        .collect();
    let combined = Manifold::batch_union(&parts);
    assert!(!combined.is_empty());
}

#[test]
fn cross_section_batch_union_many_doesnt_leak() {
    let parts: Vec<_> = (0..50)
        .map(|i| CrossSection::circle(1.0, 8).translate(i as f64 * 3.0, 0.0))
        .collect();
    let combined = CrossSection::batch_union(&parts);
    assert!(!combined.is_empty());
}

// ── Binding-specific: split/decompose ownership ────────────────────────

#[test]
fn split_by_plane_both_halves_usable() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, true);
    let (pos, neg) = cube.split_by_plane([1.0, 0.0, 0.0], 0.0);
    // Both halves should be independently usable.
    assert!(!pos.is_empty());
    assert!(!neg.is_empty());
    let pos_vol = pos.volume();
    let neg_vol = neg.volume();
    assert_relative_eq!(pos_vol + neg_vol, 1000.0, epsilon = 5.0);
    // Drop one half, other should still work.
    drop(pos);
    assert_relative_eq!(neg.volume(), neg_vol, epsilon = 0.01);
}

#[test]
fn decompose_parts_are_independent() {
    let a = Manifold::cube(5.0, 5.0, 5.0, false);
    let b = Manifold::cube(5.0, 5.0, 5.0, false).translate(20.0, 0.0, 0.0);
    let composed = Manifold::compose(&[a, b]);
    let parts = composed.decompose();
    assert_eq!(parts.len(), 2);
    // Drop composed, parts should still work.
    drop(composed);
    assert_relative_eq!(parts[0].volume(), 125.0, epsilon = 1.0);
    assert_relative_eq!(parts[1].volume(), 125.0, epsilon = 1.0);
}

// ── Remaining coverage: batch_hull ─────────────────────────────────────

#[test]
fn manifold_batch_hull() {
    let a = Manifold::cube(2.0, 2.0, 2.0, true);
    let b = Manifold::cube(2.0, 2.0, 2.0, true).translate(10.0, 0.0, 0.0);
    let hull = Manifold::batch_hull(&[a, b]);
    assert!(!hull.is_empty());
    // Hull of two separated cubes should be larger than either.
    assert!(hull.volume() > 8.0);
}

#[test]
fn cross_section_batch_hull() {
    let a = CrossSection::square(2.0, 2.0, true);
    let b = CrossSection::square(2.0, 2.0, true).translate(10.0, 0.0);
    let hull = CrossSection::batch_hull(&[a, b]);
    assert!(!hull.is_empty());
    assert!(hull.area() > 4.0);
}

// ── Remaining coverage: CrossSection rotate, mirror, bounds_rect2 ──────

#[test]
fn cross_section_rotate() {
    let cs = CrossSection::square(10.0, 2.0, true);
    let rotated = cs.rotate(90.0);
    // After 90-degree rotation, width and height should swap.
    let r = rotated.bounds();
    let dims = r.dimensions();
    assert_relative_eq!(dims[0], 2.0, epsilon = 0.1);
    assert_relative_eq!(dims[1], 10.0, epsilon = 0.1);
}

#[test]
fn cross_section_mirror() {
    let cs = CrossSection::square(10.0, 10.0, false);
    let mirrored = cs.mirror(1.0, 0.0); // mirror across Y axis
    let r = mirrored.bounds();
    // Original is [0,10] x [0,10]; mirrored across Y axis becomes [-10,0] x [0,10].
    assert!(r.min()[0] < 0.0);
}

#[test]
fn cross_section_bounds_rect2() {
    let cs = CrossSection::square(10.0, 6.0, false);
    let r = cs.bounds_rect2();
    assert_relative_eq!(r.min_x, 0.0, epsilon = 0.1);
    assert_relative_eq!(r.min_y, 0.0, epsilon = 0.1);
    assert_relative_eq!(r.max_x, 10.0, epsilon = 0.1);
    assert_relative_eq!(r.max_y, 6.0, epsilon = 0.1);
}

// ── Remaining coverage: quality globals ────────────────────────────────

#[test]
fn quality_globals_set_and_reset() {
    // These modify global state, so we reset at the end.
    reset_to_circular_defaults();

    set_circular_segments(42);
    let segs = get_circular_segments(1.0);
    assert_eq!(segs, 42);

    set_min_circular_angle(10.0);
    set_min_circular_edge_length(0.5);

    // After setting these, segment count for a given radius should change.
    let segs2 = get_circular_segments(10.0);
    assert!(segs2 > 0);

    reset_to_circular_defaults();
}

#[test]
fn reserve_ids_returns_incrementing() {
    let first = reserve_ids(5);
    let second = reserve_ids(5);
    // Second batch should start after first.
    assert!(second >= first + 5, "expected {second} >= {} + 5", first);
}

// ── Remaining coverage: Debug formatting ───────────────────────────────

#[test]
fn debug_formatting_manifold() {
    let cube = Manifold::cube(1.0, 1.0, 1.0, false);
    let dbg = format!("{cube:?}");
    assert!(dbg.contains("Manifold"));
    assert!(dbg.contains("num_vert"));
}

#[test]
fn debug_formatting_cross_section() {
    let cs = CrossSection::square(1.0, 1.0, false);
    let dbg = format!("{cs:?}");
    assert!(dbg.contains("CrossSection"));
}

#[test]
fn debug_formatting_bounding_box() {
    let bb = BoundingBox::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let dbg = format!("{bb:?}");
    assert!(dbg.contains("BoundingBox"));
}

#[test]
fn debug_formatting_rect() {
    let r = Rect::new([0.0, 0.0], [1.0, 1.0]);
    let dbg = format!("{r:?}");
    assert!(dbg.contains("Rect"));
}

// ── Remaining coverage: operator overloads ──────────────────────────────

#[test]
fn manifold_operator_add_union() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true).translate(5.0, 0.0, 0.0);
    let result = &a + &b;
    assert!(result.volume() > 1000.0);
    assert!(result.volume() < 2000.0);
}

#[test]
fn manifold_operator_sub_difference() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true);
    let result = &a - &b;
    assert!(result.is_empty() || result.volume() < 1.0);
}

#[test]
fn manifold_operator_xor_intersection() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true).translate(5.0, 0.0, 0.0);
    let result = &a ^ &b;
    assert!(result.volume() > 0.0);
    assert!(result.volume() < 1000.0);
}

#[test]
fn cross_section_operator_add() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(10.0, 10.0, true).translate(5.0, 0.0);
    let result = &a + &b;
    assert!(result.area() > 100.0);
}

#[test]
fn cross_section_operator_sub() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(10.0, 10.0, true);
    let result = &a - &b;
    assert!(result.is_empty() || result.area() < 1.0);
}

#[test]
fn cross_section_operator_xor() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(10.0, 10.0, true).translate(5.0, 0.0);
    let result = &a ^ &b;
    assert!(result.area() > 0.0);
    assert!(result.area() < 100.0);
}

// ── MeshGL/MeshGL64 advanced accessors ─────────────────────────────────

#[test]
fn meshgl_accessors_return_consistent_lengths() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let mesh = MeshGL::new(&verts, n_props, &indices);
    // merge vectors should be paired
    assert_eq!(mesh.merge_from_vert().len(), mesh.merge_to_vert().len());
    // face_id is either empty or has one per triangle
    let fid = mesh.face_id();
    assert!(fid.is_empty() || fid.len() == mesh.num_tri());
    // tangents are either empty or 4 floats per halfedge (3 halfedges per tri)
    let tang = mesh.halfedge_tangent();
    assert!(tang.is_empty() || tang.len() == mesh.num_tri() * 3 * 4);
}

#[test]
fn meshgl64_accessors_return_consistent_lengths() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = MeshGL64::new(&verts, n_props, &indices);
    assert_eq!(mesh.merge_from_vert().len(), mesh.merge_to_vert().len());
    let fid = mesh.face_id();
    assert!(fid.is_empty() || fid.len() == mesh.num_tri());
    let tang = mesh.halfedge_tangent();
    assert!(tang.is_empty() || tang.len() == mesh.num_tri() * 3 * 4);
}

#[test]
fn meshgl_run_accessors_consistent() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false).as_original();
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let mesh = MeshGL::new(&verts, n_props, &indices);
    let ri = mesh.run_index();
    let ro = mesh.run_original_id();
    let rt = mesh.run_transform();
    // run_index and run_original_id should have the same length
    assert_eq!(ri.len(), ro.len());
    // run_transform has 12 floats per run (4x3 matrix)
    assert!(rt.is_empty() || rt.len() == ri.len() * 12);
}

#[test]
fn meshgl64_run_accessors_consistent() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false).as_original();
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = MeshGL64::new(&verts, n_props, &indices);
    let ri = mesh.run_index();
    let ro = mesh.run_original_id();
    let rt = mesh.run_transform();
    assert_eq!(ri.len(), ro.len());
    assert!(rt.is_empty() || rt.len() == ri.len() * 12);
}

// ── Smooth constructors ────────────────────────────────────────────────

#[test]
fn smooth_f64_no_smoothness() {
    // With empty smoothness arrays, smooth should behave like from_mesh_f64.
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let result = Manifold::smooth_f64(&verts, n_props, &indices, &[], &[]);
    assert!(result.is_ok());
    let m = result.unwrap();
    assert!(!m.is_empty());
    assert_relative_eq!(m.volume(), 1000.0, epsilon = 1.0);
}

#[test]
fn smooth_f32_no_smoothness() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let result = Manifold::smooth_f32(&verts, n_props, &indices, &[], &[]);
    assert!(result.is_ok());
    let m = result.unwrap();
    assert!(!m.is_empty());
}

#[test]
fn smooth_mismatched_arrays_returns_error() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let result = Manifold::smooth_f64(&verts, n_props, &indices, &[0, 1], &[0.5]);
    assert!(result.is_err());
}

// ── CrossSection gaps ──────────────────────────────────────────────────

#[test]
fn cross_section_from_simple_polygon() {
    let points = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let cs = CrossSection::from_simple_polygon(&points, FillRule::EvenOdd);
    assert_relative_eq!(cs.area(), 100.0, epsilon = 0.1);
}

#[test]
fn cross_section_hull_simple_polygon() {
    // L-shape hull should be a convex polygon covering 100 sq units.
    let points = vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 5.0],
        [5.0, 5.0],
        [5.0, 10.0],
        [0.0, 10.0],
    ];
    let hull = CrossSection::hull_simple_polygon(&points);
    assert!(hull.area() >= 75.0); // at least the L-shape area
    assert!(hull.area() <= 100.5); // at most the bounding box
}

#[test]
fn cross_section_hull_polygons() {
    let polygons = vec![
        vec![[0.0, 0.0], [5.0, 0.0], [5.0, 5.0], [0.0, 5.0]],
        vec![[10.0, 10.0], [15.0, 10.0], [15.0, 15.0], [10.0, 15.0]],
    ];
    let hull = CrossSection::hull_polygons(&polygons);
    // Hull of two separated squares should be larger than either
    assert!(hull.area() > 25.0);
}

#[test]
fn cross_section_from_simple_polygon_empty() {
    let cs = CrossSection::from_simple_polygon(&[], FillRule::EvenOdd);
    assert!(cs.is_empty());
}

// ── Generic boolean ────────────────────────────────────────────────────

#[test]
fn manifold_boolean_union() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true).translate(5.0, 0.0, 0.0);
    let result = a.boolean(&b, OpType::Add);
    assert!(result.volume() > 1000.0);
    assert!(result.volume() < 2000.0);
}

#[test]
fn cross_section_boolean_subtract() {
    let a = CrossSection::square(10.0, 10.0, true);
    let b = CrossSection::square(5.0, 5.0, true);
    let result = a.boolean(&b, OpType::Subtract);
    assert_relative_eq!(result.area(), 75.0, epsilon = 0.1);
}

// ── MeshGL64 OBJ I/O ──────────────────────────────────────────────────

#[test]
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn meshgl64_obj_round_trip() {
    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let mesh = MeshGL64::new(&verts, n_props, &indices);
    let obj = mesh.to_obj();
    assert!(!obj.is_empty());
    assert!(obj.contains("v "));
    assert!(obj.contains("f "));

    let mesh2 = MeshGL64::from_obj(&obj).unwrap();
    assert!(mesh2.num_vert() > 0);
    assert!(mesh2.num_tri() > 0);
}

// ── MeshGL with tangents ───────────────────────────────────────────────

#[test]
fn meshgl_new_with_tangents() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let n_tris = indices.len() / 3;
    // 4 floats per halfedge, 3 halfedges per triangle
    let tangents = vec![0.0f32; n_tris * 3 * 4];
    let mesh = MeshGL::new_with_tangents(&verts, n_props, &indices, &tangents);
    assert_eq!(mesh.num_vert(), verts.len() / n_props);
    assert_eq!(mesh.num_tri(), n_tris);
    assert_eq!(mesh.halfedge_tangent().len(), tangents.len());
}

#[test]
fn meshgl64_new_with_tangents() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f64();
    let n_tris = indices.len() / 3;
    let tangents = vec![0.0f64; n_tris * 3 * 4];
    let mesh = MeshGL64::new_with_tangents(&verts, n_props, &indices, &tangents);
    assert_eq!(mesh.num_vert(), verts.len() / n_props);
    assert_eq!(mesh.num_tri(), n_tris);
    assert_eq!(mesh.halfedge_tangent().len(), tangents.len());
}

// ── Test gap coverage (from deep review W5) ────────────────────────────

#[test]
fn from_sdf_seq_sphere() {
    let sphere = Manifold::from_sdf_seq(
        |x, y, z| (x * x + y * y + z * z).sqrt() - 5.0,
        ([-6.0, -6.0, -6.0], [6.0, 6.0, 6.0]),
        0.5,
        0.0,
        0.001,
    );
    assert!(!sphere.is_empty());
    assert!(sphere.volume() > 200.0);
}

#[test]
fn to_mesh_f64_with_normals() {
    let sphere = Manifold::sphere(5.0, 32).calculate_normals(3, 60.0);
    let (verts, n_props, _) = sphere.to_mesh_f64_with_normals(3);
    assert!(!verts.is_empty());
    assert!(n_props >= 3);
}

#[test]
fn to_mesh_f32_with_normals() {
    let sphere = Manifold::sphere(5.0, 32).calculate_normals(3, 60.0);
    let (verts, n_props, _) = sphere.to_mesh_f32_with_normals(3);
    assert!(!verts.is_empty());
    assert!(n_props >= 3);
}

#[test]
fn cross_section_num_contour() {
    let square = CrossSection::square(10.0, 10.0, false);
    assert!(square.num_contour() >= 1);
    let empty = CrossSection::empty();
    assert_eq!(empty.num_contour(), 0);
}

#[test]
fn rect_transform() {
    let r = Rect::new([0.0, 0.0], [10.0, 10.0]);
    // Identity-ish transform (translate by 5, 5)
    let t = r.transform(&[1.0, 0.0, 0.0, 1.0, 5.0, 5.0]);
    let min = t.min();
    assert_relative_eq!(min[0], 5.0, epsilon = 0.01);
    assert_relative_eq!(min[1], 5.0, epsilon = 0.01);
}

#[test]
fn zero_size_cube() {
    let c = Manifold::cube(0.0, 0.0, 0.0, false);
    assert!(c.is_empty());
}

#[test]
fn zero_radius_sphere() {
    let s = Manifold::sphere(0.0, 16);
    assert!(s.is_empty());
}

#[test]
fn manifold_boolean_subtract_and_intersect() {
    let a = Manifold::cube(10.0, 10.0, 10.0, true);
    let b = Manifold::cube(10.0, 10.0, 10.0, true).translate(5.0, 0.0, 0.0);
    let sub = a.boolean(&b, OpType::Subtract);
    assert!(sub.volume() > 0.0);
    assert!(sub.volume() < 1000.0);
    let inter = a.boolean(&b, OpType::Intersect);
    assert!(inter.volume() > 0.0);
    assert!(inter.volume() < 1000.0);
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn meshgl_is_send() {
    let cube = Manifold::cube(5.0, 5.0, 5.0, false);
    let (verts, n_props, indices) = cube.to_mesh_f32();
    let mesh = MeshGL::new(&verts, n_props, &indices);
    let handle = std::thread::spawn(move || {
        assert!(mesh.num_vert() > 0);
    });
    handle.join().unwrap();
}

#[test]
fn invalid_mesh_returns_manifold_status_error() {
    // Degenerate triangle indices pointing at the same vertex
    let verts = [0.0f64, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let tris = [0u64, 0, 0]; // degenerate: all same vertex
    let result = Manifold::from_mesh_f64(&verts, 3, &tris);
    // Should either succeed with an empty/degenerate manifold or return an error
    if let Ok(m) = result {
        assert!(m.is_empty() || m.volume().abs() < 0.001);
    }
}

// ── wasm-specific battle-readiness smoke tests ─────────────────────────
//
// Both tests run on every target — they're not wasm-gated. The point on
// host targets is "still pass after refactors". The point on wasm32 is
// "validate the C++ exception runtime and the wasm memory.grow path
// actually work as configured by build.rs's link flags". If either of
// these traps the wasm module instead of completing, our exception or
// memory configuration is wrong.

#[test]
fn wasm_smoke_throw_path_returns_error_not_trap() {
    // Pass an out-of-bounds triangle index. The C++ kernel must detect
    // this and either return a degenerate manifold or surface an error
    // status. On wasm with -fwasm-exceptions correctly set at both
    // compile and link, internal C++ throws are caught by the C wrapper
    // and translated to status codes. If -fwasm-exceptions is missing
    // or mismatched between compile and link, this would trap-and-abort
    // the wasm module instead.
    let verts = [0.0f64, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let tris = [0u64, 1, 999]; // index 999 is out of bounds (only 3 verts)
    let result = Manifold::from_mesh_f64(&verts, 3, &tris);
    // Pass condition: we got *some* result back without trapping.
    // Either Ok(empty/degenerate) or Err(status) is fine — both prove
    // the error path didn't crash the wasm runtime.
    if let Ok(m) = result {
        assert!(m.is_empty() || m.volume().abs() < 0.001);
    }
}

#[test]
fn wasm_smoke_memory_growth_path() {
    // Allocate enough mesh data to push the wasm linear memory past its
    // initial size (we set INITIAL_MEMORY=64 MiB and ALLOW_MEMORY_GROWTH=1
    // in build.rs). 30 spheres at 32 segments each = ~30k triangles total
    // before the union; intermediate boolean state pushes much higher.
    // If memory growth isn't wired correctly, this traps with OOM.
    let parts: Vec<_> = (0..30)
        .map(|i| Manifold::sphere(2.0, 32).translate(i as f64 * 5.0, 0.0, 0.0))
        .collect();
    let combined = Manifold::batch_union(&parts);
    assert!(!combined.is_empty());
    // Sanity check: 30 disjoint spheres should have nontrivial volume.
    // 4/3 * pi * r^3 = 33.51 per sphere, * 30 ≈ 1005, allow some slack.
    assert!(combined.volume() > 800.0);
}

// ── ExecutionContext tests ──────────────────────────────────────────

#[test]
fn execution_context_initial_state() {
    let ctx = manifold_csg::ExecutionContext::new();
    assert!(!ctx.is_cancelled());
    assert_eq!(ctx.progress(), 0.0);
}

#[test]
fn execution_context_cancel_is_sticky() {
    let ctx = manifold_csg::ExecutionContext::new();
    assert!(!ctx.is_cancelled());
    ctx.cancel();
    assert!(ctx.is_cancelled());
    // Sticky: still cancelled on subsequent reads.
    assert!(ctx.is_cancelled());
}

#[test]
#[cfg_attr(
    target_os = "emscripten",
    ignore = "default build has no pthreads (-pthread requires SharedArrayBuffer + COOP/COEP from host)"
)]
fn execution_context_cross_thread_cancel() {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let ctx = Arc::new(manifold_csg::ExecutionContext::new());
    let cancel = Arc::clone(&ctx);

    let handle = thread::spawn(move || {
        // Trivially small sleep — we just want to prove cancel from another
        // thread becomes visible to the original via shared upstream state.
        thread::sleep(Duration::from_millis(5));
        cancel.cancel();
    });

    handle.join().unwrap();
    assert!(ctx.is_cancelled());
}

#[test]
fn manifold_status_with_context_no_cancel() {
    use manifold_csg_sys::ManifoldError;
    let cube = Manifold::cube(1.0, 1.0, 1.0, true);
    let ctx = manifold_csg::ExecutionContext::new();
    // Trivial Manifold; evaluation finishes immediately.
    assert_eq!(cube.status_with_context(&ctx), ManifoldError::NoError);
}

#[test]
fn manifold_status_with_context_already_cancelled() {
    let cube = Manifold::cube(1.0, 1.0, 1.0, true);
    let ctx = manifold_csg::ExecutionContext::new();
    ctx.cancel();
    // We don't assert on the specific status code — upstream may surface
    // cancellation as NoError for trivial work that doesn't poll the flag,
    // or as a specific cancellation status. We just want to prove the call
    // is well-formed and doesn't panic / leak / crash.
    let _ = cube.status_with_context(&ctx);
}
