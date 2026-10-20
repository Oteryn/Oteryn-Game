#!/usr/bin/env python3
"""Make a restored Cargo cache reusable on a fresh checkout.

Cargo decides that a workspace or path-dependency unit is stale when a source
file is newer than the cached build output. A fresh checkout stamps every file
with the checkout time, so a cache restored from protected main rebuilt every
workspace crate even when nothing in it changed.

The cache carries the commit it was built from (target/.oteryn-cache-source-sha,
written by the protected-main save job). This script fetches that exact commit,
gives every tracked file an old mtime and leaves only the files that differ from
the cached commit with their fresh checkout mtime, so Cargo rebuilds exactly the
changed units. It fails open: with no marker, no fetch or any error it changes
nothing and the build is as slow as before, never wrong.
"""
from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

MARKER = Path("target/.oteryn-cache-source-sha")
OLD_MTIME = 946684800  # 2000-01-01T00:00:00Z, older than any restored artifact


def git(*args: str) -> bytes:
    return subprocess.check_output(["git", *args])


def normalize(root: Path, marker: Path = MARKER, old_mtime: int = OLD_MTIME) -> str:
    os.chdir(root)
    if not marker.is_file():
        return "no cache source marker; mtimes left as checked out"
    cached = marker.read_text(encoding="utf-8").strip().lower()
    if re.fullmatch(r"[0-9a-f]{40}", cached) is None:
        return "invalid cache source marker; mtimes left as checked out"
    try:
        subprocess.check_call(["git", "cat-file", "-e", f"{cached}^{{commit}}"], stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        subprocess.check_call(["git", "fetch", "--no-tags", "--depth=1", "origin", cached])
    changed = {p for p in git("diff", "--no-renames", "--name-only", "-z", cached, "HEAD", "--").decode("utf-8").split("\0") if p}
    tracked = [p for p in git("ls-files", "-z").decode("utf-8").split("\0") if p]
    reset = 0
    for path in tracked:
        if path in changed or os.path.islink(path) or not os.path.isfile(path):
            continue
        os.utime(path, (old_mtime, old_mtime))
        reset += 1
    return f"cache source {cached[:12]}: {reset} unchanged files set old, {len(changed)} changed files keep fresh mtime"


def main() -> int:
    try:
        print(normalize(Path.cwd()))
    except (OSError, UnicodeDecodeError, subprocess.SubprocessError) as error:
        print(f"mtime normalization skipped: {error}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
