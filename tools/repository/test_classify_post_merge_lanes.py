#!/usr/bin/env python3
"""Exercise post-merge selection against real immutable Git ranges."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import classify_pr_test_lanes as lanes
import validate_repository_policy_core as policy
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
                        result = lanes.classify_post_merge(dict(event, **(change or {})), metadata if meta is None else meta)
                        assert type(result["rust"]) is bool and type(result["windows"]) is bool, result
                        if result["windows"]:
                            assert result["rust"] is True, result
                        return result
                result = classify()
                assert result["rust"] is True and result["windows"] is False, result
                for key, values in {"GITHUB_EVENT_NAME": ["workflow_dispatch", "merge_group", "pull_request"], "GITHUB_REF": ["refs/heads/feature", ""], "GITHUB_REF_PROTECTED": ["false", "", "TRUE"], "GITHUB_SHA": [before, "", "x" * 40], "GITHUB_REPOSITORY": ["other/repo", ""]}.items():
                    for value in values:
                        assert classify(environment={key: value})["windows"] is True, (key, value)
                for key, values in {"before": ["0" * 40, "missing", after, "f" * 40], "after": [before, "0" * 40], "ref": ["refs/heads/feature"], "forced": [True, "false", None], "created": [True, None], "deleted": [True, None], "repository": [{}, {"full_name": "other/repo"}]}.items():
                    for value in values:
                        assert classify({key: value})["windows"] is True, (key, value)
                for malformed in (None, [], {}, "invalid"):
                    result = lanes.classify_post_merge(malformed, metadata)
                    assert result["rust"] is True and result["windows"] is True, result
                assert classify(meta={})["windows"] is True
                with patch.object(lanes, "AUDITED_INPUT_SHA256", "stale"):
                    assert classify()["windows"] is True
                real_output = lanes.subprocess.check_output
                for broken in (b"M\0apps/game-server/src/main.rs", b"M\0", b"T\0apps/game-server/src/main.rs\0", b"M\0bad\xff\0"):
                    def incomplete(args, **kwargs):
                        return broken if args[:2] == ["git", "diff"] else real_output(args, **kwargs)
                    with patch.object(lanes.subprocess, "check_output", side_effect=incomplete):
                        assert classify()["windows"] is True, broken
                # A disconnected before commit is not protected ancestry proof.
                other_tree = git("rev-parse", "HEAD^{tree}")
                unrelated = git("commit-tree", other_tree, "-m", "disconnected")
                assert classify({"before": unrelated})["windows"] is True
                # Full local enumeration has no API's 300-file truncation boundary.
                for index in range(301):
                    (source.parent / f"fixture_{index}.rs").write_text("// server fixture\n")
                git("add", ".")
                git("commit", "-qm", "large server range")
                large_head = git("rev-parse", "HEAD")
                with patch.dict(os.environ, GITHUB_SHA=large_head):
                    assert classify({"after": large_head, "commits": []})["windows"] is False
                git("checkout", "-q", after)
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
                doc = root / "README.md"
                doc.write_text("original documentation\n")
                git("add", ".")
                git("commit", "-qm", "document baseline")
                doc_before = git("rev-parse", "HEAD")
                with patch.object(lanes, "AUDITED_DOC_INPUT_SHA256", lanes.input_digest(metadata, include_server=True)):
                    for rename in (False, True):
                        git("checkout", "-q", doc_before)
                        if rename:
                            git("mv", "README.md", "CHANGELOG.md")
                        else:
                            doc.write_text("revised documentation\n")
                        git("add", ".")
                        git("commit", "-qm", "neutral documentation")
                        doc_head = git("rev-parse", "HEAD")
                        with patch.dict(os.environ, GITHUB_SHA=doc_head):
                            result = classify({"before": doc_before, "after": doc_head})
                            assert result["rust"] is False and result["windows"] is False, result
                            with patch.object(lanes, "AUDITED_DOC_INPUT_SHA256", "stale"):
                                assert classify({"before": doc_before, "after": doc_head})["windows"] is True
                            with patch.object(lanes, "AUDITED_INPUT_SHA256", "stale"):
                                assert classify({"before": doc_before, "after": doc_head})["windows"] is True
                    # A changed/new consumer can acquire a document dependency.
                    # Both server and non-server trees must invalidate docs proof.
                    for path in ("apps/game-server/src/main.rs", "apps/game-server/src/doc_reader.rs",
                                 "apps/client/src/doc_reader.rs", "Cargo.lock", ".github/workflows/other.yml", "unknown.bin"):
                        git("checkout", "-q", doc_before)
                        doc.write_text("changed docs\n")
                        target = root / path
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_text("changed input\n")
                        git("add", ".")
                        git("commit", "-qm", "mixed documentation input")
                        mixed = git("rev-parse", "HEAD")
                        with patch.dict(os.environ, GITHUB_SHA=mixed):
                            result = classify({"before": doc_before, "after": mixed})
                            if path.startswith("apps/game-server/"):
                                assert result["rust"] is True and result["windows"] is False, result
                            else:
                                assert result["rust"] is True and result["windows"] is True, result
                        # A subsequent docs-only range uses the changed protected
                        # consumers; it must not inherit the previous docs proof.
                        doc.write_text("another document change\n")
                        git("add", ".")
                        git("commit", "-qm", "docs after consumer change")
                        head = git("rev-parse", "HEAD")
                        with patch.dict(os.environ, GITHUB_SHA=head):
                            if path.startswith("apps/") or path == "Cargo.lock":
                                assert classify({"before": mixed, "after": head})["windows"] is True
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
    assert "concurrency:" not in workflow, "branch concurrency can replace a pending protected-main run"
    assert "cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc" in workflow
    # Execute the actual shell fallback with a failed toolchain dependency.
    block = policy.indented_yaml_mapping_block(workflow, "lanes", 2)
    script = textwrap.dedent(block.split("        run: |\n", 1)[1])
    with tempfile.TemporaryDirectory() as directory:
        output = Path(directory) / "outputs"
        rustup = Path(directory) / "rustup"
        rustup.write_text("#!/bin/sh\nexit 1\n")
        rustup.chmod(0o755)
        result = subprocess.run(["bash", "-c", script], env=dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"], RUNNER_TEMP=directory, GITHUB_OUTPUT=str(output)), capture_output=True, text=True)
        assert result.returncode == 0 and output.read_text() == "rust=true\nwindows=true\n", result
        rustup.write_text("#!/bin/sh\nexit 0\n")
        cargo = Path(directory) / "cargo"
        cargo.write_text("#!/bin/sh\nprintf '{}\\n'\nexit \"$METADATA_EXIT\"\n")
        cargo.chmod(0o755)
        python = Path(directory) / "python"
        python.write_text("#!/bin/sh\nprintf '%s' \"$WIRE_OUTPUT\" >> \"$GITHUB_OUTPUT\"\nexit \"$CLASSIFIER_EXIT\"\n")
        python.chmod(0o755)
        full = "rust=true\nwindows=true\n"
        valid = ("rust=false\nwindows=false\n", "rust=true\nwindows=false\n", full)
        invalid = ("", "rust=false\n", "windows=false\n", "rust=false\nwindows=true\n",
                   "rust=false\nwindows=unknown\n", "rust=FALSE\nwindows=false\n",
                   "rust=false\nwindows=false", "rust=false\nwindows=false\nwindows=false\n",
                   "rust=false\nwindows=false\nother=false\n", "rust=false\nwindows=false\n\n")
        for payload in valid + invalid:
            for metadata_exit, classifier_exit in ((0, 0), (1, 0), (0, 1)):
                output.unlink()
                result = subprocess.run(["bash", "-c", script], env=dict(os.environ,
                    PATH=directory + os.pathsep + os.environ["PATH"], RUNNER_TEMP=directory, GITHUB_OUTPUT=str(output),
                    WIRE_OUTPUT=payload, METADATA_EXIT=str(metadata_exit), CLASSIFIER_EXIT=str(classifier_exit)), capture_output=True, text=True)
                expected = payload if payload in valid and metadata_exit == classifier_exit == 0 else full
                assert result.returncode == 0 and output.read_text() == expected, (payload, metadata_exit, classifier_exit, result)
    assert hasattr(policy, "validate_post_merge_rust"), "post-merge workflow regression contract is missing"
    assert not policy.validate_post_merge_rust(workflow)
    assert policy.validate_post_merge_rust(workflow + "\nconcurrency:\n  group: main\n  cancel-in-progress: false\n")
    for before_text, after_text in (("always()", "success()"), ("windows != 'false'", "windows == 'true'"), ("fetch-depth: 0", "fetch-depth: 1"), ("    needs: lanes", "    needs: policy"), ("--test durability_postgres", "--test nonexistent"), ("--post-merge", "--pr"), ("  workflow_dispatch:", "  pull_request:")):
        mutated = workflow.replace(before_text, after_text)
        assert mutated != workflow and policy.validate_post_merge_rust(mutated), before_text
    for job in ("linux", "durability-postgres", "windows"):
        block = policy.indented_yaml_mapping_block(workflow, job, 2)
        for before_text, after_text in (("needs.lanes.result != 'success'", "false"),
                                        ("github.event_name != 'push'", "false"),
                                        ("    needs: lanes\n", "")):
            mutated = block.replace(before_text, after_text)
            assert mutated != block and policy.validate_post_merge_rust(workflow.replace(block, mutated)), (job, before_text)
    print("Post-merge fixtures PASS: real Git ranges, event/protection identity, full fallback, consumer/rename/mode families and workflow wiring")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
