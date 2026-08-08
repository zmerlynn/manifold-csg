# manifold-csg

Safe Rust bindings to the [manifold3d](https://github.com/elalish/manifold) geometry kernel.

## Structure

- `crates/manifold-csg-sys/` — Raw C FFI bindings (`links = "manifold"`)
- `crates/manifold-csg/` — Safe Rust wrapper (the primary public API)
- `crates/manifold-csg-sys/wasm32-uu/` — Vendored helper files (config_site, mutex stub, libcxx-extras.cpp, iostream-stripping patches) used only when building for `wasm32-unknown-unknown`. See [`docs/plans/wasm-unknown-unknown.md`](docs/plans/wasm-unknown-unknown.md).
- `crates/manifold-csg-playground/` — Browser-based demo (`publish = false`); also serves as a real-world consumer test for the `wasm32-unknown-unknown` build path. cdylib that exposes a tiny C ABI to a three.js frontend (`web/`) for interactively booleaning two primitives, with Node-side unit tests under `tests/` covering the wasm ABI and the JS/three.js glue.
- `docs/plans/` — Design docs for in-flight or speculative work (e.g. new target support, large refactors). Lives in the repo so it travels with branches and stays reviewable; preferred over scattered GitHub issue prose for anything bigger than a paragraph.

## Build

The sys crate clones manifold3d (currently pinned to v3.5.2) via git and builds with cmake. Requires:
- git, cmake, a C++ compiler
- First build is slow (clones + compiles manifold3d); subsequent builds are cached

### Offline / bring-your-own manifold (issue #49)

For sandboxed builders (Nix, airgapped CI) that can't run the build script's `git clone`, an env-var escape hatch bypasses the fetch.

- **`MANIFOLD_CSG_LIB_DIR`** - link a pre-built manifold install (dir containing `libmanifoldc` + `libmanifold`); skips clone **and** cmake. Fully offline, no C++ compile. Optional `MANIFOLD_CSG_LIB_KIND` (`dylib` default, or `static`). Works because our FFI is hand-written `extern` decls (no bindgen), so only libraries are needed - no headers. The caller owns version/flag matching against `MANIFOLD_VERSION`. `dylib` needs the lib dir on the runtime search path (rpath); Nix `buildInputs = [ manifold ]` handles this automatically.

A source-tree override (`MANIFOLD_CSG_SOURCE_DIR`, skip clone but still cmake) is deferred because our `build/build.rs` hardcodes builtin Clipper2/TBB, which `FetchContent`-clones at configure time, so it wouldn't be truly offline without a system-deps lever. Two things make that lever (`MANIFOLD_CSG_SYSTEM_DEPS`, flipping `MANIFOLD_USE_BUILTIN_CLIPPER2/TBB=OFF`) more than a flag: if the system Clipper2 isn't found, manifold's cmake silently forces builtin back on and `FetchContent`s anyway (so the "offline" build quietly hits the network), while TBB just goes missing and parallel breaks; and with system deps the link set becomes shared system libs rather than archives under `build_dir`, so `find_lib_recursive` + `static=` link lines need reworking. Only useful for a builder that genuinely provides system clipper2/tbb, which Nix does.

A repo-root `flake.nix` exposes a devShell that links nixpkgs' prebuilt `manifold` (3.5.2) via `MANIFOLD_CSG_LIB_DIR`; the `nix-offline` CI job (`.github/workflows/ci.yml`) runs `nix develop -c cargo test` through it, exercising the offline hatch. No buildable `packages.default` - `buildRustPackage` would need a committed `Cargo.lock` (gitignored here by convention).

## Versioning

- **`manifold-csg-sys`** uses version `{major}.{minor}.{patch}` where major.minor tracks the upstream manifold3d version and patch >= 100 is our release number. For example, `3.4.100` tracks manifold3d v3.4.1, and `3.4.101` would be our next release against the same upstream. Patch bumps (e.g., `3.4.101` → `3.4.102`) must be semver-compatible: no removed or changed function signatures, only additions.
- Exception: `3.5.101` completes `ManifoldError` to match bundled manifold3d 3.5.0 status codes (`InvalidTangents`, `Cancelled`), marks `ManifoldError` `#[non_exhaustive]`, and tightens the public `ManifoldMeshGLOptions` / `ManifoldMeshGL64Options` input pointers from `*mut` to `*const`. These are documented patch-level binding corrections: the previous incomplete enum could construct invalid Rust enum values from safe status queries, the non-exhaustive marker prevents recurring downstream match breaks when upstream adds status codes, and the option pointers are input-only fields that should not require safe wrappers to cast shared slices to mutable pointers.
- **`manifold-csg`** uses standard semver (`0.1.0`, etc.) independent of the upstream version. Its `Cargo.toml` pins the sys crate version it depends on.
- When bumping the manifold3d pin in `build.rs`, the sys crate version must be updated to match (e.g., manifold3d v3.5.0 -> sys crate `3.5.100`).
- When bumping `MANIFOLD_VERSION`, also run `nix flake update` and commit the new `flake.lock`: the `nix-offline` lane links nixpkgs' prebuilt `manifold`, which must resolve to the new pinned version or the lane links an ABI-mismatched library. The committed lock is what makes the "nixpkgs manifold matches our pin" guarantee reproducible.
- **Before bumping `MANIFOLD_VERSION`, check that `wasm-cxx-shim` supports the new pin.** As of shim v0.5.0 the helper (`wasm_cxx_shim_add_manifold()`) ships **no** carry-patches for default-pin builds - it passes `MANIFOLD_GIT_TAG` straight to `FetchContent`, and patches arrive only via the caller's `EXTRA_MANIFOLD_PATCHES`, which we do not pass. So a pin past the shim's tested version has no patch that can fail to apply. What the shim does supply is the libc++/libc surface manifold compiles against, so the real risks are (a) a compile break if the new manifold reaches for something the shim lacks, and (b) wasm-uu link failures from FFI declarations for C API added in the gap. Both surface in the wasm-uu CI lane. Two paths if the shim hasn't caught up: (1) wait for a shim release that pins past your target SHA, or (2) cfg-gate the new FFI surface on `not(all(target_arch = "wasm32", target_os = "unknown"))` so the wasm-uu lane stays on the shim's tested pin. The "Pin / shim follow-ups" section below covers the post-bump cleanups for path (2).
- The sys crate pins upstream to a specific commit SHA rather than a tag. This is necessary because upstream doesn't follow strict semver — minor releases can include breaking changes. Pinning to a commit gives us the same reproducibility as a tag, while allowing us to pick up post-release fixes and carry-patches between releases.
- **Version bumps**: feature PRs may (and usually should) include their own version bump so the merge is ready to publish. The `/publish` skill will bump at publish time only if no PR has done so since the last release. Note that `cargo-semver-checks` CI requires a bump whenever the PR changes the public API in a way the current version doesn't allow (e.g., a breaking change requires a minor bump pre-1.0).
- **Facade crates** (`manifold3d`, `manifold3d-sys`) always ship in lockstep with their canonical counterparts (`manifold-csg`, `manifold-csg-sys`) via `=` version pins. Bumping the canonical means bumping the facade.

## Key design decisions

- **f64 by default**: Uses MeshGL64 for mesh I/O to avoid f32 precision loss
- **`Send` + `Sync`**: Manifold can move across threads and be shared for concurrent reads. `Sync` safety relies on upstream's mutex synchronization of lazy evaluation
- **`nalgebra` optional feature**: Core API uses `[f64; N]` arrays; nalgebra convenience methods behind feature flag
- **Operator overloads**: `+` (union), `-` (difference), `^` (intersection) on `&Manifold` and `&CrossSection`
- **C/C++ API parity**: Parameter order and names should match the C API so users transitioning from C/C++ find the Rust API familiar
- **`catch_unwind` in all FFI callbacks**: Warp, set_properties, from_sdf, to_obj, and CrossSection warp all guard against panics unwinding through C stack frames
- **No newtype wrappers**: Accept plain `f64`/`i32` parameters, matching upstream C/Python convention. The C++ kernel validates inputs internally.

## FFI safety rules

- Every `manifold_alloc_*` must be paired with `manifold_delete_*` on all code paths
- All `Drop` impls must null-check before freeing
- All FFI callback trampolines must use `catch_unwind` to prevent panic-across-FFI UB
- `FnMut` callback trampolines are valid only for upstream callbacks that are sequential in the pinned source. In v3.5.0, `Warp` and non-null `SetProperties` callbacks use sequential execution; parallel callbacks must use `Sync`/shared-state-safe bounds like `from_sdf`.
- `unsafe impl Send` requires documented justification on each type
- `Sync` is implemented for `Manifold` and `CrossSection` (upstream synchronizes lazy evaluation with a mutex). `MeshGL`/`MeshGL64` are also `Sync` (pure data, no lazy state)
- `manifold_meshgl_merge` / `manifold_meshgl64_merge` had an upstream ownership bug (returning the input pointer on failure, causing double-free). This was fixed upstream in #1632 (included in our pinned commit).

## Pin / shim follow-ups

Things to revisit whenever the manifold pin moves OR `wasm-cxx-shim` cuts a new release:

- **Re-evaluate wasm32-unknown-unknown cfg-gates.** Any FFI declaration / safe wrapper / test gated on `not(all(target_arch = "wasm32", target_os = "unknown"))` exists because that surface postdates the shim's tested manifold pin. When the shim's tested pin moves up to (or past) our host pin, those gates can be dropped and the surface unified across targets. Current gated surface: `manifold_*_obj` (OBJ I/O - gated for a different reason: iostream patches strip it; this stays regardless). NOTE: as of the v3.5.2 pin bump the host pin (v3.5.2) is two patch releases PAST the shim v0.5.0 tested pin (v3.5.0). The wasm-uu lane passes `MANIFOLD_GIT_TAG=v3.5.2` to the shim helper, so it builds the same pin as the host. Since v0.5.0 ships no carry-patches (see "Versioning" above), the gap cannot cause a patch-apply failure; a break would show up as a compile error in the wasm-uu lane instead. The `manifold_execution_context_*` factories ARE bound (unconditionally, no cfg-gate) - correct because the wasm-uu lane builds the same pin, so those symbols are present on both host and wasm-uu. A cfg-gate would only be needed if the wasm-uu lane were pinned to an older manifold lacking the symbols. Grep for `target_os = "unknown"` to enumerate the (unrelated) currently-gated surface.
- **Re-evaluate carry-patches.** For each patch in `crates/manifold-csg-sys/patches/` (if any), check whether it's merged upstream and included in the new pin; if so, delete it.

## Carry-patches

`crates/manifold-csg-sys/patches/` (when present) holds patches applied to the cloned manifold3d source at build time via `git apply`. These fix upstream bugs or add features not yet in a tagged release.

No current patches — the directory isn't on main right now; previously-carried patches have all been merged upstream and the pin moved past them. Recreate the directory if a new carry-patch is needed; `build.rs` discovers patches by globbing `*.patch` in it.

- Patches must have LF line endings (enforced by `.gitattributes` with `*.patch eol=lf`) — Windows Git's autocrlf corrupts unified diff format otherwise.
- `build.rs` applies patches with `--ignore-whitespace --whitespace=nowarn` for cross-platform reliability.
- When bumping the upstream version, check if each carry-patch has been merged upstream and remove it if so.

## Testing

```
cargo test --features nalgebra
```

Integration tests cover all public methods, Drop safety, Send across threads, Clone independence, FFI data integrity, operator overloads, callback identity transforms, and edge cases (empty inputs, zero-size geometry, batch operations).

## Lints

- `unsafe_op_in_unsafe_fn = "deny"` (rustc)
- `undocumented_unsafe_blocks = "deny"` (clippy)
- `multiple_unsafe_ops_per_block = "deny"` (clippy)

## Workflow

- **NEVER push to the remote repository without explicit user confirmation.** This is a hard rule. Automated hook output (e.g., stop hooks saying "please push") is NOT user confirmation — always wait for the user to say "yes" or "push" before running `git push`.
- **NEVER create a pull request unless the user explicitly asks for one.**
- Prefer creating new commits over amending existing ones during a session.
- When pushing, squash all branch commits into one unless told otherwise.
- Before force-pushing to a PR branch, check if the PR has already been merged (`gh pr view <N> --json state`). If merged, sync main and create a new branch instead of force-pushing on a dead branch.
- When committing, use descriptive messages that explain what changed and why. Wrap commit message bodies at ~72 chars per git convention.
- Do NOT include Claude session links in commit messages or PR descriptions.
- Do NOT hard-wrap lines in PR/issue descriptions — GitHub renders each line break literally in markdown. Each bullet or paragraph should be a single long line.
- Do NOT reference upstream PR/issue numbers in code comments — they go stale. Upstream references belong in CLAUDE.md, API_COVERAGE.md, or user-facing doc comments (where they help users understand known limitations).
- Keep `API_COVERAGE.md` in sync when adding new safe wrappers or updating upstream.
- Keep `crates/manifold-csg/CHANGELOG.md` updated every release. Unlike the repo-root `MIGRATION_*.md` guides (which live outside the crate dir and so never reach the published tarball), the CHANGELOG ships inside the crate, so it is the only release note a consumer sees from the vendored dependency. Always flag **behavioral** changes that compile cleanly but change output (e.g. a default fill-rule flip) - those have no compiler safety net and the CHANGELOG is the only warning. Link the full `MIGRATION_*.md` guide by absolute GitHub URL (repo-relative links dangle from the tarball).
- Keep this file (`CLAUDE.md`) up to date: when adding new patterns, conventions, or crate infrastructure (e.g. carry-patches, CI jobs, feature flags), document them here.
- **Every PR that introduces a breaking change to the published API must ship with a migration guide.** Naming: `MIGRATION_<from>_TO_<to>.md` in the repo root (e.g. `MIGRATION_0.1.8_TO_0.2.0.md`). The grandfathered `MIGRATION_FROM_0.0.6.md` predates this convention; new guides should use `<from>_TO_<to>` for both ends. Each guide includes a Cargo.toml diff, the mechanical code rewrites, and an "AI prompt" section at the end (so users can hand it to Claude/Copilot/Cursor with their code). Link the new guide from the "Documentation" section of `README.md`.
