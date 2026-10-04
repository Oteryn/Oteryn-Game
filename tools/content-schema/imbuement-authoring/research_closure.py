"""Validate bounded public facts separately from unknown Global/runtime details."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

from behavior_answers import QUESTION_IDS, TARGET
import capture_bounds

PACKET = Path(__file__).parent / "samples/global-research-closure.json"

QUALIFIED_COMPLETION_VALUES = {'wand_rod_strike_current_primary_threshold_and_exceptions': {'scope': 'OFFICIAL_POST_RELEASE_WAND_ROD_STRIKE_RULE',
                                                              'threshold_reference_item': 'Dream '
                                                                                          'Blossom '
                                                                                          'Staff',
                                                              'up_to_reference_level_allowed_tiers': ['Basic',
                                                                                                      'Intricate'],
                                                              'above_reference_level_tier_limitation_removed': True,
                                                              'named_exceptions': ['Deepling Fork',
                                                                                   'Cobra Wand',
                                                                                   'Lion Rod',
                                                                                   'Rending '
                                                                                   'Inferniarch '
                                                                                   'Wand',
                                                                                   'Rending '
                                                                                   'Inferniarch '
                                                                                   'Rod'],
                                                              'exception_exact_allowed_types_not_enumerated_by_primary': True,
                                                              'all_items_unrestricted': False},
 'physical_armor_stage_current_reported_test_reference': {'scope': 'NAMED_REPORTED_GAZER_SPECTRE_AND_SPIKE_TRAP_TEST_CONTEXTS_WITH_PLAYER_TO_CREATURE_CALCULATOR_MODEL',
                                                          'physical_stage': 'PERCENTAGE_RESISTANCE_BEFORE_FLAT_ARMOR',
                                                          'universal_all_attack_types_order': None,
                                                          'integer_hit_rounding': None,
                                                          'equipment_slot_iteration': None,
                                                          'full_Wheel_proficiency_pipeline': None,
                                                          'test_date': None},
 'life_leech_overkill_reported_damage_basis_2024': {'scope': 'ONLY_LIFE_LEECH_AUTHOR_REPORTED_JANUARY_2024_DAMAGE_BASIS',
                                                    'overkill_damage_counts': True,
                                                    'damage_basis': 'WOULD_DEAL_ATTACK_DAMAGE_BEFORE_TARGET_REMAINING_HP_CAP',
                                                    'current_target_continuity': None,
                                                    'integer_rounding': None,
                                                    'unequal_target_formula': None,
                                                    'zero_damage_target_count': None,
                                                    'modifier_stage_order': None},
 'vibrancy_successful_pvp_retrigger_net_result': {'reference_scope': 'ONE_ALREADY_PARALYZED_PVP_RETRIGGER_INTERACTION',
                                                  'tier': 'Powerful',
                                                  'paralysis_present_before': True,
                                                  'event': 'ADDITIONAL_PVP_PARALYSIS_ATTACK',
                                                  'recovery_chance_bps': 5000,
                                                  'paralysis_present_after_success': False,
                                                  'same_triggering_attack_leaves_reapplication_active': False}}


def validate(packet: dict) -> None:
    def require(condition, message):
        if not condition:
            raise ValueError(message)

    require(packet["schema"] == "OTERYN_IMBUEMENT_GLOBAL_RESEARCH_CLOSURE/v1",
            "unexpected closure schema")
    require(packet["target"] == TARGET and packet["activation"] == "DRAFT_NOT_RUNTIME_READY",
            "closure cannot activate runtime or change the data target")
    require(packet["full_global_parity_proven"] is False, "bounded research cannot certify full Global parity")
    require(type(packet["research_limits"]["observations_performed"]) is int
            and packet["research_limits"]["observations_performed"] == 0,
            "this batch performed no gameplay observations")
    capture_bounds.validate(packet, "global-research-closure.json")
    sources = packet["sources"]
    for source in sources.values():
        url = urlsplit(source["url"])
        require(url.scheme == "https" and bool(url.netloc), "source needs a public HTTPS URL")
        require(source["method"] in {"REMOTE_DESKTOP_CHROME_CDP", "NORMAL_PUBLIC_HTTP_OR_RETAINED_EXTRACT"},
                "unqualified source access method")
        require("OTS" not in source["source_role"], "OTS code cannot certify a public Global fact")
        require(source["captured_text"] and source["captured_text_scope"], "missing capture scope/body")
        require(hashlib.sha256(source["captured_text"].encode()).hexdigest() == source["captured_text_sha256"],
                "captured public text digest mismatch")
        require(source["original_digest"] and source["original_digest_scope"], "missing original capture identity")
        require(re.fullmatch(r"[0-9a-f]{64}", source["original_digest"]) is not None,
                "malformed original source digest")
        require(source["capture_date"] == "2026-10-01" and isinstance(source["publication_dates"], dict),
                "capture date must remain separate from publication dates")
    public_video = packet["research_limits"]["public_video"]
    recordings = packet["research_limits"].get("public_gameplay_recordings_read", 0)
    require(type(recordings) is int and recordings in {0, 1}, "unqualified public recording count")
    require(public_video["gameplay_observed"] is (recordings == 1), "public video observation mismatch")
    if recordings:
        video = sources.get("minerva_public_scroll_use_2025", {})
        require(video.get("source_role") == "DATED_PUBLIC_GAMEPLAY_RECORDING",
                "public gameplay needs a dated recording identity")
        frames = video.get("frames", [])
        require(len(frames) >= 4 and frames == public_video.get("frame_refs"),
                "public gameplay requires matching before/after frame receipts")
        require(all(type(f["timestamp_seconds"]) in {int, float}
                    and f["timestamp_seconds"] >= 0
                    and re.fullmatch(r"[0-9a-f]{64}", f["sha256"])
                    and f["visible_scope"] for f in frames), "malformed public frame identity")
    disposition = packet.get("equipment_source_disposition")
    if disposition:
        counts = disposition["counts"]
        require(disposition["status"] == "ALL_DISPUTED_PROFILES_SELECTED_AND_POPULATED"
                and counts["historical_helper_direct_table_disagreements"] == 101
                and counts["disputed_items_with_populated_selected_profiles"] == 101
                and counts["disputed_items_with_no_selected_profile"] == 0,
                "equipment history cannot become missing selected profiles")
    for conflict in packet["source_conflicts"]:
        require(all(sid in sources for sid in conflict["source_refs"]), "conflict references a missing source")
        for field in ("quote", "literal_quote"):
            if field in conflict:
                require(conflict[field] and any(conflict[field] in sources[sid]["captured_text"]
                                               for sid in conflict["source_refs"]),
                        "conflict quote missing from its attributed source")
        for claim in conflict.get("claims", []):
            require(claim["quote"] in sources[claim["source"]]["captured_text"], "conflict quote missing from source")
    groups = packet["groups"]
    ids = [g["id"] for g in groups]
    require(len(ids) == 11 and set(ids) == QUESTION_IDS - {"exact_target_snapshot"},
            "closure must account for each original behavioral question exactly once")
    facts = packet["facts"]
    fact_ids = [f["id"] for f in facts]
    indexed = {f["id"]: f for f in facts}
    for fid, expected in QUALIFIED_COMPLETION_VALUES.items():
        require(fid in indexed and indexed[fid]["value"] == expected, "qualified completion scope/value changed")
    matrix = packet["equipment_source_disposition"]["current_source_choice_matrix"]
    eligible = json.loads((PACKET.parent / "imbuement-eligibility.json").read_text())
    disputed = {row["client_id"]: row for row in eligible["items"] if row["evidence_status"] == "DERIVED_SELECTED_OVER_STALE_HELPER"}
    require(len(matrix["items"]) == 101 and {row["client_id"] for row in matrix["items"]} == set(disputed), "equipment disposition matrix coverage changed")
    for row in matrix["items"]:
        selected = disputed[row["client_id"]]
        require(row["selected_source"] == selected["selected_eligibility_source"]
                and row["selected_allowed_types"] == selected["allowed_types"]
                and row["canonical_ref"] == selected["item_ref"]
                and row["source_history_difference_retained"] is True,
                "equipment disposition matrix selection changed")
    require(sources["official_mirade_strike_39592694"]["source_role"]
            == "OFFICIAL_PUBLIC_COMMUNITY_MANAGER_STATEMENT", "primary Strike attribution changed")
    require(len(set(fact_ids)) == len(fact_ids), "duplicate public fact")
    for fact in facts:
        require(fact["group"] in ids and fact["value"] is not None, "fact needs a bounded selected value")
        require(fact["qualification"] and fact["limits"] and fact["quote_refs"], "fact lacks evidence limits")
        for ref in fact["quote_refs"]:
            require(ref["source_id"] in sources, "fact references a missing source")
            require(ref["text"] and ref["text"] in sources[ref["source_id"]]["captured_text"],
                    "quote is not present in the captured source text")
    for group in groups:
        require(group["status"] == "BOUNDED_PUBLIC_FACTS_AND_EXPLICIT_RESIDUALS",
                "group cannot silently become fully Global-confirmed")
        expected = {f["id"] for f in facts if f["group"] == group["id"]}
        require(set(group["qualified_fact_refs"]) == expected, "group omits or misassigns public facts")
        require(group["known_scope"] and group["sufficient_public_evidence"], "missing scoped acceptance")
        require(group["runtime_contract"]["owner"] == "ARCHITECT_COORDINATOR_162",
                "runtime contract needs its owning handoff")
        for field in group["remaining_public_fields"]:
            require(field["value"] is None and field["status"] == "UNCONFIRMED_OR_SOURCE_CONFLICT"
                    and field["description"], "residual fields cannot acquire invented Global values")
        for ref in group["existing_public_source_refs"]:
            catalogue = json.loads((PACKET.parent / ref["catalogue"]).read_text())
            registry = catalogue["sources"]
            source_ids = set(registry) if isinstance(registry, dict) else {s["id"] for s in registry}
            require(ref["id"] in source_ids, "retained source reference is missing from its catalogue")
    require(packet["counts"] == {"groups": len(groups), "facts": len(facts), "sources": len(sources)},
            "closure inventory counts do not match")


if __name__ == "__main__":
    packet = json.loads(PACKET.read_text())
    validate(packet)
    print(json.dumps({"status": "PASS", **packet["counts"], "full_global_parity_proven": False}))
