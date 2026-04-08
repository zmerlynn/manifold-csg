//! Basic 3D CSG operations: primitives, booleans, transforms, and queries.
//!
//! Run with: `cargo run -p manifold-csg --example basics`

use manifold_csg::Manifold;

fn main() {
    // -- Primitives ----------------------------------------------------------

    let cube = Manifold::cube(20.0, 20.0, 20.0, true); // centered at origin
    let sphere = Manifold::sphere(12.0, 64);
    let cylinder = Manifold::cylinder(30.0, 5.0, 5.0, 32, true);

    println!("Cube:     volume={:.1}, verts={}", cube.volume(), cube.num_vert());
    println!("Sphere:   volume={:.1}, verts={}", sphere.volume(), sphere.num_vert());
    println!("Cylinder: volume={:.1}, verts={}", cylinder.volume(), cylinder.num_vert());

    // -- Boolean operations --------------------------------------------------
    // Operator overloads: + (union), - (difference), ^ (intersection)

    let union = &cube + &sphere;
    let difference = &cube - &sphere;
    let intersection = &cube ^ &sphere;

    println!("\nCube + Sphere (union):        volume={:.1}", union.volume());
    println!("Cube - Sphere (difference):  volume={:.1}", difference.volume());
    println!("Cube ^ Sphere (intersection): volume={:.1}", intersection.volume());

    // You can also call the methods directly:
    let _same_union = cube.union(&sphere);

    // -- Transforms ----------------------------------------------------------

    let translated = cube.translate(10.0, 0.0, 0.0);
    let rotated = cube.rotate(0.0, 0.0, 45.0);
    let scaled = cube.scale(1.0, 2.0, 1.0);
    let mirrored = cube.mirror([1.0, 0.0, 0.0]); // mirror across YZ plane

    println!("\nTranslated volume: {:.1}", translated.volume());
    println!("Rotated volume:    {:.1}", rotated.volume());
    println!("Scaled volume:     {:.1}", scaled.volume());
    println!("Mirrored volume:   {:.1}", mirrored.volume());

    // -- Queries -------------------------------------------------------------

    let drilled = &cube - &cylinder;
    println!("\nDrilled cube:");
    println!("  volume:       {:.1}", drilled.volume());
    println!("  surface area: {:.1}", drilled.surface_area());
    println!("  genus:        {}", drilled.genus());
    println!("  vertices:     {}", drilled.num_vert());
    println!("  triangles:    {}", drilled.num_tri());
    println!("  edges:        {}", drilled.num_edge());

    if let Some(bb) = drilled.bounding_box() {
        let min = bb.min();
        let max = bb.max();
        println!("  bbox min:     [{:.1}, {:.1}, {:.1}]", min[0], min[1], min[2]);
        println!("  bbox max:     [{:.1}, {:.1}, {:.1}]", max[0], max[1], max[2]);
    }

    // -- Batch operations ----------------------------------------------------

    let parts: Vec<_> = (0..5)
        .map(|i| Manifold::sphere(3.0, 32).translate(i as f64 * 8.0, 0.0, 0.0))
        .collect();
    let combined = Manifold::batch_union(&parts);
    println!("\nBatch union of 5 spheres: volume={:.1}", combined.volume());

    // -- Mesh round-trip (f64) -----------------------------------------------

    let original = Manifold::cube(10.0, 10.0, 10.0, false);
    let (vert_props, n_props, tri_indices) = original.to_mesh_f64();
    let rebuilt = Manifold::from_mesh_f64(&vert_props, n_props, &tri_indices).unwrap();
    println!(
        "\nMesh round-trip: original volume={:.1}, rebuilt volume={:.1}",
        original.volume(),
        rebuilt.volume(),
    );

    // -- Convex hull ---------------------------------------------------------

    let hull = Manifold::hull_pts(&[
        [0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0],
        [0.0, 10.0, 0.0],
        [0.0, 0.0, 10.0],
        [5.0, 5.0, 5.0], // interior point, ignored by hull
    ]);
    println!("\nConvex hull of 5 points: volume={:.1}", hull.volume());
}
