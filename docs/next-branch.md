# The `next` branch

`next` carries the port to upstream manifold 4.0 while `main` keeps shipping 3.5.x patch releases. It tracks upstream master by commit SHA until upstream tags 4.0. Nothing is published from it. This doc is how it is maintained. When the port merges to `main`, the exit steps at the end remove `next` and this doc.

## Why a branch

Upstream master breaks most of our surface:

- the C API no longer takes caller-allocated memory, so every `manifold_alloc_*` / `_size` / `_destruct_*` function is gone
- 48 functions were removed and 8 added
- tolerance became relative precision
- Clipper2 is no longer a dependency

That's too much to land on `main` piece by piece without blocking 3.5.x releases. On `next` it can go in small PRs, each with CI.

## Rules for `next`

- **`next` was created directly at its first commit**, which pins upstream master, sets the `-dev` versions and adds `publish = false`. Creating it at `main`'s tip would fail the publish guard on its first push.

- **PRs only.** A ruleset protects `next` against deletion and force-push, and requires PRs with the required checks below. It doesn't require linear history, so sync PRs can be merged with a merge commit. Strict up-to-date is off.
- **Merge commits are allowed in the repo for `next`'s sake.** The `main` ruleset still allows only squash and rebase, so `main` stays linear. Port PRs into `next` can be squashed. Sync PRs from `main` must be merged with a merge commit (see "Syncing with main").
- **The pin is a master SHA.** Move it forward in its own PR into `next`, deliberately. The weekly upstream build (below) shows what a move would bring.
- **Versions are `0.5.0-dev` (workspace) and `3.99.100-dev` (sys crates).** Choose the real sys major.minor when upstream bumps its `CMakeLists.txt`.

## Publish guard

Nothing should be publishable from `next` by accident. A `-dev` version doesn't stop it, because crates.io accepts pre-release versions.

- Every published crate on `next` has `publish = false`, so `cargo publish` refuses however it is run. That covers `manifold-csg`, `manifold-csg-sys`, `manifold3d`, `manifold3d-sys` and `manifold3d-macros`.
- `scripts/check-publish-guard.py` runs in the Format job, which is required on both branches. It takes the branch from the PR's base or the pushed branch.
  - On `next`, it requires `publish = false` on every crate above.
  - On `main`, it forbids `publish = false` on those crates and pre-release versions. So the final `next` -> `main` merge can't bring the guard along and quietly block the release.
- The publish skill also refuses to run from any branch but `main`.

## CI lanes on `next`

`ci.yml` runs on PRs into `next` and on pushes to it. The Semver and nix-offline skips key on the version and the pin, not on the branch name, so PRs stacked on a port branch skip them too.

| Lane | On `next` |
|---|---|
| Format, Docs, Deny, MSRV | required from the start. These build but don't link test binaries, so they pass while the port is partway done. |
| Linux, Linux (no TBB), macOS, Windows | run, but not required until port step 2 is done. Until then, the test binaries reference `manifold_alloc_*` symbols that upstream removed and fail to link. The PR that finishes step 2 adds them to the `next` ruleset's required checks, which is a settings change at that point. |
| Wasm32 (unknown-unknown) | runs, but not required until the wasm-uu catch-up (port step 5) is done. Expect it red until then. Making it required is a ruleset change at that point. |
| Emscripten (wasm32) | not required while it's red, required once it builds |
| Nix (offline / prebuilt manifold) | skips itself off `main` while the pin isn't a release tag, because `flake.nix` fetches `pinnedSrc` by tag. It runs again once `next` pins a 4.0 tag. On `main` a SHA pin still fails it. |
| Semver | skips itself whenever the workspace version is a pre-release, as it is on `next`. Every port PR is breaking on purpose, and it compares against 0.4.x on crates.io. The publish guard keeps pre-release versions off `main`, so it always runs there. |
| API coverage | runs. It fetches `manifoldc.h` at the pinned ref, so a SHA works. |

## Weekly workflows

Both live on `main`, because GitHub runs schedules only from the default branch. Both run on Mondays and can be started with `workflow_dispatch`.

### Next sync (`next-sync.yml`, 15:00 UTC)

When `main` has commits that `next` lacks and no sync PR is open, this opens "Merge main into next", with head `main` and base `next`. The head is `main` itself, so an open PR always carries the latest `main` and the workflow never pushes.

