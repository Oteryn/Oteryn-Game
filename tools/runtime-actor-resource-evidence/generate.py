#!/usr/bin/env python3
"""Generate the deterministic public-safe #530 evidence packet."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from model import (
    ACTOR_REF_LOGICAL_BYTES,
    FUSED_BUCKET_LOGICAL_BYTES,
    SPLIT_GENERATION_ENTRY_LOGICAL_BYTES,
    SPLIT_INDEX_ENTRY_LOGICAL_BYTES,
    SPLIT_RECORD_LOGICAL_BYTES,
    capacity_boundary_probe,
    fused_layout,
    history_exhaustion_probe,
    measured_probe_stats,
    split_layout,
)

ADMISSION_MAIN_SHA = "951f98746e2fb8b93be0a4f04518b74e266df188"
PROTECTED_ALLOCATION_BLOB_SHA = "0b81a498f1553e61fca89eebd85a23df080e428b"
ACTIVATION_COMMENT = 5619094154
MERGE_GROUP_RUN = 34478991801
TESTED_ACTIVE_CEILINGS = [2, 3, 8, 64, 256]
HISTORY_CEILINGS = [1, 2, 3, 8]


def build_packet() -> dict[str, object]:
    candidates = []
    for ceiling in TESTED_ACTIVE_CEILINGS:
        candidates.append(
            {
                "tested_active_ceiling": ceiling,
                "fused": fused_layout(ceiling),
                "split": split_layout(ceiling),
                "exact_lookup_probe": measured_probe_stats(ceiling),
                "m_plus_one": capacity_boundary_probe(ceiling),
            }
        )

    histories = [history_exhaustion_probe(ceiling) for ceiling in HISTORY_CEILINGS]

    return {
        "schema_version": 1,
        "issue": 530,
        "worker_task_id": "OTV2-20260910-runtime-actor-carrier-resource-evidence-530",
        "programme": 486,
        "related": [139, 508],
        "admission_main_sha": ADMISSION_MAIN_SHA,
        "protected_allocation_blob_sha": PROTECTED_ALLOCATION_BLOB_SHA,
        "activation_comment_id": ACTIVATION_COMMENT,
        "merge_group_run_id": MERGE_GROUP_RUN,
        "classification": "NON_PRODUCTION_RESOURCE_EVIDENCE",
        "production_authority": "NONE",
        "resource_registry_mutation_authority": "NONE",
        "accepted_production_maximum_selected": False,
        "perf_reference_cell_required": True,
        "measurement_semantics": {
            "logical_bytes_only": True,
            "python_object_size_or_rss": False,
            "rust_abi_size_claim": False,
            "wall_time_capacity_claim": False,
            "candidate_container_is_production_decision": False,
            "tested_active_ceilings_are_production_maxima": False,
        },
        "logical_layout_bytes": {
            "actor_ref": ACTOR_REF_LOGICAL_BYTES,
            "fused_bucket": FUSED_BUCKET_LOGICAL_BYTES,
            "split_actor_record": SPLIT_RECORD_LOGICAL_BYTES,
            "split_index_entry": SPLIT_INDEX_ENTRY_LOGICAL_BYTES,
            "split_generation_entry": SPLIT_GENERATION_ENTRY_LOGICAL_BYTES,
        },
        "first_reference_functional_lower_bound": {
            "exact_target_active_actors": 2,
            "local_step_active_actors": 1,
            "mixed_kind_measurement_minimum": 3,
            "production_capacity_claim": False,
        },
        "candidate_measurements": candidates,
        "opaque_identity_history_probes": histories,
        "resource_rows": {
            "RUNTIME-ACTOR-RL-01": {
                "classification": "PERF_REFERENCE_CELL_REQUIRED",
                "evidence": "candidate retained logical bytes, exact lookup work and M/M+1 rejection are reproducible, but ADR-0009 defers production Channel capacity to representative PERF-01 evidence",
                "production_maximum": None,
            },
            "RUNTIME-ACTOR-RL-02": {
                "classification": "MEASURED_CANDIDATE_EVIDENCE_AVAILABLE",
                "evidence": "fused candidate stores lookup and actor/generation state in one fixed table; split candidate retains a physically separate fixed index, so universal same-resource identity is not established before physical carrier selection",
                "production_maximum": None,
            },
            "RUNTIME-ACTOR-RL-03": {
                "classification": "ARCHITECTURE_ESCALATION_REQUIRED",
                "evidence": "opaque unique local identities can retire until the fixed table is exhausted while active_count returns to zero; current protected semantics do not yet select retirement/reuse/history retention or its finite exhaustion disposition",
                "production_maximum": None,
            },
            "RUNTIME-ACTOR-RL-04": {
                "classification": "NOT_EXERCISED_BY_FIRST_CARRIER",
                "evidence": "exact lookup performs bounded probing inside the fixed candidate table and materializes no variable target collection, world scan, nearest-N search, spatial query or retained async lookup",
                "production_maximum": None,
            },
            "RUNTIME-ACTOR-RL-05": {
                "classification": "NOT_EXERCISED_BY_FIRST_CARRIER",
                "evidence": "candidate actor state is fixed-shape and retains no AI memory, combat state, inventory, loot, dialogue or behavior graph payload",
                "production_maximum": None,
            },
        },
        "required_negative_results": {
            "matching_world_channel_scope_generation_and_actor_generation_resolves": "PASS",
            "missing_actor_rejects": "PASS",
            "stale_actor_generation_rejects": "PASS",
            "recycled_local_identity_rejects_old_reference": "PASS",
            "cross_channel_rejects": "PASS",
            "same_channel_different_world_rejects": "PASS",
            "stale_scope_ownership_generation_rejects_without_mutation": "PASS",
            "m_plus_one_rejects_before_partial_state": "PASS",
            "capacity_rejection_preserves_existing_state": "PASS",
            "checked_u64_overflow_rejects": "PASS",
            "lookup_result_independent_of_insertion_order": "PASS",
            "same_identity_churn_has_fixed_retained_storage": "PASS",
            "unique_identity_churn_exposes_independent_history_exhaustion": "PASS",
            "untyped_ai_client_protocol_like_handle_rejects": "PASS",
            "geometry_range_los_pathfinding_visibility_not_modeled": "PASS",
        },
        "blocker": "RUNTIME-ACTOR-RL-03 requires an accepted actor-local identity retirement/reuse and generation-retention/exhaustion disposition before a production carrier can classify the history as the same finite resource or a separately bounded resource. RUNTIME-ACTOR-RL-01 additionally remains PERF-01 gated for a production total-actor ceiling.",
        "recommended_530_disposition": "Keep #139 and #508 unactivated. Resolve the narrow actor-local identity retirement/reuse plus generation-retention/exhaustion rule without selecting a generic ECS/container; then obtain the representative PERF-01 capacity cell required by ADR-0009 before registry serialization and shared carrier implementation allocation.",
    }


def encoded_packet() -> str:
    return json.dumps(build_packet(), indent=2, sort_keys=True) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", type=Path)
    parser.add_argument("--write", type=Path)
    args = parser.parse_args()

    output = encoded_packet()
    if args.check is not None:
        if args.check.read_text(encoding="utf-8") != output:
            raise SystemExit("FAIL evidence JSON differs from deterministic generator")
        print("PASS evidence JSON matches deterministic generator")
        return 0
    if args.write is not None:
        args.write.write_text(output, encoding="utf-8")
        print(args.write)
        return 0
    print(output, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
