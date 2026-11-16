//! Tests that change process-global quality settings. They live in their own
//! test binary (a separate process) so they cannot race the geometry tests in
//! integration.rs, which assume the defaults. Within this file they run under
//! one lock.

use std::sync::Mutex;

static GLOBALS: Mutex<()> = Mutex::new(());

#[test]
fn relative_precision_round_trips() {
    let _guard = GLOBALS.lock().unwrap();
    manifold_csg::set_relative_precision(1e-6);
    assert_eq!(manifold_csg::get_relative_precision(), 1e-6);
    manifold_csg::set_relative_precision(1e-12);
}

#[test]
fn reset_to_circular_defaults_also_resets_precision() {
    // Upstream's ResetToDefaults resets every quality global, not just the
    // circular ones. Documented on reset_to_circular_defaults.
    let _guard = GLOBALS.lock().unwrap();
    manifold_csg::set_relative_precision(1e-6);
    manifold_csg::reset_to_circular_defaults();
    assert_eq!(manifold_csg::get_relative_precision(), 1e-12);
}
