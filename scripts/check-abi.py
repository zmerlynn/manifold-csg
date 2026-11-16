#!/usr/bin/env python3
"""Compare the sys crate's FFI with manifold's C header, type by type.

The declarations in crates/manifold-csg-sys/src/lib.rs are written by hand
and never see manifoldc.h, and check-api-coverage.py only compares function
names, so a same-name upstream change (a retyped parameter, a changed
callback, a new struct field) would compile on our side and break at
runtime. This script compares the two sides:

1. cbindgen renders our declarations as C, with every type name prefixed
   RS_ so the two renderings cannot collide.
2. clang dumps the JSON AST of that rendering and of manifoldc.h, once per
   target data model (x86-64 Linux, i686, wasm32, x86-64 Windows), with
   -ffreestanding so no system headers are needed. Typedefs are resolved to
   builtin types in each, and the script compares:
   - every function: return type and each parameter type;
   - every callback typedef;
   - every struct with fields: the complete field list (names, order,
     types), so a field added in padding still shows up;
   - every enum: each Rust variant must name a C constant with the same
     value, and every C constant must be covered. Names map by rule
     (MANIFOLD_[TYPE_]SCREAMING_VARIANT) plus scripts/abi/exceptions.json.
3. A second compile per target puts both sides' types in one translation
   unit and _Static_asserts their layouts: sizeof, alignment, every field's
   offset, and each enum's size. That catches packed or aligned types,
   which the type comparison alone cannot see.

The Rust source (comments stripped) is also checked for what cbindgen
renders without complaint but wrongly, and those fail loudly: `#[link_name]`,
an ABI other than "C", any `#[repr]` other than plain `#[repr(C)]`, a
callback not wrapped in `Option<...>` (C cannot express non-null), and a
`#[cfg]` other than the known wasm32-unknown-unknown OBJ gate (cbindgen runs
once, on the host, so cfg is not evaluated per target). Every declared item
must come out of cbindgen with a full definition. Functions returning
function pointers and calling-convention attributes in the header are
rejected as unsupported rather than compared.

Deliberate differences: scripts/abi/exceptions.json (every branch) lists
enum variants whose names differ from the header's and the exact struct
fields we tightened to `*const`. `--baseline FILE` adds an exact list of
per-parameter differences, for the 3.5.x line on main, whose declarations
type the caller-provided `mem` buffers and add `*const` where its header was
not const-correct (scripts/abi/baseline-main.json, written with
`--write-baseline`). Each entry must still apply; a stale entry fails.

Not covered: two parameters of the same type swapping places (no type check
can see that), callback nullability beyond requiring `Option<...>`, and
calling conventions beyond rejecting non-"C" ABIs. 32-bit Windows is not a
target.

Needs cbindgen 0.29.4 and clang (CLANG, default `clang`). The header comes
from the pinned ref, like check-api-coverage.py, or from
MANIFOLD_CSG_HEADER_DIR (a directory containing manifold/manifoldc.h and
manifold/types.h). MANIFOLD_CSG_SYS_DIR points the check at another copy of
the sys crate (used by the self-test).
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
ABI_DIR = Path(__file__).resolve().parent / "abi"
HEADER_URL = "https://raw.githubusercontent.com/elalish/manifold/{ref}/bindings/c/include/manifold/{name}"
TARGETS = [
    "x86_64-unknown-linux-gnu",
    "i686-unknown-linux-gnu",
    "wasm32-unknown-unknown",
    "x86_64-pc-windows-msvc",
]
KNOWN_CFG = 'not(all(target_arch = "wasm32", target_os = "unknown"))'

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pins  # noqa: E402


class Fail(Exception):
    pass


def sys_dir():
    return Path(os.environ.get("MANIFOLD_CSG_SYS_DIR", ROOT / "crates/manifold-csg-sys"))


# ---- inputs ---------------------------------------------------------------------

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


def strip_rust_comments(src):
    """Drop // and /* */ comments (doc comments included), keeping strings.
    Good enough for the sys crate, which has no comment markers in strings."""
    src = re.sub(r"/\*.*?\*/", " ", src, flags=re.S)
    return re.sub(r"//[^\n]*", "", src)


def rust_source():
    return strip_rust_comments("\n".join(p.read_text() for p in sorted((sys_dir() / "src").rglob("*.rs"))))


def check_rust_source(src):
    """Reject constructs cbindgen would render without complaint but wrongly."""
    problems = []
    if re.search(r"\blink_name\b|\blink_ordinal\b", src):
        problems.append("#[link_name]/#[link_ordinal] in the sys crate: cbindgen renders the Rust name, not the linked symbol")
    for abi in sorted(set(re.findall(r'\bextern\s+"([^"]*)"', src))):
        if abi != "C":
            problems.append(f'extern "{abi}" in the sys crate: only "C" is compared')
    if re.search(r"\bextern\s+(?:fn|\{)", src):
        problems.append('an `extern` without an explicit ABI string in the sys crate: write extern "C"')
    for rep in sorted(set(re.findall(r"#\s*\[\s*repr\s*\(([^)]*)\)\s*\]", src))):
        if rep.replace(" ", "") != "C":
            problems.append(f"#[repr({rep})] in the sys crate: only plain #[repr(C)] is compared (cbindgen drops or renders other reprs unfaithfully)")
    for cfg in sorted(set(re.findall(r"#\s*\[\s*cfg\s*\((.*?)\)\s*\]\s*(?=\S)", src, re.S))):
        if " ".join(cfg.split()) != KNOWN_CFG:
            problems.append(f"#[cfg({' '.join(cfg.split())})] in the sys crate: cbindgen runs once on the host, so only the known wasm32-unknown-unknown OBJ gate is accepted")
    fn_ptrs = len(re.findall(r'\bextern\s+"C"\s+fn\b', src))
    wrapped = len(re.findall(r'\bOption\s*<\s*unsafe\s+extern\s+"C"\s+fn\b', src))
    if fn_ptrs != wrapped:
        problems.append(f"{fn_ptrs - wrapped} callback type(s) not wrapped in Option<...>: C cannot say a function pointer is non-null, so the comparison cannot check that")
    return problems


def declared_items(src):
    """What the sys crate declares, by kind."""
    fns = set(re.findall(r"\bpub\s+(?:unsafe\s+)?fn\s+(manifold_\w+)", src))
    structs = set(re.findall(r"\bpub\s+struct\s+(\w+)", src))
    unions = set(re.findall(r"\bpub\s+union\s+(\w+)", src))
    enums = set(re.findall(r"\bpub\s+enum\s+(\w+)", src))
    aliases = set(re.findall(r"\bpub\s+type\s+(\w+)", src))
    return fns, structs | unions, enums, aliases


def run_cbindgen(output, include_types, functions=True):
    config = (ABI_DIR / "cbindgen.toml").read_text()
    names = ", ".join(f'"{n}"' for n in sorted(include_types))
    config = config.replace("[export]\n", f"[export]\ninclude = [{names}]\n", 1)
    if not functions:
        config = config.replace('item_types = ["functions", ', "item_types = [")
    cfg = output.with_suffix(".toml")
    cfg.write_text(config)
    cmd = ["cbindgen", "--quiet", "--config", str(cfg), "--crate", "manifold-csg-sys",
           "--output", str(output), str(sys_dir())]
    try:
        res = subprocess.run(cmd, capture_output=True, text=True)
    except FileNotFoundError:
        raise Fail("cbindgen not found; install it with `cargo install cbindgen --version 0.29.4`")
    if res.returncode != 0:
        raise Fail(f"cbindgen failed:\n{res.stderr}")


def clang(args):
    exe = os.environ.get("CLANG", "clang")
    return subprocess.run([exe, "-ffreestanding", "-std=c11", "-fsyntax-only", *args],
                          capture_output=True, text=True)


def clang_ast(source, include_dir, target):
    res = clang([f"--target={target}", "-Xclang", "-ast-dump=json", "-I", str(include_dir), str(source)])
    if res.returncode != 0:
        raise Fail(f"clang failed for {source.name} on {target}:\n{res.stderr}")
    return json.loads(res.stdout)


# ---- AST to canonical declarations ---------------------------------------------

IDENT = re.compile(r"[A-Za-z_]\w*")
C_KEYWORDS = {"const", "volatile", "restrict", "struct", "enum", "union", "unsigned", "signed",
              "int", "long", "short", "char", "double", "float", "void", "_Bool"}
CALLCONV = re.compile(r"__attribute__\s*\(\(\s*(?:stdcall|cdecl|fastcall|thiscall|vectorcall|ms_abi|sysv_abi)|__(?:stdcall|cdecl|fastcall|thiscall|vectorcall)\b")


def constant_value(node):
    """The evaluated `value` of the first ConstantExpr anywhere below node."""
    if node.get("kind") == "ConstantExpr" and "value" in node:
        return int(node["value"])
    for sub in node.get("inner", []):
        v = constant_value(sub)
        if v is not None:
            return v
    return None


class Model:
    """Functions, typedefs, structs and enums from one AST."""

    def __init__(self, ast, prefix=""):
        self.prefix = prefix
        self.functions, self.typedefs, self.records, self.enums = {}, {}, {}, {}
        self.unsupported = []
        for node in ast.get("inner", []):
            if node.get("isImplicit"):
                continue
            kind, name = node.get("kind"), node.get("name")
            if kind == "FunctionDecl" and name:
                qual = node["type"]["qualType"]
                if CALLCONV.search(qual):
                    self.unsupported.append(f"fn {name}: calling-convention attribute in its type ({qual}); not compared")
                params = [(p.get("name", ""), p["type"]["qualType"])
                          for p in node.get("inner", []) if p.get("kind") == "ParmVarDecl"]
                ret = qual[: qual.index("(")].strip()
                if "(*" in qual[: qual.rindex(")")] and re.match(r"^[^(]*\(\s*\*", qual):
                    self.unsupported.append(f"fn {name}: returns a function pointer ({qual}); not supported, compare it by hand")
                self.functions[name] = (ret, params, "..." in qual)
            elif kind == "TypedefDecl" and name:
                self.typedefs[name] = node["type"]["qualType"]
                if CALLCONV.search(node["type"]["qualType"]):
                    self.unsupported.append(f"typedef {name}: calling-convention attribute; not compared")
            elif kind == "RecordDecl" and name and node.get("completeDefinition"):
                self.records[name] = [(f["name"], f["type"]["qualType"])
                                      for f in node.get("inner", []) if f.get("kind") == "FieldDecl"]
            elif kind == "EnumDecl" and name:
                consts, value = [], -1
                for c in node.get("inner", []):
                    if c.get("kind") != "EnumConstantDecl":
                        continue
                    explicit = constant_value(c)
                    if explicit is None and c.get("inner"):
                        raise Fail(f"enum {name}: cannot read the value of {c['name']} from clang's AST")
                    value = explicit if explicit is not None else value + 1
                    consts.append((c["name"], value))
                self.enums[name] = consts

    def canonical(self, qual, depth=0):
        """Resolve typedefs, then drop tags and the RS_ prefix."""
        if depth > 20:
            raise Fail(f"typedef cycle resolving {qual}")
        whole = qual.strip()
        out = qual
        for tok in sorted(set(IDENT.findall(qual))):
            if tok in C_KEYWORDS or tok not in self.typedefs:
                continue
            target = self.typedefs[tok]
            if re.sub(r"^(struct|enum|union)\s+", "", target).strip() == tok:
                continue  # typedef struct X X;
            resolved = self.canonical(target, depth + 1)
            if "*" in resolved or "(" in resolved:
                # A pointer or function-pointer typedef. Textual substitution
                # cannot keep qualifier binding (`const P` is a const pointer)
                # or declarator grammar, so it is only allowed as the whole type.
                if whole == tok:
                    return resolved
                raise Fail(f"pointer typedef {tok} used inside another type ({qual}); not supported by this check")
            out = re.sub(rf"\b{re.escape(tok)}\b", resolved, out)
        return self._tidy(out)

    def _tidy(self, t):
        t = re.sub(r"\b(struct|enum|union)\s+", "", t)
        t = re.sub(r"\bbool\b", "_Bool", t)
        if self.prefix:
            t = re.sub(rf"\b{self.prefix}(?=\w)", "", t)
        t = t.replace("*", " * ").replace("(", " ( ").replace(")", " ) ").replace(",", " , ")
        return " ".join(t.split())


# ---- comparison -----------------------------------------------------------------

def compare(c, r, decl, exceptions, baseline):
    """Return (problems, accepted): accepted lists the baseline entries used."""
    fns, structs, enums, aliases = decl
    problems, accepted = [], set()

    def differs(item, rust_t, c_t):
        if rust_t == c_t:
            return False
        key = (item, rust_t, c_t)
        if key in baseline:
            accepted.add(key)
            return False
        return True

    problems += c.unsupported + r.unsupported
    for name in sorted(fns - set(r.functions)):
        problems.append(f"cbindgen did not render fn {name}, so it is unchecked")
    for name, (rret, rparams, rvar) in sorted(r.functions.items()):
        if name not in c.functions:
            problems.append(f"fn {name}: declared in the sys crate but not in manifoldc.h")
            continue
        cret, cparams, cvar = c.functions[name]
        rr, cr = r.canonical(rret), c.canonical(cret)
        if differs(f"{name}:return", rr, cr):
            problems.append(f"fn {name}: returns {rr}, header has {cr}")
        if rvar != cvar:
            problems.append(f"fn {name}: variadic on one side only")
        if len(rparams) != len(cparams):
            problems.append(f"fn {name}: {len(rparams)} parameters, header has {len(cparams)}")
            continue
        for i, ((rn, rt), (cn, ct)) in enumerate(zip(rparams, cparams)):
            rc, cc = r.canonical(rt), c.canonical(ct)
            if differs(f"{name}:{i + 1}", rc, cc):
                problems.append(f"fn {name}: parameter {i + 1} ({rn}) is {rc}, header has {cc} ({cn})")

    r_records = {n.removeprefix(r.prefix): v for n, v in r.records.items()}
    r_enums = {n.removeprefix(r.prefix): v for n, v in r.enums.items()}
    r_aliases = {n.removeprefix(r.prefix): v for n, v in r.typedefs.items() if n.startswith(r.prefix)}
    for name in sorted(structs - set(r_records)):
        problems.append(f"struct {name}: cbindgen rendered no full definition, so its layout is unchecked")
    for name in sorted(enums - set(r_enums)):
        problems.append(f"enum {name}: cbindgen rendered no definition, so it is unchecked")
    for name in sorted(aliases - set(r_aliases)):
        problems.append(f"type {name}: cbindgen rendered no typedef, so it is unchecked")

    for name in sorted(aliases & set(r_aliases)):
        if name not in c.typedefs:
            problems.append(f"typedef {name}: declared in the sys crate but not in the header")
            continue
        rc, cc = r.canonical(r_aliases[name]), c.canonical(c.typedefs[name])
        if rc != cc:
            problems.append(f"typedef {name}: is {rc}, header has {cc}")

    const_fields = exceptions.get("added_const_fields", {})
    for name, rfields in sorted(r_records.items()):
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
            if (rf and cf and rf[0] == cf[0] and rf[0] in const_fields.get(name, [])
                    and re.sub(r"\bconst ", "", rf[1], count=1) == cf[1]):
                continue
            problems.append(f"struct {name}: field {i + 1} is {rf}, header has {cf}")

    names = exceptions.get("enum_names", {})
    for name, rconsts in sorted(r_enums.items()):
        if name not in c.enums:
            problems.append(f"enum {name}: declared in the sys crate but not in the header")
            continue
        cvals = dict(c.enums[name])
        words = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name.removeprefix("Manifold")).upper()
        covered = set()
        for rconst, value in rconsts:
            variant = rconst.removeprefix(f"{r.prefix}{name}_")
            scream = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", variant).upper()
            explicit = names.get(name, {}).get(variant)
            candidates = [explicit] if explicit else [f"MANIFOLD_{scream}", f"MANIFOLD_{words}_{scream}"]
            match = next((cand for cand in candidates if cand in cvals), None)
            if match is None:
                problems.append(f"enum {name}: no header constant for variant {variant} (tried {', '.join(candidates)}); "
                                "if the name differs on purpose, map it in scripts/abi/exceptions.json")
                continue
            covered.add(match)
            if cvals[match] != value:
                problems.append(f"enum {name}: {variant} = {value}, header {match} = {cvals[match]}")
        for cconst in sorted(set(cvals) - covered):
            problems.append(f"enum {name}: header constant {cconst} = {cvals[cconst]} has no Rust variant")
    return problems, accepted


def layout_check(tmp, target, r):
    """Compile both sides' types in one TU and assert their layouts match."""
    lines = ["#include <stddef.h>", '#include "manifold/manifoldc.h"', '#include "rust_types.h"']
    for rname, fields in sorted(r.records.items()):
        if [f for f, _ in fields] == ["_private"]:
            continue
        c = rname.removeprefix(r.prefix)
        lines.append(f'_Static_assert(sizeof({rname}) == sizeof({c}), "struct {c}: size differs");')
        lines.append(f'_Static_assert(_Alignof({rname}) == _Alignof({c}), "struct {c}: alignment differs");')
        for f, _ in fields:
            lines.append(f'_Static_assert(offsetof({rname}, {f}) == offsetof({c}, {f}), "struct {c}: offset of {f} differs");')
    for rname in sorted(r.enums):
        c = rname.removeprefix(r.prefix)
        lines.append(f'_Static_assert(sizeof({rname}) == sizeof({c}), "enum {c}: size differs");')
        lines.append(f'_Static_assert(_Alignof({rname}) == _Alignof({c}), "enum {c}: alignment differs");')
    (tmp / "layout.c").write_text("\n".join(lines) + "\n")
    res = clang([f"--target={target}", "-ferror-limit=0", "-I", str(tmp), str(tmp / "layout.c")])
    if res.returncode == 0:
        return []
    msgs = re.findall(r'static assertion failed[^"]*"([^"]+)"', res.stderr)
    return msgs or [f"layout check did not compile:\n{res.stderr}"]


