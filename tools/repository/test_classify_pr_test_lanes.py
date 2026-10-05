#!/usr/bin/env python3
"""Behavioral regressions for exact-candidate impact routing."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import textwrap
from pathlib import Path, PurePosixPath
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
MODULE = Path(__file__).with_name("classify_pr_test_lanes.py")


def load_module():
    spec = importlib.util.spec_from_file_location("risk_classifier", MODULE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def fixture(workspace_root="/repo"):
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
        "workspace_root": workspace_root,
        "workspace_members": list(roots),
        "packages": [
            {
                "id": name,
                "name": name,
                "manifest_path": f"{workspace_root}/{path}/Cargo.toml",
                "dependencies": [
                    {
                        "name": dep,
                        "path": f"{workspace_root}/{roots[dep]}",
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


def classify(module, paths, *, consumers=None):
    records = []
    normalized = []
    for item in paths:
        if isinstance(item, dict):
            records.append(item)
            normalized.append(item["filename"])
            if item.get("previous_filename"):
                normalized.append(item["previous_filename"])
        else:
            records.append({"filename": item, "status": "modified"})
            normalized.append(item)
    if consumers is None:
        consumers = {path: set() for path in normalized}
    return module.classify(
        records,
        len(records),
        fixture(),
        candidate_modes_verified=True,
        reference_consumers=consumers,
    )


def test_routing_matrix(module):
    server = "apps/game-server/src/lib.rs"
    client = "apps/client/src/lib.rs"
    foundation = "crates/foundation/src/lib.rs"

    result = classify(module, [server])
    assert result == {
        "rust": True,
        "windows": False,
        "surface": "server",
        "reason": "server-only-exact-consumer-closure",
    }, result

    result = classify(module, [client])
    assert result["rust"] is True and result["windows"] is True, result
    assert result["surface"] == "client", result

    result = classify(module, [foundation])
    assert result["rust"] is True and result["windows"] is True, result

    for path in (
        "Cargo.lock",
        "apps/game-server/Cargo.toml",
        ".cargo/config.toml",
        ".github/workflows/merge-gate.yml",
        ".github/workflows/merge-group-gate.yml",
        ".github/workflows/rust.yml",
        ".github/actions/custom/action.yml",
        "tools/repository/classify_pr_test_lanes.py",
        "docs/migration/input.json",
    ):
        result = classify(module, [path])
        assert result["rust"] is True and result["windows"] is True, (path, result)

    auxiliary = (
        "README.md",
        "AGENTS.md",
        "docs/architecture/example.md",
        "docs/agents/PROJECT_LANES.json",
        "docs/agents/tasks/active/task.md",
        "docs/agents/evidence/unconsumed.json",
        "tools/agents/probe.py",
        "tools/reference-world-corridor-census/offline.py",
        ".github/workflows/content-census.yml",
    )
    for path in auxiliary:
        result = classify(module, [path])
        assert result["rust"] is False and result["windows"] is False, (path, result)
        assert result["reason"] == "unconsumed-auxiliary-inputs", (path, result)

    incident = [
        ".github/workflows/item-wiki-first-census.yml",
        "docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json",
        "docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md",
        "tools/reference-world-corridor-census/item_wiki_first_census.py",
        "tools/reference-world-corridor-census/item_wiki_first_census_self_test.py",
    ]
    result = classify(module, incident)
    assert result["rust"] is False and result["windows"] is False, result
    assert result["reason"] == "unconsumed-auxiliary-inputs", result

    evidence = "docs/agents/evidence/runtime.json"
    result = classify(module, [evidence], consumers={evidence: {"oteryn-game-server"}})
    assert result["rust"] is True and result["windows"] is False, result
    assert result["reason"] == "server-only-exact-consumer-closure", result

    dynamic = "docs/runtime/generated/item.json"
    result = classify(module, [dynamic], consumers={dynamic: {"oteryn-game-server"}})
    assert result["rust"] is True and result["windows"] is False, result

    helper = "tools/content/helper.py"
    result = classify(module, [helper], consumers={helper: {module.CONTROL_CONSUMER}})
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "canonical-control-consumer-affected", result

    governance = "AGENTS.md"
    result = classify(module, [governance], consumers={governance: {"oteryn-client"}})
    assert result["rust"] is True and result["windows"] is True, result

    mixed_consumers = {server: set(), evidence: set()}
    result = classify(module, [server, evidence], consumers=mixed_consumers)
    assert result["rust"] is True and result["windows"] is False, result

    unknown = "unowned/runtime-input.bin"
    result = classify(module, [unknown], consumers={unknown: set()})
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "unmodelled-input", result

    result = classify(module, [unknown], consumers={unknown: {"oteryn-game-server"}})
    assert result["rust"] is True and result["windows"] is False, result

    cross = [{
        "filename": "docs/agents/tasks/archive/task.md",
        "status": "renamed",
        "previous_filename": server,
    }]
    result = classify(module, cross)
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "cross-surface-rename", result

    atlas = sorted(module.ATLAS_FULLWORLD_PATHS)[0]
    result = classify(module, [atlas])
    assert result == {
        "rust": False,
        "windows": False,
        "surface": "atlas-fullworld",
        "reason": "audited-atlas-fullworld-source",
    }, result

    for path in (
        "tools/game-atlas-fullworld-source/animated.py",
        "tools/game-atlas-creatures/export.py",
    ):
        result = classify(module, [path])
        assert result["rust"] is True and result["windows"] is True, (path, result)

    assert module.routing_health(module.full("classifier-input-failure")) == "degraded"
    assert module.routing_health(module.full("unmodelled-input")) == "unmodelled"
    assert module.routing_health(result) == "modelled"
    print("Routing matrix PASS: product, auxiliary, exact consumers, controls, Atlas and fail-closed cases")


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def test_exact_candidate_reference_scan(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "ci@example.invalid")
        git(root, "config", "user.name", "CI")
        metadata = fixture()
        metadata_root = PurePosixPath(metadata["workspace_root"])

        for package in metadata["packages"]:
            relative_manifest = PurePosixPath(package["manifest_path"]).relative_to(metadata_root)
            manifest = root.joinpath(*relative_manifest.parts)
            package_root = manifest.parent
            package_root.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\n", encoding="utf-8")

        server = root / "apps/game-server/src/lib.rs"
        server.parent.mkdir(parents=True, exist_ok=True)
        server.write_text(
            'const DATA: &[u8] = include_bytes!("../../../docs/agents/evidence/server.json");\n'
            'fn policy() { let _ = std::fs::read_to_string("AGENTS.md"); }\n'
            'fn generated() { let _ = std::fs::read_dir("../../../docs/runtime/generated"); }\n'
            'fn exact_manifest() { let _ = std::fs::read_to_string("docs/agents/evidence/runtime/manifest.json"); }\n'
            'fn unrelated_names() { let _ = "manifest.json"; let _ = "assets/catalog.json"; }\n',
            encoding="utf-8",
        )
        client = root / "apps/client/src/lib.rs"
        client.parent.mkdir(parents=True, exist_ok=True)
        client.write_text(
            'fn theme() { let _ = std::fs::read_to_string("docs/client-theme.json"); }\n',
            encoding="utf-8",
        )
        merge_gate = root / ".github/workflows/merge-gate.yml"
        merge_gate.parent.mkdir(parents=True, exist_ok=True)
        merge_gate.write_text("run: python tools/content/helper.py\n", encoding="utf-8")
        for control_name in ("merge-group-gate.yml", "rust.yml"):
            (root / ".github/workflows" / control_name).write_text("name: control\n", encoding="utf-8")
        for path in (
            "docs/agents/evidence/server.json",
            "AGENTS.md",
            "docs/client-theme.json",
            "docs/runtime/generated/item.json",
            "tools/content/helper.py",
            "docs/unconsumed.json",
            "docs/agents/evidence/runtime/manifest.json",
            "tools/content-schema/monster-authoring/samples/rat/manifest.json",
            "tools/content-schema/monster-authoring/samples/rat/catalog.json",
        ):
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("{}\n", encoding="utf-8")

        git(root, "add", ".")
        git(root, "commit", "-qm", "fixture")
        sha = git(root, "rev-parse", "HEAD")
        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            found = module.candidate_reference_consumers(
                metadata,
                sha,
                [
                    "docs/agents/evidence/server.json",
                    "AGENTS.md",
                    "docs/client-theme.json",
                    "docs/runtime/generated/item.json",
                    "tools/content/helper.py",
                    "docs/unconsumed.json",
                    "docs/agents/evidence/runtime/manifest.json",
                    "tools/content-schema/monster-authoring/samples/rat/manifest.json",
                    "tools/content-schema/monster-authoring/samples/rat/catalog.json",
                ],
            )
        finally:
            os.chdir(old_cwd)

        assert found["docs/agents/evidence/server.json"] == {"oteryn-game-server"}, found
        assert found["AGENTS.md"] == {"oteryn-game-server"}, found
        assert found["docs/client-theme.json"] == {"oteryn-client"}, found
        assert found["docs/runtime/generated/item.json"] == {"oteryn-game-server"}, found
        assert found["tools/content/helper.py"] == {module.CONTROL_CONSUMER}, found
        assert found["docs/unconsumed.json"] == set(), found
        assert found["docs/agents/evidence/runtime/manifest.json"] == {"oteryn-game-server"}, found
        assert found["tools/content-schema/monster-authoring/samples/rat/manifest.json"] == set(), found
        assert found["tools/content-schema/monster-authoring/samples/rat/catalog.json"] == set(), found
    print("Exact candidate reference scan PASS: file, directory, package and canonical-control consumers")


def test_documentation_fast_path(module):
    """Docs-only candidates skip product lanes; anything else stays conservative."""
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "ci@example.invalid")
        git(root, "config", "user.name", "CI")
        metadata = fixture()
        metadata_root = PurePosixPath(metadata["workspace_root"])
        for package in metadata["packages"]:
            relative_manifest = PurePosixPath(package["manifest_path"]).relative_to(metadata_root)
            manifest = root.joinpath(*relative_manifest.parts)
            manifest.parent.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\n", encoding="utf-8")
        files = {
            # Doc-comment prose citing documentation directories (the #1636 shape).
            "apps/game-server/src/lib.rs": (
                "//! Decision (`docs/architecture/reviews/\n"
                "//! OTERYN_GAME_EXAMPLE.md`) and docs/architecture/ index.\n"
                "//! See docs/agents/tasks/archive/ for task records.\n"
            ),
            # A quoted directory literal and an exact Markdown file remain consumers.
            "apps/client/src/lib.rs": (
                'fn guides() { let _ = std::fs::read_dir("../../docs/guides"); }\n'
                'const NOTE: &str = include_str!("../../../docs/contracts/NOTE.md");\n'
            ),
            "docs/architecture/reviews/OTERYN_GAME_EXAMPLE.md": "# old\n",
            "docs/guides/old.md": "# old\n",
            "docs/contracts/NOTE.md": "# old\n",
        }
        for control_name in sorted(module.CANONICAL_CONTROL_PATHS):
            files[control_name] = "name: control\n"
        for path, text in files.items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-qm", "base")
        base = git(root, "rev-parse", "HEAD")

        def candidate(changes):
            git(root, "checkout", "-q", "--detach", base)
            for path, text in changes.items():
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(text, encoding="utf-8")
            git(root, "add", ".")
            git(root, "commit", "-qm", "candidate")
            head = git(root, "rev-parse", "HEAD")
            old_cwd = os.getcwd()
            os.chdir(root)
            try:
                records = module.git_diff_records(base, head)
                return module.classify(
                    records,
                    len(records),
                    metadata,
                    candidate_modes_verified=module.candidate_modes_safe(head),
                    candidate_sha=head,
                )
            finally:
                os.chdir(old_cwd)

        docs_only = {
            "docs/architecture/reviews/OTERYN_GAME_NEW_DECISION.md": "# decision\n",
            "docs/agents/tasks/archive/OTV2-example.md": "# task\n",
            "README.md": "# readme\n",
        }
        result = candidate(docs_only)
        assert result["rust"] is False and result["windows"] is False, result
        assert result["reason"] == "unconsumed-auxiliary-inputs", result

        # Mixed documentation and code never takes the documentation skip.
        result = candidate(docs_only | {"apps/game-server/src/lib.rs": "//! changed\n"})
        assert result["rust"] is True and result["reason"] != "unconsumed-auxiliary-inputs", result
        result = candidate(docs_only | {"apps/client/src/main.rs": "fn main() {}\n"})
        assert result["rust"] is True and result["windows"] is True, result

        # Canonical workflow, composite action and classifier changes run FULL.
        for control in (
            ".github/workflows/merge-group-gate.yml",
            ".github/workflows/merge-gate.yml",
            ".github/actions/setup/action.yml",
            "tools/repository/classify_pr_test_lanes.py",
        ):
            result = candidate(docs_only | {control: "changed\n"})
            assert result["rust"] is True and result["windows"] is True, (control, result)

        # Unknown roots stay FULL even next to documentation.
        result = candidate(docs_only | {"unowned/input.bin": "x\n"})
        assert result["rust"] is True and result["windows"] is True, result
        assert result["reason"] == "unmodelled-input", result

        # Documentation a package really consumes keeps its consumer lanes.
        for consumed in ("docs/guides/new.md", "docs/contracts/NOTE.md"):
            result = candidate({consumed: "# changed\n"})
            assert result["rust"] is True and result["windows"] is True, (consumed, result)

    for path in (
        "docs/architecture/reviews/X.md", "docs/agents/tasks/archive/T.md",
        "README.md", "CONTRIBUTING.md",
    ):
        assert module.documentation_path(path), path
    for path in (
        "docs/agents/evidence/E.md", "docs/migration/M.md", "docs/AGENTS.md",
        "AGENTS.md", "docs/contracts/R.json", ".github/workflows/x.md",
        ".github/pull_request_template.md", "apps/client/README.md", "tools/x/README.md",
    ):
        assert not module.documentation_path(path), path
    assert module.quoted_directory_reference(b'read_dir("../docs/guides")', "docs/guides")
    assert module.quoted_directory_reference(b"'docs/guides/'", "docs/guides")
    assert not module.quoted_directory_reference(b"//! see docs/guides/\n", "docs/guides")
    assert not module.quoted_directory_reference(b"`docs/guides`", "docs/guides")
    print("Documentation fast path PASS: docs-only skip, mixed/control/unknown/consumed FULL")


def test_candidate_modes(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "ci@example.invalid")
        git(root, "config", "user.name", "CI")
        (root / "regular.txt").write_text("ok\n", encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-qm", "regular")
        regular = git(root, "rev-parse", "HEAD")
        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            assert module.candidate_modes_safe(regular) is True
            link_blob = subprocess.check_output(
                ["git", "-C", str(root), "hash-object", "-w", "--stdin"],
                input="regular.txt",
                text=True,
            ).strip()
            subprocess.check_call([
                "git", "-C", str(root), "update-index", "--add", "--cacheinfo",
                "120000", link_blob, "link.txt",
            ])
            git(root, "commit", "-qm", "symlink")
            special = git(root, "rev-parse", "HEAD")
            assert module.candidate_modes_safe(special) is False
        finally:
            os.chdir(old_cwd)
    print("Candidate mode PASS: regular trees accepted, special modes fail closed")


def test_large_pr_fallback(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        git(root, "init", "-q")
        git(root, "config", "user.email", "ci@example.invalid")
        git(root, "config", "user.name", "CI")
        base = root / "base.txt"
        base.write_text("base\n", encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-qm", "base")
        before = git(root, "rev-parse", "HEAD")
        for index in range(301):
            target = root / f"docs/generated/{index}.md"
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("x\n", encoding="utf-8")
        git(root, "add", ".")
        git(root, "commit", "-qm", "large")
        after = git(root, "rev-parse", "HEAD")
        git(root, "checkout", "-q", before)
        old_cwd = os.getcwd()
        os.chdir(root)
        try:
            with patch.dict(
                os.environ,
                {
                    "ENUMERATION_COMPLETE": "false",
                    "CHANGED_FILE_COUNT": "301",
                    "EXPECTED_HEAD": after,
                },
                clear=False,
            ):
                files, count, complete = module.pr_file_records()
        finally:
            os.chdir(old_cwd)
        assert complete is True and count == 301 and len(files) == 301, (count, len(files))
        assert before != after
    print("Large PR fallback PASS: exact Git trees recover complete changed-file evidence")


def test_metadata_events():
    """PR edits must qualify normally without cancelling the product run."""
    core_path = ROOT / "tools/repository/validate_repository_policy_core.py"
    spec = importlib.util.spec_from_file_location("metadata_policy_core", core_path)
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text(encoding="utf-8")
    assert "      - edited\n" in gate, "base retargets must retain native qualification"
    assert "classify_event" not in gate and "metadata_only" not in gate
    scope = core.indented_yaml_mapping_block(gate, "scope", 2)
    main_guard = "github.event.pull_request.base.ref == 'main' || !github.event.pull_request.base.ref"
    assert f"    if: {main_guard}\n" in scope
    assert not re.search(r"^    needs:", scope, flags=re.MULTILINE)
    aggregate = core.indented_yaml_mapping_block(gate, "validate", 2)
    final = core.indented_yaml_mapping_block(gate, "game_gate", 2)
    aggregate_guard = f"    if: always() && ({main_guard})\n"
    assert aggregate_guard in aggregate and aggregate_guard in final
    assert "    name: game-gate\n" in final and "    needs: [scope, validate]\n" in final
    assert '        run: test "$LEGACY_VALIDATE" = "success"\n' in final

    # Evaluate the exact hosted group expression for overlapping runs. An edit
    # gets its own group; retarget and synchronize still supersede stale product
    # qualification, and unrelated PRs never share a cancellation group.
    line = next(line for line in gate.splitlines() if line.startswith("  group:"))
    expected = "  group: ${{ ((github.event.action == 'edited' && github.event.changes.base == null) || ((github.event.action == 'labeled' || github.event.action == 'unlabeled') && github.event.label.name != 'full-ci')) && format('merge-gate-edit-{0}-{1}', github.event.pull_request.number, github.run_id) || format('merge-gate-{0}', github.event.pull_request.number) }}"
    assert line == expected
    def group(action, base_changed, number, run_id):
        expression = line.split("${{", 1)[1].rsplit("}}", 1)[0].strip()
        expression = expression.replace("github.event.action", "action")
        expression = expression.replace("github.event.changes.base", "base")
        expression = expression.replace("github.event.pull_request.number", "number")
        expression = expression.replace("github.run_id", "run_id")
        expression = expression.replace("&&", "and").replace("||", "or").replace("null", "None")
        return eval(expression, {"__builtins__": {}}, {
            "action": action, "base": {"ref": {"from": "other"}} if base_changed else None,
            "number": number, "run_id": run_id,
            "format": lambda template, *args: template.format(*args),
        })
    product = group("synchronize", False, 42, 1)
    edits = [group("edited", False, 42, n) for n in (2, 3)]
    assert len(set([product, *edits])) == 3
    assert group("edited", True, 42, 4) == product
    assert group("synchronize", False, 42, 5) == product
    assert group("synchronize", False, 43, 6) != product
    # Execute the actual final fence with fresh/retargeted live responses. A
    # returning main with a newer base must never publish the old edit's gate.
    import urllib.request
    script = textwrap.dedent(final.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    expected_head, expected_base = "a" * 40, "b" * 40
    live = {"number": 42, "state": "open", "head": {
        "sha": expected_head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
        "base": {"ref": "main", "sha": expected_base}}
    def accepts_live(pull):
        response = io.StringIO(json.dumps(pull))
        env = {"REPOSITORY": "Oteryn/Oteryn-Game", "PR_NUMBER": "42",
               "EXPECTED_HEAD": expected_head, "EXPECTED_BASE": expected_base, "GH_TOKEN": "fixture"}
        with patch.dict(os.environ, env), patch.object(urllib.request, "urlopen", return_value=response), contextlib.redirect_stdout(io.StringIO()):
            try:
                exec(compile(script, "isolated-edit-live-fence", "exec"), {})
                return True
            except (SystemExit, AttributeError, TypeError, ValueError):
                return False
    assert accepts_live(live)
    for changed in (
        {"number": 43}, {"state": "closed"},
        {"base": {"ref": "stack", "sha": expected_base}},
        {"base": {"ref": "main", "sha": "c" * 40}},
        {"head": {"sha": "d" * 40, "repo": {"full_name": "Oteryn/Oteryn-Game"}}},
        {"head": {"sha": expected_head, "repo": {"full_name": "fork/Game"}}},
        {"base": None}, {"head": None},
    ):
        assert not accepts_live(dict(live, **changed)), changed
    assert not accepts_live({}) and not accepts_live(None)
    assert "        if: (github.event.action == 'edited' && github.event.changes.base == null) || ((github.event.action == 'labeled' || github.event.action == 'unlabeled') && github.event.label.name != 'full-ci')\n" in final
    assert "          EXPECTED_BASE: ${{ needs.scope.outputs.base_sha }}\n" in final
    assert "      pull-requests: read\n" in final
    print("Metadata concurrency PASS: isolated edits, retarget cancellation and canonical full game-gate")


def test_aggregate():
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text(encoding="utf-8")
    block = gate.split("  validate:\n", 1)[1].split("  game_gate:\n", 1)[0]
    script = textwrap.dedent(block.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    mandatory = ("SCOPE", "LANES", "GOVERNANCE", "DEPENDENCY_REVIEW", "ROUTING_CONTRACT")
    fast_rust = ("RUST_POLICY", "RUST_FAST", "RUST_SUPPLY_CHAIN")
    heavy = ("CODEQL", "RUST_LINUX", "RUST_WINDOWS", "ATLAS_FULLWORLD")
    qualification = ("NODE_BOOT", "SERVER_SEAM")
    env = dict.fromkeys(mandatory + fast_rust + heavy + qualification, "success")
    env.update(
        FULL_CI="true",
        RUST_REQUIRED="true",
        WINDOWS_REQUIRED="true",
        ATLAS_FULLWORLD_REQUIRED="true",
        SERVER_QUALIFICATION_REQUIRED="true",
    )

    def accepts(changes):
        with patch.dict(os.environ, dict(env, **changes)), contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            try:
                exec(compile(script, "merge-gate:aggregate", "exec"), {})
                return True
            except SystemExit:
                return False

    light = dict.fromkeys(heavy + qualification, "skipped") | {"FULL_CI": "false"}
    assert accepts({})
    assert accepts(light)
    assert accepts({"WINDOWS_REQUIRED": "false", "RUST_WINDOWS": "skipped"})
    assert accepts(dict.fromkeys(fast_rust + heavy[1:], "skipped") | {
        "RUST_REQUIRED": "false",
        "WINDOWS_REQUIRED": "false",
        "ATLAS_FULLWORLD_REQUIRED": "false",
    })
    assert not accepts({"RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "true"})
    for value in ("", "maybe"):
        assert not accepts(light | {"FULL_CI": value}), value
    # Fast Rust checks stay required on every Rust PR, with or without full-ci.
    for name in fast_rust:
        assert not accepts({name: "failure"}), name
        assert not accepts(light | {name: "skipped"}), name
    # Heavy jobs are skipped without full-ci but never tolerate failure, and
    # once requested they are required.
    for name in heavy:
        assert not accepts({name: "failure"}), name
        assert not accepts({name: "skipped"}), name
        assert not accepts(light | {name: "failure"}), name
        assert not accepts(light | {name: "cancelled"}), name
    # Requested physical server qualifications are required unless explicitly deselected.
    for name in qualification:
        assert not accepts({name: "failure"}), name
        assert not accepts({name: "skipped"}), name
        assert not accepts({name: "skipped", "SERVER_QUALIFICATION_REQUIRED": ""}), name
        assert not accepts({name: "failure", "SERVER_QUALIFICATION_REQUIRED": "false"}), name
        assert not accepts(light | {name: "failure"}), name
    assert accepts(dict.fromkeys(qualification, "skipped") | {"SERVER_QUALIFICATION_REQUIRED": "false"})
    for name in mandatory:
        assert not accepts({name: "failure"}), name
        assert not accepts(light | {name: "failure"}), name
    print("Aggregate PASS: fast lanes always required, full-ci heavy lanes required once requested, invalid routing fails closed")


def test_server_qualification(module):
    def required(*paths, previous=None):
        files = [{"filename": path} for path in paths]
        if previous is not None:
            files[0]["previous_filename"] = previous
        return module.server_qualification_required(files, len(files))

    for path in (
        "apps/game-server/src/gameplay_transport/resume.rs",
        "apps/game-server/src/durability/fresh_admission.rs",
        "apps/game-server/src/foundation/protocol.rs",
        "apps/game-server/src/node/serve.rs",
        "apps/game-server/src/main.rs",
        "apps/game-server/src/content/activation.rs",
        "apps/game-server/src/content/project/native_entry.rs",
        "apps/game-server/src/content/project/native_entry_room.json",
        "apps/game-server/src/content/project/v2.rs",
        "apps/game-server/src/content/project/v2/creature.rs",
        "apps/game-server/migrations/0009_character_progression.sql",
        "apps/game-server/Cargo.toml",
        "crates/foundation/src/lib.rs",
        "crates/protocol-oteryn/src/lib.rs",
        "crates/simulation-determinism/src/lib.rs",
        "crates/foundation/Cargo.toml",
        "crates/protocol-oteryn/Cargo.toml",
        "crates/simulation-determinism/Cargo.toml",
        "Cargo.lock",
        "vendor/tokio-1.53.1/src/lib.rs",
        "tools/qualification/node_boot/run.sh",
        "tools/qualification/wp5_s3a/compose.yml",
        ".github/workflows/merge-gate.yml",
    ):
        assert required(path) is True, path
    for path in (
        "apps/game-server/src/combat.rs",
        "apps/game-server/src/ability/mod.rs",
        "apps/game-server/src/ai/mod.rs",
        "apps/game-server/src/interaction/mod.rs",
        "apps/game-server/src/content/cw2_b1_import.rs",
        "apps/game-server/src/content/reference_playable.rs",
        "apps/game-server/tests/durability_postgres.rs",
        "apps/game-server/examples/materialize_content_world_project_v2.rs",
        "apps/client/src/main.rs",
        "docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md",
        "tools/content/quests.py",
        "crates/input-platform/src/lib.rs",
    ):
        assert required(path) is False, path
    assert required("docs/a.md", "apps/game-server/src/ai/mod.rs") is False
    assert required("docs/a.md", previous="apps/game-server/src/durability/mod.rs") is True
    # Both sides of a rename and removals can change the shipped dependency.
    for crate in ("foundation", "protocol-oteryn", "simulation-determinism"):
        path = f"crates/{crate}/src/lib.rs"
        assert required("docs/removed.md", previous=path) is True, path
        assert required(path, previous="docs/added.md") is True, path
        assert module.server_qualification_required([
            {"filename": path, "status": "removed"},
        ], 1) is True, path
        assert required("docs/a.md", path) is True, path
    # Fail closed on incomplete or malformed enumeration.
    assert module.server_qualification_required([], 0) is True
    assert module.server_qualification_required([{"filename": "docs/a.md"}], 2) is True
    assert module.server_qualification_required([{"filename": "docs/a.md"}], 1, complete=False) is True
    assert module.server_qualification_required([{"filename": "../x"}], 1) is True
    assert module.server_qualification_required(["docs/a.md"], 1) is True
    print("Server qualification PASS: boot/transport/durability paths select it, unrelated paths skip, malformed input fails closed")


def test_world_bundle(module):
    def required(*paths, previous=None):
        files = [{"filename": path} for path in paths]
        if previous is not None:
            files[0]["previous_filename"] = previous
        return module.world_bundle_required(files, len(files))

    for path in (
        "content/world/project.json",
        "content/world/pins/oteryn.json",
        "content/world/pins/oteryn.ruleset.json",
        "content/houses/houses-00000-00000.json",
        "content/creatures/definitions/creatures-00000-00000.json",
        "content/items/definitions/items-00000-00000.json",
        "tools/world-bundle-compiler/src/main.rs",
        "crates/world-bundle/src/bundle.rs",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        ".cargo/config.toml",
    ):
        assert required(path) is True, path
        assert required("docs/removed.md", previous=path) is True, path
        assert required("docs/a.md", path) is True, path
    for path in (
        "docs/architecture/a.md",
        "vendor/tokio-1.53.1/src/lib.rs",
        "content/creatures/spawns/a.json",
        "content/items/other/a.json",
        "apps/game-server/src/main.rs",
        "crates/world-bundle-extra/src/lib.rs",
        "tools/world-bundle-compiler-extra/a.rs",
        "apps/game-server/Cargo.toml",
        ".github/workflows/merge-gate.yml",
    ):
        assert required(path) is False, path
    assert required("docs/a.md", previous="docs/b.md") is False
    # Fail closed on incomplete or malformed enumeration.
    assert module.world_bundle_required([], 0) is True
    assert module.world_bundle_required([{"filename": "docs/a.md"}], 2) is True
    assert module.world_bundle_required([{"filename": "docs/a.md"}], 1, complete=False) is True
    assert module.world_bundle_required([{"filename": "../x"}], 1) is True
    assert module.world_bundle_required([{"filename": "docs/a.md", "previous_filename": "../x"}], 1) is True
    assert module.world_bundle_required(["docs/a.md"], 1) is True
    # The compiler lists the same inputs for its inputs_digest.
    source = (ROOT / "tools/world-bundle-compiler/src/main.rs").read_text(encoding="utf-8")
    listed = re.search(r"const INPUT_PATHS: &\[&str\] = &\[(.*?)\];", source, re.S)
    assert listed, "INPUT_PATHS missing"
    rust = set(re.findall(r'"([^"]+)"', listed.group(1)))
    python = set(module.WORLD_BUNDLE_INPUT_FILES) | set(module.WORLD_BUNDLE_INPUT_PREFIXES)
    assert rust == python, (rust ^ python)
    print("World bundle PASS: each compiler input selects the lane, a docs-only change skips, malformed input fails closed")


def test_cli_fallback(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        invalid = root / "metadata.json"
        invalid.write_text("{}", encoding="utf-8")
        output = root / "output"
        result = subprocess.run(
            [sys.executable, str(MODULE), str(invalid)],
            env=dict(os.environ, GITHUB_OUTPUT=str(output), EXPECTED_HEAD="0" * 40),
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result
        wire = output.read_text(encoding="utf-8")
        assert "rust=true\n" in wire and "windows=true\n" in wire, wire
        assert "routing_health=degraded\n" in wire, wire
        assert "server_qualification=true\n" in wire, wire
        assert "world_bundle=true\n" in wire, wire
    print("CLI fallback PASS: malformed classifier inputs remain conservative FULL")


def test_repository_tool_dependencies(module):
    path = "tools/repository/test_validate_game_atlas_semantic_search_triggers.py"
    result = classify(module, [path])
    assert not result["rust"] and not result["windows"], result
    for consumer in (module.CONTROL_CONSUMER, "oteryn-client"):
        result = classify(module, [path], consumers={path: {consumer}})
        assert result["rust"] and result["windows"], result
    result = classify(module, [path], consumers={path: {module.SERVER}})
    assert result["rust"] and not result["windows"], result
    for control in (
        "classify_pr_test_lanes.py", "apply_github_settings.py",
        "validate_repository_policy.py", "validate_repository_policy_core.py",
        "validate_pr_gate_pg_sim.py", "validate_pr_routing_contract.py",
        "test_validate_pr_gate_pg_sim.py", "test_validate_merge_group_pg_sim.py",
        "test_classify_pr_test_lanes.py", "test_classify_post_merge_lanes.py",
        "test_classify_content_routing.py", "future_unknown_controller.py",
    ):
        result = classify(module, ["tools/repository/" + control])
        assert result["rust"] and result["windows"], (control, result)
    result = module.classify([{"filename": path}], 1, fixture(), candidate_modes_verified=False)
    assert result["rust"] and result["windows"], result
    print("Repository tool dependencies PASS: auxiliary case, consumer promotion, security and unknown FULL")


def main() -> int:
    module = load_module()
    test_metadata_events()
    test_repository_tool_dependencies(module)
    test_routing_matrix(module)
    test_exact_candidate_reference_scan(module)
    test_documentation_fast_path(module)
    test_candidate_modes(module)
    test_large_pr_fallback(module)
    test_aggregate()
    test_server_qualification(module)
    test_world_bundle(module)
    test_cli_fallback(module)
    print("Exact-candidate PR routing regressions PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
