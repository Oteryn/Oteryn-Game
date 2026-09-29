#!/usr/bin/env python3
"""Behavioral regressions for exact-candidate impact routing."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import os
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


def test_aggregate():
    gate = (ROOT / ".github/workflows/merge-gate.yml").read_text(encoding="utf-8")
    block = gate.split("  validate:\n", 1)[1].split("  game_gate:\n", 1)[0]
    script = textwrap.dedent(block.split("python - <<'PY'\n", 1)[1].rsplit("          PY", 1)[0])
    mandatory = ("SCOPE", "LANES", "GOVERNANCE", "DEPENDENCY_REVIEW", "CODEQL", "ROUTING_CONTRACT")
    rust = ("RUST_POLICY", "RUST_LINUX", "RUST_SUPPLY_CHAIN")
    conditional = ("RUST_WINDOWS", "ATLAS_FULLWORLD")
    qualification = ("NODE_BOOT", "SERVER_SEAM")
    env = dict.fromkeys(mandatory + rust + conditional + qualification, "success")
    env.update(
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

    assert accepts({})
    assert accepts({"WINDOWS_REQUIRED": "false", "RUST_WINDOWS": "skipped"})
    assert accepts(dict.fromkeys(rust + conditional, "skipped") | {
        "RUST_REQUIRED": "false",
        "WINDOWS_REQUIRED": "false",
        "ATLAS_FULLWORLD_REQUIRED": "false",
    })
    assert not accepts({"RUST_REQUIRED": "false", "WINDOWS_REQUIRED": "true"})
    # Physical server qualifications are required unless explicitly deselected.
    for name in qualification:
        assert not accepts({name: "failure"}), name
        assert not accepts({name: "skipped"}), name
        assert not accepts({name: "skipped", "SERVER_QUALIFICATION_REQUIRED": ""}), name
        assert not accepts({name: "failure", "SERVER_QUALIFICATION_REQUIRED": "false"}), name
    assert accepts(dict.fromkeys(qualification, "skipped") | {"SERVER_QUALIFICATION_REQUIRED": "false"})
    for name in mandatory:
        assert not accepts({name: "failure"}), name
    print("Aggregate PASS: selected lanes remain required and invalid routing combinations fail closed")


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
    ):
        assert required(path) is False, path
    assert required("docs/a.md", "apps/game-server/src/ai/mod.rs") is False
    assert required("docs/a.md", previous="apps/game-server/src/durability/mod.rs") is True
    # Fail closed on incomplete or malformed enumeration.
    assert module.server_qualification_required([], 0) is True
    assert module.server_qualification_required([{"filename": "docs/a.md"}], 2) is True
    assert module.server_qualification_required([{"filename": "docs/a.md"}], 1, complete=False) is True
    assert module.server_qualification_required([{"filename": "../x"}], 1) is True
    assert module.server_qualification_required(["docs/a.md"], 1) is True
    print("Server qualification PASS: boot/transport/durability paths select it, unrelated paths skip, malformed input fails closed")


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
    print("CLI fallback PASS: malformed classifier inputs remain conservative FULL")


def main() -> int:
    module = load_module()
    test_routing_matrix(module)
    test_exact_candidate_reference_scan(module)
    test_candidate_modes(module)
    test_large_pr_fallback(module)
    test_aggregate()
    test_server_qualification(module)
    test_cli_fallback(module)
    print("Exact-candidate PR routing regressions PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