- **Merge it with a merge commit.** Squash or rebase would copy `main`'s commits instead of making them ancestors, so every later sync would bring them again and conflict with itself.
- **On a conflict**, make a branch from `next`, `git merge origin/main`, resolve, open that as the sync PR (merge-commit it too), and close the bot's.
- **Closing it by hand** defers it until `main` moves.
- It uses the `FLAKE_LOCK_TOKEN` secret (see `flake-lock.yml`). The PRs show up as yours, so GitHub doesn't email you about them. Look for them on Mondays.

### Next vs upstream master (`next-upstream.yml`, 15:30 UTC)

This checks out `next` and points `MANIFOLD_VERSION` at upstream master's HEAD, without committing anything. It then builds the sys crate, runs `check-api-coverage.py` (which names any C function added or removed since the pin), and runs `cargo test`. The step summary lists the upstream commits between `next`'s pin and master. A red run is the notification.

While the port is partway done, `next` doesn't pass at its own pin either, so this will be red. Check the summary for which step failed. A failed sys build or coverage step at master, when the same steps pass on `next` itself, is a new upstream break.

## Moving the pin

1. Read the latest "Next vs upstream master" summary for what the move brings.
2. In a PR into `next`, set `MANIFOLD_VERSION` to the new master SHA. Keep the line's exact shape, because `scripts/pins.py` parses it.
3. Fix what breaks in the same PR if it's small. Otherwise land the pin move with the fix as the next PR.

## Port order

Each step is one or more PRs into `next`, and each leaves the branch building:

1. **Sys declarations.**
   - Remove the 48 functions, drop the `mem` parameters, change the `*mut` inputs to `*const`, and add the 8 new functions.
   - `check-api-coverage.py` should pass after this.
2. **Safe crate memory model.**
   - Replace each alloc-then-construct pair with a direct call plus a null check, in a shared helper. Do it one module at a time if that reviews better.
   - The FFI safety rules in CLAUDE.md about pairing `manifold_alloc_*` with deletes become rules about null-checking returned pointers. Update them.
   - Tests link again once this step is done. Make Linux, Linux (no TBB), macOS and Windows required on `next` then.
3. **Safe API breaks.**
   - Tolerance becomes relative precision, and the removed smoothing calls go.
   - Expose winding number, `manifold_version` and the fill-rule cross-section constructors.
   - Start `MIGRATION_0.4_TO_0.5.md` here and extend it in every later step.
4. **Test drift.**
   - Update expected numbers that changed because of upstream kernel changes (boolean tie-breaking, degenerate removal).
   - Note each one in the CHANGELOG's unreleased section. They compile cleanly but change output.
5. **wasm-uu catch-up.** This can run alongside steps 2-4, because it is mostly C++ build work.
   - **Ours:** `wasm32-uu/CMakeLists.txt` makes its default target depend on `Clipper2`, and `build/wasm.rs` links `libClipper2.a`. Both go.
   - **Shim:** `WasmCxxShimManifold.cmake` sets `CLIPPER2_*` options and `-DCLIPPER2_MAX_DECIMAL_PRECISION=8`. Those are harmless when nothing reads them, but its header comment promises a `Clipper2` target, so that needs a fix.
   - **Shim:** the unknown is whether master's manifold compiles against the shim's libc++/libc surface. Run the shim's own `manifold` CI job with `MANIFOLD_GIT_TAG` set to our `next` pin, and fill gaps there.
   - **Shim release:** cut one (v0.6.0) whose tested pin is our `next` pin or later. Bump `WASM_CXX_SHIM_TAG` on `next`, and add the lane to the `next` ruleset's required checks.
   - `main` requires this lane, so the final merge can't land without this step.
6. **Docs.** Update `API_COVERAGE.md`, CLAUDE.md, and the README dependency snippet.

## Exit

When upstream tags 4.0.0 or a 4.0 pre-release:

1. Pin the tag. `nix-offline` then runs on `next` again, after updating `pinnedSrc` in `flake.nix` as for any pin bump.
2. Optionally publish `0.5.0-alpha.N` / `4.0.100-alpha.N` from `next`. That takes a deliberate PR into `next` that drops `publish = false` and lets the guard check allow alphas there. Say so in its title.
3. Tag the tip of `next` as `next-history` so the per-step history survives. Then open the `next` -> `main` PR. It removes `publish = false` and the `-dev` suffixes, and the guard check fails it if either is left. Squash it, since `main` allows only squash and rebase.
4. Release 0.5.0 / 4.0.100 with the migration guide.
5. Clean up:
   - Delete `next-sync.yml`, `next-upstream.yml`, the `next` handling in `ci.yml` and `check-publish-guard.py`, and this doc with its CLAUDE.md section.
   - Delete the `next` ruleset and the `next` branch.
   - Turn merge commits back off for the repo.
