#!/usr/bin/env python3
"""Make sure the published crates can't be released from `next`.

On `next`, every published crate must say `publish = false`, so cargo itself
refuses `cargo publish` there, however it is run. A pre-release version does
not stop it: crates.io accepts those.

On `main`, the reverse: no published crate may say `publish = false`, and no
version may carry a pre-release suffix. That stops the final `next` -> `main`
merge from bringing the guard or a `-dev` version along and quietly blocking
the next release.

The branch is the PR's base (GITHUB_BASE_REF) or else the pushed branch
(GITHUB_REF_NAME), or the first argument when run by hand. Any other branch
is not checked. See docs/next-branch.md.

    python3 scripts/check-publish-guard.py next
"""

import os
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = [
    "manifold-csg",
    "manifold-csg-sys",
    "manifold3d",
    "manifold3d-sys",
    "manifold3d-macros",
]


def load(path):
    with open(path, "rb") as f:
        return tomllib.load(f)


def main():
    if len(sys.argv) > 1:
        branch = sys.argv[1]
    else:
        branch = os.environ.get("GITHUB_BASE_REF") or os.environ.get("GITHUB_REF_NAME", "")
    if branch not in ("main", "next"):
        print(f"publish guard: not checked on branch {branch!r}")
        return 0

    workspace_version = load(ROOT / "Cargo.toml")["workspace"]["package"]["version"]
    problems = []
    for name in CRATES:
        package = load(ROOT / "crates" / name / "Cargo.toml")["package"]
        version = package["version"]
        if isinstance(version, dict):  # version.workspace = true
            version = workspace_version
        # publish = [] blocks publishing just like false does.
        blocked = package.get("publish", True) in (False, [])
        if branch == "next":
            if not blocked:
                problems.append(f"{name}: needs `publish = false` on next")
        else:
            if blocked:
                problems.append(f"{name}: `publish = false` must not reach main")
            if "-" in version:
                problems.append(f"{name}: pre-release version {version} must not reach main")

    for p in problems:
        print(f"::error::{p}")
    if problems:
        print("See docs/next-branch.md, 'Publish guard'.")
        return 1
    print(f"publish guard: ok for {branch} ({len(CRATES)} crates)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
