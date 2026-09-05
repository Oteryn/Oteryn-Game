#!/usr/bin/env python3
"""Exercise post-merge selection against real immutable Git ranges."""
from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import classify_pr_test_lanes as lanes
from test_classify_pr_test_lanes import fixture

ROOT = Path(__file__).resolve().parents[2]


def main():
    assert hasattr(lanes, "classify_post_merge"), "protected-main push adapter is missing"
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        def git(*args):
            return subprocess.check_output(["git", "-C", directory, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", *args], stderr=subprocess.PIPE).decode().strip()
        git("init", "-q")
        metadata = json.loads(json.dumps(fixture()).replace("/repo", directory))
        for package in metadata["packages"]:
            manifest = Path(package["manifest_path"])
            manifest.parent.mkdir(parents=True)
            manifest.write_text("[package]\n")
        source = root / "apps/game-server/src/main.rs"
        source.parent.mkdir()
        source.write_text("// original\n")
        git("add", ".")
        git("commit", "-qm", "base")
        before = git("rev-parse", "HEAD")
        source.write_text("// server repair\n")
        git("add", ".")
        git("commit", "-qm", "server")
        after = git("rev-parse", "HEAD")
        event = dict(before=before, after=after, ref="refs/heads/main", forced=False, deleted=False, created=False, repository={"full_name": "Oteryn/Oteryn-Game"})
        env = dict(GITHUB_EVENT_NAME="push", GITHUB_REF="refs/heads/main", GITHUB_REF_PROTECTED="true", GITHUB_SHA=after, GITHUB_REPOSITORY="Oteryn/Oteryn-Game")
        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            with patch.dict(os.environ, env), patch.object(lanes, "AUDITED_INPUT_SHA256", lanes.input_digest(metadata)):
                def classify(change=None, environment=None, meta=None):
                    with patch.dict(os.environ, environment or {}):
                        return lanes.classify_post_merge(dict(event, **(change or {})), metadata if meta is None else meta)
                result = classify()
                assert result["rust"] is True and result["windows"] is False, result
                for key, values in {"GITHUB_EVENT_NAME": ["workflow_dispatch", "merge_group", "pull_request"], "GITHUB_REF": ["refs/heads/feature", ""], "GITHUB_REF_PROTECTED": ["false", "", "TRUE"], "GITHUB_SHA": [before, "", "x" * 40], "GITHUB_REPOSITORY": ["other/repo", ""]}.items():
                    for value in values:
                        assert classify(environment={key: value})["windows"] is True, (key, value)
                for key, values in {"before": ["0" * 40, "missing", after, "f" * 40], "after": [before, "0" * 40], "ref": ["refs/heads/feature"], "forced": [True, "false", None], "created": [True, None], "deleted": [True, None], "repository": [{}, {"full_name": "other/repo"}]}.items():
                    for value in values:
                        assert classify({key: value})["windows"] is True, (key, value)
                for malformed in (None, [], {}, "invalid"):
                    assert lanes.classify_post_merge(malformed, metadata)["windows"] is True
                assert classify(meta={})["windows"] is True
                with patch.object(lanes, "AUDITED_INPUT_SHA256", "stale"):
                    assert classify()["windows"] is True
                # Independent Git history, not event commit arrays, determines every path.
                for path in ("apps/client/src/lib.rs", "crates/simulation-determinism/src/lib.rs", "crates/foundation/src/lib.rs", "Cargo.lock", "apps/game-server/Cargo.toml", ".github/workflows/other.yml", "unknown.bin", "tools/repository/other.py"):
                    git("checkout", "-q", after)
                    target = root / path
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_text("changed\n")
                    git("add", ".")
                    git("commit", "-qm", "mixed input")
                    head = git("rev-parse", "HEAD")
                    with patch.dict(os.environ, GITHUB_SHA=head):
                        assert classify({"after": head, "commits": []})["windows"] is True, path
                git("checkout", "-q", after)
                git("mv", "apps/game-server/src/main.rs", "apps/client/moved.rs")
                git("commit", "-qm", "rename across consumers")
                head = git("rev-parse", "HEAD")
                with patch.dict(os.environ, GITHUB_SHA=head):
                    assert classify({"after": head})["windows"] is True
                git("checkout", "-q", after)
                (root / "link").symlink_to("apps/game-server/src/main.rs")
                git("add", ".")
                git("commit", "-qm", "special mode")
                head = git("rev-parse", "HEAD")
                with patch.dict(os.environ, GITHUB_SHA=head):
                    assert classify({"after": head})["windows"] is True
        finally:
            os.chdir(old_cwd)
    workflow = (ROOT / ".github/workflows/rust.yml").read_text()
    assert "  lanes:\n" in workflow, "post-merge lane job is missing"
    assert "  sim-windows-golden:\n" not in workflow, "unreachable SIM job remains"
    assert "    paths:" not in workflow, "unknown main inputs must trigger FULL"
    assert "    needs: lanes\n" in workflow
    assert "needs.lanes.result != 'success' || needs.lanes.outputs.windows != 'false'" in workflow
    assert "cancel-in-progress: false" in workflow, "a later push must not cancel an earlier required post-merge run"
    assert "cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc" in workflow
    print("Post-merge fixtures PASS: real Git ranges, event/protection identity, full fallback, consumer/rename/mode families and workflow wiring")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
