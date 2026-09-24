#!/usr/bin/env python3
"""Validate the Oteryn Game agent-governance bootstrap using stdlib only."""

from __future__ import annotations

from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "docs/agents/GOVERNANCE_CONTRACT.json"
LANES_PATH = ROOT / "docs/agents/PROJECT_LANES.json"
CONTRACT_LOCK_PATH = ROOT / "docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json"
LIMITS_REGISTRY_PATH = ROOT / "docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
PROMPT_LIFECYCLE_PATH = ROOT / "docs/agents/PROMPT_LIFECYCLE.json"
HANDOVER_LIFECYCLE_PATH = ROOT / "docs/agents/HANDOVER_LIFECYCLE.json"
PROGRAM_LIFECYCLE_PATH = ROOT / "docs/agents/PROGRAM_LIFECYCLE.json"
EXPECTED_REPOSITORY = "Oteryn/Oteryn-Game"


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


def validate_prompt_lifecycle(registry: dict, errors: list[str]) -> None:
    prompts_dir = ROOT / "docs/agents/prompts"
    actual = {
        path.relative_to(ROOT).as_posix()
        for path in prompts_dir.rglob("*.md")
        if path.name != "README.md"
    }
    entries = registry.get("prompts", [])
    if not isinstance(entries, list):
        errors.append("prompt lifecycle registry prompts must be a list")
        return

    seen_ids: set[str] = set()
    seen_paths: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"prompt lifecycle entry {index} must be an object")
            continue
        prompt_id = entry.get("prompt_id")
        path = entry.get("path")
        version = entry.get("version")
        status = entry.get("status")
        owner = entry.get("owner")
        scope = entry.get("scope")
        reusable = entry.get("reusable")
        superseded_by = entry.get("superseded_by")
        supersession_rule = entry.get("supersession_rule")

        if not isinstance(prompt_id, str) or not prompt_id:
            errors.append(f"prompt lifecycle entry {index} has invalid prompt_id")
            continue
        if prompt_id in seen_ids:
            errors.append(f"duplicate prompt lifecycle id: {prompt_id}")
        seen_ids.add(prompt_id)
        if not isinstance(path, str) or not path:
            errors.append(f"prompt {prompt_id} has invalid path")
            continue
        if path in seen_paths:
            errors.append(f"duplicate prompt lifecycle path: {path}")
        seen_paths.add(path)
        if not isinstance(version, str) or re.fullmatch(r"\d+\.\d+", version) is None:
            errors.append(f"prompt {prompt_id} has invalid version")
        if status not in {"reusable", "retired"}:
            errors.append(f"prompt {prompt_id} has unsupported status: {status}")
        if not isinstance(owner, str) or not owner.strip():
            errors.append(f"prompt {prompt_id} must define owner")
        if not isinstance(scope, str) or not scope.strip():
            errors.append(f"prompt {prompt_id} must define scope")
        if not isinstance(reusable, bool):
            errors.append(f"prompt {prompt_id} reusable must be boolean")
        if not isinstance(supersession_rule, str) or not supersession_rule.strip():
            errors.append(f"prompt {prompt_id} must define supersession_rule")
        if status == "retired":
            if reusable is not False:
                errors.append(f"retired prompt {prompt_id} cannot be reusable")
            if not isinstance(superseded_by, str) or not superseded_by.strip():
                errors.append(f"retired prompt {prompt_id} must name superseded_by")
        elif superseded_by is not None and (not isinstance(superseded_by, str) or not superseded_by.strip()):
            errors.append(f"prompt {prompt_id} superseded_by must be null or a non-empty string")

    missing = sorted(actual - seen_paths)
    extra = sorted(seen_paths - actual)
    if missing:
        errors.append(f"prompt lifecycle registry missing paths: {', '.join(missing)}")
    if extra:
        errors.append(f"prompt lifecycle registry has unknown paths: {', '.join(extra)}")


