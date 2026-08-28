#!/usr/bin/env python3
"""Validate the Oteryn Game agent-governance bootstrap using stdlib only."""

from __future__ import annotations

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
CODEX_REVIEW_POLICY_PATH = ROOT / "docs/agents/CODEX_REVIEW_POLICY.json"
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
        for path in prompts_dir.glob("*.md")
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


def validate_active_task_packets(errors: list[str]) -> None:
    active_dir = ROOT / "docs/agents/tasks/active"
    if not active_dir.is_dir():
        return
    terminal_statuses = {"completed", "closed", "merged", "terminal", "archived", "done"}
    for path in sorted(active_dir.glob("*.md")):
        if path.name == "README.md":
            continue
        relative = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")
        issue = re.search(r"(?m)^issue:\s*([1-9][0-9]*)\s*$", text)
        pr = re.search(r"(?m)^pr:\s*([1-9][0-9]*)\s*$", text)
        if issue is None and pr is None:
            errors.append(f"active task packet {relative} must name a positive issue or pr")
        status_match = re.search(r"(?m)^status:\s*([^\n#]+?)\s*$", text)
        if status_match is not None:
            status = status_match.group(1).strip().strip('"\'').lower()
            if status in terminal_statuses:
                errors.append(f"active task packet {relative} has terminal status {status}")