# ---- driver ---------------------------------------------------------------------

def load_baseline(path):
    if not path:
        return set()
    return {tuple(e) for e in json.loads(Path(path).read_text())["accepted"]}


def run(baseline_path=None, write_baseline=None):
    src = rust_source()
    problems = check_rust_source(src)
    decl = declared_items(src)
    exceptions = json.loads((ABI_DIR / "exceptions.json").read_text())
    baseline = load_baseline(baseline_path)
    per_target, used, recorded = {}, set(), set()
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        ref = fetch_headers(tmp)
        types = decl[1] | decl[2] | decl[3]
        run_cbindgen(tmp / "rust.h", types)
        run_cbindgen(tmp / "rust_types.h", types, functions=False)
        (tmp / "header.c").write_text('#include "manifold/manifoldc.h"\n')
        (tmp / "rust.c").write_text('#include <stdbool.h>\n#include "rust.h"\n')
        for target in TARGETS:
            c = Model(clang_ast(tmp / "header.c", tmp, target))
            r = Model(clang_ast(tmp / "rust.c", tmp, target), prefix="RS_")
            found, accepted = compare(c, r, decl, exceptions, baseline if not write_baseline else set())
            if write_baseline:
                for p in found:
                    m = re.match(r"fn (\w+): (?:returns (.+), header has (.+)|parameter (\d+) \(\w*\) is (.+), header has (.+) \(\w*\))$", p)
                    if not m:
                        raise Fail(f"cannot baseline this difference, fix it instead: {p}")
                    if m.group(2):
                        recorded.add((f"{m.group(1)}:return", m.group(2), m.group(3)))
                    else:
                        recorded.add((f"{m.group(1)}:{m.group(4)}", m.group(5), m.group(6)))
                found = []
            used |= accepted
            found += layout_check(tmp, target, Model(clang_ast(tmp / "rust.c", tmp, target), prefix="RS_"))
            for p in found:
                per_target.setdefault(p, []).append(target)
        stats = (len(r.functions), sum(1 for f in r.records.values() if [x for x, _ in f] != ["_private"]),
                 sum(1 for n in r.typedefs if n.startswith("RS_") and n.removeprefix("RS_") in decl[3]),
                 len(r.enums))
    for p, targets in per_target.items():
        where = "all targets" if len(targets) == len(TARGETS) else ", ".join(targets)
        problems.append(f"{p} [{where}]")
    for entry in sorted(baseline - used):
        problems.append(f"stale baseline entry, no longer needed: {list(entry)}")
    if write_baseline:
        Path(write_baseline).write_text(json.dumps(
            {"_comment": "Exact differences check-abi.py accepts for this branch's declarations. "
                         "Regenerate with --write-baseline only after reviewing the diff.",
             "accepted": sorted(list(e) for e in recorded)}, indent=1) + "\n")
    return ref, problems, stats


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--baseline", help="JSON file of exact differences to accept (main's 3.5.x line)")
    ap.add_argument("--write-baseline", metavar="FILE",
                    help="record every function difference into FILE instead of failing on it")
    args = ap.parse_args()
    try:
        ref, problems, (n_fn, n_st, n_td, n_en) = run(args.baseline, args.write_baseline)
    except Fail as e:
        print(f"FAIL: {e}")
        return 1
    if problems:
        for p in problems:
            print(f"FAIL: {p}")
        print(f"FAIL: {len(problems)} problem(s) comparing the sys crate with manifoldc.h at {ref}")
        return 1
    print(f"OK: {n_fn} functions, {n_st} structs, {n_td} callback typedefs and {n_en} enums match "
          f"manifoldc.h at {ref} in types and layout, on {', '.join(TARGETS)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
