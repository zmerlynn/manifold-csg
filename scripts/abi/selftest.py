#!/usr/bin/env python3
"""Prove scripts/check-abi.py still catches mismatches, by mutation.

Runs the check on the real tree (must pass), then on copies of the sys crate
or of the pinned header with one deliberate ABI break each (each must fail).
Arguments are passed through to check-abi.py, so CI runs this with the same
--baseline as the real check. The mutations use spellings shared by the 3.5.x
line on main and by next, so the same self-test works on both.

    python3 scripts/abi/selftest.py [--baseline scripts/abi/baseline-main.json]
"""

import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
CHECK = ROOT / "scripts/check-abi.py"

# (name, file, regex, replacement). File "lib.rs" mutates the sys crate copy;
# "types.h" / "manifoldc.h" mutate the header copy.
MUTATIONS = [
    ("struct field retyped", "lib.rs", r"pub distance: f64,", "pub distance: f32,"),
    ("enum value shifted", "lib.rs", r"\bAdd = 0,", "Add = 9,"),
    ("return retyped", "lib.rs", r"pub fn manifold_volume\(m: \*const ManifoldManifold\) -> f64;",
     "pub fn manifold_volume(m: *const ManifoldManifold) -> f32;"),
    ("packed repr", "lib.rs", r"#\[repr\(C\)\]", "#[repr(C, packed)]"),
    ("header struct packed", "types.h", r"typedef struct ManifoldVec3 \{",
     "typedef struct __attribute__((packed, aligned(1))) ManifoldVec3 {"),
    ("header param added", "manifoldc.h", r"double manifold_volume\(((?:const )?ManifoldManifold\* m)\);",
     r"double manifold_volume(\1, int extra);"),
]


def run(args, env):
    res = subprocess.run([sys.executable, str(CHECK), *args], capture_output=True, text=True, env=env)
    return res.returncode, res.stdout + res.stderr


def mutate(path, pattern, repl):
    text = path.read_text()
    new, n = re.subn(pattern, repl, text, count=1)
    if n != 1:
        raise SystemExit(f"selftest: mutation pattern {pattern!r} not found in {path.name}; update the self-test")
    path.write_text(new)


def main():
    args = sys.argv[1:]
    base_env = dict(os.environ)
    rc, out = run(args, base_env)
    if rc != 0:
        print(out)
        print("selftest: the unmodified tree must pass first")
        return 1
    failures = 0
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        headers = tmp / "headers"
        # Reuse check-abi.py's own header fetch so the self-test sees the same pin.
        sys.path.insert(0, str(CHECK.parent))
        import importlib.util
        spec = importlib.util.spec_from_file_location("check_abi", CHECK)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        mod.fetch_headers(headers)
        for name, target, pattern, repl in MUTATIONS:
            env = dict(base_env)
            case = tmp / re.sub(r"\W+", "_", name)
            if target == "lib.rs":
                # The whole workspace, not just the crate: cbindgen runs
                # `cargo metadata`, which needs the workspace root for the
                # crate's inherited Cargo.toml fields.
                shutil.copytree(ROOT, case / "repo", ignore=shutil.ignore_patterns("target", ".git"))
                mutate(case / "repo/crates/manifold-csg-sys/src/lib.rs", pattern, repl)
                env["MANIFOLD_CSG_SYS_DIR"] = str(case / "repo/crates/manifold-csg-sys")
            else:
                shutil.copytree(headers, case / "headers")
                mutate(case / "headers/manifold" / target, pattern, repl)
                env["MANIFOLD_CSG_HEADER_DIR"] = str(case / "headers")
            rc, out = run(args, env)
            first = next((line for line in out.splitlines() if line.startswith("FAIL")), "")
            if rc == 0:
                failures += 1
                print(f"selftest: NOT CAUGHT: {name}")
            elif re.match(r"FAIL: (cbindgen|clang) failed|FAIL: cannot fetch|FAIL: cbindgen not found", first):
                failures += 1
                print(f"selftest: BROKEN (the check did not run, so this proves nothing): {name}: {first[:150]}")
            else:
                print(f"selftest: caught {name}: {first[:150]}")
    if failures:
        print(f"selftest: {failures} mutation(s) passed the check")
        return 1
    print(f"selftest: all {len(MUTATIONS)} mutations caught")
    return 0


if __name__ == "__main__":
    sys.exit(main())
