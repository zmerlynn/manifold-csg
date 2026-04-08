//! Advanced operations: SDF, warp, OBJ I/O, properties, and threading.
//!
//! Run with: `cargo run -p manifold-csg --example advanced`

use manifold_csg::Manifold;

fn main() {
    // -- SDF (signed distance function) --------------------------------------
    // Construct geometry from a mathematical function.

    let sdf_sphere = Manifold::from_sdf(
        |x, y, z| (x * x + y * y + z * z).sqrt() - 5.0, // sphere of radius 5
        ([-6.0, -6.0, -6.0], [6.0, 6.0, 6.0]),           // bounding box
        0.5,                                                // edge length
        0.0,                                                // isosurface level
        0.01,                                               // tolerance
    );
    println!(
        "SDF sphere: volume={:.1} (expected ~{:.1})",
        sdf_sphere.volume(),
        4.0 / 3.0 * std::f64::consts::PI * 125.0, // 4/3 * pi * r^3
    );

    // -- Warp (vertex deformation) -------------------------------------------
    // Twist a cube around the Z axis.

    let cube = Manifold::cube(10.0, 10.0, 30.0, true).refine(4);
    let twisted = cube.warp(|x, y, z| {
        let angle = z * 0.1; // twist increases with height
        let cos = angle.cos();
        let sin = angle.sin();
        [x * cos - y * sin, x * sin + y * cos, z]
    });
    println!("Twisted cube: volume={:.1}", twisted.volume());

    // -- OBJ round-trip ------------------------------------------------------

    let original = Manifold::sphere(5.0, 32);
    let obj_string = original.to_obj();
    println!(
        "\nOBJ export: {} bytes, first line: {:?}",
        obj_string.len(),
        obj_string.lines().next().unwrap_or(""),
    );

    let reimported = Manifold::from_obj(&obj_string).unwrap();
    println!(
        "OBJ round-trip: original volume={:.1}, reimported volume={:.1}",
        original.volume(),
        reimported.volume(),
    );

    // -- Custom vertex properties --------------------------------------------
    // Add UV coordinates as vertex properties.

    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let with_uvs = cube.set_properties(5, |new_props, pos, _old| {
        // First 3 properties are position (x, y, z)
        new_props[0] = pos[0];
        new_props[1] = pos[1];
        new_props[2] = pos[2];
        // Properties 3-4 are UV, derived from position
        new_props[3] = pos[0] / 10.0;
        new_props[4] = pos[1] / 10.0;
    });
    println!(
        "\nCube with UVs: num_prop={} (was {})",
        with_uvs.num_prop(),
        cube.num_prop(),
    );

    // -- Normals and curvature -----------------------------------------------

    let sphere = Manifold::sphere(5.0, 64);
    let with_normals = sphere.calculate_normals(3, 60.0);
    let with_curvature = sphere.calculate_curvature(3, 4);
    println!(
        "Sphere with normals: num_prop={}",
        with_normals.num_prop(),
    );
    println!(
        "Sphere with curvature: num_prop={}",
        with_curvature.num_prop(),
    );

    // -- Send across threads -------------------------------------------------
    // Manifold is Send, so you can build geometry on worker threads.

    let handle = std::thread::spawn(|| {
        let a = Manifold::cube(10.0, 10.0, 10.0, true);
        let b = Manifold::sphere(7.0, 32);
        let result = &a - &b;
        result.volume()
    });
    let volume = handle.join().unwrap();
    println!("\nBuilt on another thread: volume={:.1}", volume);

    // -- Smoothing -----------------------------------------------------------

    let box_mesh = Manifold::cube(10.0, 10.0, 10.0, true);
    let smoothed = box_mesh
        .calculate_normals(3, 60.0)
        .smooth_by_normals(3)
        .refine(4);
    println!(
        "Smoothed cube: volume={:.1}, verts={}",
        smoothed.volume(),
        smoothed.num_vert(),
    );

    // -- Decompose into components -------------------------------------------

    let a = Manifold::sphere(3.0, 32).translate(-10.0, 0.0, 0.0);
    let b = Manifold::sphere(3.0, 32).translate(10.0, 0.0, 0.0);
    let composed = Manifold::compose(&[a, b]);
    let parts = composed.decompose();
    println!(
        "\nComposed 2 spheres -> decompose -> {} parts",
        parts.len(),
    );
}
