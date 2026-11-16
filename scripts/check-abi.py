#!/usr/bin/env python3
"""Compare the sys crate's FFI with manifold's C header, type by type.

The declarations in crates/manifold-csg-sys/src/lib.rs are written by hand
and never see manifoldc.h, and check-api-coverage.py only compares function
names, so a same-name upstream change (a retyped parameter, a changed
callback, a new struct field) would compile on our side and break at
runtime. This script compares the two sides' canonical C types:

1. cbindgen renders our declarations as C, with every type name prefixed
   RS_ so the two sides cannot collide.
2. clang dumps the AST of that rendering and of manifoldc.h, once per
   target data model (x86-64 Linux, i686, wasm32, x86-64 Windows), with
   -ffreestanding so no system headers are needed.
3. Typedefs are resolved to their underlying types in each AST (uint64_t
   becomes `unsigned long` or `unsigned long long` depending on the target,
   ManifoldSdf becomes its function-pointer type), the RS_ prefix and
   struct/enum tags are dropped, and the results are compared:
   - every function we declare: return type and each parameter type;
   - every typedef we declare (callbacks included);
   - every struct we declare with fields: the complete field list, names,
     order and types (so a field added in padding still shows up);
   - every enum: each Rust variant must name a C constant with the same
     value, and every C constant must be covered. Names map by rule
     (MANIFOLD_[TYPE_]SCREAMING_VARIANT) plus explicit entries in
     scripts/abi/exceptions.json.

The Rust source is also checked for things cbindgen cannot render
faithfully: `#[link_name]` attributes and non-"C" ABIs are rejected, and
every `pub fn` / `pub struct` / `pub enum` / `pub type` must appear in
cbindgen's output.

Policy flags exist for the 3.5.x line on main, whose declarations are
deliberately ABI-compatible but not identical to its header:
--allow-typed-mem accepts a typed pointer where the header has `void *` for
a parameter named `mem` or `mem_*` (the caller-provided buffers), and
--allow-added-const accepts a Rust `*const` where the header's pointer is
not const. Nothing else is relaxed. The `next` branch needs neither.

scripts/abi/exceptions.json lists the deliberate differences accepted on
every branch: enum variants whose names differ from the header's, and
structs whose pointer fields we tightened to `*const` (input-only fields).

Not covered: two parameters of the same type swapping places (no type check
can see that), and calling-convention attributes beyond rejecting non-"C"
ABIs.

Needs cbindgen and clang (CLANG, default `clang`). The header comes from the
pinned ref, like check-api-coverage.py, or from MANIFOLD_CSG_HEADER_DIR (a
directory containing manifold/manifoldc.h and manifold/types.h).
"""

import argparse
import json
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
ABI_DIR = Path(__file__).resolve().parent / "abi"
HEADER_URL = "https://raw.githubusercontent.com/elalish/manifold/{ref}/bindings/c/include/manifold/{name}"
TARGETS = [
    "x86_64-unknown-linux-gnu",
    "i686-unknown-linux-gnu",
    "wasm32-unknown-unknown",
    "x86_64-pc-windows-msvc",
]

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pins  # noqa: E402


class Fail(Exception):
    pass


# ---- inputs --------------------------------------------------------------------

def fetch_headers(dest):
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
            raise Fail(f"cannot fetch {url}: {last}. Set MANIFOLD_CSG_HEADER_DIR to run offline.")
    return ref


def rust_source():
    return "\n".join(p.read_text() for p in sorted((SYS / "src").rglob("*.rs")))


def check_rust_source(src):
    """Reject constructs cbindgen would render without complaint but wrongly."""
    problems = []
    if re.search(r"\blink_name\b", src):
        problems.append("#[link_name] in the sys crate: cbindgen renders the Rust name, not the linked symbol")
    for abi in sorted(set(re.findall(r'extern\s+"([^"]+)"', src))):
        if abi != "C":
            problems.append(f'extern "{abi}" in the sys crate: only "C" is compared faithfully')
    return problems


