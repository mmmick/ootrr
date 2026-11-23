"""Nightly build versions.

python tools/nightly_version.py next <yyyymmdd>[.<n>]
    Prints the nightly version: the next patch after Cargo.toml's version, as a
    prerelease, so 0.8.2 and 20261121.57 give 0.8.3-nightly.20261121.57. It sorts
    above the release it was built after and below the release it leads to.

python tools/nightly_version.py set <version>
    Writes <version> into Cargo.toml and into Cargo.lock's entries for the
    workspace's own packages, so a build with --locked still works.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOML = ROOT / "Cargo.toml"
LOCK = ROOT / "Cargo.lock"
WS_VERSION = re.compile(r'(\[workspace\.package\][^\[]*?\nversion = ")([^"]+)(")')
SEMVER = re.compile(r"\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?")


def current() -> str:
    m = WS_VERSION.search(TOML.read_text(encoding="utf-8"))
    if not m:
        sys.exit("Cargo.toml has no [workspace.package] version")
    return m.group(2)


def next_nightly(day: str) -> str:
    if not re.fullmatch(r"\d{8}(\.[1-9]\d*)?", day):
        sys.exit(f"the build stamp must be yyyymmdd or yyyymmdd.n, not {day!r}")
    base = current().split("-")[0].split("+")[0]
    major, minor, patch = (int(x) for x in base.split("."))
    return f"{major}.{minor}.{patch + 1}-nightly.{day}"


def set_version(v: str) -> None:
    if not SEMVER.fullmatch(v):
        sys.exit(f"{v!r} is not a version cargo accepts")
    old = current()
    t = TOML.read_text(encoding="utf-8")
    TOML.write_text(WS_VERSION.sub(lambda m: m.group(1) + v + m.group(3), t, count=1), encoding="utf-8", newline="\n")
    blocks = LOCK.read_text(encoding="utf-8").split("\n[[package]]\n")
    n = 0
    for i, b in enumerate(blocks):
        if i == 0 or "\nsource = " in b:
            continue
        b2, k = re.subn(rf'^version = "{re.escape(old)}"$', f'version = "{v}"', b, count=1, flags=re.M)
        blocks[i] = b2
        n += k
    if n == 0:
        sys.exit(f"Cargo.lock has no workspace package at version {old}")
    LOCK.write_text("\n[[package]]\n".join(blocks), encoding="utf-8", newline="\n")
    print(f"version {old} -> {v} in Cargo.toml and {n} Cargo.lock entries")


def main() -> None:
    if len(sys.argv) != 3 or sys.argv[1] not in ("next", "set"):
        sys.exit(__doc__)
    if sys.argv[1] == "next":
        print(next_nightly(sys.argv[2]))
    else:
        set_version(sys.argv[2])


if __name__ == "__main__":
    main()
