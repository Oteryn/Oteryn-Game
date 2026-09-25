#!/usr/bin/env python3
"""Regressions for content/import exact-consumer impact routing."""
from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile

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
        metadata = fixture(str(root))
        for package in metadata["packages"]:
            manifest = Path(package["manifest_path"])
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


def main() -> int:
    module = load_module()
    tests = (
        test_pr_905_content_tree_shape_is_not_full,
        test_content_real_consumers_still_select_product_lanes,
        test_unknown_inputs_remain_fail_closed,
        test_exact_candidate_scan_finds_existing_content_world_consumer,
    )
    for test in tests:
        test(module)
        print(f"PASS {test.__name__}")
    print("Content/import exact-consumer routing regressions PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
