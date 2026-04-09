# TODO

## Skills

- [ ] `/release` skill — automate crate publishing. Should handle:
  - Version bumping (with sys crate versioning scheme: `{upstream_major}.{upstream_minor}.{100+N}`)
  - Publishing order (sys crate first, then safe crate)
  - `cargo publish --dry-run` verification before actual publish
  - Git tagging (`v0.1.0` for safe crate, `sys-v3.4.100` for sys crate)
  - Changelog generation from git log

- [ ] `/update-upstream` skill — bump the manifold3d pin. Should handle:
  - Updating the git tag in `build.rs`
  - Diffing the C header (`manifoldc.h`) for new/changed/removed functions
  - Updating `manifold-csg-sys` version to match new upstream
  - Updating `manifold-csg` dependency on sys crate
  - Running tests to verify nothing broke

## Safe wrapper gaps (low priority)

- [ ] MeshGL/MeshGL64 advanced accessors (run_index, face_id, tangents)
- [ ] MeshGL/MeshGL64 `merge()` — wrappable but needs care: on failure, the C API returns the *input* pointer instead of the output buffer (aliased ownership → double-free). Safe wrapper must check `returned_ptr == input_ptr` and handle accordingly.
- [ ] `manifold_smooth` / `manifold_smooth64` constructors (from half-edge indices)
- [ ] `manifold_level_set_seq` — sequential SDF for single-threaded runtimes
- [ ] `manifold_get_meshgl_w_normals` / `manifold_get_meshgl64_w_normals` — mesh export with normals

## Documentation & Publishing

- [ ] Re-enable `semver` CI job once manifold-csg is published on crates.io (disabled via `if: false` in `ci.yml`)
- [ ] README badges (crates.io version, docs.rs, CI status) — add once published
- [ ] Make doc-tests runnable (currently `rust,ignore`) — runnable examples exist in `examples/` but inline doc examples still need `rust,ignore` due to build infra requirements

## Ergonomics (nice-to-have)

- [ ] Extract vec-building helper to reduce boilerplate in batch operations
