---
name: publish
description: Publish crates to crates.io with pre-flight checks
user-invocable: true
---

# Publish

Publish our 4 crates to crates.io with pre-flight validation. The workspace
has two canonical crates (`manifold-csg-sys`, `manifold-csg`) and two thin
facade re-exports (`manifold3d-sys`, `manifold3d`) under the same version
numbers. Facades always ship in lockstep with their canonical counterparts
via `=` version pins.

## Arguments

- No arguments: publish all 4 crates in dependency order
- `--dry-run`: run all checks but don't actually publish

## Pre-flight checks

Run all checks before publishing. Stop on first failure.

### 1. Clean working tree

```
git status
```

Must be on `main` with no uncommitted changes. Refuse to publish from a feature branch.

### 2. Build and test

```
cargo test --features nalgebra
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Version bump

Version bumps happen at publish time, not in feature PRs. Read CLAUDE.md for the versioning scheme, then:

- Check if versions have already been bumped (breaking PRs may have bumped to pass semver CI).
- If not already bumped:
  - **`manifold-csg-sys`**: bump the patch component (e.g., `3.4.102` → `3.4.103`) whenever the upstream contents differ from the last publish — pinned commit changed, carry-patches added/removed, or FFI declarations changed. If the upstream major.minor changed (new manifold3d release), update major.minor accordingly.
  - **`manifold-csg`** (workspace version): bump patch for additive changes, minor for breaking changes (pre-1.0, minor bumps can break).
  - **`manifold3d-sys`** version must match the new `manifold-csg-sys` version exactly (= pin).
  - **`manifold3d`** inherits the workspace version, so bumps automatically. Its `manifold-csg` dependency `=` pin must be updated to the new workspace version.
  - Update `manifold-csg`'s `manifold-csg-sys` dependency version to match. Update `manifold3d-sys`'s `manifold-csg-sys` `=` pin and `manifold3d`'s `manifold-csg` `=` pin.
- Commit the version bumps on a branch, open a PR, and get it merged before proceeding. Branch protection requires this — you cannot push version bumps directly to main.

### 4. Carry-patch audit

For each patch in `crates/manifold-csg-sys/patches/`:
- Check if the upstream PR has been merged (use `gh api`)
- If merged AND included in our pinned commit, warn that the patch can be removed
- If the patch fails to apply during build, stop — the pin and patches are inconsistent

### 5. Changelog / release notes

Check if there's a changelog or release tag. Remind the user to tag the release after publishing:
```
git tag -a v<version> -m "Release <version>"
git push origin v<version>
```

### 6. Dry run

Run a dry run for each crate in publish order:

```
cargo publish --dry-run -p manifold-csg-sys
cargo publish --dry-run -p manifold3d-sys
cargo publish --dry-run -p manifold-csg
cargo publish --dry-run -p manifold3d
```

Facade dry runs will fail if their canonical counterparts aren't already on crates.io at the expected version — this is expected for first publish and version bumps. The real dry-run validation for facades happens after their canonical is published.

For each crate, check:
- **Crate size** — warn if over 500KB (we don't vendor C++ source, so it should be small)
- **File list** — `cargo package --list -p <crate>` — verify no secrets, build artifacts, or unnecessary files are included
- **License files** — `LICENSE-APACHE` and `LICENSE-MIT` must be present

### 7. docs.rs build simulation

```
DOCS_RS=1 cargo doc --no-deps --features nalgebra \
  -p manifold-csg-sys -p manifold-csg -p manifold3d-sys -p manifold3d
```

Must build without errors. docs.rs runs with `--network=none`, so `build.rs` must detect `DOCS_RS` and skip the C clone/build.

### 8. API stability check

Before first publish or any version where the public API changed:
- List all `pub` items in `manifold-csg` — are any provisional or likely to change?
- Flag return types that lock us in (e.g., `Vec<f64>` vs a newtype)
- Remind: once published, the API is semver-locked. For pre-1.0 crates (`0.x.y`), minor bumps (`0.1.0` → `0.2.0`) can contain breaking changes, but patch bumps (`0.1.0` → `0.1.1`) must be backwards-compatible. Post-1.0, breaking changes require a major bump.

### 9. Documentation

```
cargo doc --features nalgebra --no-deps
```

Must build without warnings. Spot-check that crate-level docs render correctly.

## Publish

Only after all checks pass. **Always get explicit user confirmation before running `cargo publish`.**

### Order matters

Dependencies must reach crates.io before their dependents:

1. `manifold-csg-sys` (no internal deps)
2. `manifold3d-sys` (facade of `manifold-csg-sys`)
3. `manifold-csg` (depends on `manifold-csg-sys`)
4. `manifold3d` (facade of `manifold-csg`)

Between each publish, crates.io may take a moment to index the new version. If the next publish fails with "no matching package", wait and retry.

```
cargo publish -p manifold-csg-sys
cargo publish -p manifold3d-sys
cargo publish -p manifold-csg
cargo publish -p manifold3d
```

### Post-publish

- Tag the release: `git tag -a v<version> -m "Release <version>"` (using the `manifold-csg` version number — facades share the workspace version)
- Push the tag: `git push origin v<version>`
- Verify on crates.io that all 4 crates appear at the new version
- Verify docs.rs builds succeed for all 4

## Rules

- **NEVER publish without explicit user confirmation** — dry run is the default mindset
- Do NOT publish from a dirty working tree or a non-main branch
- Do NOT publish if tests or clippy fail
- If any publish succeeds but a later one fails, report this clearly — published versions can only be yanked, not deleted. Common failure: facade fails because its canonical isn't indexed yet; retry after a delay.
- Sys crate patch bumps must be semver-compatible (additions only)
- Facade crates must always share the exact version of their canonical counterpart (`=` pin in Cargo.toml enforces this)