def validate_handover_lifecycle(registry: dict, errors: list[str]) -> None:
    roots = [ROOT / "docs/agents/evidence", ROOT / "docs/agents/reports"]
    actual: set[str] = set()
    for directory in roots:
        if not directory.is_dir():
            continue
        for path in directory.glob("*.md"):
            lowered = path.name.lower()
            if "handoff" in lowered or "handover" in lowered:
                actual.add(path.relative_to(ROOT).as_posix())

    entries = registry.get("handovers", [])
    if not isinstance(entries, list):
        errors.append("handover lifecycle registry handovers must be a list")
        return
    seen_ids: set[str] = set()
    seen_paths: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"handover lifecycle entry {index} must be an object")
            continue
        handover_id = entry.get("handover_id")
        path = entry.get("path")
        if not isinstance(handover_id, str) or not handover_id:
            errors.append(f"handover lifecycle entry {index} has invalid handover_id")
            continue
        if handover_id in seen_ids:
            errors.append(f"duplicate handover lifecycle id: {handover_id}")
        seen_ids.add(handover_id)
        if not isinstance(path, str) or not path:
            errors.append(f"handover {handover_id} has invalid path")
            continue
        if path in seen_paths:
            errors.append(f"duplicate handover lifecycle path: {path}")
        seen_paths.add(path)
        if entry.get("status") != "historical":
            errors.append(f"handover {handover_id} must have historical status")
        if entry.get("authoritative") is not False:
            errors.append(f"handover {handover_id} must be explicitly non-authoritative")
        expiry_rule = entry.get("expiry_rule")
        if not isinstance(expiry_rule, str) or not expiry_rule.strip():
            errors.append(f"handover {handover_id} must define expiry_rule")
        superseded_by = entry.get("superseded_by")
        if not isinstance(superseded_by, list) or not superseded_by or not all(
            isinstance(value, str) and value.strip() for value in superseded_by
        ):
            errors.append(f"handover {handover_id} must define superseded_by")

    missing = sorted(actual - seen_paths)
    extra = sorted(seen_paths - actual)
    if missing:
        errors.append(f"handover lifecycle registry missing paths: {', '.join(missing)}")
    if extra:
        errors.append(f"handover lifecycle registry has unknown paths: {', '.join(extra)}")


def validate_program_lifecycle(registry: dict, errors: list[str]) -> None:
    programs_dir = ROOT / "docs/agents/programs"
    archive_dir = programs_dir / "archive"
    actual = {
        path.relative_to(ROOT).as_posix()
        for path in archive_dir.rglob("*.md")
        if path.name != "README.md"
    } if archive_dir.is_dir() else set()

    entries = registry.get("programs", [])
    if not isinstance(entries, list):
        errors.append("program lifecycle registry programs must be a list")
        return

    seen_ids: set[str] = set()
    seen_paths: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            errors.append(f"program lifecycle entry {index} must be an object")
            continue
        program_id = entry.get("program_id")
        path = entry.get("path")
        if not isinstance(program_id, str) or not program_id:
            errors.append(f"program lifecycle entry {index} has invalid program_id")
            continue
        if program_id in seen_ids:
            errors.append(f"duplicate program lifecycle id: {program_id}")
        seen_ids.add(program_id)
        if not isinstance(path, str) or not path:
            errors.append(f"program {program_id} has invalid path")
            continue
        if path in seen_paths:
            errors.append(f"duplicate program lifecycle path: {path}")
        seen_paths.add(path)
        if not path.startswith("docs/agents/programs/archive/"):
            errors.append(f"historical program {program_id} must live under programs/archive")
        if entry.get("status") != "historical":
            errors.append(f"program {program_id} must have historical status")
        if entry.get("authoritative") is not False:
            errors.append(f"program {program_id} must be explicitly non-authoritative")
        terminal_evidence = entry.get("terminal_evidence")
        if not isinstance(terminal_evidence, list) or not terminal_evidence or not all(
            isinstance(value, str) and value.strip() for value in terminal_evidence
        ):
            errors.append(f"program {program_id} must define terminal_evidence")
        superseded_by = entry.get("superseded_by")
        if not isinstance(superseded_by, list) or not superseded_by or not all(
            isinstance(value, str) and value.strip() for value in superseded_by
        ):
            errors.append(f"program {program_id} must define superseded_by")

    missing = sorted(actual - seen_paths)
    extra = sorted(seen_paths - actual)
    if missing:
        errors.append(f"program lifecycle registry missing archived paths: {', '.join(missing)}")
    if extra:
        errors.append(f"program lifecycle registry has unknown archived paths: {', '.join(extra)}")

    if programs_dir.is_dir() and archive_dir.is_dir():
        current_names = {
            path.name for path in programs_dir.glob("*.md") if path.name != "README.md"
        }
        archive_names = {
            path.name for path in archive_dir.rglob("*.md") if path.name != "README.md"
        }
        for duplicate in sorted(current_names & archive_names):
            errors.append(
                f"program record exists in both current and archive: {duplicate}"
            )


