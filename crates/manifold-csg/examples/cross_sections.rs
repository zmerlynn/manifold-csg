//! 2D cross-section operations and extrusion to 3D.
//!
//! Run with: `cargo run -p manifold-csg --example cross_sections`

use manifold_csg::{CrossSection, JoinType, Manifold};

fn main() {
    // -- 2D primitives -------------------------------------------------------

    let square = CrossSection::square(20.0, 20.0, true);
    let circle = CrossSection::circle(10.0, 64);

    println!(
        "Square: area={:.1}, verts={}",
        square.area(),
        square.num_vert()
    );
    println!(
        "Circle: area={:.1}, verts={}",
        circle.area(),
        circle.num_vert()
    );

    // -- 2D booleans ---------------------------------------------------------
    // Same operators as Manifold: + (union), - (difference), ^ (intersection)

    let with_hole = &square - &circle;
    println!("\nSquare - Circle: area={:.1}", with_hole.area());

    // -- Geometric offset (Clipper2) -----------------------------------------
    // Positive delta grows the shape, negative delta shrinks it.

    let rounded = square.offset(3.0, JoinType::Round, 2.0, 32);
    let shrunk = square.offset(-2.0, JoinType::Round, 2.0, 32);

    println!("\nOffset +3 (round): area={:.1}", rounded.area());
    println!("Offset -2 (round): area={:.1}", shrunk.area());

    // -- Custom polygon ------------------------------------------------------

    let l_shape = CrossSection::from_polygons(&[vec![
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 10.0],
        [10.0, 10.0],
        [10.0, 20.0],
        [0.0, 20.0],
    ]]);
    println!("\nL-shape: area={:.1}", l_shape.area());

    // -- Extrude to 3D -------------------------------------------------------

    let extruded = l_shape.extrude(15.0);
    println!("L-shape extruded: volume={:.1}", extruded.volume());

    // Extrude with twist and taper
    let fancy = Manifold::extrude_with_options(
        &circle, 30.0,  // height
        20,    // slices (more = smoother twist)
        180.0, // twist degrees
        0.5,   // scale_x at top
        0.5,   // scale_y at top
    );
    println!("Twisted/tapered cylinder: volume={:.1}", fancy.volume());

    // -- Revolve (solid of revolution) ---------------------------------------

    // Create an L-shaped profile and revolve it around Y
    let profile = CrossSection::from_polygons(&[vec![
        [5.0, 0.0],
        [10.0, 0.0],
        [10.0, 5.0],
        [7.0, 5.0],
        [7.0, 2.0],
        [5.0, 2.0],
    ]]);
    let revolved = Manifold::revolve(&profile, 64, 360.0);
    println!("Revolved L-profile: volume={:.1}", revolved.volume());

    // -- Slicing (3D back to 2D) ---------------------------------------------

    let cube = Manifold::cube(10.0, 10.0, 10.0, false);
    let slice = cube.slice_to_cross_section(5.0); // slice at z=5
    println!("\nCube sliced at z=5: area={:.1}", slice.area());

    // -- 2D hull -------------------------------------------------------------

    let a = CrossSection::circle(5.0, 32).translate(0.0, 0.0);
    let b = CrossSection::circle(5.0, 32).translate(20.0, 0.0);
    let hull = CrossSection::batch_hull(&[a, b]);
    println!("Hull of two circles: area={:.1}", hull.area());

    // -- Warp (vertex deformation) -------------------------------------------

    let warped = square.warp(|x, y| {
        // Pinch the shape toward the center
        [x * 0.8, y * 0.8]
    });
    println!("\nWarped square: area={:.1}", warped.area());
}
