#!/usr/bin/env python3
"""Protected-main regressions for exact-candidate impact routing."""
from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
from unittest.mock import patch

MODULE = Path(__file__).with_name("classify_pr_test_lanes.py")
SPEC = importlib.util.spec_from_file_location("post_merge_classifier", MODULE)
assert SPEC is not None and SPEC.loader is not None
lanes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lanes)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def metadata(root: Path):
    roots = {
        "oteryn-game-server": "apps/game-server",
        "oteryn-client": "apps/client",
        "oteryn-synthetic-client-harness": "tools/synthetic-client-harness",
        "oteryn-simulation-determinism": "crates/simulation-determinism",
        "oteryn-foundation": "crates/foundation",
    }
    edges = {
        "oteryn-game-server": ["oteryn-foundation", "oteryn-simulation-determinism"],
        "oteryn-client": ["oteryn-foundation"],
        "oteryn-synthetic-client-harness": ["oteryn-foundation"],
    }
    return {
        "workspace_root": str(root),
        "workspace_members": list(roots),
        "packages": [
            {
                "id": name,
                "name": name,
                "manifest_path": str(root / path / "Cargo.toml"),
                "dependencies": [
                    {
                        "name": dep,
                        "path": str(root / roots[dep]),
                        "kind": None,
                        "target": None,
                        "optional": False,
                    }
                    for dep in edges.get(name, [])
                ],
            }
            for name, path in roots.items()
        ],
    }


def event(before, after):
    return {
        "before": before,
        "after": after,
        "ref": "refs/heads/main",
        "forced": False,
        "created": False,
        "deleted": False,
        "repository": {"full_name": "Oteryn/Oteryn-Game"},
    }


def classify(root, meta, before, after, extra_env=None):
    env = {
        "GITHUB_EVENT_NAME": "push",
        "GITHUB_REF": "refs/heads/main",
        "GITHUB_REF_PROTECTED": "true",
        "GITHUB_SHA": after,
        "GITHUB_REPOSITORY": "Oteryn/Oteryn-Game",
    }
    if extra_env:
        env.update(extra_env)
    old_cwd = os.getcwd()
    os.chdir(root)
    try:
        with patch.dict(os.environ, env, clear=False):
            return lanes.classify_post_merge(event(before, after), meta)
    finally:
        os.chdir(old_cwd)


def commit_change(root: Path, path: str, content: str, message: str):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    git(root, "add", ".")
    git(root, "commit", "-qm", message)
    return git(root, "rev-parse", "HEAD")


