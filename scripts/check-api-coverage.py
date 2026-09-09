#!/usr/bin/env python3
"""Verify the sys crate binds the whole C API and API_COVERAGE.md accounts for it.

The coverage table is maintained by hand, so it drifts silently: a function
gets bound, the table is not updated, and the Summary keeps reporting a total
that no longer matches reality. Upstream drifts the other way just as quietly:
a release adds a C function, nothing here notices, and the binding gap sits
there for releases. This checks the three invariants that catch both.

  1. Every function in upstream's `manifoldc.h`, at the tag `MANIFOLD_VERSION`
     pins, is declared in the sys crate.
  2. Every `pub fn manifold_*` in the sys crate appears in the document, either
     by name or under one of the `manifold_{alloc,delete,destruct}_*` groups.
  3. The Summary totals equal the per-section row counts, and their sum equals
     the number of declarations.

Invariant 1 is the one with a track record: `manifold_meshgl_backside`,
`manifold_meshgl_has_normals` and their `meshgl64` twins arrived in upstream
v3.5.0 and went unbound until 3.5.105, because checks 2 and 3 only ever look
at what we already bind.

Check 1 reads the header over the network, since the pinned upstream source is
not in a plain checkout. Point `MANIFOLD_CSG_HEADER` at a local `manifoldc.h`
to run offline; it is your job to make sure that file matches the pin.

Run from the repo root. Exits non-zero with a diff on failure.
"""

import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SYS = ROOT / "crates/manifold-csg-sys/src/lib.rs"
DOC = ROOT / "API_COVERAGE.md"
BUILD_RS = ROOT / "crates/manifold-csg-sys/build.rs"

HEADER_URL = (
    "https://raw.githubusercontent.com/elalish/manifold/"
    "{ref}/bindings/c/include/manifold/manifoldc.h"
)

GROUPED = re.compile(r"^manifold_(alloc|delete|destruct)_")
# Return type, then the name, then the opening paren. Anchored at the start of
# a line so a name mentioned inside a parameter list or a nested call cannot
# masquerade as a declaration.
HEADER_FN = re.compile(r"^[A-Za-z_][\w \t*]*?\b(manifold_[a-z0-9_]+)\s*\(", re.M)


class CheckError(Exception):
    """The check could not run, as opposed to running and finding a gap."""


def pinned_version():
    m = re.search(
        r'MANIFOLD_VERSION:\s*&str\s*=\s*"([^"]+)"', BUILD_RS.read_text()
    )
    if not m:
        raise CheckError(
            f"no MANIFOLD_VERSION string in {BUILD_RS.relative_to(ROOT)}; "
            "the constant was renamed or reshaped"
        )
    return m.group(1)


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def header_text(ref):
    override = os.environ.get("MANIFOLD_CSG_HEADER")
    if override:
        path = Path(override)
        try:
            return path.read_text()
        except OSError as e:
            raise CheckError(f"cannot read MANIFOLD_CSG_HEADER={override}: {e}") from e

    url = HEADER_URL.format(ref=ref)
    last = None
    for _ in range(3):
        try:
            with urllib.request.urlopen(url, timeout=30) as resp:
                return resp.read().decode("utf-8")
        except (urllib.error.URLError, OSError) as e:
            last = e
    raise CheckError(
        f"cannot fetch {url}: {last}. Set MANIFOLD_CSG_HEADER to a local "
        "manifoldc.h matching the pin to run this check offline."
    )


def check_header(declared):
    """Return the C functions the sys crate does not declare."""
    ref = pinned_version()
    text = strip_comments(header_text(ref))
    in_header = set(HEADER_FN.findall(text))
    if not in_header:
        raise CheckError(
            "parsed zero functions out of manifoldc.h; the header layout "
            "changed and this check is no longer looking at the right thing"
        )
    return ref, len(in_header), sorted(in_header - declared)


def bucket(status):
    if status.startswith("["):
        return "wrapped"
    if status.startswith("Internal"):
        return "internal"
    return "unused"


def main():
    declared = set(re.findall(r"pub fn (manifold_[a-z0-9_]+)", SYS.read_text()))
    text = DOC.read_text()

    documented = set(re.findall(r"`(manifold_[a-z0-9_]+)`", text))
    missing = sorted(
        fn for fn in declared - documented if not GROUPED.match(fn)
    )

    # Per-section row counts, with the three grouped rows expanded.
    counts = {"wrapped": 0, "internal": 0, "unused": 0}
    section = None
    for line in text.split("\n"):
        heading = re.match(r"^## (.+)$", line)
        if heading:
            section = heading.group(1).strip()
            continue
        if section is None or section == "Summary" or not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 2 or not re.match(r"^`manifold_[a-z0-9_]+`$", cells[0]):
            continue
        counts[bucket(cells[1])] += 1
    counts["internal"] += sum(1 for fn in declared if GROUPED.match(fn))

    m = re.search(r"\|\s*\*\*Total\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|\s*\*\*(\d+)\*\*\s*\|", text)
    if not m:
        print("FAIL: could not find the Summary Total row in API_COVERAGE.md")
        return 1
    claimed = {
        "wrapped": int(m.group(1)),
        "internal": int(m.group(2)),
        "unused": int(m.group(3)),
    }

    ok = True
    try:
        ref, n_header, unbound = check_header(declared)
    except CheckError as e:
        print(f"FAIL: {e}")
        return 1
    if unbound:
        ok = False
        print(
            f"FAIL: {len(unbound)} function(s) in manifoldc.h at {ref} "
            "are not declared in the sys crate:"
        )
        for fn in unbound:
            print(f"  {fn}")
    if missing:
        ok = False
        print(f"FAIL: {len(missing)} bound function(s) missing from API_COVERAGE.md:")
        for fn in missing:
            print(f"  {fn}")
    if claimed != counts:
        ok = False
        print("FAIL: Summary totals disagree with the tables above them")
        for k in ("wrapped", "internal", "unused"):
            flag = "" if claimed[k] == counts[k] else "   <-- mismatch"
            print(f"  {k:9} table={counts[k]:4}  summary={claimed[k]:4}{flag}")
    total = sum(counts.values())
    if total != len(declared):
        ok = False
        print(f"FAIL: rows total {total} but the sys crate declares {len(declared)}")

    if ok:
        print(
            f"OK: {n_header} functions in manifoldc.h at {ref}, all bound; "
            f"{len(declared)} declarations, all documented; "
            f"summary matches ({counts['wrapped']} wrapped, "
            f"{counts['internal']} internal, {counts['unused']} bound-unused)"
        )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