def validate_active_task_packets(
    errors: list[str],
    task_statuses: object | None = None,
    task_modes: object | None = None,
    limits: object | None = None,
) -> None:
    active_dir = ROOT / "docs/agents/tasks/active"
    if not active_dir.is_dir():
        return

    default_statuses = {
        "investigating",
        "implementing",
        "validating",
        "ready",
        "waiting",
        "blocked",
        "completed",
    }
    default_modes = {
        "IMPLEMENT",
        "AUDIT",
        "CONTRACT",
        "REPAIR",
        "COORDINATE",
        "MIGRATE",
        "GOVERNANCE",
        "BUILD",
    }

    if task_statuses is None:
        allowed_statuses = default_statuses
    elif isinstance(task_statuses, list) and task_statuses and all(
        isinstance(value, str) and value for value in task_statuses
    ):
        allowed_statuses = set(task_statuses)
    else:
        errors.append("governance task_statuses must be a non-empty string list")
        allowed_statuses = set()

    if task_modes is None:
        allowed_modes = default_modes
    elif isinstance(task_modes, list) and task_modes and all(
        isinstance(value, str) and value for value in task_modes
    ):
        allowed_modes = set(task_modes)
    else:
        errors.append("governance task_modes must be a non-empty string list")
        allowed_modes = set()

    max_characters = 12000
    max_lines = 300
    if limits is not None:
        if not isinstance(limits, dict):
            errors.append("active_task_limits must be an object")
        else:
            chars = limits.get("max_characters")
            lines = limits.get("max_lines")
            if not isinstance(chars, int) or chars <= 0:
                errors.append("active_task_limits.max_characters must be a positive integer")
            else:
                max_characters = chars
            if not isinstance(lines, int) or lines <= 0:
                errors.append("active_task_limits.max_lines must be a positive integer")
            else:
                max_lines = lines

    terminal_statuses = {"completed", "closed", "merged", "terminal", "archived", "done"}
    for path in sorted(active_dir.glob("*.md")):
        if path.name == "README.md":
            continue
        relative = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")

        if len(text) > max_characters or len(text.splitlines()) > max_lines:
            errors.append(
                f"active task packet {relative} exceeded bounded current-state size "
                f"({len(text)} chars/{len(text.splitlines())} lines; "
                f"max {max_characters}/{max_lines})"
            )

        issue = re.search(r"(?m)^issue:\s*([1-9][0-9]*)\s*$", text)
        pr = re.search(r"(?m)^pr:\s*([1-9][0-9]*)\s*$", text)
        if issue is None and pr is None:
            errors.append(f"active task packet {relative} must name a positive issue or pr")

        mode_match = re.search(r"(?m)^mode:\s*([^\n#]+?)\s*$", text)
        if mode_match is None:
            errors.append(f"active task packet {relative} must define mode")
        else:
            mode = mode_match.group(1).strip().strip('"\'')
            if mode not in allowed_modes:
                errors.append(f"active task packet {relative} has unsupported mode {mode}")

        status_match = re.search(r"(?m)^status:\s*([^\n#]+?)\s*$", text)
        if status_match is None:
            errors.append(f"active task packet {relative} must define status")
        else:
            status = status_match.group(1).strip().strip('"\'').lower()
            if status not in allowed_statuses:
                errors.append(f"active task packet {relative} has unsupported status {status}")
            if status in terminal_statuses:
                errors.append(f"active task packet {relative} has terminal status {status}")
            if status in {"validating", "ready"} and pr is None:
                errors.append(
                    f"active task packet {relative} with status {status} "
                    "must bind a positive canonical pr"
                )

    archive_dir = ROOT / "docs/agents/tasks/archive"
    if archive_dir.is_dir():
        active_names = {
            path.name for path in active_dir.glob("*.md") if path.name != "README.md"
        }
        archive_names = {
            path.name for path in archive_dir.glob("*.md") if path.name != "README.md"
        }
        for duplicate in sorted(active_names & archive_names):
            errors.append(
                f"task packet exists in both active and archive: {duplicate}"
            )


