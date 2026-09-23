#!/usr/bin/env python3
"""Validate PR routing snapshot health against the exact candidate tree."""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
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


def changed_audited_inputs(module, metadata: dict, records: list[dict]) -> list[str]:
    affected: set[str] = set()
    for item in records:
        if not isinstance(item, dict):
            raise ValueError("invalid changed-file record")
        for key in ("filename", "previous_filename"):
            path = item.get(key)
            if path is None:
                continue
            if not module.valid_path(path):
                raise ValueError("invalid changed-file path")
            if module.audited_input_path(metadata, path):
                affected.add(path)
    return sorted(affected)


def evaluate_snapshot_health(
    module,
    metadata: dict,
    actual: str,
    records: list[dict] | None = None,
    *,
    protected_main: bool = False,
) -> tuple[str, list[str]]:
    if actual == module.AUDITED_INPUT_SHA256:
        return "healthy", []
    if protected_main:
        return "stale-protected-main", []
    changed = changed_audited_inputs(module, metadata, records or [])
    return ("degraded-candidate" if changed else "stale-inherited"), changed


def verify_classifier_matrix(module, metadata: dict) -> None:
    digest = module.AUDITED_INPUT_SHA256
    docs_digest = module.AUDITED_DOC_INPUT_SHA256

    def classify(paths, *, docs_consumers_verified=None):
        records = [dict(filename=path, status="modified") for path in paths]
        return module.classify(
            records,
            len(records),
            metadata,
            digest,
            docs_digest=docs_digest,
            candidate_modes_verified=True,
            docs_consumers_verified=docs_consumers_verified,
        )

    server = f"{module.REQUIRED[module.SERVER]}/src/lib.rs"
    atlas = sorted(module.ATLAS_FULLWORLD_PATHS)[0]
    client = f"{module.REQUIRED['oteryn-client']}/src/lib.rs"

    result = classify([server])
    if not (result["rust"] is True and result["windows"] is False and result["surface"] == "server"):
        raise ValueError(f"server-only routing contract changed: {result}")

    result = classify([server, atlas])
    if not (
        result["rust"] is True
        and result["windows"] is False
        and result["surface"] == "server"
        and result["reason"].endswith("-plus-atlas-fullworld")
    ):
        raise ValueError(f"server + Atlas routing contract changed: {result}")

    result = classify([atlas])
    if result != {
        "rust": False,
        "windows": False,
        "surface": "atlas-fullworld",
        "reason": "audited-atlas-fullworld-source",
    }:
        raise ValueError(f"Atlas-only routing contract changed: {result}")

    result = classify([client])
    if not (result["rust"] is True and result["windows"] is True):
        raise ValueError(f"client routing contract changed: {result}")

    result = classify(["tools/unreviewed/routing_probe.py"])
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "unmodelled-input"
    ):
        raise ValueError(f"unknown-input fail-closed contract changed: {result}")

    result = classify(["Cargo.lock"])
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "explicit-build-or-dependency-input"
    ):
        raise ValueError(f"build-input fail-closed contract changed: {result}")

    result = classify(["AGENTS.md"], docs_consumers_verified=True)
    if result != {
        "rust": False,
        "windows": False,
        "surface": "agent-governance",
        "reason": "agent-governance-only",
    }:
        raise ValueError(f"agent-governance routing contract changed: {result}")

    result = classify(["AGENTS.md"], docs_consumers_verified=False)
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "unreviewed-document-consumer-inputs"
    ):
        raise ValueError(f"agent-governance consumer-proof contract changed: {result}")

    result = classify([server, "AGENTS.md"], docs_consumers_verified=True)
    if not (result["rust"] is True and result["windows"] is False and result["surface"] == "server"):
        raise ValueError(f"server + governance routing contract changed: {result}")

    result = classify([server, "AGENTS.md"], docs_consumers_verified=False)
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "unreviewed-document-consumer-inputs"
    ):
        raise ValueError(f"server + governance consumer-proof contract changed: {result}")

    result = classify(["docs/agents/evidence/runtime-input.json"])
    if not (
        result["rust"] is True
        and result["windows"] is True
        and result["reason"] == "unmodelled-input"
    ):
        raise ValueError(f"runtime-consumed agent-evidence fail-closed contract changed: {result}")


def write_summary(health: str, declared: str, actual: str, changed: list[str]) -> None:
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not path:
        return
    lines = [
        "### Routing contract health",
        "",
        f"- Health: **{health}**",
        f"- Declared non-server snapshot: `{declared}`",
        f"- Candidate non-server snapshot: `{actual}`",
    ]
    if changed:
        lines.append(f"- Candidate-caused audited inputs: {len(changed)}")
        lines.extend(f"  - `{item}`" for item in changed[:20])
        if len(changed) > 20:
            lines.append(f"  - … and {len(changed) - 20} more")
    lines.extend(["", "Classifier routing matrix: **PASS**", ""])
    with open(path, "a", encoding="utf-8") as handle:
        handle.write("\n".join(lines))


def write_outputs(health: str, declared: str, actual: str) -> None:
    path = os.environ.get("GITHUB_OUTPUT")
    if not path:
        return
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(f"routing_health={health}\n")
        handle.write(f"snapshot_declared={declared}\n")
        handle.write(f"snapshot_actual={actual}\n")


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

        verify_classifier_matrix(module, metadata)
        declared = module.AUDITED_INPUT_SHA256
        actual = module.input_digest(metadata)

        records = None
        if actual != declared and not protected_main:
            base = exact_sha("EXPECTED_BASE")
            records = changed_records(module, base, expected_head)
        health, changed = evaluate_snapshot_health(
            module,
            metadata,
            actual,
            records,
            protected_main=protected_main,
        )

        write_summary(health, declared, actual, changed)
        write_outputs(health, declared, actual)

        if health == "healthy":
            print(f"ROUTING_CONTRACT_HEALTHY snapshot={actual}")
            return 0
        if health == "degraded-candidate":
            print(
                "::warning::ROUTING_CONTRACT_DEGRADED_BY_CANDIDATE "
                f"declared={declared} actual={actual} changed_audited_inputs={len(changed)}"
            )
            return 0
        if health == "stale-inherited":
            print(
                "::warning::ROUTING_CONTRACT_STALE_INHERITED "
                f"declared={declared} actual={actual}; "
                "candidate changed no audited routing inputs and trusted-base health is checked separately"
            )
            return 0

        print(
            f"ROUTING_CONTRACT_STALE health={health} declared={declared} actual={actual}",
            file=sys.stderr,
        )
        return 1
    except (
        OSError,
        ValueError,
        KeyError,
        IndexError,
        TypeError,
        AttributeError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as exc:
        print(f"ROUTING_CONTRACT_INVALID: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