def declared_items(src):
    """Names the sys crate declares: functions, and structs/enums/type aliases."""
    fns = set(re.findall(r"\bpub\s+(?:unsafe\s+)?fn\s+(manifold_\w+)", src))
    types = set(re.findall(r"\bpub\s+(?:struct|enum|union|type)\s+(\w+)", src))
    return fns, types


def run_cbindgen(output, include_types):
    config = (ABI_DIR / "cbindgen.toml").read_text()
    names = ", ".join(f'"{n}"' for n in sorted(include_types))
    config = config.replace("[export]\n", f"[export]\ninclude = [{names}]\n", 1)
    cfg = output.with_suffix(".toml")
    cfg.write_text(config)
    cmd = ["cbindgen", "--quiet", "--config", str(cfg), "--crate", "manifold-csg-sys",
           "--output", str(output), str(SYS)]
    try:
        subprocess.run(cmd, check=True)
    except FileNotFoundError:
        raise Fail("cbindgen not found; install it with `cargo install cbindgen --version 0.29.4`")


def clang_ast(source, include_dir, target):
    clang = os.environ.get("CLANG", "clang")
    cmd = [clang, f"--target={target}", "-ffreestanding", "-std=c11", "-fsyntax-only",
           "-Xclang", "-ast-dump=json", "-I", str(include_dir), str(source)]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        raise Fail(f"clang failed for {source.name} on {target}:\n{res.stderr}")
    return json.loads(res.stdout)


# ---- AST to canonical declarations ---------------------------------------------

IDENT = re.compile(r"[A-Za-z_]\w*")
C_KEYWORDS = {"const", "volatile", "restrict", "struct", "enum", "union", "unsigned", "signed",
              "int", "long", "short", "char", "double", "float", "void", "_Bool"}


def enum_value(const, previous):
    """Value of an EnumConstantDecl: its explicit initializer, else previous+1."""
    for sub in const.get("inner", []):
        if sub.get("kind") == "ConstantExpr" and "value" in sub:
            return int(sub["value"])
    return previous + 1


class Model:
    """Functions, typedefs, structs and enums from one AST, with typedefs
    resolvable to builtin types."""

    def __init__(self, ast, prefix=""):
        self.prefix = prefix
        self.functions, self.typedefs, self.records, self.enums = {}, {}, {}, {}
        for node in ast.get("inner", []):
            if node.get("isImplicit"):
                continue
            kind, name = node.get("kind"), node.get("name")
            if kind == "FunctionDecl" and name:
                params = [(p.get("name", ""), p["type"]["qualType"])
                          for p in node.get("inner", []) if p.get("kind") == "ParmVarDecl"]
                qual = node["type"]["qualType"]
                ret = qual[: qual.index("(")].strip()
                self.functions[name] = (ret, params, "..." in qual)
            elif kind == "TypedefDecl" and name:
                self.typedefs[name] = node["type"]["qualType"]
            elif kind == "RecordDecl" and name and node.get("completeDefinition"):
                self.records[name] = [(f["name"], f["type"]["qualType"])
                                      for f in node.get("inner", []) if f.get("kind") == "FieldDecl"]
            elif kind == "EnumDecl" and name:
                consts, value = [], -1
                for c in node.get("inner", []):
                    if c.get("kind") != "EnumConstantDecl":
                        continue
                    value = enum_value(c, value)
                    consts.append((c["name"], value))
                self.enums[name] = consts

    def canonical(self, qual, depth=0):
        """Resolve typedefs, then drop tags and the RS_ prefix."""
        if depth > 20:
            raise Fail(f"typedef cycle resolving {qual}")
        out = qual
        for tok in sorted(set(IDENT.findall(qual))):
            if tok in C_KEYWORDS:
                continue
            target = self.typedefs.get(tok)
            if target is None:
                continue
            base = re.sub(r"^(struct|enum|union)\s+", "", target).strip()
            if base == tok:  # typedef struct X X;
                continue
            resolved = self.canonical(target, depth + 1)
            if "(*)" in resolved:
                if out.strip() == tok:
                    return self._tidy(resolved)
                out = re.sub(rf"\b{re.escape(tok)}\b", f"({resolved})", out)
            else:
                out = re.sub(rf"\b{re.escape(tok)}\b", resolved, out)
        return self._tidy(out)

    def _tidy(self, t):
        t = re.sub(r"\b(struct|enum|union)\s+", "", t)
        if self.prefix:
            t = re.sub(rf"\b{self.prefix}(?=\w)", "", t)
        t = t.replace("*", " * ").replace("(", " ( ").replace(")", " ) ").replace(",", " , ")
        return " ".join(t.split())


