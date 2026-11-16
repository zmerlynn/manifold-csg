#!/usr/bin/env python3
"""Have a C compiler check the sys crate's FFI against manifold's C header.

The declarations in crates/manifold-csg-sys/src/lib.rs are written by hand
and never see manifoldc.h, and check-api-coverage.py only compares function
names. A same-name upstream change (a reordered or retyped parameter, a new
struct field) would compile on our side and break at runtime. This script
makes the C compiler do the comparison, with no parsing of our own:

- functions: cbindgen renders our `extern "C"` declarations as C, and the
  result is compiled after manifoldc.h. Redeclaring a function with an
  incompatible prototype is a C error ("conflicting types"), so any
  difference in parameter count, order, type, pointee constness or return
  type fails the compile and names the function.
- types: cbindgen renders our `#[repr(C)]` structs and enums under an RS_
  prefix, and _Static_asserts compare each with the header's: size,
  alignment, every field's offset and size, and every enum value (matched by
  position).

Needs cbindgen and a C compiler (CC, default `cc`). The header comes from
the pinned ref, like check-api-coverage.py, or from MANIFOLD_CSG_HEADER_DIR
(a directory containing manifold/manifoldc.h and manifold/types.h).

    python3 scripts/check-abi.py                   # functions and types
    python3 scripts/check-abi.py --types-only      # skip the function check

--types-only exists for the 3.5.x line on main, whose declarations are
deliberately ABI-compatible but not identical to the header: they type the
caller-provided `mem` buffers and use `*const` where the 3.5.x header was
not const-correct. C's check is stricter than the ABI there. The `next`
branch matches the header exactly, so it runs the full check.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SYS = ROOT / "crates/manifold-csg-sys"
CONFIGS = Path(__file__).resolve().parent / "abi"
HEADER_URL = "https://raw.githubusercontent.com/elalish/manifold/{ref}/bindings/c/include/manifold/{name}"

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pins  # noqa: E402


def fetch_headers(dest):
    """Put manifold/manifoldc.h and manifold/types.h under dest."""
    override = os.environ.get("MANIFOLD_CSG_HEADER_DIR")
    out = dest / "manifold"
    out.mkdir(parents=True)
    if override:
        for name in ("manifoldc.h", "types.h"):
            shutil.copy(Path(override) / "manifold" / name, out / name)
        return f"MANIFOLD_CSG_HEADER_DIR={override}"
    ref = pins.read("manifold")
    for name in ("manifoldc.h", "types.h"):
        url = HEADER_URL.format(ref=ref, name=name)
        last = None
        for _ in range(3):
            try:
                with urllib.request.urlopen(url, timeout=30) as resp:
                    (out / name).write_bytes(resp.read())
                break
            except OSError as e:
                last = e
        else:
            sys.exit(f"cannot fetch {url}: {last}. Set MANIFOLD_CSG_HEADER_DIR to run offline.")
    return ref


def sys_type_names():
    """Every struct and enum the sys crate declares at top level."""
    text = (SYS / "src/lib.rs").read_text()
    return re.findall(r"^pub (?:struct|enum) (\w+)", text, re.M)


def cbindgen(config, output, include=()):
    """Run cbindgen with a config from scripts/abi/. `include` forces types
    into the output that no function mentions, which cbindgen otherwise
    drops without a word."""
    config_text = (CONFIGS / config).read_text()
    if include:
        names = ", ".join(f'"{n}"' for n in include)
        config_text = config_text.replace("[export]\n", f"[export]\ninclude = [{names}]\n", 1)
    config_path = output.with_suffix(".toml")
    config_path.write_text(config_text)
    cmd = ["cbindgen", "--quiet", "--config", str(config_path),
           "--crate", "manifold-csg-sys", "--output", str(output), str(SYS)]
    try:
        subprocess.run(cmd, check=True)
    except FileNotFoundError:
        sys.exit("cbindgen not found; install it with `cargo install cbindgen`")


STRUCT = re.compile(r"typedef struct (\w+) \{(.*?)\} \1;", re.S)
ENUM = re.compile(r"typedef enum (\w+) \{(.*?)\} \1;", re.S)
FIELD = re.compile(r"(\w+)(?:\[\d*\])?;\s*$")


def rust_types(text):
    """RS_-prefixed structs (name -> fields) and enums (name -> values)."""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    structs, enums = {}, {}
    for name, body in STRUCT.findall(text):
        fields = [FIELD.search(line).group(1) for line in body.split("\n") if line.strip()]
        if fields == ["_private"]:
            continue  # opaque handle: no layout to compare
        structs[name] = fields
    for name, body in ENUM.findall(text):
        enums[name] = [v.split("=")[0].strip() for v in body.split(",") if v.strip()]
    return structs, enums


def c_enum_constants(types_h):
    """The header's enums: name -> constants in declaration order."""
    text = re.sub(r"//[^\n]*", "", types_h)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return {
        name: [v.split("=")[0].strip() for v in body.split(",") if v.strip()]
        for name, body in ENUM.findall(text)
    }


