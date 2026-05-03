# wasm32-uu patches

Patches applied to manifold and Clipper2 source trees during the
`wasm32-unknown-unknown` build path. Sibling to (and distinct from) the
host carry-patches under `../../patches/`, which target the same
manifold tree but a different build configuration.

## Files

- `0001-manifold-ifdef-iostream.patch` — wraps manifold's iostream-using
  OBJ I/O paths under `MANIFOLD_NO_IOSTREAM`. Generated against our
  pinned manifold SHA (post-3.4.1 master). Three blocks across
  `bindings/c/manifoldc.cpp` and `src/impl.cpp`.
- `0002-clipper2-strip-iostream.patch` — strips `<iostream>` from
  Clipper2 headers. Verbatim from
  [wasm-cxx-shim's reference impl](https://github.com/zmerlynn/wasm-cxx-shim/blob/main/test/manifold-link/patches/0001-clipper2-strip-iostream.patch);
  applies to the SHA manifold pins (`46f6391...`, see
  `crates/manifold-csg-sys/build.rs::CLIPPER2_SHA`).

## Patch convention: `-p0`

Both files are generated with `git diff --no-prefix`, which omits the
standard `a/` and `b/` path prefixes. They must be applied with
`git apply -p0`, NOT the default `-p1`.

If you regenerate or hand-edit a patch here, **double-check the file
header**:

```
diff --git a/src/impl.cpp b/src/impl.cpp     ← -p1 form (DEFAULT git diff)
diff --git src/impl.cpp src/impl.cpp         ← -p0 form (git diff --no-prefix)
```

The host `../../patches/` directory uses the `-p1` form (default git
diff). The two conventions don't mix.

## Updating a patch when the manifold pin moves

`build.rs` has a `git apply --check` style assertion that fails loudly
when a patch can't apply against the pinned upstream commit. If you
bump `MANIFOLD_VERSION` in `build.rs` and the wasm32-uu lane fails:

1. Check out `out_dir/manifold-src-wasm32-uu/` after a successful clone
   (built into `target/wasm32-unknown-unknown/debug/build/manifold-csg-sys-*/out/`).
2. Apply the host carry-patches first (they're applied before the
   wasm32-uu ones).
3. Hand-edit the `MANIFOLD_NO_IOSTREAM` blocks in `bindings/c/manifoldc.cpp`
   and `src/impl.cpp` to match the new line numbers.
4. Regenerate: `git diff --no-prefix bindings/c/manifoldc.cpp src/impl.cpp > new.patch`.
5. Splice the new patch body under the existing header comment in
   `0001-manifold-ifdef-iostream.patch`.

Same flow for Clipper2 if the upstream pin moves.
