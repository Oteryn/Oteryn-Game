#!/usr/bin/env python3
"""Fail-closed processor for PERF-01 actor-carrier reference-cell evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import pathlib
import sys
from typing import Any

U64_MAX = (1 << 64) - 1
CONTRACT = "PERF-01-ACTOR-CARRIER-CELL-V1"
PHYSICAL_EVIDENCE_SOURCE = "PHYSICAL_REFERENCE_CELL"
PROCESSABLE_STATE = "MEASUREMENT_INCOMPLETE"
FORBIDDEN_SOURCES = {"AI01-ACTIVE-ACTORS=256", "ISSUE-537-SYNTHETIC", "CORRECTNESS_FIXTURE"}


class EvidenceError(ValueError):
    def __init__(self, code: str, detail: str):
        super().__init__(detail)
        self.code = code
        self.detail = detail


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def require_map(value: Any, name: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError("REQUIRED_INPUT_MISSING", name)
    return value


def require_fields(value: dict[str, Any], fields: tuple[str, ...], code: str) -> None:
    missing = [field for field in fields if field not in value or value[field] in (None, "", [])]
    if missing:
        raise EvidenceError(code, ",".join(missing))


IDENTITY_FIELDS = (
    "repository", "source_revision", "artifact_revision", "artifact_digest",
    "lockfile_digest", "toolchain_identity", "protocol_revision", "content_revision",
    "world_revision", "ruleset_revision", "sim_revision", "workload_revision",
    "capacity_evidence_source",
)


def is_int(value: Any) -> bool:
    """JSON integers used by the contract never accept Python booleans."""
    return type(value) is int


def require_nonempty_string(value: Any, name: str, code: str = "REQUIRED_INPUT_MISSING") -> None:
    if not isinstance(value, str) or not value:
        raise EvidenceError(code, name)


def require_bool(value: Any, name: str, code: str) -> None:
    if type(value) is not bool:
        raise EvidenceError(code, name)


def require_u64(value: Any, name: str, code: str, *, positive: bool = False) -> None:
    lower = 1 if positive else 0
    if not is_int(value) or not lower <= value <= U64_MAX:
        raise EvidenceError(code, name)


def require_cpu_set(value: Any, name: str) -> list[int]:
    if (
        not isinstance(value, list)
        or not value
        or any(not is_int(cpu) or cpu < 0 for cpu in value)
        or len(value) != len(set(value))
    ):
        raise EvidenceError("CELL_FINGERPRINT_INCOMPLETE", name)
    return value


FINGERPRINT_FIELDS = (
    "runner_group", "runner_label", "runner_name", "os_family", "cpu_vendor",
    "cpu_model", "architecture", "online_logical_cpus", "topology", "selected_cpu",
    "smt_state", "numa_topology", "frequency_policy", "governor", "turbo_state",
    "total_physical_ram_bytes", "os_distribution", "os_version", "kernel_release",
    "kernel_architecture", "effective_cpu_limit", "effective_memory_limit_bytes",
    "rlimit_as_bytes", "allowed_affinity", "page_size_bytes", "host_swap_present",
)
ENFORCEMENT_FIELDS = (
    "execution_mode", "container", "affinity_set_succeeded", "affinity_readback",
    "rlimit_as_soft_bytes", "rlimit_as_hard_bytes", "rlimit_readback_proven",
    "process_swap_observations_bytes", "exclusive_job_proven", "benchmark_instances",
    "workflow_run_id", "workflow_job_id", "native_process_proven",
    "process_start_invariants_proven", "host_background_activity_recorded", "contention_absent",
)
OBJECTIVE_FIELDS = (
    "latency_p50_us_max", "latency_p95_us_max", "latency_p99_us_max",
    "queue_age_p99_us_max", "cpu_utilization_basis_points_max", "rss_bytes_max",
    "swap_bytes_max", "memory_growth_bytes_max", "unexpected_rejections_max",
)
WORKLOAD_FIELDS = (
    "window_us", "actor_order", "lookup_per_actor", "mutation_rate_numerator",
    "mutation_rate_denominator", "reuse_rate_numerator", "reuse_rate_denominator",
    "fixed_seed", "warmup_windows", "measured_windows", "population_plan",
    "percentile_method", "minimum_samples_per_metric", "minimum_repetitions",
    "soak_duration_seconds",
    "world_id", "channel_id", "session_generation_owner", "exact_current_reference_lookup",
    "scan_or_candidate_collection", "same_slot_reuse", "generation_advance_checked",
    "stale_reference_check", "kind_balance_max_difference", "rational_accumulator",
    "rotation_covers_all_before_repeat", "remove_reuse_preserves_population",
    "excluded_payloads", "process_reset_between_repetitions",
)


def validate_fingerprint(expected: Any, observed: Any) -> str:
    expected = require_map(expected, "cell_fingerprint.expected")
    observed = require_map(observed, "cell_fingerprint.observed")
    require_fields(expected, FINGERPRINT_FIELDS, "CELL_FINGERPRINT_INCOMPLETE")
    require_fields(observed, FINGERPRINT_FIELDS, "CELL_FINGERPRINT_INCOMPLETE")
    if set(expected) != set(FINGERPRINT_FIELDS) or set(observed) != set(FINGERPRINT_FIELDS):
        raise EvidenceError("CELL_FINGERPRINT_INCOMPLETE", "fingerprint fields must be exact")
    for field, required in (
        ("runner_group", "game-runners"), ("runner_label", "oteryn-game"),
        ("runner_name", "oteryn-synology-game"), ("os_family", "Linux"),
    ):
        if expected[field] != required:
            raise EvidenceError("CONTRACT_MISMATCH", field)
    for field in (
        "cpu_vendor", "cpu_model", "architecture", "smt_state", "numa_topology",
        "frequency_policy", "governor", "turbo_state", "os_distribution", "os_version",
        "kernel_release", "kernel_architecture", "effective_cpu_limit",
    ):
        require_nonempty_string(expected[field], f"cell_fingerprint.{field}", "CELL_FINGERPRINT_INCOMPLETE")
    online = require_cpu_set(expected["online_logical_cpus"], "online_logical_cpus")
    allowed = require_cpu_set(expected["allowed_affinity"], "allowed_affinity")
    selected = expected["selected_cpu"]
    require_u64(selected, "selected_cpu", "CELL_FINGERPRINT_INCOMPLETE")
    effective_allowed = set(online).intersection(allowed)
    if selected not in online or selected not in allowed or not effective_allowed or selected != min(effective_allowed):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "selected CPU is not the lowest effective allowed CPU")
    require_map(expected["topology"], "cell_fingerprint.topology")
    for field in (
        "total_physical_ram_bytes", "effective_memory_limit_bytes", "rlimit_as_bytes",
        "page_size_bytes",
    ):
        require_u64(expected[field], field, "CELL_FINGERPRINT_INCOMPLETE", positive=True)
    if expected["rlimit_as_bytes"] > 1_073_741_824:
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "rlimit_as_bytes")
    require_bool(expected["host_swap_present"], "host_swap_present", "CELL_FINGERPRINT_INCOMPLETE")
    expected_digest = digest(expected)
    if observed != expected or digest(observed) != expected_digest:
        raise EvidenceError("CELL_FINGERPRINT_MISMATCH", "observed cell differs from frozen cell")
    return expected_digest


def validate_enforcement(value: Any, selected_cpu: int, rlimit_as_bytes: int) -> None:
    value = require_map(value, "enforcement")
    require_fields(value, ENFORCEMENT_FIELDS, "REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE")
    if set(value) != set(ENFORCEMENT_FIELDS):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "enforcement fields must be exact")
    bool_fields = (
        "affinity_set_succeeded", "rlimit_readback_proven", "exclusive_job_proven",
        "native_process_proven", "process_start_invariants_proven",
        "host_background_activity_recorded", "contention_absent",
    )
    if any(type(value[field]) is not bool for field in bool_fields):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "boolean enforcement evidence required")
    if any(not isinstance(value[field], str) or not value[field] for field in ("execution_mode", "container", "workflow_run_id", "workflow_job_id")):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "string enforcement evidence required")
    if any(not is_int(value[field]) for field in ("rlimit_as_soft_bytes", "rlimit_as_hard_bytes", "benchmark_instances")):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "integer enforcement evidence required")
    if (
        not isinstance(value["affinity_readback"], list)
        or any(not is_int(cpu) or cpu < 0 for cpu in value["affinity_readback"])
        or not isinstance(value["process_swap_observations_bytes"], list)
        or any(not is_int(item) or item < 0 for item in value["process_swap_observations_bytes"])
    ):
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "list enforcement evidence required")
    valid = (
        value["execution_mode"] == "native-release-process"
        and value["container"] == "none"
        and value["affinity_set_succeeded"] is True
        and value["affinity_readback"] == [selected_cpu]
        and 0 < value["rlimit_as_soft_bytes"] <= 1_073_741_824
        and 0 < value["rlimit_as_hard_bytes"] <= 1_073_741_824
        and value["rlimit_as_soft_bytes"] <= value["rlimit_as_hard_bytes"]
        and value["rlimit_as_soft_bytes"] == rlimit_as_bytes
        and value["rlimit_as_hard_bytes"] == rlimit_as_bytes
        and value["rlimit_readback_proven"] is True
        and value["process_swap_observations_bytes"]
        and all(x == 0 for x in value["process_swap_observations_bytes"])
        and value["exclusive_job_proven"] is True
        and value["benchmark_instances"] == 1
        and value["native_process_proven"] is True
        and value["process_start_invariants_proven"] is True
        and value["host_background_activity_recorded"] is True
        and value["contention_absent"] is True
    )
    if not valid:
        raise EvidenceError("REFERENCE_CELL_ENFORCEMENT_UNAVAILABLE", "cell boundary not proven")


def validate_contract_inputs(document: dict[str, Any]) -> None:
    identities = require_map(document.get("identities"), "identities")
    require_fields(identities, IDENTITY_FIELDS, "REQUIRED_INPUT_MISSING")
    if set(identities) != set(IDENTITY_FIELDS) or any(not isinstance(identities[field], str) or not identities[field] for field in IDENTITY_FIELDS):
        raise EvidenceError("REQUIRED_INPUT_MISSING", "identities must contain exact nonempty string fields")
    if identities["repository"] != "Oteryn/Oteryn-Game":
        raise EvidenceError("CONTRACT_MISMATCH", "repository")
    if len(identities["source_revision"]) != 40 or any(character not in "0123456789abcdef" for character in identities["source_revision"]):
        raise EvidenceError("CONTRACT_MISMATCH", "source_revision")
    for field in ("artifact_digest", "lockfile_digest"):
        value = identities[field]
        if len(value) != 71 or not value.startswith("sha256:") or any(character not in "0123456789abcdef" for character in value[7:]):
            raise EvidenceError("CONTRACT_MISMATCH", field)
    if identities["capacity_evidence_source"] != PHYSICAL_EVIDENCE_SOURCE:
        code = "NON_CAPACITY_EVIDENCE_REJECTED" if identities["capacity_evidence_source"] in FORBIDDEN_SOURCES else "CONTRACT_MISMATCH"
        raise EvidenceError(code, "capacity_evidence_source")
    objectives = require_map(document.get("objectives"), "objectives")
    require_fields(objectives, OBJECTIVE_FIELDS, "REQUIRED_INPUT_MISSING")
    if set(objectives) != set(OBJECTIVE_FIELDS) or any(not is_int(objectives[field]) for field in OBJECTIVE_FIELDS):
        raise EvidenceError("CONTRACT_MISMATCH", "objective fields and integer types")
    expected_objectives = (10_000, 25_000, 50_000, 25_000, 9_000, 858_993_459, 0, 10_737_418, 0)
    if tuple(objectives[k] for k in OBJECTIVE_FIELDS) != expected_objectives:
        raise EvidenceError("CONTRACT_MISMATCH", "objectives")
    workload = require_map(document.get("workload"), "workload")
    require_fields(workload, WORKLOAD_FIELDS, "REQUIRED_INPUT_MISSING")
    if set(workload) != set(WORKLOAD_FIELDS):
        raise EvidenceError("CONTRACT_MISMATCH", "workload fields must be exact")
    integer_workload_fields = (
        "window_us", "lookup_per_actor", "mutation_rate_numerator", "mutation_rate_denominator",
        "reuse_rate_numerator", "reuse_rate_denominator", "fixed_seed", "warmup_windows",
        "measured_windows", "minimum_samples_per_metric", "minimum_repetitions",
        "soak_duration_seconds", "kind_balance_max_difference",
    )
    boolean_workload_fields = (
        "exact_current_reference_lookup", "scan_or_candidate_collection", "same_slot_reuse",
        "generation_advance_checked", "stale_reference_check", "rational_accumulator",
        "rotation_covers_all_before_repeat", "remove_reuse_preserves_population",
        "process_reset_between_repetitions",
    )
    if any(not is_int(workload[field]) for field in integer_workload_fields):
        raise EvidenceError("CONTRACT_MISMATCH", "integer workload fields")
    if any(type(workload[field]) is not bool for field in boolean_workload_fields):
        raise EvidenceError("CONTRACT_MISMATCH", "boolean workload fields")
    for field in ("actor_order", "population_plan", "excluded_payloads"):
        if not isinstance(workload[field], list):
            raise EvidenceError("CONTRACT_MISMATCH", field)
    for field in ("percentile_method", "world_id", "channel_id", "session_generation_owner"):
        require_nonempty_string(workload[field], f"workload.{field}", "CONTRACT_MISMATCH")
    if not (
        workload["window_us"] == 50_000
        and workload["actor_order"] == ["PLAYER", "CREATURE", "NPC_SYSTEM"]
        and workload["lookup_per_actor"] == 1
        and (workload["mutation_rate_numerator"], workload["mutation_rate_denominator"]) == (1, 10)
        and (workload["reuse_rate_numerator"], workload["reuse_rate_denominator"]) == (1, 100)
        and workload["percentile_method"] == "nearest-rank"
        and workload["minimum_repetitions"] >= 5
        and workload["soak_duration_seconds"] >= 1800
        and workload["warmup_windows"] > 0
        and workload["measured_windows"] > 0
        and workload["minimum_samples_per_metric"] > 0
        and workload["world_id"] != workload["channel_id"]
        and workload["session_generation_owner"] == "single-logical-mutation-owner"
        and workload["exact_current_reference_lookup"] is True
        and workload["scan_or_candidate_collection"] is False
        and workload["same_slot_reuse"] is True
        and workload["generation_advance_checked"] is True
        and workload["stale_reference_check"] is True
        and workload["kind_balance_max_difference"] == 1
        and workload["rational_accumulator"] is True
        and workload["rotation_covers_all_before_repeat"] is True
        and workload["remove_reuse_preserves_population"] is True
        and workload["excluded_payloads"] == ["VARIABLE_BEHAVIOR", "COMBAT", "AI", "INVENTORY", "PATHFINDING", "LOOT", "NETWORK", "PERSISTENCE"]
        and workload["process_reset_between_repetitions"] is True
    ):
        raise EvidenceError("CONTRACT_MISMATCH", "workload")


def nearest_rank(samples: Any, percentile: int, minimum: int) -> int:
    if not isinstance(samples, list) or len(samples) < minimum or any(type(x) is not int or x < 0 for x in samples):
        raise EvidenceError("SAMPLE_SET_INCOMPLETE", f"p{percentile}")
    ordered = sorted(samples)
    rank = (percentile * len(ordered) + 99) // 100
    return ordered[rank - 1]


def disposition(repetition: dict[str, Any], objectives: dict[str, int], minimum: int) -> tuple[bool, list[str], dict[str, int]]:
    repetition = require_map(repetition, "repetition")
    require_fields(repetition, ("latency_us", "queue_age_us", "cpu_utilization_basis_points", "rss_bytes", "peak_rss_bytes", "swap_observations_bytes", "memory_growth_bytes", "unexpected_rejections", "attempted_operations", "completed_operations", "stale_references_rejected", "missed_windows"), "SAMPLE_SET_INCOMPLETE")
    scalar_fields = ("cpu_utilization_basis_points", "rss_bytes", "peak_rss_bytes", "memory_growth_bytes", "unexpected_rejections", "attempted_operations", "completed_operations", "missed_windows")
    if any(type(repetition[field]) is not int or repetition[field] < 0 for field in scalar_fields):
        raise EvidenceError("SAMPLE_SET_INCOMPLETE", "nonnegative integer observations required")
    if not isinstance(repetition["swap_observations_bytes"], list) or not repetition["swap_observations_bytes"] or any(type(x) is not int or x < 0 for x in repetition["swap_observations_bytes"]):
        raise EvidenceError("SAMPLE_SET_INCOMPLETE", "swap observations")
    if type(repetition["stale_references_rejected"]) is not bool:
        raise EvidenceError("SAMPLE_SET_INCOMPLETE", "stale-reference disposition")
    summary = {
        "latency_p50_us": nearest_rank(repetition["latency_us"], 50, minimum),
        "latency_p95_us": nearest_rank(repetition["latency_us"], 95, minimum),
        "latency_p99_us": nearest_rank(repetition["latency_us"], 99, minimum),
        "queue_age_p99_us": nearest_rank(repetition["queue_age_us"], 99, minimum),
    }
    failures = []
    checks = (
        (summary["latency_p50_us"] <= objectives["latency_p50_us_max"], "latency_p50"),
        (summary["latency_p95_us"] <= objectives["latency_p95_us_max"], "latency_p95"),
        (summary["latency_p99_us"] <= objectives["latency_p99_us_max"], "latency_p99"),
        (summary["queue_age_p99_us"] <= objectives["queue_age_p99_us_max"], "queue_age_p99"),
        (repetition["cpu_utilization_basis_points"] <= objectives["cpu_utilization_basis_points_max"], "cpu"),
        (repetition["rss_bytes"] <= objectives["rss_bytes_max"] and repetition["peak_rss_bytes"] <= objectives["rss_bytes_max"], "rss"),
        (repetition["swap_observations_bytes"] and all(x == objectives["swap_bytes_max"] for x in repetition["swap_observations_bytes"]), "swap"),
        (repetition["memory_growth_bytes"] <= objectives["memory_growth_bytes_max"], "memory_growth"),
        (repetition["unexpected_rejections"] <= objectives["unexpected_rejections_max"], "unexpected_rejections"),
        (repetition["attempted_operations"] == repetition["completed_operations"], "incomplete_operations"),
        (repetition["stale_references_rejected"] is True, "stale_reference"),
        (repetition["missed_windows"] == 0, "missed_windows"),
    )
    failures.extend(name for passed, name in checks if not passed)
    return not failures, failures, summary


def checked_candidate(saturation: int, passing: int, lower_limits: list[int]) -> int:
    values = [saturation, passing, *lower_limits]
    if any(type(x) is not int or x < 0 or x > U64_MAX for x in values) or saturation > U64_MAX // 7:
        raise EvidenceError("CHECKED_INTEGER_OVERFLOW", "candidate operands")
    return min((saturation * 7) // 10, passing, *lower_limits)


def validate_boundary(boundary: Any, candidate: int) -> None:
    boundary = require_map(boundary, "boundary")
    require_fields(boundary, ("configured_maximum", "occupancy_m_admitted", "m_plus_one_rejected", "rejected_before_mutation", "no_eviction_or_recycle", "generation_advanced", "stale_references_rejected", "state_unchanged_after_rejection", "overflow_rejected_before_allocation"), "BOUNDARY_EVIDENCE_INCOMPLETE")
    if candidate == U64_MAX:
        raise EvidenceError("CHECKED_INTEGER_OVERFLOW", "M + 1")
    valid = boundary["configured_maximum"] == candidate and all(boundary[k] is True for k in (
        "occupancy_m_admitted", "m_plus_one_rejected", "rejected_before_mutation", "no_eviction_or_recycle",
        "generation_advanced", "stale_references_rejected", "state_unchanged_after_rejection", "overflow_rejected_before_allocation"))
    if not valid:
        raise EvidenceError("BOUNDARY_EVIDENCE_INCOMPLETE", "exact M/M+1 atomic boundary not proven")


def process(document: Any) -> dict[str, Any]:
    document = require_map(document, "document")
    missing_top_level = [field for field in ("schema_version", "contract", "state", "candidate_M", "perf_01_status") if field not in document]
    if missing_top_level:
        raise EvidenceError("REQUIRED_INPUT_MISSING", ",".join(missing_top_level))
    if document["schema_version"] != 1 or not is_int(document["schema_version"]):
        raise EvidenceError("CONTRACT_MISMATCH", "schema_version")
    if not isinstance(document["contract"], str) or not isinstance(document["perf_01_status"], str) or document["contract"] != CONTRACT or document["perf_01_status"] != "OPEN":
        raise EvidenceError("CONTRACT_MISMATCH", "contract or perf_01_status")
    if not isinstance(document["state"], str) or document["state"] not in {"PLACEHOLDER_UNBOUND", PROCESSABLE_STATE}:
        raise EvidenceError("INPUT_STATE_FORBIDDEN", "only unbound placeholder or incomplete measurement input is processable")
    if document["candidate_M"] is not None:
        code = "PLACEHOLDER_CAPACITY_FORBIDDEN" if document["state"] == "PLACEHOLDER_UNBOUND" else "PREPOPULATED_CAPACITY_FORBIDDEN"
        raise EvidenceError(code, "candidate_M")
    if document["state"] == "PLACEHOLDER_UNBOUND":
        if set(document) != {"schema_version", "contract", "state", "candidate_M", "perf_01_status"}:
            raise EvidenceError("MALFORMED_PLACEHOLDER", "placeholder fields must be exact")
        return {"schema_version": 1, "contract": CONTRACT, "state": "PLACEHOLDER_UNBOUND", "candidate_M": None, "perf_01_status": "OPEN"}
    validate_contract_inputs(document)
    fp = require_map(document.get("cell_fingerprint"), "cell_fingerprint")
    if set(fp) != {"expected", "observed"}:
        raise EvidenceError("CELL_FINGERPRINT_INCOMPLETE", "cell_fingerprint fields must be exact")
    fp_digest = validate_fingerprint(fp.get("expected"), fp.get("observed"))
    validate_enforcement(document.get("enforcement"), fp["expected"]["selected_cpu"], fp["expected"]["rlimit_as_bytes"])
    identities = document["identities"]
    baseline = require_map(document.get("comparison_baseline_identities"), "comparison_baseline_identities")
    require_fields(baseline, IDENTITY_FIELDS, "REQUIRED_INPUT_MISSING")
    if set(baseline) != set(IDENTITY_FIELDS) or any(not isinstance(baseline[field], str) or not baseline[field] for field in IDENTITY_FIELDS):
        raise EvidenceError("REQUIRED_INPUT_MISSING", "comparison identities must contain exact nonempty string fields")
    if baseline["capacity_evidence_source"] != PHYSICAL_EVIDENCE_SOURCE:
        code = "NON_CAPACITY_EVIDENCE_REJECTED" if baseline["capacity_evidence_source"] in FORBIDDEN_SOURCES else "CONTRACT_MISMATCH"
        raise EvidenceError(code, "comparison capacity_evidence_source")
    if any(identities[k] != baseline[k] for k in IDENTITY_FIELDS):
        raise EvidenceError("EVIDENCE_IDENTITY_MISMATCH", "source/artifact/toolchain/content/ruleset/workload drift")

    workload, objectives = document["workload"], document["objectives"]
    populations = document.get("populations")
    if not isinstance(populations, list) or not populations:
        raise EvidenceError("MEASUREMENT_INCOMPLETE", "populations")
    plan = workload["population_plan"]
    if not isinstance(plan, list) or len(plan) < 2 or any(not is_int(population) or population <= 0 or population > U64_MAX for population in plan):
        raise EvidenceError("PROGRESSIVE_EVIDENCE_INCOMPLETE", "positive u64 populations required")
    if any(current >= following for current, following in zip(plan, plan[1:])):
        raise EvidenceError("PROGRESSIVE_EVIDENCE_INCOMPLETE", "population plan must be strictly increasing")
    if any(not isinstance(population, dict) for population in populations):
        raise EvidenceError("MEASUREMENT_INCOMPLETE", "population entries must be objects")
    if any(not is_int(population.get("population")) for population in populations):
        raise EvidenceError("PROGRESSIVE_EVIDENCE_INCOMPLETE", "population values must be integers")
    if [p.get("population") for p in populations] != plan[:len(populations)] or len(populations) != len(plan):
        raise EvidenceError("PROGRESSIVE_EVIDENCE_INCOMPLETE", "population plan")
    outcomes, summaries = [], []
    for population in populations:
        repetitions = population.get("repetitions")
        if not isinstance(repetitions, list) or len(repetitions) < workload["minimum_repetitions"]:
            raise EvidenceError("MEASUREMENT_INCOMPLETE", "minimum repetitions")
        results = [disposition(r, objectives, workload["minimum_samples_per_metric"]) for r in repetitions]
        passes = [result[0] for result in results]
        failures = [set(result[1]) for result in results]
        if all(passes):
            outcome, first_failure = "PASS", None
        elif not any(passes) and len(set.intersection(*failures)) > 0:
            outcome, first_failure = "SATURATION", sorted(set.intersection(*failures))[0]
        else:
            outcome, first_failure = "INCONCLUSIVE", None
        outcomes.append(outcome)
        summaries.append({"population": population["population"], "outcome": outcome, "first_violated_objective": first_failure, "repetitions": [r[2] for r in results]})
    if "INCONCLUSIVE" in outcomes:
        raise EvidenceError("POPULATION_INCONCLUSIVE", "mixed or non-reproducible result")
    saturation_indices = [i for i, x in enumerate(outcomes) if x == "SATURATION"]
    if not saturation_indices or any(x != "PASS" for x in outcomes[:saturation_indices[0]]):
        raise EvidenceError("SATURATION_NOT_REPRODUCIBLE", "first failing population")
    saturation_index = saturation_indices[0]
    if any(x == "SATURATION" for x in outcomes[:saturation_index]):
        raise EvidenceError("SATURATION_NOT_REPRODUCIBLE", "not first failure")
    passing_indices = [i for i in range(saturation_index) if outcomes[i] == "PASS"]
    if not passing_indices:
        raise EvidenceError("QUALIFIED_PASS_MISSING", "no lower passing population")
    passing_index = passing_indices[-1]
    soak = require_map(document.get("soak"), "soak")
    if not is_int(soak.get("population")) or not is_int(soak.get("duration_seconds")):
        raise EvidenceError("SOAK_INCOMPLETE", "integer population and duration required")
    if soak["population"] != populations[passing_index]["population"] or soak["duration_seconds"] < workload["soak_duration_seconds"]:
        raise EvidenceError("SOAK_INCOMPLETE", "highest passing population or duration")
    soak_pass, soak_failures, soak_summary = disposition(soak.get("measurement", {}), objectives, workload["minimum_samples_per_metric"])
    if not soak_pass:
        raise EvidenceError("SOAK_FAILED", ",".join(soak_failures))
    lower_limits = document.get("lower_coupled_limits")
    if not isinstance(lower_limits, list) or not lower_limits or any(not is_int(limit) or limit < 0 or limit > U64_MAX for limit in lower_limits):
        raise EvidenceError("REQUIRED_INPUT_MISSING", "lower_coupled_limits")
    candidate = checked_candidate(populations[saturation_index]["population"], populations[passing_index]["population"], lower_limits)
    validate_boundary(document.get("boundary"), candidate)
    return {
        "schema_version": 1, "contract": CONTRACT, "state": "MEASUREMENT_COMPLETE_NOT_ACCEPTED",
        "authority": "PROVISIONAL_EVIDENCE_ONLY", "candidate_M": candidate,
        "saturation_S": populations[saturation_index]["population"],
        "qualified_passing_P": populations[passing_index]["population"],
        "cell_fingerprint_digest": fp_digest, "population_summaries": summaries,
        "soak_summary": soak_summary, "perf_01_status": "OPEN",
        "production_capacity": None, "rl_01_status": "PERF_REFERENCE_CELL_REQUIRED",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        result = process(json.loads(args.input.read_text(encoding="utf-8")))
        rendered = canonical_bytes(result)
        if args.check:
            if args.output is None or not args.output.exists() or args.output.read_bytes() != rendered:
                raise EvidenceError("BYTE_STABILITY_CHECK_FAILED", "output differs")
        elif args.output:
            args.output.write_bytes(rendered)
        else:
            sys.stdout.buffer.write(rendered)
        return 0
    except (EvidenceError, json.JSONDecodeError, OSError) as error:
        code = error.code if isinstance(error, EvidenceError) else "INVALID_INPUT"
        sys.stderr.buffer.write(canonical_bytes({"error": code, "detail": str(error)}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
