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
    server = f"{module.REQUIRED[module.SERVER]}/src/lib.rs"
    client = f"{module.REQUIRED['oteryn-client']}/src/lib.rs"
    evidence = "docs/agents/evidence/runtime-input.json"
    helper = "tools/content/helper.py"
    incident = (
        ".github/workflows/item-wiki-first-census.yml",
        "docs/agents/evidence/OTV2-20260923-item-wiki-first-census.json",
        "docs/agents/tasks/active/OTV2-20260923-item-wiki-first-census.md",
        "tools/reference-world-corridor-census/item_wiki_first_census.py",
        "tools/reference-world-corridor-census/item_wiki_first_census_self_test.py",
    )
    cases = (
        ("server-only", (server,), {"rust": True, "windows": False, "surface": "server", "reason": "server-only-exact-consumer-closure"}, ()),
        ("client", (client,), {"rust": True, "windows": True, "surface": "client", "reason": "windows-consumer-affected"}, ()),
        ("shared", ("crates/foundation/src/lib.rs",), {"rust": True, "windows": True, "reason": "windows-consumer-affected"}, ()),
        ("governance-aux", ("AGENTS.md",), {"rust": False, "windows": False, "surface": "agent-governance", "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("docs-aux", ("docs/architecture/example.md",), {"rust": False, "windows": False, "surface": "docs", "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("evidence-aux", (evidence,), {"rust": False, "windows": False, "surface": "auxiliary", "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("workflow-aux", (".github/workflows/offline-content.yml",), {"rust": False, "windows": False, "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("tool-aux", ("tools/reference-world-corridor-census/offline.py",), {"rust": False, "windows": False, "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("server-consumed-aux", (evidence,), {"rust": True, "windows": False, "reason": "server-only-exact-consumer-closure"}, ((evidence, (module.SERVER,)),)),
        ("client-consumed-aux", (evidence,), {"rust": True, "windows": True, "reason": "windows-consumer-affected"}, ((evidence, ("oteryn-client",)),)),
        ("canonical-ci-consumer", (helper,), {"rust": True, "windows": True, "surface": "control-plane", "reason": "canonical-control-consumer-affected"}, ((helper, (module.CONTROL_CONSUMER,)),)),
        ("cargo-lock", ("Cargo.lock",), {"rust": True, "windows": True, "surface": "dependencies-build", "reason": "explicit-build-or-dependency-input"}, ()),
        ("merge-gate-control", (".github/workflows/merge-gate.yml",), {"rust": True, "windows": True, "surface": "control-plane", "reason": "explicit-build-or-control-input"}, ()),
        ("merge-group-control", (".github/workflows/merge-group-gate.yml",), {"rust": True, "windows": True, "reason": "explicit-build-or-control-input"}, ()),
        ("rust-workflow-control", (".github/workflows/rust.yml",), {"rust": True, "windows": True, "reason": "explicit-build-or-control-input"}, ()),
        ("repository-tool-control", ("tools/repository/classify_pr_test_lanes.py",), {"rust": True, "windows": True, "reason": "explicit-build-or-control-input"}, ()),
        ("pr-803-regression", incident, {"rust": False, "windows": False, "surface": "auxiliary", "reason": "unconsumed-auxiliary-inputs"}, ()),
        ("unknown-fail-closed", ("unowned/input.bin",), {"rust": True, "windows": True, "surface": "unknown", "reason": "unmodelled-input"}, ()),
        ("atlas-fullworld", (sorted(module.ATLAS_FULLWORLD_PATHS)[0],), {"rust": False, "windows": False, "surface": "atlas-fullworld", "reason": "audited-atlas-fullworld-source"}, ()),
    )

    names = set()
    for name, paths, expected, edges in cases:
        if not name or name in names or not paths or len(set(paths)) != len(paths):
            raise ValueError(f"invalid routing contract case: {name!r}")
        names.add(name)
        references = {path: set() for path in paths}
        for path, consumers in edges:
            if path not in references:
                raise ValueError(f"{name}: consumer edge targets undeclared path")
            references[path].update(consumers)
        result = module.classify(
            [{"filename": path, "status": "modified"} for path in paths],
            len(paths),
            metadata,
            candidate_modes_verified=True,
            reference_consumers=references,
        )
        mismatch = {
            key: (value, result.get(key))
            for key, value in expected.items()
            if result.get(key) != value
        }
        if mismatch:
            raise ValueError(f"{name}: routing contract drift {mismatch}; result={result}")


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
