# A `next` branch for upstream manifold 4.0

Status: proposal, nothing created yet.

## Goal

Track upstream manifold master on a long-lived `next` branch while `main` keeps shipping 3.5.x patch releases. Do the breaking port in small reviewed PRs, with CI. Nothing goes to crates.io from `next` until upstream tags a 4.0 release or pre-release.

This plan covers setting up `next`. Its last step deletes this file and replaces it with a doc on how `next` is maintained (see "Setup step 7"). The port itself, and the eventual merge to `main`, are described there.

## What upstream master changed (as of 2026-11-16)

Master is 130 commits ahead of v3.5.5 and has diverged from it, because upstream cuts release tags on a side branch. Master's `CMakeLists.txt` still says 3.5.1, so "4.0" is the plan rather than a version string yet.

- **C API: caller-allocated memory is gone.** "C API: Remove caller passed buffer" removed every `manifold_alloc_*`, `manifold_destruct_*` and `manifold_*_size` function. Constructors and operations now return a heap pointer directly (`manifold_cube(x, y, z, center)` instead of `manifold_cube(mem, ...)`), and inputs became `const`. `manifold_delete_*` stays. Our sys crate declares 13 alloc functions and the safe crate calls `manifold_alloc_*` in about 224 places, so this touches nearly every wrapper.
- **The header went from 294 functions to 254.** 48 were removed (the alloc/size/destruct set above, plus `manifold_set_tolerance` / `manifold_get_tolerance`, `manifold_smooth`, `manifold_smooth64`, `manifold_smooth_out`, `manifold_execution_context_smooth*` and `manifold_cross_section_compose`). 8 were added: `manifold_set_relative_precision` / `manifold_get_relative_precision`, `manifold_version`, `manifold_winding_number`, and `manifold_cross_section_{even_odd,non_zero}_{polygons,simple_polygon}`.
- **Tolerance became relative precision** ("Remove tolerance and simplify epsilon"), and the duplicate smoothing APIs were removed ("Remove duplicate Smoothing APIs"). These are safe-API breaks, not only sys-level ones.
- **Clipper2 is no longer a dependency.** `manifoldDeps.cmake` has no Clipper2 at all, and the `MANIFOLD_USE_BUILTIN_CLIPPER2` and `MANIFOLD_CROSS_SECTION` options are gone. Our `build/build.rs` requires and links a `Clipper2` static library, so the sys build fails on master before any Rust compiles.
- Boolean tie-breaking and degenerate removal changed, so expect numeric drift in tests that compare exact volumes or vertex counts.

## Setup steps

Steps 1, 2 and 6 change GitHub settings and need approval at the time. Steps 3-5 are one PR into `main`, because scheduled workflows only run from the default branch.

### 1. Allow merge commits, for `next` only

Keeping `next` in sync means merging `main` into it. Squash or rebase merges would copy `main`'s commits instead of making them ancestors, so each later sync would bring the same changes again and conflict with itself. The repo currently disallows merge commits (`allow_merge_commit: false`). Turn that on at the repo level. The `main` ruleset already limits `main` to squash and rebase, so `main` stays linear.

### 2. Create and protect `next`

Create `next` from `main`. Add a ruleset for `next` with: no deletion, no force-push, PRs required with 0 approvals, merge methods merge/squash/rebase, and the required checks listed under "Lanes". Leave out `required_linear_history`. Leave strict up-to-date off, so the weekly sync PR does not have to be updated before it can merge.

### 3. CI on `next`

- `ci.yml` already runs `pull_request` for any base. Add `next` to `push.branches`.
- Skip the Semver job on `next` (both PRs into it and pushes to it). Every port PR is breaking on purpose, and the job compares against 0.4.x on crates.io.
- Skip `nix-offline` on `next` while the pin is a SHA. `flake.nix` fetches `pinnedSrc` by tag and `pins.py manifold --tag` rejects a SHA, so the lane can't evaluate. It tests the offline link path, which the port doesn't change.
- `pages.yml` stays main-only.

### 4. Weekly sync PR (`next-sync.yml`)

Runs Mondays (after upstream-watch and flake-lock) plus `workflow_dispatch`. If `main` has commits that `next` lacks and no sync PR is open, it opens a PR with head `main` and base `next`, titled "Merge main into next". The head is `main` itself, so an open PR always carries the latest `main` and the workflow never pushes anything. Merge it with a merge commit.