def validate_active_task_live_state(
    request: Callable[[str], object],
    packet_paths: set[str] | None = None,
) -> list[str]:
    """Reject selected active packets whose canonical GitHub authority is terminal.

    packet_paths=None validates the complete active set for protected-main
    health. A concrete set validates only candidate-touched packets, so an
    unrelated lifecycle transition cannot retroactively invalidate an immutable
    PR head. The caller owns authentication and transport.
    """
    active_dir = ROOT / "docs/agents/tasks/active"
    if not active_dir.is_dir():
        return []

    if packet_paths is not None:
        invalid = sorted(
            relative for relative in packet_paths
            if not (
                relative.startswith("docs/agents/tasks/active/")
                and relative.endswith(".md")
                and relative != "docs/agents/tasks/active/README.md"
            )
        )
        if invalid:
            raise ValueError(f"invalid active-task scope: {', '.join(invalid)}")

    packets: list[tuple[str, int | None, int | None]] = []
    references: set[tuple[str, int]] = set()
    for path in sorted(active_dir.glob("*.md")):
        if path.name == "README.md":
            continue
        relative = path.relative_to(ROOT).as_posix()
        if packet_paths is not None and relative not in packet_paths:
            continue
        text = path.read_text(encoding="utf-8")
        issue_match = re.search(r"(?m)^issue:\s*([1-9][0-9]*)\s*$", text)
        pr_match = re.search(r"(?m)^pr:\s*([1-9][0-9]*)\s*$", text)
        issue = int(issue_match.group(1)) if issue_match is not None else None
        pr = int(pr_match.group(1)) if pr_match is not None else None
        packets.append((relative, issue, pr))
        if issue is not None:
            references.add(("issues", issue))
        if pr is not None:
            references.add(("pulls", pr))

    def fetch(reference: tuple[str, int]) -> tuple[tuple[str, int], object]:
        kind, number = reference
        return reference, request(
            f"https://api.github.com/repos/{EXPECTED_REPOSITORY}/{kind}/{number}"
        )

    with ThreadPoolExecutor(max_workers=min(8, max(1, len(references)))) as executor:
        payloads = dict(executor.map(fetch, sorted(references)))

    errors: list[str] = []
    for relative, issue, pr in packets:
        pr_open = False
        if pr is not None:
            payload = payloads[("pulls", pr)]
            if not isinstance(payload, dict) or payload.get("state") not in {"open", "closed"}:
                raise ValueError(f"invalid GitHub pull response for #{pr}")
            pr_open = payload["state"] == "open"
            if not pr_open:
                disposition = "merged" if payload.get("merged_at") is not None else "closed"
                errors.append(
                    f"active task packet {relative} names terminal canonical PR #{pr} ({disposition})"
                )
        if issue is not None:
            payload = payloads[("issues", issue)]
            if not isinstance(payload, dict) or payload.get("state") not in {"open", "closed"}:
                raise ValueError(f"invalid GitHub issue response for #{issue}")
            if payload["state"] == "closed" and not pr_open:
                errors.append(
                    f"active task packet {relative} names closed Issue #{issue} without an open canonical PR"
                )
    return errors

def _markdown_section(text: str, heading: str) -> str:
    marker = f"## {heading}\n"
    start = text.find(marker)
    if start < 0:
        return ""
    body_start = start + len(marker)
    next_heading = text.find("\n## ", body_start)
    return text[body_start:] if next_heading < 0 else text[body_start:next_heading]