def type_asserts(structs, enums, c_enums):
    lines, problems = [], []
    for rs, fields in structs.items():
        c = rs.removeprefix("RS_")
        lines.append(f'_Static_assert(sizeof({rs}) == sizeof({c}), "{c}: size differs");')
        lines.append(f'_Static_assert(_Alignof({rs}) == _Alignof({c}), "{c}: alignment differs");')
        for f in fields:
            lines.append(f'_Static_assert(offsetof({rs}, {f}) == offsetof({c}, {f}), "{c}.{f}: offset differs");')
            lines.append(f'_Static_assert(sizeof((({rs}*)0)->{f}) == sizeof((({c}*)0)->{f}), "{c}.{f}: size differs");')
    for rs, variants in enums.items():
        c = rs.removeprefix("RS_")
        if c not in c_enums:
            problems.append(f"enum {c} is declared in the sys crate but not in types.h")
            continue
        if len(variants) != len(c_enums[c]):
            problems.append(f"enum {c}: sys crate has {len(variants)} variants, types.h has {len(c_enums[c])}")
        lines.append(f'_Static_assert(sizeof({rs}) == sizeof({c}), "{c}: size differs");')
        for ours, theirs in zip(variants, c_enums[c]):
            lines.append(f'_Static_assert((long long){ours} == (long long){theirs}, "{ours} != {theirs}");')
    return lines, problems


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--types-only", action="store_true", help="skip the function signature check")
    args = ap.parse_args()

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        source = fetch_headers(tmp)
        declared = sys_type_names()
        cbindgen("cbindgen-types.toml", tmp / "rust_types.h", include=declared)
        structs, enums = rust_types((tmp / "rust_types.h").read_text())
        emitted = {n.removeprefix("RS_") for n in (*structs, *enums)}
        opaque = set(re.findall(r"typedef struct RS_(\w+) \{\s*uint8_t _private", (tmp / "rust_types.h").read_text()))
        missing = [n for n in declared if n not in emitted | opaque]
        c_enums = c_enum_constants((tmp / "manifold/types.h").read_text())
        asserts, problems = type_asserts(structs, enums, c_enums)
        problems += [f"cbindgen did not emit sys type {n}, so its layout is unchecked" for n in missing]

        includes = ['#include <stddef.h>', '#include "manifold/manifoldc.h"', '#include "rust_types.h"']
        if not args.types_only:
            cbindgen("cbindgen-functions.toml", tmp / "rust_functions.h")
            includes.append('#include "rust_functions.h"')
        (tmp / "check.c").write_text("\n".join(includes + asserts) + "\n")

        cc = os.environ.get("CC", "cc")
        result = subprocess.run(
            [cc, "-std=c11", "-fsyntax-only", "-Werror", "-I", str(tmp), str(tmp / "check.c")],
            capture_output=True, text=True,
        )
        n_fns = 0 if args.types_only else len(re.findall(r"^extern ", (tmp / "rust_functions.h").read_text(), re.M))
        if result.returncode != 0 or problems:
            sys.stdout.write(result.stdout + result.stderr)
            for p in problems:
                print(f"FAIL: {p}")
            print(f"FAIL: the sys crate's FFI does not match manifoldc.h at {source}")
            return 1
        what = "types" if args.types_only else f"{n_fns} functions and types"
        print(f"OK: {what} match manifoldc.h at {source} "
              f"({len(structs)} structs, {len(enums)} enums)")
        return 0


if __name__ == "__main__":
    sys.exit(main())
