#!/usr/bin/env python3
"""Validate exact-candidate impact-routing semantics and consumer evidence."""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

CLASSIFIER_PATH = Path(__file__).with_name("classify_pr_test_lanes.py")


def load_classifier():
    spec = importlib.util.spec_from_file_location("routing_classifier", CLASSIFIER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("unable to load routing classifier")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def exact_sha(name: str) -> str:
    value = os.environ.get(name, "").strip().lower()
    if re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ValueError(f"{name} must be an exact lowercase SHA")
    return value


def changed_records(module, base: str, head: str) -> list[dict]:
    completeness = os.environ.get("ENUMERATION_COMPLETE", "")
    if completeness == "true":
        records = json.loads(os.environ.get("CHANGED_FILE_RECORDS", "[]"))
        count_text = os.environ.get("CHANGED_FILE_COUNT", "")
        if re.fullmatch(r"[1-9][0-9]*", count_text) is None:
            raise ValueError("invalid changed-file count")
        if not isinstance(records, list) or len(records) != int(count_text):
            raise ValueError("changed-file transport is incomplete")
        return records
    if completeness == "false":
        return module.git_diff_records(base, head)
    raise ValueError("invalid changed-file enumeration state")


def record_paths(module, records: list[dict]) -> list[str]:
    paths: list[str] = []
    seen: set[str] = set()
    for item in records:
        if not isinstance(item, dict):
            raise ValueError("invalid changed-file record")
        for key in ("filename", "previous_filename"):
            path = item.get(key)
            if path is None:
                continue
            if not module.valid_path(path):
                raise ValueError("invalid changed-file path")
            if path not in seen:
                seen.add(path)
                paths.append(path)
    if not paths:
        raise ValueError("empty changed-file set")
    return paths


def verify_classifier_matrix(module, metadata: dict) -> None:
    def classify(paths, *, consumers=None):
        records = [dict(filename=path, status="modified") for path in paths]
        if consumers is None:
            consumers = {path: set() for path in paths}
        return module.classify(
            records,
            len(records),
            metadata,
            candidate_modes_verified=True,
            reference_consumers=consumers,
        )

    server = f"{module.REQUIRED[module.SERVER]}/src/lib.rs"
    client = f"{module.REQUIRED['oteryn-client']}/src/lib.rs"
    evidence = "docs/agents/evidence/runtime-input.json"
    standalone_workflow = ".github/workflows/offline-content.yml"
    offline_tool = "tools/reference-world-corridor-census/offline.py"

    result = classify([server])
    if result != {
        "rust": True,
        "windows": False,
        "surface": "server",
        "reason": "server-only-exact-consumer-closure",
    }:
        raise ValueError(f"server-only routing contract changed: {result}")

    result = classify([client])
    if not (result["rust"] is True and result["windows"] is True):
        raise ValueError(f"client routing contract changed: {result}")

    for path in ("AGENTS.md", "docs/architecture/example.md", evidence, standalone_workflow, offline_tool):
        result = classify([path])
        if not (
            result["rust"] is False
            and result["windows"] is False
            and result["reason"] == "unconsumed-auxiliary-inputs"
        ):
            raise ValueError(f"unconsumed auxiliary routing changed for {path}: {result}")

    result = classify([evidence], consumers={evidence: {module.SERVER}})
    if not (
        result["rust"] is True
        and result["windows"] is False
        and result["reason"] == "server-only-exact-consumer-closure"
    ):
        raise ValueError(f"server-consumed auxiliary routing changed: {result}")

    result = classify([evidence], consumers={evidence: {"oteryn-client"}})
    if not (result["rust"] is True and result["windows"] is True):
        raise ValueError(f"client-consumed auxiliary routing changed: {result}")

    for path in (
        "Cargo.lock",
        ".github/workflows/merge-gate.yml",
        ".github/workflows/merge-group-gate.yml",
        ".github/workflows/rust.yml",
        "tools/repository/classify_pr_test_lanes.py",
    ):
        result = classify([path])
        if not (result["rust"] is True and result["windows"] is True):
            raise ValueError(f"control/build input must stay FULL for {path}: {result}")

    incident = [
        ".github/workflows/item-wiki-first-census.yml",
        "docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json",
        "docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md",
        "tools/reference-world-corridor-census/item_wiki_first_census.py",
        "tools/reference-world-corridor-census/item_wiki_first_census_self_test.py",
    ]
    result = classify(incident)
    if not (
        result["rust"] is False
        and result["windows"] is False
        and result["reason"] == "unconsumed-auxiliary-inputs"
    ):
        raise ValueError(f"PR #803 regression shape changed: {result}")

    unknown = "unowned/input.bin"
    result = classify([unknown])
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "unmodelled-input"
    ):
        raise ValueError(f"unknown-input fail-closed contract changed: {result}")

    atlas = sorted(module.ATLAS_FULLWORLD_PATHS)[0]
    result = classify([atlas])
    if result != {
        "rust": False,
        "windows": False,
        "surface": "atlas-fullworld",
        "reason": "audited-atlas-fullworld-source",
    }:
        raise ValueError(f"Atlas-only routing contract changed: {result}")


