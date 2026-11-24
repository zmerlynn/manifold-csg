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
| Format, Docs, Deny, MSRV | required from the start |
| Linux, Linux (no TBB), macOS, Windows, Emscripten (wasm32), API coverage | required since the C API port (#71-#74) finished. Before that the test binaries referenced removed symbols and could not link. |
| Wasm32 (unknown-unknown) | required since the wasm-uu catch-up (#77, wasm-cxx-shim v0.6.0). |
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

Each step is one or more PRs into `next`, and each leaves the branch building.

1. **C API port (done, #71-#74).** The sys declarations and the safe crate's memory model could not land separately, because each signature change breaks its callers. So they went in one module at a time, each PR regenerating its sys declarations from the pinned header together with the safe callers:
   - #71: BoundingBox, Rect, triangulation, ExecutionContext, ray casting, plus the `owned()` helper.
   - #72: MeshGL and MeshGL64, plus `copy_out()` for borrowed data getters.
   - #73: CrossSection and the polygon helpers. `FillRule::Negative` and `CrossSection::compose` went with upstream.
   - #74: Manifold. The removed smoothing and tolerance APIs went, `as_original` took an `id`, and `winding_number`, relative precision and `manifold_version` arrived. Tests linked again and all passed.

   The safe API breaks the plan had as a separate step landed with their modules, along with `MIGRATION_0.4.2_TO_0.5.0.md`. The tests turned up two upstream behavior changes, and both are in the CHANGELOG: primitives start with original ID 0, and `Manifold::compose` became a union (upstream later removed it).
2. **Test drift.** None so far beyond the two behavior changes above. Re-check after each pin move.
   - The move to master `2cac6b03` (upstream's C API cleanup for 4.0) brought API changes rather than drift: per-call `CircularQuality` replaced the global circular setters, normals always go in MeshGL channels 3-5 (the old index counted from after position), `RayHit` reports original and face IDs, the MeshGL options structs gained `struct_size`, and `Manifold::compose` went. A segment count passed to a partial `revolve` changed meaning, from divisions across the arc to a full-circle count. The CHANGELOG and migration guide cover them.
   - Update expected numbers that change because of upstream kernel changes (boolean tie-breaking, degenerate removal).
   - Note each one in the CHANGELOG's unreleased section and the migration guide. They compile cleanly but change output.
3. **wasm-uu catch-up (done, #77).** wasm-cxx-shim v0.6.0 tests the `next` pin itself. Building manifold master against the shim needed `frexp`/`fmod` in its libm, a fake advancing `steady_clock::now()`, `__throw_system_error` and an adapted `std::pmr` pool in the reference `libcxx-extras.cpp`, and a `<mutex>` stub that reuses libc++'s own lock types; both repos dropped Clipper2 from the wasm build. Codex adversarial reviews of the shim change and of `main...next` found real issues, fixed in the shim PR and in #76.
   - The fake clock exists only because upstream's `src/boolean2_diagnostics.h` names `std::chrono::steady_clock` in every build. elalish/manifold#1963 gates that timing behind `MANIFOLD_DEBUG || MANIFOLD_TIMING`. Once it lands and the pin moves past it, drop the fake clock and turn `_LIBCPP_HAS_MONOTONIC_CLOCK` back off, in the shim's reference files and our `wasm32-uu/` copies.
   - The vendored `wasm32-uu/` files are copies of the shim's `test/manifold-link/` references; re-copy them on every `WASM_CXX_SHIM_TAG` bump (see CLAUDE.md "Pin / shim follow-ups").
4. **Docs.** `API_COVERAGE.md` and the CLAUDE.md FFI rules were updated with the C API port. The README dependency snippet is left for the exit.

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
