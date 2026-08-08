#!/usr/bin/env python3
"""Verify API_COVERAGE.md accounts for every function bound in manifold-csg-sys.

The coverage table is maintained by hand, so it drifts silently: a function
gets bound, the table is not updated, and the Summary keeps reporting a total
that no longer matches reality. This checks the two invariants that catch it:

  1. Every `pub fn manifold_*` in the sys crate appears in the document, either
     by name or under one of the `manifold_{alloc,delete,destruct}_*` groups.
  2. The Summary totals equal the per-section row counts, and their sum equals
     the number of declarations.

Run from the repo root. Exits non-zero with a diff on failure.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SYS = ROOT / "crates/manifold-csg-sys/src/lib.rs"
DOC = ROOT / "API_COVERAGE.md"

GROUPED = re.compile(r"^manifold_(alloc|delete|destruct)_")


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
            f"OK: {len(declared)} declarations, all documented; "
            f"summary matches ({counts['wrapped']} wrapped, "
            f"{counts['internal']} internal, {counts['unused']} bound-unused)"
        )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
