#!/usr/bin/env python3
"""Git merge driver for hand-written sources that mechanics extend side by side (`merge=oteryn-additive`).

Git calls it as `oteryn_additive_driver.py %O %A %B %L %P` (base, ours, theirs, marker size, path).
Two mechanics in flight usually both append at the same place: a `mod` line, a struct field and its
`Default` value, a trait method, a dependency. Git reports that as a conflict although nothing was
changed, only added. This driver keeps every hunk `git merge-file` merges cleanly, and resolves a
conflicted hunk only when its base is empty, i.e. both sides inserted lines and neither edited or
deleted one: it keeps the incoming (`origin/main`) lines first, then ours, and one copy when both
inserted the same lines. Lines are never deduplicated one by one: when git slides both insertions
across a shared `}` line, each side's block is only valid whole. The same slide makes two entries
appended to one JSON array an insert-only hunk; a `.json` result must still parse, or the file keeps
its markers. The Game server manifest also combines a new feature next to a feature whose
array gained members. Other conflicts keep their markers and stop the merge for a person.
The driver prints how many hunks it combined; `cargo fmt`, the registry validators and CI check the
result.
GitHub's server-side merge ignores custom drivers; this is a local-merge aid only.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tomllib


GAME_SERVER_MANIFEST = "apps/game-server/Cargo.toml"


def merge_cargo_feature(
    base: list[str], edited: list[str], appended: list[str]
) -> list[str] | None:
    if len(base) != 1 or len(edited) != 1 or appended[:1] != base:
        return None
    try:
        old = tomllib.loads(base[0])
        new = tomllib.loads(edited[0])
        extra = tomllib.loads("".join(appended[1:]))
    except tomllib.TOMLDecodeError:
        return None
    if len(old) != 1 or old.keys() != new.keys() or not extra or old.keys() & extra.keys():
        return None
    if not all(
        isinstance(members, list) and all(isinstance(member, str) for member in members)
        for features in (old, new, extra)
        for members in features.values()
    ):
        return None
    feature = next(iter(old))
    old_members, new_members = old[feature], new[feature]
    if len(new_members) <= len(old_members) or [
        member for member in new_members if member in old_members
    ] != old_members:
        return None
    return edited + appended[1:]


def combine(merged: str, marker_size: int, path: str = "") -> tuple[str, int, int]:
    """Resolve supported additive hunks. Returns (text, combined, remaining)."""
    open_, base_, mid_, close_ = ("<" * marker_size, "|" * marker_size, "=" * marker_size, ">" * marker_size)
    out: list[str] = []
    combined = remaining = 0
    lines = merged.splitlines(keepends=True)
    i = 0
    while i < len(lines):
        if not lines[i].startswith(open_ + " ") and lines[i].rstrip("\r\n") != open_:
            out.append(lines[i])
            i += 1
            continue
        # A conflict hunk: ours, then (diff3) base, then theirs.
        sections: dict[str, list[str]] = {"ours": [], "base": [], "theirs": []}
        part, start, j = "ours", i, i + 1
        while j < len(lines):
            line = lines[j].rstrip("\r\n")
            if line.startswith(base_) and part == "ours":
                part = "base"
            elif line == mid_ and part in ("ours", "base"):
                part = "theirs"
            elif line.startswith(close_) and part == "theirs":
                break
            else:
                sections[part].append(lines[j])
            j += 1
        if j == len(lines):
            out.extend(lines[start:])  # Unterminated: leave it untouched for a person.
            remaining += 1
            break
        cargo_feature = None
        if sections["base"] and path == GAME_SERVER_MANIFEST:
            table = next((line.strip() for line in reversed(out) if line.startswith("[")), "")
            if table == "[features]":
                cargo_feature = merge_cargo_feature(
                    sections["base"], sections["ours"], sections["theirs"]
                ) or merge_cargo_feature(
                    sections["base"], sections["theirs"], sections["ours"]
                )
        if cargo_feature is not None:
            out.extend(cargo_feature)
            combined += 1
        elif sections["base"]:
            out.extend(lines[start : j + 1])
            remaining += 1
        else:
            out.extend(sections["theirs"])
            if sections["ours"] != sections["theirs"]:
                out.extend(sections["ours"])
            combined += 1
        i = j + 1
    return "".join(out), combined, remaining


def main(argv: list[str]) -> int:
    if len(argv) != 6:
        print("usage: oteryn_additive_driver.py %O %A %B %L %P", file=sys.stderr)
        return 2
    base, ours, theirs, marker, path = argv[1:]
    marker_size = int(marker) if marker.isdigit() else 7
    result = subprocess.run(
        ["git", "merge-file", "-p", "--diff3", f"--marker-size={marker_size}"]
        + ["-L", "ours", "-L", "base", "-L", "incoming", ours, base, theirs],
        capture_output=True,
    )
    if result.returncode < 0 or result.returncode > 127:
        return 2  # git merge-file itself failed: leave the conflict to a person
    try:
        merged = result.stdout.decode("utf-8")
    except UnicodeDecodeError:
        with open(ours, "wb") as handle:
            handle.write(result.stdout)
        return 1
    text, combined, remaining = combine(merged, marker_size, path)
    if combined and not remaining:
        try:
            if path.endswith(".json"):
                json.loads(text)
            elif path == GAME_SERVER_MANIFEST:
                tomllib.loads(text)
        except ValueError:
            text, combined, remaining = merged, 0, 1
    with open(ours, "w", encoding="utf-8", newline="") as handle:
        handle.write(text)
    if combined:
        print(f"oteryn-additive: {path}: combined {combined} additive hunk(s)", file=sys.stderr)
    return 1 if remaining else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
