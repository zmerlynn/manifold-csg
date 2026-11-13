{
  description = "manifold-csg - safe Rust bindings to the manifold3d geometry kernel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  # This flake targets the *offline* build path (issue #49): it links a
  # pre-built `manifold` via the crate's `MANIFOLD_CSG_LIB_DIR` escape hatch
  # instead of letting build.rs clone + compile manifold3d.
  #
  # The manifold it links always matches MANIFOLD_VERSION in
  # crates/manifold-csg-sys/build.rs, which the flake reads directly so the
  # pin has one source of truth. When nixpkgs' `manifold` is that version,
  # the flake uses it as-is (prebuilt, from cache.nixos.org). When nixpkgs is
  # behind or ahead, it rebuilds nixpkgs' recipe from the pinned tag, using
  # `pinnedSrc` below. Either way a version mismatch cannot link: it is an
  # evaluation error, not a convention. So a pin bump does not wait on
  # nixpkgs, and `nix flake update` is optional rather than lockstep.
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
        inherit (pkgs) lib;

        # MANIFOLD_VERSION from build.rs, without the leading "v". Match the
        # declaration line exactly, one line at a time: a whole-file regex
        # would also match a commented-out old pin or a doc example, and with
        # a greedy `.*` the last one wins. Exactly one declaration must match.
        # scripts/pins.py applies the same rule for everything outside Nix,
        # and the nix-offline CI lane checks that the two agree, so change
        # both together.
        pinLines = lib.filter (l: l != null) (
          map (
            l: builtins.match ''pub\(crate\) const MANIFOLD_VERSION: &str = "([^"]*)";'' (lib.removeSuffix "\r" l)
          ) (lib.splitString "\n" (builtins.readFile ./crates/manifold-csg-sys/build.rs))
        );
        pinRaw =
          if builtins.length pinLines == 1 then
            builtins.head (builtins.head pinLines)
          else
            throw "flake.nix: expected exactly one `pub(crate) const MANIFOLD_VERSION` line in crates/manifold-csg-sys/build.rs, found ${toString (builtins.length pinLines)}";
        # A SHA or branch pin (a deliberate exception, see CLAUDE.md
        # "Versioning") cannot be fetched by tag, so it fails here rather than
        # linking some other version.
        pinMatch = builtins.match "v([0-9]+[.][0-9]+[.][0-9]+)" pinRaw;
        pin =
          if pinMatch == null then
            throw "flake.nix: MANIFOLD_VERSION is \"${pinRaw}\", not a vX.Y.Z release tag. The nix-offline lane only follows tag pins; for a SHA pin, change the fallback in flake.nix to fetch by `rev` (or skip the lane)."
          else
            builtins.head pinMatch;

        # Source hash for the pinned tag, used when nixpkgs' manifold is a
        # different version. Update both fields on every pin bump; get the hash
        # with `nix flake prefetch github:elalish/manifold/v<version>`. The
        # assert below checks this on every evaluation, not just when the
        # fallback is taken, so a stale entry fails before it is needed.
        pinnedSrc = {
          version = "3.5.5";
          hash = "sha256-zhnpJExiHnKSUVNwAZuYwOJHaL+g3tgGp72oHG/Jy0E=";
        };

        fromNixpkgs = pkgs.manifold.version == pin;
        manifold =
          assert lib.assertMsg (pinnedSrc.version == pin) (
            "flake.nix: pinnedSrc.version is ${pinnedSrc.version} but MANIFOLD_VERSION is v${pin}. "
            + "Update pinnedSrc (hash: nix flake prefetch github:elalish/manifold/v${pin})."
          );
          if fromNixpkgs then
            pkgs.manifold
          else
            # nixpkgs' src is derived from finalAttrs.version, so overriding
            # the version alone would keep the old hash. Replace src outright.
            pkgs.manifold.overrideAttrs (old: {
              version = pin;
              # Patches in nixpkgs' recipe (none today) are written against
              # nixpkgs' version, not ours. Drop them rather than apply them
              # to a different tree.
              patches = [ ];
              src = pkgs.fetchFromGitHub {
                # The version must be in the name. With the default name
                # ("source") the store path depends only on the hash, so a
                # stale hash resolves to the previous tag's tree already in the
                # store and Nix never fetches. Naming it per version makes a
                # stale hash a fresh path, which fetches and fails the check.
                name = "manifold-${pin}-source";
                owner = "elalish";
                repo = "manifold";
                tag = "v${pin}";
                inherit (pinnedSrc) hash;
              };
              # Belt and braces for the same trap: check the source itself.
              preConfigure = ''
                got=$(sed -nE 's/^set\(MANIFOLD_VERSION_(MAJOR|MINOR|PATCH) ([0-9]+)\)$/\2/p' CMakeLists.txt | paste -sd.)
                if [ "$got" != "${pin}" ]; then
                  echo "flake.nix: source is manifold $got, expected ${pin} (stale pinnedSrc.hash?)" >&2
                  exit 1
                fi
              '';
              # Upstream's own gtest suite. Our cargo tests run against this
              # build anyway, so neither build nor run it: CI has no Nix store
              # cache, and every run in the lag window pays this compile.
              # Appended rather than rewritten so it holds whatever form
              # nixpkgs spells its flag in: the last -D wins.
              cmakeFlags = old.cmakeFlags ++ [ "-DMANIFOLD_TEST=OFF" ];
              doCheck = false;
            });
        # libmanifold.so / libmanifoldc.so live here. nixpkgs builds manifold
        # shared with the C bindings on (CROSS_SECTION => CBIND) and records its
        # Clipper2/TBB deps as absolute-store RUNPATH entries, so linking just
        # manifoldc + manifold (the crate's dylib path) resolves the rest.
        manifoldLib = "${manifold}/lib";
      in
      {
        devShells.default = pkgs.mkShell {
          # `manifold` in buildInputs makes its headers/libs discoverable and,
          # combined with LD_LIBRARY_PATH below, lets the test binaries find
          # libmanifold.so at runtime (the dylib path is link-time only).
          buildInputs = [ manifold ];
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

          # Drive the crate's offline hatch: link the pinned prebuilt manifold,
          # skipping the clone + C++ compile entirely.
          MANIFOLD_CSG_LIB_DIR = manifoldLib;
          # dylib kind needs the lib dir on the runtime search path too.
          LD_LIBRARY_PATH = manifoldLib;

          shellHook = ''
            echo "manifold-csg devShell: linking manifold ${manifold.version} from ${manifoldLib}"
            echo "  (${
              if fromNixpkgs then
                "prebuilt from nixpkgs"
              else
                "nixpkgs has ${pkgs.manifold.version}; built from the pinned tag"
            })"
            echo "  (offline build via MANIFOLD_CSG_LIB_DIR - no manifold3d clone/compile)"
          '';
        };
      }
    );
}