def validate_context_economy(errors: list[str]) -> None:
    routing_path = ROOT / "docs/agents/CONTEXT_ROUTING.md"
    routing = routing_path.read_text(encoding="utf-8") if routing_path.is_file() else ""
    for fragment in (
        "do not bulk-read complete Issue or PR comment timelines",
        "do not read the complete `PROMPT_LIFECYCLE.json` to invoke one known alias",
        "prompt evaluation is for prompt authoring/material changes/lifecycle evaluation",
    ):
        if fragment not in routing:
            errors.append(f"context routing missing context-economy rule: {fragment}")

    worker_prompts = (
        "docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md",
        "docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md",
        "docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md",
        "docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md",
        "docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md",
    )
    for relative in worker_prompts:
        path = ROOT / relative
        if not path.is_file():
            errors.append(f"missing context-economy worker prompt: {relative}")
            continue
        startup = _markdown_section(path.read_text(encoding="utf-8"), "Mandatory startup")
        if "OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md" in startup:
            errors.append(f"{relative} must not load owner operator runbook in technical startup")

    work_path = ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md"
    if work_path.is_file():
        work = work_path.read_text(encoding="utf-8")
        legacy = (
            "fresh-read protected `main`, root/nearest `AGENTS.md`, META binding, "
            "`PROMPT_LIFECYCLE.json`"
        )
        if legacy in work:
            errors.append("Work coordinator reintroduced full-registry mandatory startup")

    prompts_readme = ROOT / "docs/agents/prompts/README.md"
    if prompts_readme.is_file() and "Before reuse, evaluate the selected prompt against" in prompts_readme.read_text(encoding="utf-8"):
        errors.append("prompt README reintroduced per-invocation prompt evaluation")


def validate_current_state_hygiene(errors: list[str]) -> None:
    live_path = ROOT / "docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md"
    if not live_path.is_file():
        errors.append("missing current implementation allocation snapshot")
    else:
        live = live_path.read_text(encoding="utf-8")
        if "CURRENT-STATE ROUTING ONLY" not in live:
            errors.append("live allocations must declare current-state-only routing")
        if len(live.splitlines()) > 180 or len(live) > 12000:
            errors.append("live allocations exceeded bounded current-state size")
        for legacy_heading in (
            "## Completed allocation",
            "## Historical completed allocation",
            "## Prior Work allocation checkpoint",
            "## Current Work checkpoint",
            "## Prospective allocation",
        ):
            if legacy_heading in live:
                errors.append(
                    f"live allocations reintroduced historical ledger heading: {legacy_heading}"
                )

    work_task = ROOT / "docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md"
    if not work_task.is_file():
        errors.append("missing active Work delivery coordinator task")
    else:
        text = work_task.read_text(encoding="utf-8")
        if len(text.splitlines()) > 180 or len(text) > 12000:
            errors.append("active Work coordinator task exceeded bounded current-state size")
        if text.count("## Context checkpoint") != 1:
            errors.append("active Work coordinator task must contain exactly one Context checkpoint")
        for legacy_heading in (
            "## Current Work checkpoint",
            "## Prior Work allocation checkpoint",
            "## Current continuation and",
            "## Current prospective",
        ):
            if legacy_heading in text:
                errors.append(
                    f"active Work coordinator task reintroduced historical ledger heading: {legacy_heading}"
                )


