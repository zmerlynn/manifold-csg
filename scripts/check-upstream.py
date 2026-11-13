#!/usr/bin/env python3
"""Report when the release tags this crate pins have fallen behind upstream.

It looks at:

  1. MANIFOLD_VERSION           vs the newest elalish/manifold release
  2. WASM_CXX_SHIM_TAG          vs the newest wasm-cxx-shim release

Whether flake.lock's nixpkgs ships the pinned manifold is not checked here.
flake.nix builds the pinned tag itself when it does not, and
.github/workflows/flake-lock.yml opens the `nix flake update` PR once
nixos-unstable catches up.

Only release tags are compared for drift. A SHA or branch pin is skipped,
not flagged: per CLAUDE.md those are a deliberate exception (a post-release
fix, a carry-patch base) and do not want a weekly reminder. Skips are still
*disclosed* in the report, so a run that checked less than usual can never
read as a clean bill of health.

Three result classes, deliberately distinct:

  findings  something is wrong and will stay wrong until someone acts. Drift,
            and structural breakage (a const that no longer parses, a repo
            with no comparable releases). Hashed, so a change here is what
            earns a notification.
  errors    a transient lookup failure. NOT hashed, and sets complete=false,
            because an incomplete run must not rewrite the stored state or
            reopen an issue that was deliberately deferred.
  skips     a check that could not apply, e.g. version comparison against a
            SHA pin. Informational; never drift.

Prints a markdown report on stdout. Writes `drift`, `complete`, `title` and
`hash` to $GITHUB_OUTPUT. Exit status is 0 whenever a report was produced;
non-zero is reserved for crashes, where the report would be untrustworthy.
"""

import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD_RS = ROOT / "crates/manifold-csg-sys/build.rs"
WASM_RS = ROOT / "crates/manifold-csg-sys/build/wasm.rs"

UA = "manifold-csg-upstream-watch (+https://github.com/zmerlynn/manifold-csg)"
MANIFOLD = "elalish/manifold"
SHIM = "zmerlynn/wasm-cxx-shim"

# Plain release tags only: `v3.5.2` or `3.5.2`. Anything with a pre-release or
# build suffix, and anything SHA-shaped, is deliberately not comparable here.
RELEASE_TAG = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)$")


class CheckError(Exception):
    """A structural problem: a const that no longer parses, a repo with no
    comparable releases.

    Kept separate from transient network failures because the two need
    opposite handling. A structural break never self-heals, so it has to be a
    *finding* -- hashed, and therefore notified. A network blip does self-heal,
    so it must not rewrite the stored hash or reopen a deferred issue."""


def version(tag):
    m = RELEASE_TAG.match(tag)
    return tuple(int(g) for g in m.groups()) if m else None


def api(path, retries=4):
    """GET api.github.com/{path}, retrying transient failures.

    Without retries a single 5xx at 13:00 Monday costs a full week of
    coverage, since the next scheduled run is seven days out. 403 is retried
    too: GitHub uses it for secondary rate limits, not just for auth.
    """
    url = f"https://api.github.com/{path}"
    headers = {"User-Agent": UA}
    if tok := os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = f"Bearer {tok}"
    last = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=30) as r:
                body = r.read().decode()
            return json.loads(body)
        except urllib.error.HTTPError as e:
            last = e
            if e.code in (403, 429) or 500 <= e.code < 600:
                # Transient: rate limits (GitHub uses 403 for the secondary
                # one) and server errors. Fall through to the retry below.
                pass
            elif 400 <= e.code < 500:
                # Any other 4xx is permanent: a moved package path, a renamed
                # repo, a revoked token, a repo taken down. Stated as a range
                # rather than a list, because a list has to stay complete --
                # and anything it misses becomes a transient error, which sets
                # complete=false and freezes the issue untouched every week
                # while real drift goes unreported.
                raise CheckError(f"{e.code} for {path} ({e.reason})") from e
            else:
                raise
            delay = e.headers.get("Retry-After") if e.headers else None
            if delay and delay.isdigit() and attempt < retries - 1:
                time.sleep(min(int(delay), 60))
                continue
        except Exception as e:  # noqa: BLE001 - timeouts, DNS, reset
            last = e
        if attempt < retries - 1:
            time.sleep(2**attempt)
    raise last


def const(path, name):
    """Read a `const NAME: &str = "..."`, anchored so a doc comment or a
    commented-out line cannot be picked up instead."""
    try:
        text = path.read_text()
    except OSError as e:
        raise CheckError(f"cannot read {path.name} ({e})") from e
    m = re.search(
        rf'^\s*(?:pub(?:\([^)]*\))?\s+)?const {name}: &str = "([^"]+)";',
        text,
        re.M,
    )
    if not m:
        raise CheckError(f"could not find `const {name}` in {path.name}")
    return m.group(1)