def validate_codex_review_policy(policy: dict, errors: list[str]) -> None:
    if policy.get("schema_version") != 1:
        errors.append("Codex review policy schema_version must be 1")
    if policy.get("repository") != EXPECTED_REPOSITORY:
        errors.append("Codex review policy repository mismatch")
    if policy.get("policy_id") != "OTV2-CODEX-INDEPENDENT-REVIEW-01":
        errors.append("unexpected Codex review policy id")

    authority = policy.get("authority", {})
    if not isinstance(authority, dict):
        errors.append("Codex review authority must be an object")
        authority = {}
    if not isinstance(authority.get("owner_authorized_issue"), int) or authority.get("owner_authorized_issue", 0) <= 0:
        errors.append("Codex review policy must name a positive owner-authorized issue")
    if authority.get("effective_only_after_protected_main_merge") is not True:
        errors.append("Codex standing authorization must activate only after protected-main merge")
    if authority.get("standing_authorization") is not True:
        errors.append("Codex review standing authorization must be explicit")
    if authority.get("owner_confirmation_per_covered_run") is not False:
        errors.append("covered Codex review must not require per-run owner relay")

    transport = policy.get("transport", {})
    if not isinstance(transport, dict):
        errors.append("Codex review transport must be an object")
        transport = {}
    if transport.get("canonical_surface") != "github_pull_request":
        errors.append("Codex review canonical surface must be github_pull_request")
    if transport.get("preferred_native_trigger") != "@codex review":
        errors.append("Codex review native trigger must be @codex review")
    if transport.get("capability_must_be_proven_before_trigger") is not True:
        errors.append("Codex capability must be proven before trigger")
    if transport.get("review_result_must_be_durable_on_github") is not True:
        errors.append("Codex review result must be durable on GitHub")
    if transport.get("fabricated_or_assumed_codex_execution_forbidden") is not True:
        errors.append("fabricated Codex execution must be forbidden")

    expected_authorized = {
        "READ_ONLY_EXACT_HEAD_REVIEW",
        "AUDIT",
        "NON_MUTATING_TEST_EXECUTION",
        "REPRODUCE",
        "FUZZ",
        "STATIC_ANALYSIS",
    }
    authorized = policy.get("authorized_review_operations", [])
    if not isinstance(authorized, list) or set(authorized) != expected_authorized:
        errors.append("Codex authorized review operations changed outside bounded standing scope")

    required_prohibited = {
        "IMPLEMENT_FIX",
        "TRACKED_REPOSITORY_MUTATION",
        "COMMIT",
        "PUSH",
        "MERGE",
        "AUTO_MERGE",
        "BRANCH_PROTECTION_CHANGE",
        "PRODUCTION_MUTATION",
        "PROTECTED_ENVIRONMENT_MUTATION",
        "SECRET_OR_CREDENTIAL_ACCESS",
        "LIVE_ACCOUNT_OR_SESSION_MUTATION",
        "LIVE_DATA_MUTATION",
        "EXTERNAL_REPOSITORY_WRITE",
        "SCOPE_EXPANSION",
    }
    prohibited = policy.get("prohibited_operations", [])
    if not isinstance(prohibited, list) or not required_prohibited.issubset(set(prohibited)):
        errors.append("Codex review policy is missing required prohibited operations")
    if isinstance(authorized, list) and isinstance(prohibited, list) and set(authorized) & set(prohibited):
        errors.append("Codex authorized and prohibited operations must be disjoint")

    risk = policy.get("risk_routing", {})
    if not isinstance(risk, dict):
        errors.append("Codex risk_routing must be an object")
        risk = {}
    if risk.get("precedence") != [
        "CODEX_REQUIRED",
        "CODEX_OPTIONAL",
        "CODEX_NOT_REQUIRED_BY_THIS_POLICY",
    ]:
        errors.append("Codex risk routing precedence changed")
    if risk.get("multi_tag_rule") != "HIGHEST_PRECEDENCE_VALIDATED_MATCH_WINS":
        errors.append("Codex multi-tag routing must use validated highest precedence")
    lane_tags = risk.get("lane_lead_self_tags", {})
    if not isinstance(lane_tags, dict):
        errors.append("lane_lead_self_tags must be an object")
        lane_tags = {}
    if lane_tags.get("allowed") is not True or lane_tags.get("may_only_escalate") is not True:
        errors.append("lane lead risk tags must be escalation-only")
    if lane_tags.get("may_not_reduce_required_review") is not True:
        errors.append("lane lead must not reduce required review")
    if set(lane_tags.get("cannot_by_themselves_select", [])) != {
        "CODEX_OPTIONAL",
        "CODEX_NOT_REQUIRED_BY_THIS_POLICY",
    }:
        errors.append("lane lead self-tags must not select optional/not-required routing")

    expected_downgrade_sources = {
        "OWNER_DECISION_RECORDED_IN_GOVERNING_ISSUE_OR_TASK",
        "SOL_SUPERVISING_ARCHITECT_CLASSIFICATION_RECORDED_IN_GOVERNING_ISSUE_OR_TASK",
        "CANONICAL_RISK_CONTRACT",
        "MECHANICAL_CHANGED_SCOPE_RULE",
    }
    if set(risk.get("authoritative_downgrade_sources", [])) != expected_downgrade_sources:
        errors.append("Codex authoritative downgrade sources changed")
    downgrade = risk.get("downgrade_metadata_contract", {})
    if not isinstance(downgrade, dict):
        errors.append("Codex downgrade metadata contract must be an object")
        downgrade = {}
    required_downgrade_fields = {
        "risk_classification",
        "risk_classification_source_role",
        "risk_classification_source_ref",
    }
    if set(downgrade.get("required_fields", [])) != required_downgrade_fields:
        errors.append("Codex downgrade metadata required fields changed")
    if set(downgrade.get("permitted_source_roles", [])) != {
        "OWNER",
        "SOL_SUPERVISING_ARCHITECT",
        "CANONICAL_RISK_CONTRACT",
    }:
        errors.append("Codex downgrade source roles changed")
    for field in (
        "source_must_not_be_allocated_worker",
        "source_role_must_be_proven_from_canonical_authority_not_self_declared",
        "source_record_must_be_on_protected_main_or_merged_governing_allocation",
        "source_record_must_exist_before_candidate_freeze",
    ):
        if downgrade.get(field) is not True:
            errors.append(f"Codex downgrade invariant must remain true: {field}")

    docs_rule = risk.get("mechanical_changed_scope_rules", {}).get("ORDINARY_DOCS_ONLY", {})
    if not isinstance(docs_rule, dict):
        errors.append("ordinary-docs Codex routing rule must be an object")
        docs_rule = {}
    if docs_rule.get("all_changed_paths_must_be_under") != ["docs/"]:
        errors.append("ordinary-docs rule must require all paths under docs/")
    if set(docs_rule.get("excluded_prefixes", [])) != {
        "docs/agents/",
        "docs/architecture/",
        "docs/contracts/",
    }:
        errors.append("ordinary-docs rule exclusions changed")
    if docs_rule.get("route") != "CODEX_NOT_REQUIRED_BY_THIS_POLICY":
        errors.append("ordinary-docs rule must route to not-required")
    if risk.get("low_risk_path_local_implementation", {}).get("requires_validated_downgrade_metadata_contract") is not True:
        errors.append("low-risk local implementation must require validated downgrade metadata")
    if risk.get("optional_route_requires_validated_downgrade_metadata_contract") is not True:
        errors.append("optional Codex route must require validated downgrade metadata")
    if risk.get("unvalidated_or_conflicting_classification") != "CODEX_REQUIRED":
        errors.append("unvalidated/conflicting Codex classification must fail closed to required")
    if risk.get("control_plane_must_not_invent_risk_tags") is not True:
        errors.append("control plane must not invent Codex risk tags")
    if risk.get("control_plane_may_mechanically_reject_unvalidated_downgrade") is not True:
        errors.append("control plane must reject unvalidated Codex downgrade")

    expected_required_risks = {
        "DURABLE_PERSISTENCE",
        "DURABLE_SCHEMA",
        "MIGRATION",
        "CONCURRENCY_RACE_CONTENTION",
        "AUTHENTICATION",
        "SESSION",
        "RECONNECT",
        "FENCING",
        "TRUST_BOUNDARY",
        "PROTOCOL_WIRE",
        "STABLE_IDENTITY",
        "SECURITY",
        "DURABLE_ITEM_VALUE_ECONOMY_CUSTODY",
    }
    if set(risk.get("CODEX_REQUIRED", [])) != expected_required_risks:
        errors.append("Codex required risk classes changed")
    if set(risk.get("CODEX_OPTIONAL", [])) != {
        "COMPLEX_REFACTOR",
        "BROAD_CODE_AWARENESS",
        "TEST_OR_FUZZ_INTENSIVE_CHANGE",
    }:
        errors.append("Codex optional risk classes changed")
    if set(risk.get("CODEX_NOT_REQUIRED_BY_THIS_POLICY", [])) != {
        "ORDINARY_DOCS_ONLY",
        "LOW_RISK_PATH_LOCAL_IMPLEMENTATION",
    }:
        errors.append("Codex not-required classes changed")

    independence = policy.get("independence", {})
    if not isinstance(independence, dict):
        errors.append("Codex independence must be an object")
        independence = {}
    for field in (
        "fresh_reviewer_task_required",
        "reviewer_must_not_have_materially_authored_candidate",
        "reviewer_must_not_have_materially_modified_candidate",
        "same_codex_task_that_implemented_change_is_not_independent",
        "exact_final_head_required",
        "head_change_invalidates_prior_qualification",
    ):
        if independence.get(field) is not True:
            errors.append(f"Codex independence invariant must remain true: {field}")

    loop = policy.get("lane_lead_loop", {})
    if not isinstance(loop, dict):
        errors.append("Codex lane lead loop must be an object")
        loop = {}
    for field in ("candidate_freeze_owner", "review_request_owner", "finding_repair_owner", "re_review_owner"):
        if loop.get(field) != "ALLOCATED_LANE_LEAD":
            errors.append(f"Codex lane loop ownership changed: {field}")
    if loop.get("owner_manual_relay_required") is not False:
        errors.append("owner manual relay must not be required for covered review")

    fallback = policy.get("fallback_rule", {})
    if not isinstance(fallback, dict) or fallback.get("owner_manual_prompt_relay_is_not_the_default_fallback") is not True:
        errors.append("Codex fallback must not default to owner manual relay")

    control_plane = policy.get("control_plane", {})
    if not isinstance(control_plane, dict):
        errors.append("Codex control_plane must be an object")
        control_plane = {}
    if control_plane.get("technical_discretion") != "NONE":
        errors.append("Codex control plane technical discretion must remain NONE")
    if control_plane.get("routing_method") != "MECHANICALLY_VALIDATE_RISK_INPUTS_THEN_APPLY_PRECEDENCE":
        errors.append("Codex control plane routing must remain mechanical")
    for field in (
        "must_verify_exact_head_review_evidence",
        "must_reject_unvalidated_low_risk_or_optional_downgrade",
        "must_not_adjudicate_technical_findings",
        "technical_findings_return_to_lane_lead",
    ):
        if control_plane.get(field) is not True:
            errors.append(f"Codex control-plane invariant must remain true: {field}")

    gate = policy.get("gate_semantics", {})
    if not isinstance(gate, dict):
        errors.append("Codex gate_semantics must be an object")
        gate = {}
    if gate.get("codex_review_may_satisfy_independent_technical_review") is not True:
        errors.append("Codex review qualification capability changed")
    if set(gate.get("successful_review_evidence", [])) != {
        "EXPLICIT_PASS_BOUND_TO_EXACT_HEAD",
        "NATIVE_CODEX_NO_SUGGESTIONS_SIGNAL_BOUND_TO_EXACT_HEAD_TRIGGER",
    }:
        errors.append("Codex successful review evidence changed")
    if set(gate.get("blocking_finding_severities", [])) != {"P0", "P1"}:
        errors.append("Codex blocking finding severities must remain P0/P1")
    required_gate_conditions = {
        "fresh_non_authoring_reviewer_task",
        "exact_final_head",
        "qualified_for_requested_risk",
        "durable_github_evidence",
        "successful_review_evidence_present",
        "zero_unresolved_blocking_findings_on_exact_head",
        "zero_unresolved_required_review_threads",
        "no_material_head_change_after_review",
    }
    conditions = gate.get("conditions", [])
    if not isinstance(conditions, list) or not required_gate_conditions.issubset(set(conditions)):
        errors.append("Codex gate is missing required exact-head success conditions")
    if gate.get("does_not_replace_explicit_separate_governance_audit") is not True:
        errors.append("Codex review must not replace separate governance audit")
    if gate.get("does_not_replace_required_self_review") is not True:
        errors.append("Codex review must not replace self-review")
    if gate.get("green_ci_alone_is_not_review") is not True:
        errors.append("green CI alone must not count as Codex review")


def main() -> int:
    errors: list[str] = []
    contract = load_json(CONTRACT_PATH, errors)
    lanes = load_json(LANES_PATH, errors)
    contract_lock = load_json(CONTRACT_LOCK_PATH, errors)
    limits_registry = load_json(LIMITS_REGISTRY_PATH, errors)
    prompt_lifecycle = load_json(PROMPT_LIFECYCLE_PATH, errors)
    handover_lifecycle = load_json(HANDOVER_LIFECYCLE_PATH, errors)
    codex_review_policy = load_json(CODEX_REVIEW_POLICY_PATH, errors)

    validate_prompt_lifecycle(prompt_lifecycle, errors)
    validate_handover_lifecycle(handover_lifecycle, errors)
    validate_active_task_packets(errors)
    validate_codex_review_policy(codex_review_policy, errors)

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
