# TODO

## Safe wrapper gaps (low priority)

- [ ] MeshGL/MeshGL64 construction with full options (`w_options` — merge vectors + run data)

## Ergonomics (nice-to-have)

- [ ] Extract vec-building helper to reduce boilerplate in batch operations (`batch_boolean` and `batch_hull` in Manifold and CrossSection, `compose` in Manifold)

## FFI robustness

- [ ] RAII guards for intermediate C handles. Paths that hold several raw handles before the final owner exists (polygon construction in `PolygonsFfi` and `from_simple_polygon*`, the level-set box, fill-rule constructors) leak the earlier ones if a later `crate::owned()` panics on null. Upstream allocates with throwing `new`, so null does not happen today.
