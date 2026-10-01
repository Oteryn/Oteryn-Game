#!/usr/bin/env python3
"""Fixture tests for regenerate_content.py --resolve: which merge conflicts it may resolve.

Builds a tiny git repository in a temp dir with a real stopped merge; no cargo, no network.
Run with `python3 tools/content-migration/test_regenerate_content.py`.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import regenerate_content as rc

SOURCES = "imports/tibiawiki/sources.json"
DERIVED = "content/items/index.json"
INTERACTIONS = "content/interactions/index.json"
PIN_TEST = f'const DOCUMENTS: [(&str, usize, &str); 1] = [\n    ("definitions/reference.json", 1, "{"0" * 64}"),\n];\n'


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def write(root: Path, path: str, value: object) -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    text = value if isinstance(value, str) else json.dumps(value, indent=2) + "\n"
    target.write_text(text, encoding="utf-8")


def sources(*batches: str) -> dict:
    return {
        "sources": [
            {"key": "oteryn:source.tibiawiki", "import_batch_id": batch}
            for batch in batches
        ]
    }


def stopped_merge(root: Path, conflicting: list[str]) -> None:
    """Base, then `ours` and `theirs` each change every path in `conflicting`, then merge."""
    git(root, "init", "-q", "-b", "main")
    manifest = {"managed_files": [{"path": SOURCES}, {"path": DERIVED}]}
    write(root, "content/manifest.json", manifest)
    write(root, rc.PIN_PATH, PIN_TEST)
    write(root, SOURCES, sources("base"))
    write(root, DERIVED, {"items": ["base"]})
    write(root, INTERACTIONS, {"contract": "hand-written", "notes": "base"})
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "theirs")
    for side in ("theirs", "ours"):
        if side == "ours":
            git(root, "checkout", "-q", "main")
        for path in conflicting:
            if path == SOURCES:
                write(root, path, sources("base", f"{side}-batch"))
            elif path == INTERACTIONS:
                write(root, path, {"contract": f"{side}-contract", "notes": side})
            else:
                write(root, path, {"items": ["base", side]})
        git(root, "commit", "-q", "-am", side)
    merged = subprocess.run(
        ["git", "merge", "-q", "theirs"], cwd=root, capture_output=True
    )
    assert merged.returncode != 0, "fixture merge must stop on conflicts"


def resolve_in(root: Path) -> tuple[int, list[str]]:
    rc.ROOT, rc.PIN_TEST, rc.WORLD = root, root / rc.PIN_PATH, root / "content/world"
    try:
        return 0, rc.resolve_conflicts()
    except SystemExit as stop:
        return int(stop.code), []


def unmerged(root: Path) -> list[str]:
    return git(root, "diff", "--name-only", "--diff-filter=U").split()


def test_import_conflict_fails_closed_and_keeps_both_sides() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [SOURCES, DERIVED])
        code, _ = resolve_in(root)
        assert code == 2, code
        # Nothing was written: both paths stay unmerged, and each side's captured row survives.
        assert unmerged(root) == sorted([SOURCES, DERIVED]), unmerged(root)
        ours = json.loads(git(root, "show", f":2:{SOURCES}"))
        theirs = json.loads(git(root, "show", f":3:{SOURCES}"))
        assert [row["import_batch_id"] for row in ours["sources"]] == [
            "base",
            "ours-batch",
        ]
        assert [row["import_batch_id"] for row in theirs["sources"]] == [
            "base",
            "theirs-batch",
        ]
        text = (root / SOURCES).read_text(encoding="utf-8")
        assert "ours-batch" in text and "theirs-batch" in text


def test_interactions_index_conflict_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [INTERACTIONS])
        code, _ = resolve_in(root)
        assert code == 2, code
        assert unmerged(root) == [INTERACTIONS], unmerged(root)


def test_derived_only_conflict_is_resolved() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [DERIVED])
        code, resolved = resolve_in(root)
        assert code == 0 and resolved == [DERIVED], (code, resolved)
        assert json.loads((root / DERIVED).read_text(encoding="utf-8")) == {
            "items": ["base", "theirs"]
        }


def test_git_failure_while_enumerating_conflicts_fails_closed() -> None:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        stopped_merge(root, [SOURCES, DERIVED])
        before = {
            path: (root / path).read_bytes() for path in (SOURCES, DERIVED, rc.PIN_PATH)
        }
        corrupt = root / "corrupt-index"
        corrupt.write_bytes(b"not an index")
        commands: list[list[str]] = []
        saved = rc.run, sys.argv, os.environ.get("GIT_INDEX_FILE")
        rc.run, sys.argv = commands.append, ["regenerate_content.py", "--resolve"]
        os.environ["GIT_INDEX_FILE"] = str(corrupt)
        rc.ROOT, rc.PIN_TEST, rc.WORLD = (
            root,
            root / rc.PIN_PATH,
            root / "content/world",
        )
        try:
            rc.main()
            raise AssertionError(
                "--resolve must stop when git cannot list the conflicts"
            )
        except SystemExit as stop:
            assert stop.code not in (0, None), stop.code
        finally:
            rc.run, sys.argv = saved[0], saved[1]
            if saved[2] is None:
                os.environ.pop("GIT_INDEX_FILE", None)
            else:
                os.environ["GIT_INDEX_FILE"] = saved[2]
        # Nothing ran and nothing was written: the merge is exactly as git left it.
        assert commands == [], commands
        assert {path: (root / path).read_bytes() for path in before} == before
        assert unmerged(root) == sorted([SOURCES, DERIVED]), unmerged(root)


def main() -> None:
    tests = [
        test_import_conflict_fails_closed_and_keeps_both_sides,
        test_interactions_index_conflict_fails_closed,
        test_derived_only_conflict_is_resolved,
        test_git_failure_while_enumerating_conflicts_fails_closed,
    ]
    for test in tests:
        test()
    print(f"PASS {len(tests)} checks")


if __name__ == "__main__":
    main()
