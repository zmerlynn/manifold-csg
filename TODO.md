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

- [ ] MeshGL/MeshGL64 construction with full options (`w_options` — merge vectors + run data)

## Documentation & Publishing

## Ergonomics (nice-to-have)

- [ ] Extract vec-building helper to reduce boilerplate in batch operations
