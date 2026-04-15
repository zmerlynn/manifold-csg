---
name: publish
description: Publish crates to crates.io with pre-flight checks
user-invocable: true
---

# Publish

Publish `manifold-csg-sys` and/or `manifold-csg` to crates.io with pre-flight validation.

## Arguments

- No arguments: publish both crates (sys first, then safe)
- `sys`: publish only `manifold-csg-sys`
- `safe`: publish only `manifold-csg`
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
cargo clippy --all-targets --features nalgebra -- -D warnings
```

### 3. Version bump

Version bumps happen at publish time, not in feature PRs. Read CLAUDE.md for the versioning scheme, then:

- Check if versions have already been bumped (breaking PRs may have bumped to pass semver CI).
- If not already bumped:
  - **sys crate**: bump the patch component (e.g., `3.4.102` → `3.4.103`) whenever the upstream contents differ from the last publish — pinned commit changed, carry-patches added/removed, or FFI declarations changed. If the upstream major.minor changed (new manifold3d release), update major.minor accordingly.
  - **safe crate**: bump patch for additive changes, minor for breaking changes (pre-1.0, minor bumps can break).
  - Update the safe crate's `manifold-csg-sys` dependency version to match the new sys version.
- Commit the version bumps before proceeding to dry run.

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

```
cargo publish --dry-run -p manifold-csg-sys
cargo publish --dry-run -p manifold-csg
```

Both must succeed. Check the output for:
- **Crate size** — warn if over 500KB (we don't vendor C++ source, so it should be small)
- **File list** — `cargo package --list -p <crate>` — verify no secrets, build artifacts, or unnecessary files are included
- **License files** — `LICENSE-APACHE` and `LICENSE-MIT` must be present

### 7. docs.rs build simulation

```
DOCS_RS=1 cargo doc --no-deps --features nalgebra -p manifold-csg-sys -p manifold-csg
```

Must build without errors. docs.rs runs with `--network=none`, so `build.rs` must detect `DOCS_RS` and skip the C clone/build. This catches the failure mode where docs.rs can't build our crate.

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

1. **Publish sys crate first** — the safe crate depends on it via `version = "x.y.z"`, and crates.io must have it before the safe crate can resolve.
2. **Wait for crates.io indexing** — there can be a brief delay. Retry the safe crate publish if it fails with "no matching package" on the first attempt.
3. **Publish safe crate**.

```
cargo publish -p manifold-csg-sys
# wait a moment for indexing
cargo publish -p manifold-csg
```

### Post-publish

- Tag the release: `git tag -a v<safe-version> -m "Release <safe-version>"`
- Push the tag: `git push origin v<safe-version>`
- Verify on crates.io that both crates appear and docs.rs builds succeed

## Rules

- **NEVER publish without explicit user confirmation** — dry run is the default mindset
- Do NOT publish from a dirty working tree or a non-main branch
- Do NOT publish if tests or clippy fail
- If the sys crate publish succeeds but the safe crate fails, report this clearly — the sys crate is already live and can't be unpublished (only yanked)
- Sys crate patch bumps must be semver-compatible (additions only)