def main() -> int:
    errors: list[str] = []
    contract = load_json(CONTRACT_PATH, errors)
    lanes = load_json(LANES_PATH, errors)
    contract_lock = load_json(CONTRACT_LOCK_PATH, errors)
    limits_registry = load_json(LIMITS_REGISTRY_PATH, errors)
    prompt_lifecycle = load_json(PROMPT_LIFECYCLE_PATH, errors)
    handover_lifecycle = load_json(HANDOVER_LIFECYCLE_PATH, errors)
    program_lifecycle = load_json(PROGRAM_LIFECYCLE_PATH, errors)

    validate_prompt_lifecycle(prompt_lifecycle, errors)
    validate_handover_lifecycle(handover_lifecycle, errors)
    validate_program_lifecycle(program_lifecycle, errors)
    validate_active_task_packets(
        errors,
        task_statuses=contract.get("task_statuses"),
        task_modes=contract.get("task_modes"),
        limits=contract.get("active_task_limits"),
    )
    validate_context_economy(errors)
    validate_current_state_hygiene(errors)

    if contract.get("repository") != EXPECTED_REPOSITORY:
        errors.append("governance repository must be Oteryn/Oteryn-Game")
    if contract.get("default_branch") != "main":
        errors.append("default branch must be main")
    if contract.get("task_prefix") != "OTV2":
        errors.append("task prefix must be OTV2")
    if contract.get("merge_method") != "squash":
        errors.append("merge method must be squash")
    if contract.get("write_allowlist") != [EXPECTED_REPOSITORY]:
        errors.append("write_allowlist must contain only Oteryn/Oteryn-Game")

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


    lock_policy = contract_lock.get("policy", {})
    if lock_policy.get("canonical_revisions_must_be_merged") is not True:
        errors.append("cross-repository contract lock must require merged canonical revisions")
    if lock_policy.get("mutable_pr_heads_are_canonical") is not False:
        errors.append("cross-repository contract lock must reject mutable PR heads as canonical")
    locked_required = lock_policy.get("required_fields_when_locked", [])
    if not isinstance(locked_required, list) or not all(isinstance(value, str) for value in locked_required):
        errors.append("cross-repository locked required fields must be a string list")
    lock_entries = contract_lock.get("contracts", [])
    if not isinstance(lock_entries, list):
        errors.append("cross-repository contracts must be a list")
        lock_entries = []
    for index, entry in enumerate(lock_entries):
        if not isinstance(entry, dict):
            errors.append(f"cross-repository contract entry {index} must be an object")
            continue
        status = entry.get("status")
        if status == "PENDING_CANONICAL_MERGE":
            for field in ("canonical_commit", "schema_revision", "schema_sha256"):
                if entry.get(field) is not None:
                    errors.append(f"pending contract entry {index} must leave {field} unset")
            pending_pr = entry.get("pending_pull_request")
            if not isinstance(pending_pr, int) or pending_pr <= 0:
                errors.append(f"pending contract entry {index} must name a positive pull request number")
            if entry.get("accepted_for_fnd02") is not False:
                errors.append(f"pending contract entry {index} cannot be accepted for FND-02")
        elif status == "LOCKED":
            for field in locked_required:
                if entry.get(field) in (None, "", []):
                    errors.append(f"locked contract entry {index} missing {field}")
            commit = entry.get("canonical_commit")
            digest = entry.get("schema_sha256")
            revision = entry.get("schema_revision")
            if not isinstance(commit, str) or re.fullmatch(r"[0-9a-f]{40}", commit) is None:
                errors.append(f"locked contract entry {index} has invalid canonical commit")
            if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
                errors.append(f"locked contract entry {index} has invalid schema digest")
            if not isinstance(revision, int) or revision <= 0:
                errors.append(f"locked contract entry {index} has invalid schema revision")
        else:
            errors.append(f"cross-repository contract entry {index} has unsupported status: {status}")

    required_limit_fields = limits_registry.get("required_entry_fields", [])
    if not isinstance(required_limit_fields, list) or not all(isinstance(value, str) for value in required_limit_fields):
        errors.append("resource-limit required fields must be a string list")
        required_limit_fields = []
    if len(required_limit_fields) != len(set(required_limit_fields)):
        errors.append("resource-limit required fields must be unique")
    limit_entries = limits_registry.get("entries", [])
    if not isinstance(limit_entries, list):
        errors.append("resource-limit entries must be a list")
        limit_entries = []
    seen_limit_ids: set[str] = set()
    for index, entry in enumerate(limit_entries):
        if not isinstance(entry, dict):
            errors.append(f"resource-limit entry {index} must be an object")
            continue
        missing = [field for field in required_limit_fields if field not in entry]
        if missing:
            errors.append(f"resource-limit entry {index} missing fields: {', '.join(missing)}")
        limit_id = entry.get("id")
        if not isinstance(limit_id, str) or not limit_id:
            errors.append(f"resource-limit entry {index} has invalid id")
        elif limit_id in seen_limit_ids:
            errors.append(f"duplicate resource-limit id: {limit_id}")
        else:
            seen_limit_ids.add(limit_id)
        if entry.get("hard_maximum") is None:
            errors.append(f"resource-limit entry {index} must define an absolute hard maximum")

    root_agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8") if (ROOT / "AGENTS.md").is_file() else ""
    override = (ROOT / "AGENTS.override.md").read_text(encoding="utf-8") if (ROOT / "AGENTS.override.md").is_file() else ""
    cross_repo = (ROOT / "docs/agents/CROSS_REPO_CONTRACTS.md").read_text(encoding="utf-8") if (ROOT / "docs/agents/CROSS_REPO_CONTRACTS.md").is_file() else ""

    mandatory_phrases = [
        "Oteryn/Oteryn-Game",
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