# ---- comparison -----------------------------------------------------------------

def without_one_const(t):
    """`const T *` -> `T *` (the pointee qualifier only)."""
    return re.sub(r"\bconst ", "", t, count=1)


def param_relaxed(rust_t, c_t, pname, allow_mem, allow_const):
    """True when a deliberate main-branch difference makes these equal."""
    if allow_mem and (pname == "mem" or pname.startswith("mem_")) and c_t == "void *" and rust_t.endswith("*"):
        return True
    if allow_const and rust_t.endswith("*") and without_one_const(rust_t) == c_t:
        return True
    return False


def screaming(camel):
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", camel).upper()


def compare(c, r, fns, types, exceptions, allow_mem, allow_const):
    problems = []
    for name in sorted(fns - set(r.functions)):
        problems.append(f"cbindgen did not render fn {name}, so it is unchecked")
    for name, (rret, rparams, rvar) in sorted(r.functions.items()):
        if name not in c.functions:
            problems.append(f"fn {name}: declared in the sys crate but not in manifoldc.h")
            continue
        cret, cparams, cvar = c.functions[name]
        rr, cr = r.canonical(rret), c.canonical(cret)
        if rr != cr and not (allow_const and rr.endswith("*") and without_one_const(rr) == cr):
            problems.append(f"fn {name}: returns {rr}, header has {cr}")
        if rvar != cvar:
            problems.append(f"fn {name}: variadic on one side only")
        if len(rparams) != len(cparams):
            problems.append(f"fn {name}: {len(rparams)} parameters, header has {len(cparams)}")
            continue
        for i, ((rn, rt), (cn, ct)) in enumerate(zip(rparams, cparams)):
            rc, cc = r.canonical(rt), c.canonical(ct)
            if rc != cc and not param_relaxed(rc, cc, cn or rn, allow_mem, allow_const):
                problems.append(f"fn {name}: parameter {i + 1} ({rn}) is {rc}, header has {cc} ({cn})")

    rendered = {n.removeprefix(r.prefix) for n in (*r.typedefs, *r.records, *r.enums)}
    for name in sorted(types - rendered):
        problems.append(f"cbindgen did not render type {name}, so it is unchecked")

    for rname, rt in sorted(r.typedefs.items()):
        if not rname.startswith(r.prefix):
            continue  # stdint.h / stddef.h typedefs, not ours
        name = rname.removeprefix(r.prefix)
        base = re.sub(r"^(struct|enum|union)\s+", "", rt).strip().removeprefix(r.prefix)
        if base == name:
            continue  # typedef struct RS_X RS_X;
        if name not in c.typedefs:
            problems.append(f"typedef {name}: declared in the sys crate but not in the header")
            continue
        rc, cc = r.canonical(rt), c.canonical(c.typedefs[name])
        if rc != cc:
            problems.append(f"typedef {name}: is {rc}, header has {cc}")

    for rname, rfields in sorted(r.records.items()):
        name = rname.removeprefix(r.prefix)
        if [f for f, _ in rfields] == ["_private"]:
            continue  # opaque handle
        if name not in c.records:
            problems.append(f"struct {name}: declared with fields in the sys crate but not defined in the header")
            continue
        rlist = [(f, r.canonical(t)) for f, t in rfields]
        clist = [(f, c.canonical(t)) for f, t in c.records[name]]
        for i in range(max(len(rlist), len(clist))):
            rf = rlist[i] if i < len(rlist) else None
            cf = clist[i] if i < len(clist) else None
            if rf == cf:
                continue
            const_ok = allow_const or name in exceptions["added_const_fields"]
            if (const_ok and rf and cf and rf[0] == cf[0] and rf[1].endswith("*")
                    and without_one_const(rf[1]) == cf[1]):
                continue
            problems.append(f"struct {name}: field {i + 1} is {rf}, header has {cf}")

    for rname, rconsts in sorted(r.enums.items()):
        name = rname.removeprefix(r.prefix)
        if name not in c.enums:
            problems.append(f"enum {name}: declared in the sys crate but not in the header")
            continue
        cvals = dict(c.enums[name])
        words = screaming(name.removeprefix("Manifold"))
        covered = set()
        for rconst, value in rconsts:
            variant = rconst.removeprefix(f"{rname}_")
            explicit = exceptions["enum_names"].get(name, {}).get(variant)
            candidates = [explicit] if explicit else [
                f"MANIFOLD_{screaming(variant)}", f"MANIFOLD_{words}_{screaming(variant)}"]
            match = next((cand for cand in candidates if cand in cvals), None)
            if match is None:
                problems.append(
                    f"enum {name}: no header constant for variant {variant} (tried {', '.join(candidates)}); "
                    "if the name differs on purpose, map it in scripts/abi/exceptions.json")
                continue
            covered.add(match)
            if cvals[match] != value:
                problems.append(f"enum {name}: {variant} = {value}, header {match} = {cvals[match]}")
        for cconst in sorted(set(cvals) - covered):
            problems.append(f"enum {name}: header constant {cconst} = {cvals[cconst]} has no Rust variant")
    return problems


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--allow-typed-mem", action="store_true",
                    help="accept a typed pointer for a header `void *mem` parameter (main's 3.5.x line)")
    ap.add_argument("--allow-added-const", action="store_true",
                    help="accept a Rust *const where the header pointer is not const (main's 3.5.x line)")
    args = ap.parse_args()
    ref = "?"
    try:
        src = rust_source()
        problems = check_rust_source(src)
        fns, types = declared_items(src)
        exceptions = json.loads((ABI_DIR / "exceptions.json").read_text())
        per_target = {}
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            ref = fetch_headers(tmp)
            run_cbindgen(tmp / "rust.h", types)
            (tmp / "header.c").write_text('#include "manifold/manifoldc.h"\n')
            (tmp / "rust.c").write_text('#include <stdbool.h>\n#include "rust.h"\n')
            for target in TARGETS:
                c = Model(clang_ast(tmp / "header.c", tmp, target))
                r = Model(clang_ast(tmp / "rust.c", tmp, target), prefix="RS_")
                for p in compare(c, r, fns, types, exceptions, args.allow_typed_mem, args.allow_added_const):
                    per_target.setdefault(p, []).append(target)
        for p, targets in per_target.items():
            where = "all targets" if len(targets) == len(TARGETS) else ", ".join(targets)
            problems.append(f"{p} [{where}]")
        n_structs = sum(1 for f in r.records.values() if [x for x, _ in f] != ["_private"])
        n_typedefs = sum(1 for n, t in r.typedefs.items() if n.startswith("RS_")
                         and re.sub(r"^(struct|enum|union)\s+", "", t).strip() != n)
    except Fail as e:
        print(f"FAIL: {e}")
        return 1
    if problems:
        for p in problems:
            print(f"FAIL: {p}")
        print(f"FAIL: {len(problems)} difference(s) between the sys crate and manifoldc.h at {ref}")
        return 1
    print(f"OK: {len(r.functions)} functions, {n_structs} structs, {n_typedefs} typedefs and "
          f"{len(r.enums)} enums match manifoldc.h at {ref} on {', '.join(TARGETS)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