def validate_reference_map(module, metadata: dict, head: str, paths: list[str]) -> tuple[dict[str, set[str]], int]:
    references = module.candidate_reference_consumers(metadata, head, paths)
    roots, _ = module.graph(metadata)
    allowed = set(roots) | {module.CONTROL_CONSUMER}
    edges = 0
    if set(references) != set(paths):
        raise ValueError("candidate reference map does not cover the complete changed-path set")
    for path, consumers in references.items():
        if not isinstance(consumers, set) or not consumers <= allowed:
            raise ValueError(f"invalid candidate consumer set for {path}")
        edges += len(consumers)
    return references, edges


def write_summary(paths: list[str], edges: int) -> None:
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not path:
        return
    lines = [
        "### Exact-candidate routing contract",
        "",
        "- Health: **healthy**",
        f"- Changed/renamed paths inspected: {len(paths)}",
        f"- Exact product/control consumer edges: {edges}",
        "",
        "Classifier routing matrix: **PASS**",
        "",
    ]
    with open(path, "a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def write_outputs(edges: int) -> None:
    path = os.environ.get("GITHUB_OUTPUT")
    if not path:
        return
    with open(path, "a", encoding="utf-8") as handle:
        handle.write("routing_health=healthy\n")
        handle.write(f"consumer_edges={edges}\n")


def main() -> int:
    protected_main = len(sys.argv) == 3 and sys.argv[1] == "--protected-main"
    metadata_arg = sys.argv[2] if protected_main else (sys.argv[1] if len(sys.argv) == 2 else None)
    if metadata_arg is None:
        print("usage: validate_pr_routing_contract.py [--protected-main] <metadata.json>", file=sys.stderr)
        return 2

    try:
        module = load_classifier()
        metadata = json.loads(Path(metadata_arg).read_text(encoding="utf-8"))
        expected_head = exact_sha("EXPECTED_HEAD")
        actual_head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip().lower()
        if actual_head != expected_head:
            raise ValueError("checked-out revision does not match expected head")
        if not module.candidate_modes_safe(expected_head):
            raise ValueError("exact candidate contains unsupported Git modes")

        verify_classifier_matrix(module, metadata)

        if protected_main:
            paths = ["tools/repository/classify_pr_test_lanes.py"]
            edges = 0
        else:
            base = exact_sha("EXPECTED_BASE")
            records = changed_records(module, base, expected_head)
            paths = record_paths(module, records)
            _references, edges = validate_reference_map(module, metadata, expected_head, paths)

        write_summary(paths, edges)
        write_outputs(edges)
        print(
            f"ROUTING_CONTRACT_HEALTHY exact_head={expected_head} "
            f"paths={len(paths)} consumer_edges={edges}"
        )
        return 0
    except (
        OSError,
        ValueError,
        KeyError,
        IndexError,
        TypeError,
        AttributeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
        UnicodeError,
    ) as exc:
        print(f"ROUTING_CONTRACT_INVALID: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
