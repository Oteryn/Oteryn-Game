#!/usr/bin/env python3
"""Validate the Oteryn v2 agent-governance bootstrap using stdlib only."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "docs/agents/GOVERNANCE_CONTRACT.json"
LANES_PATH = ROOT / "docs/agents/PROJECT_LANES.json"
EXPECTED_REPOSITORY = "blakinio/Oteryn-v2"


def load_json(path: Path, errors: list[str]) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        errors.append(f"missing JSON file: {path.relative_to(ROOT)}")
        return {}
    except json.JSONDecodeError as exc:
        errors.append(f"invalid JSON in {path.relative_to(ROOT)}: {exc}")
        return {}
    if not isinstance(value, dict):
        errors.append(f"expected JSON object: {path.relative_to(ROOT)}")
        return {}
    return value


def require_file(relative: str, errors: list[str]) -> None:
    path = ROOT / relative
    if not path.is_file():
        errors.append(f"missing required file: {relative}")


def main() -> int:
    errors: list[str] = []
    contract = load_json(CONTRACT_PATH, errors)
    lanes = load_json(LANES_PATH, errors)

    if contract.get("repository") != EXPECTED_REPOSITORY:
        errors.append("governance repository must be blakinio/Oteryn-v2")
    if contract.get("default_branch") != "main":
        errors.append("default branch must be main")
    if contract.get("task_prefix") != "OTV2":
        errors.append("task prefix must be OTV2")
    if contract.get("merge_method") != "squash":
        errors.append("merge method must be squash")
    if contract.get("write_allowlist") != [EXPECTED_REPOSITORY]:
        errors.append("write_allowlist must contain only blakinio/Oteryn-v2")

    for relative in contract.get("required_documents", []):
        if isinstance(relative, str):
            require_file(relative, errors)
        else:
            errors.append("required_documents entries must be strings")
    for relative in contract.get("required_architecture", []):
        if isinstance(relative, str):
            require_file(relative, errors)
        else:
            errors.append("required_architecture entries must be strings")

    required_task_paths = [
        "docs/agents/tasks/TASK_TEMPLATE.md",
        "docs/agents/tasks/active/README.md",
        "docs/agents/tasks/archive/README.md",
    ]
    for relative in required_task_paths:
        require_file(relative, errors)

    workflow = contract.get("validation", {}).get("workflow")
    command = contract.get("validation", {}).get("command")
    if isinstance(workflow, str):
        require_file(workflow, errors)
    else:
        errors.append("validation.workflow must be a string")
    if command != "python tools/agents/validate_governance.py":
        errors.append("unexpected governance validation command")

    if lanes.get("repository") != EXPECTED_REPOSITORY:
        errors.append("project lanes repository mismatch")
    lane_ids = {
        lane.get("id")
        for lane in lanes.get("lanes", [])
        if isinstance(lane, dict)
    }
    expected_lanes = {
        "governance",
        "architecture-contracts",
        "protocol",
        "server-runtime",
        "persistence",
        "client-runtime",
        "content-migration",
        "platform-integration",
        "release-security",
    }
    missing_lanes = sorted(expected_lanes - lane_ids)
    if missing_lanes:
        errors.append(f"missing project lanes: {', '.join(missing_lanes)}")

    root_agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8") if (ROOT / "AGENTS.md").is_file() else ""
    override = (ROOT / "AGENTS.override.md").read_text(encoding="utf-8") if (ROOT / "AGENTS.override.md").is_file() else ""
    cross_repo = (ROOT / "docs/agents/CROSS_REPO_CONTRACTS.md").read_text(encoding="utf-8") if (ROOT / "docs/agents/CROSS_REPO_CONTRACTS.md").is_file() else ""

    mandatory_phrases = [
        "blakinio/Oteryn-v2",
        "protocol-oteryn",
        "multichannel",
        "WorldId",
        "ChannelId",
        "session-generation",
    ]
    for phrase in mandatory_phrases:
        if phrase not in root_agents:
            errors.append(f"AGENTS.md missing mandatory phrase: {phrase}")

    if "write_allowlist" not in CONTRACT_PATH.read_text(encoding="utf-8"):
        errors.append("machine-readable write allowlist is missing")
    if "protocol-oteryn" not in cross_repo or "protocol-canary" not in cross_repo:
        errors.append("cross-repository policy must state both target and rejected legacy protocol direction")
    if "requires an explicit owner-approved ADR" not in cross_repo:
        errors.append("cross-repository policy must gate protocol-canary reintroduction")

    referenced = set(re.findall(r"docs/agents/[A-Z0-9_./-]+\.md", override))
    for relative in sorted(referenced):
        require_file(relative, errors)

    for forbidden in ["Laravel / PHP implementation policy", "Precompiled Header Policy", "Docker Quickstart Policy", "live-capital authority"]:
        if forbidden in root_agents:
            errors.append(f"AGENTS.md contains foreign repository policy: {forbidden}")

    if errors:
        print("Governance validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Governance validation passed for {EXPECTED_REPOSITORY}.")
    print(f"Validated {len(contract.get('required_documents', []))} required policy documents and {len(lane_ids)} project lanes.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