- It needs a token with Pull requests read/write, for the same reasons as flake-lock: the repo does not let Actions create PRs, and a `GITHUB_TOKEN` PR triggers no CI. `FLAKE_LOCK_TOKEN` already has that permission. Reuse it, and rename the secret to something generic later if it bothers anyone.
- On a conflict GitHub marks the PR unmergeable. Resolve it on a scratch branch (`git merge main` on top of `next`), open that as the sync PR instead, and close the bot's. The next week's run sees an open sync PR, or nothing left to sync, and does nothing.
- A hand-closed sync PR is not reopened until `main` gets new commits.

### 5. Weekly upstream build (`next-upstream.yml`)

Runs Mondays plus `workflow_dispatch`. Checks out `next`, rewrites `MANIFOLD_VERSION` to upstream master's current HEAD, and runs `cargo test` on Linux. A red job is the notification (GitHub emails it), the same escalation upstream-watch uses. The step summary says how many upstream commits the real `next` pin is behind master and lists them by title, so a red run arrives with the likely cause.

This finds upstream breaks between our deliberate pin moves. It does not move the pin. A pin move stays a hand-made PR into `next`.

### 6. Pin and versions on `next`

First PR into `next`. Pin `MANIFOLD_VERSION` to a master SHA, drop Clipper2 from the link list and the removed cmake options, and set versions to `0.5.0-dev` (workspace) and `3.99.100-dev` (sys crates), unpublished. A pre-release suffix means nobody can mistake a `next` build for a crates.io release. Pick the real sys major.minor when upstream bumps `CMakeLists.txt`.

The sys crate should build after this. The safe crate won't until port step 3 below.

### 7. Replace this plan with a maintenance doc

Delete `docs/plans/next-branch.md`. Add `docs/next-branch.md` describing how `next` is maintained, built from the sections below and from what steps 1-6 actually ended up as: the rulesets, the lane table, the two workflows and their failure handling, how to move the pin, the port checklist, and the exit. Add a short "next branch" section to `CLAUDE.md` that points to it. This lands on `main`, with the workflows.

## Lanes

| Lane | On `next` |
|---|---|
| Linux, Linux (no TBB), macOS, Windows, MSRV, Format, Docs, Deny | required |
| Wasm32 (unknown-unknown) | not required at first. wasm-cxx-shim v0.5.0's helper builds Clipper2 alongside manifold, so it might break on master until the shim catches up. Making it required again is an exit criterion. |
| Emscripten (wasm32) | required if it builds, otherwise treated like wasm-uu |
| nix-offline | skipped while the pin is a SHA |
| Semver | skipped |

## Port order (moves to the maintenance doc)

Each step is one PR into `next` and leaves the branch building:

1. **Sys declarations.** Remove the 48 functions, drop `mem` parameters, change the `*mut` inputs to `*const`, and add the 8 new functions. `check-api-coverage.py` should pass after this. It formats the pin into a raw.githubusercontent URL, so it works with a SHA pin.
2. **Safe crate memory model.** Replace each alloc-then-construct pair with a direct call plus a null check, in a shared helper. This is the large mechanical step, so do it one module at a time if that reviews better. The FFI safety rules in CLAUDE.md about pairing `manifold_alloc_*` with deletes become rules about null-checking returned pointers. Update them.
3. **Safe API breaks.** Tolerance becomes relative precision and the removed smoothing calls go. Expose winding number, `manifold_version` and the fill-rule cross-section constructors. Start `MIGRATION_0.4_TO_0.5.md` here and extend it in every later step.
4. **Test drift.** Update expected numbers that changed because of upstream kernel changes, and note each one in the CHANGELOG's unreleased section. Those changes compile cleanly but change output.
5. **wasm-uu.** Get the lane building again, after a shim release if needed, and make it required on `next`.
6. Docs: `API_COVERAGE.md`, CLAUDE.md, and the README dependency snippet.

## Exit (moves to the maintenance doc)

When upstream tags 4.0.0 or a pre-release:

1. Pin the tag, so `nix-offline` runs on `next` again.
2. Optionally publish `0.5.0-alpha.N` / `4.0.100-alpha.N` from `next`.
3. Before the final merge, tag the branch tip as `next-history` so the per-step history survives. Then open the `next` -> `main` PR and squash it, since the `main` ruleset allows only squash and rebase.
4. Release 0.5.0 / 4.0.100 with the migration guide.
5. Delete the two weekly workflows, the `next` ruleset and the maintenance doc. Turn merge commits back off.
