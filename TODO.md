# TODO

## Safe wrapper gaps (low priority)

- [ ] MeshGL/MeshGL64 construction with full options (`w_options` — merge vectors + run data)

## Ergonomics (nice-to-have)

- [ ] Extract vec-building helper to reduce boilerplate in batch operations (`batch_boolean` and `batch_hull` in Manifold and CrossSection, `compose` in Manifold)

## FFI robustness

- [ ] ABI gate in CI. `check-api-coverage.py` compares function *names* between `manifoldc.h` and the sys crate, not signatures, struct layouts or enum values, and the sys crate never includes the C header, so a same-name upstream change (reordered or retyped parameters, a new struct field) would compile and pass. Add a clang-AST comparison of every declaration plus a compiled C/Rust size/alignment/offset/discriminant probe, run at the pin and in `next-upstream.yml` against master. (A codex review of `next` built such a comparator by hand and found the current 254 declarations match.)
- [ ] RAII guards for intermediate C handles. Paths that hold several raw handles before the final owner exists (polygon construction in `PolygonsFfi` and `from_simple_polygon*`, the level-set box, fill-rule constructors) leak the earlier ones if a later `crate::owned()` panics on null. Upstream allocates with throwing `new`, so null does not happen today.
