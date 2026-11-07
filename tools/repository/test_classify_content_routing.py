#!/usr/bin/env python3
"""Regressions for content/import exact-consumer impact routing."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath

MODULE = Path(__file__).with_name("classify_pr_test_lanes.py")


def load_module():
    spec = importlib.util.spec_from_file_location("content_routing_classifier", MODULE)
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


def test_pr_905_content_tree_shape_is_not_full(module):
    paths = [
        "content/content.lock.json",
        "content/cosmetics/mounts/index.json",
        "content/cosmetics/mounts/mounts-00000-00251.json",
        *[
            f"content/items/definitions/items-{index:05d}-{index + 499:05d}.json"
            for index in range(0, 5000, 500)
        ],
        "content/items/index.json",
        "content/manifest.json",
        "content/project.json",
        "imports/crystalserver/batches.json",
        "imports/crystalserver/sources.json",
        "imports/tibiawiki/batches.json",
        "imports/tibiawiki/bindings/items.json",
        "imports/tibiawiki/bindings/mounts.json",
        "imports/tibiawiki/sources.json",
        "tools/content-migration/test_world_project_v2_to_tree.py",
        "tools/content-migration/validate_world_project_v2_to_tree.py",
        "tools/content-migration/world_project_v2_to_tree.py",
        "docs/agents/evidence/OTV2-20260925-full-content-tree-migration-v1.json",
        "docs/agents/tasks/active/OTV2-20260925-full-content-tree-migration-v1.md",
    ]
    result = classify(module, paths)
    assert result == {
        "rust": False,
        "windows": False,
        "surface": "auxiliary",
        "reason": "unconsumed-auxiliary-inputs",
    }, result


def test_content_real_consumers_still_select_product_lanes(module):
    server_input = "content/world/definitions/reference.json"
    result = classify(
        module,
        [server_input],
        consumers={server_input: {module.SERVER}},
    )
    assert result == {
        "rust": True,
        "windows": False,
        "surface": "server",
        "reason": "server-only-exact-consumer-closure",
    }, result

    client_input = "content/items/index.json"
    result = classify(
        module,
        [client_input],
        consumers={client_input: {"oteryn-client"}},
    )
    assert result["rust"] is True and result["windows"] is True, result
    assert result["surface"] == "client", result
    assert result["reason"] == "windows-consumer-affected", result

    control_input = "content/project.json"
    result = classify(
        module,
        [control_input],
        consumers={control_input: {module.CONTROL_CONSUMER}},
    )
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "canonical-control-consumer-affected", result


def test_unknown_inputs_remain_fail_closed(module):
    unknown = "unowned/input.bin"
    result = classify(module, [unknown])
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "unmodelled-input", result

    renamed = [{
        "filename": unknown,
        "status": "renamed",
        "previous_filename": "content/items/index.json",
    }]
    result = classify(module, renamed)
    assert result["rust"] is True and result["windows"] is True, result
    assert result["reason"] == "cross-surface-rename", result


def test_exact_candidate_scan_finds_existing_content_world_consumer(module):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        metadata = fixture()
        metadata_root = PurePosixPath(metadata["workspace_root"])
        for package in metadata["packages"]:
            relative_manifest = PurePosixPath(package["manifest_path"]).relative_to(metadata_root)
            manifest = root.joinpath(*relative_manifest.parts)
            manifest.parent.mkdir(parents=True, exist_ok=True)
            manifest.write_text("[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n", encoding="utf-8")

        world_input = root / "content/world/definitions/reference.json"
        world_input.parent.mkdir(parents=True, exist_ok=True)
        world_input.write_text("{}\n", encoding="utf-8")
        successor_input = root / "content/items/index.json"
        successor_input.parent.mkdir(parents=True, exist_ok=True)
        successor_input.write_text("{}\n", encoding="utf-8")

        server_test = root / "apps/game-server/tests/content_world_project_repository.rs"
        server_test.parent.mkdir(parents=True, exist_ok=True)
        server_test.write_text(
            'fn project_root() { let _ = "content/world"; }\n',
            encoding="utf-8",
        )

        workflows = root / ".github/workflows"
        workflows.mkdir(parents=True, exist_ok=True)
        for name in ("merge-gate.yml", "merge-group-gate.yml", "rust.yml"):
            (workflows / name).write_text(f"name: {name}\n", encoding="utf-8")

        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(
            [
                "git", "-c", "user.name=Oteryn CI", "-c", "user.email=ci@example.invalid",
                "commit", "-q", "-m", "fixture",
            ],
            cwd=root,
            check=True,
        )
        sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        previous = Path.cwd()
        try:
            os.chdir(root)
            consumers = module.candidate_reference_consumers(
                metadata,
                sha,
                [
                    "content/world/definitions/reference.json",
                    "content/items/index.json",
                ],
            )
        finally:
            os.chdir(previous)

    assert consumers["content/world/definitions/reference.json"] == {module.SERVER}, consumers
    assert consumers["content/items/index.json"] == set(), consumers


def classify_deferred(module, paths, consumers):
    records = [{"filename": path, "status": "modified"} for path in paths]
    return module.classify(
        records,
        len(records),
        fixture(),
        candidate_modes_verified=True,
        reference_consumers=consumers,
        defer_content_consumers=True,
    )


def test_pr_content_deferral_is_explicit_and_narrow(module):
    deferred = {
        "rust": False,
        "windows": False,
        "surface": "content",
        "reason": module.CONTENT_DEFERRED_REASON,
    }
    world = "content/world/pins/world-bundle.json"
    items = "content/items/index.json"
    task = "docs/agents/tasks/active/OTV2-20261012-example.md"

    # Merge Queue and post-merge routing never defer: the default stays exact-consumer.
    result = classify(module, [world], consumers={world: {module.SERVER}})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result

    # Pull-request routing defers server-, client- and mixed-consumed Content data.
    for consumers in ({module.SERVER}, {"oteryn-client"}, {module.SERVER, "oteryn-foundation"}):
        result = classify_deferred(module, [world, task], {world: consumers, task: set()})
        assert result == deferred, (consumers, result)
    result = classify_deferred(module, [items], {items: {"oteryn-client"}})
    assert result == deferred, result

    # Never deferred: canonical-control consumers, Rust or build files, Rust sources
    # in the same change, unknown consumers and non-Content consumed inputs.
    result = classify_deferred(module, [world], {world: {module.CONTROL_CONSUMER}})
    assert result["rust"] is True and result["reason"] == "canonical-control-consumer-affected", result
    rust_file = "content/world/generated.rs"
    result = classify_deferred(module, [rust_file], {rust_file: {module.SERVER}})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result
    for build in ("content/world/build.rs", "content/world/Cargo.toml"):
        result = classify_deferred(module, [build], {build: set()})
        assert result["rust"] is True and result["windows"] is True, (build, result)
    server_source = "apps/game-server/src/lib.rs"
    result = classify_deferred(module, [world, server_source], {world: {module.SERVER}, server_source: set()})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result
    result = classify_deferred(module, [world], {world: {"not-a-workspace-package"}})
    assert result["rust"] is True and result["reason"] == "unverified-reference-consumers", result
    evidence = "docs/agents/evidence/runtime-input.json"
    result = classify_deferred(module, [world, evidence], {world: {module.SERVER}, evidence: {module.SERVER}})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result
    result = classify_deferred(module, [world], {world: set()})
    assert result["reason"] == "unconsumed-auxiliary-inputs", result


def test_deploy_inputs_use_exact_consumer_routing(module):
    script = "deploy/synology-game/deploy-ops.sh"
    result = classify(module, [script])
    assert result["rust"] is False and result["windows"] is False, result
    assert result["reason"] == "unconsumed-auxiliary-inputs", result

    topology = "deploy/synology-game/preprod-topology.toml"
    result = classify(module, [topology], consumers={topology: {module.SERVER}})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result

    # Deferral is Content-only: a consumed deploy input is never deferred.
    result = classify_deferred(module, [topology], {topology: {module.SERVER}})
    assert result["rust"] is True and result["reason"] == "server-only-exact-consumer-closure", result


def run_main(module, event_name):
    world = "content/world/project.json"
    saved = (module.pr_file_records, module.candidate_modes_safe, module.candidate_reference_consumers)
    keys = ("GITHUB_EVENT_NAME", "GITHUB_OUTPUT", "EXPECTED_HEAD")
    previous_env = {key: os.environ.get(key) for key in keys}
    previous_argv = sys.argv
    with tempfile.TemporaryDirectory() as directory:
        metadata = Path(directory) / "metadata.json"
        metadata.write_text(json.dumps(fixture()), encoding="utf-8")
        output = Path(directory) / "output"
        output.write_text("", encoding="utf-8")
        try:
            module.pr_file_records = lambda: ([{"filename": world, "status": "modified"}], 1, True)
            module.candidate_modes_safe = lambda sha: True
            module.candidate_reference_consumers = lambda metadata, sha, paths: {
                path: {module.SERVER, "oteryn-foundation"} for path in paths
            }
            os.environ.update(GITHUB_OUTPUT=str(output), EXPECTED_HEAD="a" * 40)
            if event_name is None:
                os.environ.pop("GITHUB_EVENT_NAME", None)
            else:
                os.environ["GITHUB_EVENT_NAME"] = event_name
            sys.argv = ["classify_pr_test_lanes.py", str(metadata)]
            with contextlib.redirect_stdout(io.StringIO()):
                assert module.main() == 0
            lines = output.read_text(encoding="utf-8").splitlines()
        finally:
            module.pr_file_records, module.candidate_modes_safe, module.candidate_reference_consumers = saved
            sys.argv = previous_argv
            for key, value in previous_env.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value
    return dict(line.split("=", 1) for line in lines)


def test_main_defers_content_only_for_pull_request_events(module):
    outputs = run_main(module, "pull_request")
    assert outputs["rust"] == "false" and outputs["windows"] == "false", outputs
    assert outputs["reason"] == module.CONTENT_DEFERRED_REASON, outputs
    assert outputs["world_bundle"] == "true", outputs
    assert outputs["routing_health"] == "modelled", outputs
    for event_name in ("merge_group", "push", "pull_request_target", None):
        outputs = run_main(module, event_name)
        assert outputs["rust"] == "true" and outputs["windows"] == "true", (event_name, outputs)
        assert outputs["reason"] == "windows-consumer-affected", (event_name, outputs)


def main() -> int:
    module = load_module()
    tests = (
        test_pr_905_content_tree_shape_is_not_full,
        test_content_real_consumers_still_select_product_lanes,
        test_unknown_inputs_remain_fail_closed,
        test_exact_candidate_scan_finds_existing_content_world_consumer,
        test_pr_content_deferral_is_explicit_and_narrow,
        test_deploy_inputs_use_exact_consumer_routing,
        test_main_defers_content_only_for_pull_request_events,
    )
    for test in tests:
        test(module)
        print(f"PASS {test.__name__}")
    print("Content/import exact-consumer routing regressions PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
