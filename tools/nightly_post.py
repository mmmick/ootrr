"""Writes the nightly release notes and the Discord message.

python tools/nightly_post.py notes <version> <since> <download_url> <out.md>
    Release notes: the commits on main since the last nightly, then how to install.

python tools/nightly_post.py discord <version> <since> <download_url> <out.json>
    A Discord webhook body with the same commit list, cut to fit an embed.
"""

import json
import os
import subprocess
import sys

EMBED_MAX = 3800
FIRST_NIGHTLY = 15


def git(*a: str) -> str:
    return subprocess.run(["git", *a], check=True, capture_output=True, text=True, encoding="utf-8").stdout


def is_ancestor(since: str) -> bool:
    return bool(since) and subprocess.run(["git", "merge-base", "--is-ancestor", since, "HEAD"], capture_output=True).returncode == 0


def changes(since: str) -> list[str]:
    span = [f"{since}..HEAD"] if is_ancestor(since) else [f"-{FIRST_NIGHTLY}", "HEAD"]
    out: list[str] = []
    for h in git("rev-list", "--first-parent", *span).split():
        subject = git("log", "-1", "--format=%s", h).strip()
        parents = git("rev-list", "--parents", "-n1", h).split()
        if len(parents) > 2:
            title = next((l.strip() for l in git("log", "-1", "--format=%b", h).splitlines() if l.strip()), "")
            pr = next((w for w in subject.split() if w.startswith("#")), "")
            if title:
                subject = f"{title} ({pr})" if pr else title
        out.append(f"- {subject} ({h[:7]})")
    return out


def compare_url(since: str) -> str:
    repo = os.environ.get("GITHUB_REPOSITORY", "ootrando/ootrr")
    head = git("rev-parse", "HEAD").strip()
    if is_ancestor(since):
        return f"https://github.com/{repo}/compare/{since[:12]}...{head[:12]}"
    return f"https://github.com/{repo}/commits/{head[:12]}"


def notes(version: str, since: str, url: str) -> str:
    head = git("rev-parse", "HEAD").strip()
    repo = os.environ.get("GITHUB_REPOSITORY", "ootrando/ootrr")
    c = changes(since)
    heading = "Changes since the last nightly" if is_ancestor(since) else f"Latest {len(c)} changes"
    return "\n".join([
        f"Nightly build {version} of OOTRR, built from [{head[:10]}](https://github.com/{repo}/commit/{head}) on main. "
        f"Its source is on the [nightly/{version}](https://github.com/{repo}/tree/nightly/{version}) branch.",
        "",
        "This release is replaced by the next nightly. The Source code downloads GitHub adds below only hold a placeholder, "
        "so get the source from the branch above.",
        "",
        "**Nightly builds are untested.** They can have bugs that break a seed or a save, so back up your saves first. "
        "Report bugs in the nightly-bug-reports forum on Discord, with the version above in your post. "
        "Seeds made with a nightly may not match seeds made with a release.",
        "",
        f"## {heading}",
        "",
        *(c or ["- No new commits."]),
        "",
        f"Full list: {compare_url(since)}",
        "",
        "## Downloads",
        "",
        "No game files, keys or dumps are included. You need your own extracted game files.",
        "",
        "Windows: run the setup exe, or unpack the portable zip anywhere and double-click ootrr.exe. "
        "macOS: open the pkg. Linux: use the deb, or unpack the tar.gz and run install.sh. "
        "Check SHA256SUMS.txt to verify a download.",
        "",
    ])


def discord(version: str, since: str, url: str) -> dict:
    c = changes(since)
    lines: list[str] = []
    used = 0
    for i, l in enumerate(c):
        if used + len(l) + 1 > EMBED_MAX:
            lines.append(f"...and {len(c) - i} more")
            break
        lines.append(l)
        used += len(l) + 1
    desc = "\n".join(lines) if lines else "No new commits."
    desc += f"\n\n[Full list]({compare_url(since)}) · [Downloads]({url})"
    return {
        "username": "OOTRR Nightly",
        "allowed_mentions": {"parse": []},
        "embeds": [{
            "title": f"OOTRR nightly {version}",
            "url": url,
            "description": desc,
            "color": 0x7289DA,
            "footer": {"text": "Nightly builds are untested. Back up your saves, and report bugs in nightly-bug-reports."},
        }],
    }


def main() -> None:
    if len(sys.argv) != 6 or sys.argv[1] not in ("notes", "discord"):
        sys.exit(__doc__)
    kind, version, since, url, out = sys.argv[1:]
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        if kind == "notes":
            f.write(notes(version, since, url))
        else:
            json.dump(discord(version, since, url), f, ensure_ascii=False)


if __name__ == "__main__":
    main()