def newest_release(repo):
    """Highest released version, by version number rather than by date.

    `/releases/latest` is "most recent by date", so a backported patch cut
    after a newer release would shadow it. Taking the max over the 100 most
    recent releases avoids that, and lets us drop drafts and pre-releases
    explicitly. The list is date-descending, so the highest version is always
    on the first page for these repos; if a repo ever has no comparable tag in
    that window, the caller raises rather than reporting "current".
    """
    best = None
    for r in api(f"repos/{repo}/releases?per_page=100"):
        if r["draft"] or r["prerelease"]:
            continue
        if (v := version(r["tag_name"])) and (best is None or v > best[0]):
            best = (v, r["tag_name"])
    return best[1] if best else None


def check_pin(label, path, name, repo):
    """(finding, skip) for one pinned tag."""
    pinned = const(path, name)
    pin_v = version(pinned)
    if pin_v is None:
        return None, (
            f"- **{label}**: pinned `{pinned}`, which is not a release tag, so "
            "no drift comparison was made (assumed deliberate)."
        )
    latest = newest_release(repo)
    if latest is None:
        raise CheckError(f"{repo} has no comparable release tags")
    if version(latest) <= pin_v:
        return None, None
    return (
        f"- **{label}**: pinned `{pinned}`, newest release is "
        f"[`{latest}`](https://github.com/{repo}/releases/tag/{latest})"
    ), None


def main():
    findings, errors, skips = [], [], []

    def run(label, fn):
        try:
            finding, skip = fn()
            if finding:
                findings.append(finding)
            if skip:
                skips.append(skip)
        except CheckError as e:
            # Structural: never self-heals, so it belongs with the findings
            # where it is hashed and therefore notified.
            findings.append(f"- **{label}**: check is broken and needs fixing ({e})")
        except Exception as e:  # noqa: BLE001 - transient: network, DNS, 5xx
            errors.append(f"- **{label}**: lookup failed, status unknown ({e})")

    for label, path, name, repo in (
        ("manifold3d", BUILD_RS, "MANIFOLD_VERSION", MANIFOLD),
        ("wasm-cxx-shim", WASM_RS, "WASM_CXX_SHIM_TAG", SHIM),
    ):
        run(label, lambda p=path, n=name, r=repo, l=label: check_pin(l, p, n, r))

    out = []
    if findings:
        out.append("Needs attention:\n")
        out.append("\n".join(findings))
        out.append(
            "\nBefore bumping `MANIFOLD_VERSION`, check that `wasm-cxx-shim` "
            "supports the new pin, and update `pinnedSrc` in `flake.nix` "
            "(version + hash) or the `nix-offline` lane fails to evaluate. "
            "Do not wait for nixpkgs. See the Versioning section of CLAUDE.md."
        )
    if errors:
        if out:
            out.append("")
        out.append("Checks that could not run. Treat these as unknown, not as clear:\n")
        out.append("\n".join(errors))
    if skips:
        if out:
            out.append("")
        out.append("Checks that did not apply:\n")
        out.append("\n".join(skips))
    if not findings and not errors:
        headline = "No outstanding drift." if skips else "All pinned release tags are current."
        out = [headline, ""] + out if out else [headline]
    print("\n".join(out))

    # Hash the findings ONLY. Error text carries exception detail that varies
    # between outages, and hashing it would rewrite the issue body, fire a
    # spurious "findings changed" comment, and reopen an issue the maintainer
    # had deliberately deferred -- over a blip. `complete` is the other half of
    # that guarantee: an incomplete run must not advance the stored state.
    digest = hashlib.sha256("\n".join(findings).encode()).hexdigest()[:12]

    if findings:
        title = "Upstream pins need attention"
    elif errors:
        title = "Upstream pin check could not run"
    elif skips:
        # The body already withholds "all current" when something was skipped;
        # the title is the email subject and the issue-list row, so it has to
        # do the same or it becomes the most visible overstatement.
        title = "Upstream pins are current (some checks skipped)"
    else:
        title = "Upstream pins are current"

    if gho := os.environ.get("GITHUB_OUTPUT"):
        with open(gho, "a") as f:
            f.write(f"drift={'true' if (findings or errors) else 'false'}\n")
            f.write(f"complete={'false' if errors else 'true'}\n")
            f.write(f"hash={digest}\n")
            f.write(f"title={title}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
