"""Print the reliability checks a diff calls for, before it is pushed.

The items in reliability_checklist.json are failures that reached a PR gate or
main more than once. This tool reads the diff (changed paths and changed lines)
and lists the items it matches, with the check that would have caught each one.
It is advisory: it always exits 0 unless its own input is broken.

    python3 tools/repository/reliability_checklist.py                 # origin/main...HEAD
    python3 tools/repository/reliability_checklist.py --base main --head HEAD
    python3 tools/repository/reliability_checklist.py --paths a.rs b.sql
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHECKLIST = Path(__file__).with_name("reliability_checklist.json")
REQUIRED = ("id", "title", "paths", "diff", "check", "source", "examples")


def load(path: Path = CHECKLIST) -> list[dict]:
    data = json.loads(path.read_text(encoding="utf-8"))
    items = data["items"]
    for item in items:
        missing = [key for key in REQUIRED if key not in item]
        if missing:
            raise ValueError(f"{item.get('id', '?')}: missing {missing}")
        if not item["paths"] and not item["diff"]:
            raise ValueError(f"{item['id']}: no trigger")
    return items


def path_matches(pattern: str, path: str) -> bool:
    if pattern.endswith("/**"):
        return path.startswith(pattern[:-2])
    return fnmatch.fnmatchcase(path, pattern)


def changed_lines(diff: str) -> list[str]:
    return [
        line
        for line in diff.splitlines()
        if line[:1] in "+-" and not line.startswith(("+++ ", "--- "))
    ]


def match(
    items: list[dict], paths: list[str], lines: list[str]
) -> list[tuple[dict, str]]:
    hits = []
    for item in items:
        reason = next(
            (f"path {p}" for p in paths for g in item["paths"] if path_matches(g, p)),
            None,
        )
        if reason is None:
            for pattern in item["diff"]:
                regex = re.compile(pattern)
                line = next((l for l in lines if regex.search(l)), None)
                if line is not None:
                    reason = f"line {line.strip()[:100]}"
                    break
        if reason is not None:
            hits.append((item, reason))
    return hits


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def render(hits: list[tuple[dict, str]]) -> str:
    if not hits:
        return "Reliability checklist: no item matches this diff.\n"
    out = [f"Reliability checklist: {len(hits)} item(s) match this diff.", ""]
    for item, reason in hits:
        out.append(f"- [{item['id']}] {item['title']}")
        out.append(f"  matched: {reason}")
        out.append(f"  check:   {item['check']}")
        out.append(f"  source:  {item['source']}")
    return "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument(
        "--paths", nargs="*", help="check these paths instead of a git diff"
    )
    args = parser.parse_args(argv)
    items = load()
    if args.paths is not None:
        paths, lines = args.paths, []
    else:
        span = f"{args.base}...{args.head}"
        paths = git("diff", "--name-only", span).split()
        lines = changed_lines(git("diff", "--unified=0", span))
    sys.stdout.write(render(match(items, paths, lines)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
