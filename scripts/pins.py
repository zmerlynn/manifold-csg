#!/usr/bin/env python3
"""Read the pinned versions out of the sys crate's build scripts.

The one parser for these pins outside the build itself (build.rs uses the
consts directly). check-upstream.py, check-api-coverage.py and the
flake-lock workflow all go through here, so they cannot drift apart.
flake.nix cannot call it, since Nix evaluation is pure, so it keeps its own
copy of the MANIFOLD_VERSION rule, and the nix-offline CI lane checks that
the version the flake links equals `pins.py manifold --tag`.

The rule is deliberately strict: exactly one line that is exactly the
declaration, nothing before it and nothing after the semicolon. A loose
match is how a commented-out old pin or a doc example gets read instead of
the real one, silently. A strict one turns any reshaping into an error.

    python3 scripts/pins.py manifold          # MANIFOLD_VERSION, as written
    python3 scripts/pins.py manifold --tag    # X.Y.Z, or fail unless vX.Y.Z
    python3 scripts/pins.py shim              # WASM_CXX_SHIM_TAG
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SYS = ROOT / "crates/manifold-csg-sys"

# name -> (file, exact declaration up to the opening quote)
PINS = {
    "manifold": (SYS / "build.rs", 'pub(crate) const MANIFOLD_VERSION: &str = "'),
    "shim": (SYS / "build/wasm.rs", 'const WASM_CXX_SHIM_TAG: &str = "'),
}

TAG = re.compile(r"v([0-9]+\.[0-9]+\.[0-9]+)")


class PinError(Exception):
    """The declaration is missing, duplicated, or not the expected shape."""


def read(name):
    path, decl = PINS[name]
    try:
        text = path.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError) as e:
        raise PinError(f"cannot read {path.relative_to(ROOT)} ({e})") from e
    # Split exactly as flake.nix does: on "\n" only, then drop one trailing
    # "\r". str.splitlines() would also split on \f, \x85, U+2028 and more.
    lines = [line.removesuffix("\r") for line in text.split("\n")]
    pat = re.compile(re.escape(decl) + r'([^"]*)";')
    found = [m.group(1) for line in lines if (m := pat.fullmatch(line))]
    if len(found) != 1:
        raise PinError(
            f"expected exactly one line `{decl}...\";` in "
            f"{path.relative_to(ROOT)}, found {len(found)}"
        )
    if not found[0]:
        raise PinError(f"{name} pin in {path.relative_to(ROOT)} is empty")
    return found[0]


def tag(name):
    """The pin as X.Y.Z, for consumers that only follow release tags."""
    value = read(name)
    m = TAG.fullmatch(value)
    if not m:
        raise PinError(f"{name} pin is {value!r}, not a vX.Y.Z release tag")
    return m.group(1)


def main(argv):
    args = argv[1:]
    want_tag = "--tag" in args
    names = [a for a in args if a != "--tag"]
    if len(names) != 1 or names[0] not in PINS:
        print(f"usage: pins.py {{{','.join(PINS)}}} [--tag]", file=sys.stderr)
        return 2
    try:
        print(tag(names[0]) if want_tag else read(names[0]))
    except PinError as e:
        print(f"pins.py: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
