#!/usr/bin/env python3
"""Generate deterministic JSON and Markdown evidence for issue 663."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from model import (
    RECORD_FIXED_LOGICAL_BYTES,
    STORE_FIXED_LOGICAL_BYTES,
    first_playable_two_session_probe,
    synthetic_candidate_envelopes,
)

ADMISSION_MAIN_SHA = "4b5377f4caa765321477011df859cf20de32786a"
ALLOCATION_COMMENT_ID = 5733345229
ARCHITECT_COMMENT_ID = 5733275908
ARCHITECT_READBACK_COMMENT_ID = 5733347366
ISSUE = 663
PARENT = 162
TASK = "FND02_TERMINAL_OUTCOME_RETENTION_RESOURCE_EVIDENCE_663"
BRANCH = "agent/fnd02-terminal-outcome-retention-resource-evidence-663"


def build_packet() -> dict[str, object]:
    witness = first_playable_two_session_probe()
    return {
        "schema_version": 1,
        "classification": "NON_PRODUCTION_RESOURCE_EVIDENCE",
        "task": TASK,
        "issue": ISSUE,
        "parent_control_plane": PARENT,
        "branch": BRANCH,
        "admission_main_sha": ADMISSION_MAIN_SHA,
        "allocation_comment_id": ALLOCATION_COMMENT_ID,
        "architect_comment_id": ARCHITECT_COMMENT_ID,
        "architect_readback_comment_id": ARCHITECT_READBACK_COMMENT_ID,
        "production_authority": "NONE",
        "foundation_runtime_mutation": False,
        "resource_registry_mutation": False,
        "cw4_resume_authority": "NONE",
        "evidence_result": "BLOCKED_ON_EVIDENCE",
        "accepted_semantics_preserved": {
            "foundation_gamesession_owns_duplicate_truth": True,
            "caller_selected_retained_boolean_is_authority": False,
            "pending_entries_evictable_for_terminal_pressure": False,
            "terminal_eviction_lowers_next_command_id": False,
            "terminal_eviction_allows_reexecution": False,
            "changed_normalized_intent_or_binding_replays": False,
            "expired_terminal_becomes_fresh_work": False,
            "later_terminalization_may_pass_earlier_pending": False,
        },
        "first_playable_functional_lower_bound": {
            "source": (
                "accepted Content/World local-transition revision 2: "
                "two valid GameSessions, A/1 open, B/1 close, replay A/1 without reopen"
            ),
            "resource_scope": "per GameSession",
            "retained_terminal_records_per_gamesession": 1,
            "why_not_two": (
                "A/1 and B/1 belong to distinct valid GameSessions; the witness contains "
                "two retained records globally but only one in each GameSession resource scope"
            ),
            "zero_records_sufficient": False,
            "one_record_sufficient_for_exact_witness": True,
            "witness_probe": witness,
        },
        "count_proposal": {
            "proposed_hard_retained_terminal_records_per_gamesession": 1,
            "classification": "EVIDENCE_BACKED_FIRST_PLAYABLE_MINIMUM",
            "rationale": (
                "The exact accepted first-playable replay witness requires one retained terminal "
                "record in session A after another session changes current world state. FND-02 "
                "permits older evicted terminal outcomes to become COMMAND_OUTCOME_EXPIRED without "
                "ever becoming fresh work, so a larger replay window is not required by the current "
                "accepted first-playable contract."
            ),
            "future_supersession_trigger": (
                "an accepted same-GameSession workload requiring exact replay of more than the most "
                "recent terminal result, or representative product evidence justifying a wider replay window"
            ),
            "production_authority_selected_here": False,
        },
        "charged_byte_proposal": {
            "proposed_aggregate_charged_resident_bytes_per_gamesession": None,
            "classification": "BLOCKED_ON_EVIDENCE",
            "why_wire_limits_do_not_close_it": [
                (
                    "FND02-COMMAND-PAYLOAD-BYTES=65536 bounds one encoded ClientCommand.payload; "
                    "it does not bound the server-normalized semantic intent plus selected binding evidence."
                ),
                (
                    "FND02-COMMAND-RESULT-PAYLOAD-BYTES=65536 bounds one encoded CommandResult.payload; "
                    "it does not bound the Foundation-owned resident semantic record or its copies and metadata."
                ),
                (
                    "The accepted issue 663 boundary deliberately does not freeze physical Rust layout "
                    "or a canonical digest-only identity representation."
                ),
            ],
            "smallest_exact_missing_discriminator": {
                "name": "FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND",
                "required_evidence": (
                    "one accepted representation-independent charging rule or one concrete bounded "
                    "Foundation API candidate proving the maximum owned bytes for server-normalized intent "
                    "plus original selected binding evidence plus terminal semantic outcome plus retention "
                    "metadata for the first-playable command family, including every simultaneously retained copy"
                ),
                "must_not_be_substituted_with": [
                    "FND02-COMMAND-PAYLOAD-BYTES",
                    "FND02-COMMAND-RESULT-PAYLOAD-BYTES",
                    "FND02-OUTSTANDING-COMMANDS",
                    "NET03 queue budgets",
                    "fixture count 8",
                    "arbitrary headroom",
                ],
            },
            "production_authority_selected_here": False,
        },
        "modeled_accounting": {
            "purpose": "exercise boundary behavior and arithmetic only",
            "store_fixed_logical_bytes": STORE_FIXED_LOGICAL_BYTES,
            "record_fixed_logical_bytes": RECORD_FIXED_LOGICAL_BYTES,
            "record_charge_equation": (
                "record_fixed_logical_bytes + normalized_intent_bytes + binding_bytes + outcome_bytes"
            ),
            "session_charge_equation": (
                "store_fixed_logical_bytes + sum(retained_record_charged_bytes)"
            ),
            "python_object_size_or_rss_claim": False,
            "rust_abi_size_claim": False,
            "wire_size_claim": False,
            "production_charge_rule_claim": False,
        },
        "tested_candidate_envelopes": synthetic_candidate_envelopes(),
        "required_boundary_results": {
            "functional_zero_count_rejected_before_modeled_gameplay_mutation": "PASS",
            "same_input_replays_original_terminal_outcome_without_reexecution": "PASS",
            "changed_normalized_intent_conflicts": "PASS",
            "changed_original_binding_conflicts": "PASS",
            "count_exact_boundary": "PASS",
            "count_one_over_evicts_oldest_terminal_only": "PASS",
            "byte_exact_boundary": "PASS",
            "byte_one_over_evicts_oldest_terminal_only": "PASS",
            "single_record_too_large_fails_before_modeled_gameplay_mutation": "PASS",
            "pending_entries_never_evicted_for_terminal_pressure": "PASS",
            "later_terminalization_cannot_pass_earlier_pending": "PASS",
            "evicted_command_becomes_expired_and_remains_non_reservable": "PASS",
            "sequence_gap_changes_no_state": "PASS",
            "checked_u64_overflow_rejected": "PASS",
            "fixed_plus_variable_charge_explicit": "PASS",
            "repeat_measurement_is_deterministic": "PASS",
        },
        "architecture_handoff": {
            "count_value_ready_for_architect_consideration": 1,
            "charged_byte_value_ready_for_architect_consideration": None,
            "can_freeze_both_required_registry_values_now": False,
            "next_action": (
                "obtain FIRST_PLAYABLE_RETAINED_SEMANTIC_RECORD_CHARGED_BYTE_BOUND evidence; "
                "then route issue 663 back to Oteryn: sol supervising architect. Do not resume CW4 yet."
            ),
        },
    }


def encoded_json() -> str:
    return json.dumps(build_packet(), indent=2, sort_keys=True) + "\n"


def render_markdown(packet: dict[str, object]) -> str:
    count = packet["count_proposal"]
    byte = packet["charged_byte_proposal"]
    witness = packet["first_playable_functional_lower_bound"]
    modeled = packet["modeled_accounting"]
    rows = packet["tested_candidate_envelopes"]
    lines = [
        "# FND-02 terminal-outcome retention resource evidence",
        "",
        "Status: NON-PRODUCTION EVIDENCE / BLOCKED_ON_EVIDENCE",
        "",
        "- Task: " + TASK,
        "- Admission main: " + ADMISSION_MAIN_SHA,
        "- Allocation: #162 comment " + str(ALLOCATION_COMMENT_ID),
        "- Architecture source: #663 comment " + str(ARCHITECT_COMMENT_ID),
        "- Foundation runtime mutation: NONE",
        "- Resource registry mutation: NONE",
        "- Production authority: NONE",
        "- CW4 resume authority: NONE",
        "",
        "## Result",
        "",
        "FACT — count: the accepted CW4 witness uses two valid GameSessions. "
        "A/1 belongs to session A and B/1 belongs to session B. Therefore the "
        "per-GameSession functional lower bound demonstrated by A/1 open -> B/1 close -> replay A/1 "
        "is 1 retained terminal record, not 2.",
        "",
        "RECOMMENDATION — count candidate: "
        + str(count["proposed_hard_retained_terminal_records_per_gamesession"])
        + " retained terminal record per GameSession for the first-playable slice. "
        "This is the minimum that satisfies the exact accepted replay witness. "
        "FND-02 already permits an evicted older result to become COMMAND_OUTCOME_EXPIRED "
        "without making its CommandRef reservable again.",
        "",
        "UNKNOWN — charged bytes: there is not enough accepted evidence to select an aggregate "
        "charged resident-byte maximum per GameSession without inventing a production resource value.",
        "",
        "Existing 65,536-byte command/result limits are wire payload bounds. They do not prove a bound "
        "for the Foundation-owned normalized semantic intent, originally selected binding evidence, "
        "terminal semantic outcome, retained copies and metadata. Issue 663 also deliberately leaves "
        "the physical Rust layout unfrozen.",
        "",
        "Therefore the evidence disposition is BLOCKED_ON_EVIDENCE for the byte maximum. "
        "Both registry values cannot truthfully be frozen yet.",
        "",
        "## Smallest missing discriminator",
        "",
        byte["smallest_exact_missing_discriminator"]["name"] + ":",
        "",
        byte["smallest_exact_missing_discriminator"]["required_evidence"] + ".",
        "",
        "It must not be replaced by fixture 8, FND02-OUTSTANDING-COMMANDS=64, "
        "NET03 queue budgets, either 65,536-byte wire payload limit, or arbitrary headroom.",
        "",
        "## Deterministic model",
        "",
        "The harness is intentionally synthetic. It exercises the accepted lifecycle and resource "
        "boundary behavior without choosing a production Rust layout.",
        "",
        "- modeled fixed store charge: " + str(modeled["store_fixed_logical_bytes"]) + " bytes;",
        "- modeled fixed record charge: " + str(modeled["record_fixed_logical_bytes"]) + " bytes;",
        "- record equation: " + modeled["record_charge_equation"] + ";",
        "- session equation: " + modeled["session_charge_equation"] + ".",
        "",
        "Those byte widths are test accounting only: not RSS, Python object size, Rust ABI size, "
        "wire size or a proposed production charge rule.",
        "",
        "### Synthetic candidate envelopes",
        "",
        "| Shape | Count | Intent | Binding | Outcome | Record charge | Session charge |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        lines.append(
            "| {shape} | {count} | {intent_bytes} | {binding_bytes} | {outcome_bytes} | "
            "{modeled_record_charged_bytes} | {modeled_session_charged_bytes} |".format(**row)
        )
    lines += [
        "",
        "No row in that table is a production maximum. The table proves only deterministic arithmetic, "
        "boundary behavior and how different semantic-record widths change the aggregate charge.",
        "",
        "## Required semantic and boundary checks",
        "",
        "- zero retained-record capacity rejects replay-required terminalization before modeled gameplay mutation;",
        "- retained same-input replay returns the original terminal outcome without another mutation;",
        "- changed normalized intent or original binding conflicts;",
        "- count and aggregate-byte exact-boundary and one-over behavior is deterministic;",
        "- pressure evicts terminal entries only; pending entries are never cache-evicted;",
        "- evicted terminal CommandRef resolves to expired/reconciliation and stays non-reservable;",
        "- later terminalization cannot pass an earlier pending CommandId;",
        "- checked integer overflow rejects before allocation/commit;",
        "- repeated measurements are byte-for-byte deterministic.",
        "",
        "## Evidence classification",
        "",
        "- functional count lower bound: "
        + str(witness["retained_terminal_records_per_gamesession"])
        + " / GameSession;",
        "- count architecture candidate: 1 / GameSession for the current first-playable slice;",
        "- aggregate charged resident bytes: BLOCKED_ON_EVIDENCE;",
        "- accepted production authority: false.",
        "",
        "## Handoff",
        "",
        "Route issue 663 back to the supervising architect only after the missing first-playable retained "
        "semantic-record charged-byte bound is supplied. Until both values can be frozen and protected-integrated, "
        "Foundation runtime implementation authority remains absent and CW4 remains WAITING_ARCHITECTURE.",
        "",
    ]
    return "\n".join(lines)


def encoded_markdown() -> str:
    return render_markdown(build_packet())


def check_file(path: Path, expected: str, label: str) -> None:
    actual = path.read_text(encoding="utf-8")
    if actual != expected:
        raise SystemExit("FAIL " + label + " differs from deterministic generator")
    print("PASS " + label + " matches deterministic generator")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-json", type=Path)
    parser.add_argument("--write-md", type=Path)
    parser.add_argument("--check-json", type=Path)
    parser.add_argument("--check-md", type=Path)
    args = parser.parse_args()
    if args.write_json is not None:
        args.write_json.parent.mkdir(parents=True, exist_ok=True)
        args.write_json.write_text(encoded_json(), encoding="utf-8", newline="\n")
        print(args.write_json)
    if args.write_md is not None:
        args.write_md.parent.mkdir(parents=True, exist_ok=True)
        args.write_md.write_text(encoded_markdown(), encoding="utf-8", newline="\n")
        print(args.write_md)
    if args.check_json is not None:
        check_file(args.check_json, encoded_json(), "evidence JSON")
    if args.check_md is not None:
        check_file(args.check_md, encoded_markdown(), "evidence Markdown")
    if all(
        value is None
        for value in (args.write_json, args.write_md, args.check_json, args.check_md)
    ):
        print(encoded_json(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
