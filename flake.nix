{
  description = "manifold-csg - safe Rust bindings to the manifold3d geometry kernel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  # This flake targets the *offline* build path (issue #49): it links the
  # pre-built `manifold` package from nixpkgs via the crate's
  # `MANIFOLD_CSG_LIB_DIR` escape hatch instead of letting build.rs clone +
  # compile manifold3d. nixpkgs' `manifold` is 3.5.2, matching this crate's
  # MANIFOLD_VERSION pin exactly, so there's no ABI/version drift. The
  # committed flake.lock pins the exact nixpkgs revision that resolves
  # manifold 3.5.2, so `nix develop` is reproducible (run `nix flake update`
  # to advance, which must stay in step with MANIFOLD_VERSION).
  #
  # It exposes a devShell (and CI uses `nix develop -c cargo test ...` to
  # exercise the hatch). A buildable `packages.default` via
  # `rustPlatform.buildRustPackage` is intentionally omitted: that needs a
  # committed `Cargo.lock`, which this workspace deliberately gitignores.
  # Commit a lock first if you want a `nix build` package output.
  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        # libmanifold.so / libmanifoldc.so live here. nixpkgs builds manifold
        # shared with the C bindings on (CROSS_SECTION => CBIND) and records its
        # Clipper2/TBB deps as absolute-store RUNPATH entries, so linking just
        # manifoldc + manifold (the crate's dylib path) resolves the rest.
        manifoldLib = "${pkgs.manifold}/lib";
      in
      {
        devShells.default = pkgs.mkShell {
          # `manifold` in buildInputs makes its headers/libs discoverable and,
          # combined with LD_LIBRARY_PATH below, lets the test binaries find
          # libmanifold.so at runtime (the dylib path is link-time only).
          buildInputs = [ pkgs.manifold ];
          nativeBuildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            # cmake isn't invoked on the MANIFOLD_CSG_LIB_DIR path (build.rs
            # returns before any cmake call), but keep it available for anyone
            # who unsets the override and falls back to the from-source build.
            pkgs.cmake
          ];

          # Drive the crate's offline hatch: link nixpkgs' prebuilt manifold,
          # skipping the clone + C++ compile entirely.
          MANIFOLD_CSG_LIB_DIR = manifoldLib;
          # dylib kind needs the lib dir on the runtime search path too.
          LD_LIBRARY_PATH = manifoldLib;

          shellHook = ''
            echo "manifold-csg devShell: linking prebuilt manifold from ${manifoldLib}"
            echo "  (offline build via MANIFOLD_CSG_LIB_DIR - no manifold3d clone/compile)"
          '';
        };
      }
    );
}