def main() -> int:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "ci@example.invalid")
        git(root, "config", "user.name", "CI")
        meta = metadata(root)

        for package in meta["packages"]:
            manifest = Path(package["manifest_path"])
            manifest.parent.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\n", encoding="utf-8")

        server = root / "apps/game-server/src/lib.rs"
        server.parent.mkdir(parents=True, exist_ok=True)
        server.write_text(
            'const EVIDENCE: &[u8] = include_bytes!("../../../docs/runtime/server.json");\n',
            encoding="utf-8",
        )
        client = root / "apps/client/src/lib.rs"
        client.parent.mkdir(parents=True, exist_ok=True)
        client.write_text(
            'const THEME: &str = include_str!("../../../docs/runtime/client.json");\n',
            encoding="utf-8",
        )
        for path in ("docs/runtime/server.json", "docs/runtime/client.json"):
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("{}\n", encoding="utf-8")

        control = root / ".github/workflows"
        control.mkdir(parents=True, exist_ok=True)
        (control / "merge-gate.yml").write_text("run: python tools/control/helper.py\n", encoding="utf-8")
        (control / "merge-group-gate.yml").write_text("name: merge-group\n", encoding="utf-8")
        (control / "rust.yml").write_text("name: rust\n", encoding="utf-8")
        helper = root / "tools/control/helper.py"
        helper.parent.mkdir(parents=True, exist_ok=True)
        helper.write_text("print('helper')\n", encoding="utf-8")

        git(root, "add", ".")
        git(root, "commit", "-qm", "base")
        base = git(root, "rev-parse", "HEAD")

        # Pure auxiliary changes use no product runtime lanes.
        aux_cases = (
            ("docs/architecture/note.md", "note\n"),
            ("AGENTS.md", "policy\n"),
            (".github/workflows/offline-census.yml", "name: offline\n"),
            ("tools/reference-world-corridor-census/offline.py", "print('offline')\n"),
        )
        for index, (path, value) in enumerate(aux_cases):
            git(root, "checkout", "-q", base)
            head = commit_change(root, path, value, f"aux-{index}")
            result = classify(root, meta, base, head)
            assert result["rust"] is False and result["windows"] is False, (path, result)
            assert result["reason"] == "unconsumed-auxiliary-inputs", (path, result)

        # Real auxiliary inputs inherit the consuming package's lane.
        git(root, "checkout", "-q", base)
        head = commit_change(root, "docs/runtime/server.json", '{"v":1}\n', "server evidence")
        result = classify(root, meta, base, head)
        assert result["rust"] is True and result["windows"] is False, result
        assert result["reason"] == "server-only-exact-consumer-closure", result

        git(root, "checkout", "-q", base)
        head = commit_change(root, "docs/runtime/client.json", '{"v":1}\n', "client evidence")
        result = classify(root, meta, base, head)
        assert result["rust"] is True and result["windows"] is True, result

        # Canonical workflow consumers remain FULL even when the helper is non-Cargo.
        git(root, "checkout", "-q", base)
        head = commit_change(root, "tools/control/helper.py", "print('changed')\n", "control helper")
        result = classify(root, meta, base, head)
        assert result["rust"] is True and result["windows"] is True, result
        assert result["reason"] == "canonical-control-consumer-affected", result

        # Direct package changes still route through Cargo reverse closure.
        git(root, "checkout", "-q", base)
        head = commit_change(root, "apps/game-server/src/lib.rs",
                             server.read_text(encoding="utf-8") + "// server\n", "server source")
        result = classify(root, meta, base, head)
        assert result["rust"] is True and result["windows"] is False, result

        git(root, "checkout", "-q", base)
        head = commit_change(root, "apps/client/src/lib.rs",
                             client.read_text(encoding="utf-8") + "// client\n", "client source")
        result = classify(root, meta, base, head)
        assert result["rust"] is True and result["windows"] is True, result

        # Canonical routing controls and Cargo/build inputs remain FULL.
        for index, (path, value) in enumerate((
            (".github/workflows/rust.yml", "name: rust-changed\n"),
            ("tools/repository/probe.py", "print('control')\n"),
            ("Cargo.lock", "changed\n"),
        )):
            git(root, "checkout", "-q", base)
            head = commit_change(root, path, value, f"control-{index}")
            result = classify(root, meta, base, head)
            assert result["rust"] is True and result["windows"] is True, (path, result)

        # Invalid event/protection identity always fails closed.
        git(root, "checkout", "-q", base)
        head = commit_change(root, "docs/note.md", "note\n", "invalid-event-probe")
        result = classify(root, meta, base, head, {"GITHUB_REF_PROTECTED": "false"})
        assert result["rust"] is True and result["windows"] is True, result
        assert result["reason"] == "not-a-normal-protected-main-push", result

        # Special candidate modes remain conservative FULL.
        if hasattr(os, "symlink"):
            git(root, "checkout", "-q", base)
            os.symlink("docs/runtime/server.json", root / "link.json")
            git(root, "add", ".")
            git(root, "commit", "-qm", "special mode")
            special = git(root, "rev-parse", "HEAD")
            result = classify(root, meta, base, special)
            assert result["rust"] is True and result["windows"] is True, result
            assert result["reason"] == "unverified-or-special-candidate-modes", result

    print("Post-merge exact-candidate routing regressions PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
