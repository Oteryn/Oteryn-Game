#!/usr/bin/env python3
"""Build and validate an offline imbuement authoring candidate; never activate it."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

import capture_bounds

HERE = Path(__file__).resolve().parent
FACTS = HERE / "samples/imbuement-sources-2026-10-01.json"
FACTS_SHA256 = "7ee2221886abdc80d27ce5cd13e58bdecab8b3c5210a7e0e7a36efc52771070e"
CATALOGUE = HERE / "samples/imbuements-candidate.json"
REPORT = HERE / "samples/imbuement-source-comparison.json"
SCHEMA = HERE / "imbuement.schema.json"
EVIDENCE_PINS = {
    "owner-authoring-policy.json": "c71b9351980fe70b8a83aafcfcca3e03a1ff9445427fedd842214ee89ae73079",
    "global-research-closure.json": "0ccab4e6907570413f74e5c5a05d3a390900af4b92b7516ea7dcdbd15c73050a",
    "imbuement-bindings.json": "1b7640df513afe5cf79297c1701282e288c28dfc6587de19cbf2655fa980bfea",
    "imbuement-access.json": "c05984cb8f0ac0e41b4f8bebd7bc2c93be24ea3cbbb86c0d95859e0a8a917f2d",
    "imbuement-eligibility.json": "712a1181da5db3494144fe1fce92e33478dd13cd624ea6ed46b63cef57eb40a2",
    "global-rules-evidence.json": "5f04de7e15226dfa57b36cd9d6f6d03c847f020bbc0efebe118bde02742e3413",
    "imbuement-combat.json": "5f14f4d9748e956fe36d779dbebcd6daeb90ff27ff0eb6abadbe7237cc7d57e7",
    "crystal-imbuements-evidence.json": "2a5723725bd1046e36dc54453ec836810f2b1e972a05d4886eeebeafefbe45fe",
    "missing-item-definitions.json": "0b90ec4d5afe9a8aca3f4838dc0916ba06755a315b745d4dd68f954f91569c30",
    "missing-item-source-facts.json": "48441ebbf409780739a318808dc292cec2b8b0225437d1a92aacd614604e6461",
    "current-behavior-answers.json": "1e0d89d2b40d5db0aadb91769d5178c51c6c0475129b8348e7da575ed751f91a",
    "global-observation-plan.json": "a4913ee8dc03ba6fd80e620a44fde56a3610e73b19912247e56a5e3d53d81afb",
}
# Independently anchor reviewed capture requirements. Mutable scenario fields
# cannot redefine the evidence necessary to qualify their own claims.
CAPTURE_GROUPS_SHA256 = "b18a064a812601e7791aa3f86e4b33b747fe3d48e1da50cc210e44da8720fda2"
MINIMUM_CAPTURE_GROUPS = {
    "etcher_consumption": {"item_state", "inventory_resources"},
    "scroll_application_equipped_target": {"item_state", "inventory_resources"},
    "fine_grained_timers": {"item_state", "timer_context"},
    "combat_pipeline": {"item_state", "combat_context"},
    "vibrancy_pvp_gate": {"item_state", "pvp_context"},
    "transaction_payment_sources": {"item_state", "inventory_resources", "bank_stash"},
    "failed_transaction_consumption_and_rollback": {"item_state", "inventory_resources", "bank_stash"},
    "native_effect_composition": {"item_state", "combat_context", "native_stats"},
    "transfer_preserves_imbuement_state_and_remaining_duration": {"item_state", "timer_context", "transfer_context"},
    "etcher_npc_purchase_requires_premium": {"item_state", "inventory_resources"},
    "scroll_consumption": {"item_state", "inventory_resources"},
}


def validate_observation_plan(plan, packets):
    ledger = packets["global-rules-evidence.json"]
    if (plan["schema"] != "OTERYN_IMBUEMENT_GLOBAL_OBSERVATION_PLAN/v1"
            or plan["activation"] != "DRAFT_NOT_RUNTIME_READY"
            or plan["target"] != ledger["target"]
            or plan["status"] != "PUBLIC_EVIDENCE_NOT_SUFFICIENT"):
        raise ValueError("observation plan cannot certify or activate unobserved behavior")
    if (plan.get("acceptance_profile") != "global-research-closure.json"
            or plan.get("evidence_alternatives") != ["PUBLIC_REFERENCE_LOOKUP", "DATED_PUBLIC_RECORDING_OR_LOG"]):
        raise ValueError("planned captures cannot be the sole acceptance route for public facts")
    catalogues = {"global-rules-evidence.json", "imbuement-combat.json"}
    if set(plan["source_catalogues"]) != catalogues:
        raise ValueError("observation plan references unsupported catalogues")
    requirements = plan["requirements"]
    ids = [r["id"] for r in requirements]
    if len(ids) != len(set(ids)) or set(ids) != {r["id"] for r in ledger["unresolved"]}:
        raise ValueError("observation requirements differ from the complete unresolved ledger")
    source_ids = {}
    rule_ids = {}
    for name in catalogues:
        sources = packets[name]["sources"]
        source_ids[name] = set(sources) if isinstance(sources, dict) else {s["id"] for s in sources}
        rule_ids[name] = {r["id"] for r in packets[name]["rules"]}
    scenario_ids = []
    groups = plan["capture_field_groups"]
    group_digest = hashlib.sha256(json.dumps(groups, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    if group_digest != CAPTURE_GROUPS_SHA256 or set(ids) != set(MINIMUM_CAPTURE_GROUPS):
        raise ValueError("observation capture requirements differ from independently reviewed minima")
    for requirement in requirements:
        if (requirement["status"] != "PUBLIC_EVIDENCE_NOT_SUFFICIENT"
                or requirement["ledger_ref"] != {"catalogue": "global-rules-evidence.json",
                                                  "section": "unresolved", "id": requirement["id"]}
                or requirement.get("engine_answer_profile") != "current-behavior-answers.json#" + requirement["id"]
                or requirement.get("public_reference_method") != "PUBLIC_REFERENCE_LOOKUP"
                or requirement.get("capture_requirement_scope") != "SUPPLEMENTAL_PLANNED_CAPTURE_ONLY"
                or requirement.get("research_closure_ref") != "global-research-closure.json#" + requirement["id"]
                or not requirement["source_and_capture_limitations"]
                or not requirement["source_refs"] or not requirement["scenarios"]):
            raise ValueError("observation requirement lacks its qualified ledger context")
        for field, known in (("source_refs", source_ids), ("related_rule_refs", rule_ids)):
            for ref in requirement[field]:
                if ref["catalogue"] not in known or ref["id"] not in known[ref["catalogue"]]:
                    raise ValueError("observation plan has an unresolved evidence reference")
        for scenario in requirement["scenarios"]:
            scenario_ids.append(scenario["id"])
            required_groups = MINIMUM_CAPTURE_GROUPS[requirement["id"]] | {"public_context"}
            if not required_groups <= set(scenario["capture_group_refs"]):
                raise ValueError("observation scenario omits the minimum evidence context")
            if (scenario["status"] != "PLANNED_NOT_OBSERVED"
                    or scenario["method"] != "DATED_PUBLIC_RECORDING_OR_LOG"
                    or scenario["expected_global_result"] is not None
                    or not scenario["procedures"] or not scenario["visible_claim_only"]
                    or not scenario["capture_fields"] or not scenario["capture_group_refs"]):
                raise ValueError("planned scenarios cannot select expected Global outcomes")
            for group in scenario["capture_group_refs"]:
                if group not in groups or not groups[group] or not set(groups[group]) <= set(scenario["capture_fields"]):
                    raise ValueError("observation scenario omits required capture fields")
    if len(set(scenario_ids)) != len(scenario_ids) or any(not sid for sid in scenario_ids):
        raise ValueError("observation scenario identities must be unique and nonempty")
    if (any(type(v) is not int for v in plan["counts"].values())
            or plan["counts"] != {"planned_requirements": len(requirements),
                          "planned_scenarios": len(scenario_ids),
                          "observations_collected_by_this_plan": 0}):
        raise ValueError("planned scenario counts cannot masquerade as observed evidence")


def supporting():
    expected = {"imbuement-bindings.json", "imbuement-access.json",
                "imbuement-eligibility.json", "global-rules-evidence.json",
                "imbuement-combat.json", "crystal-imbuements-evidence.json",
                "missing-item-definitions.json", "missing-item-source-facts.json",
                "global-observation-plan.json", "current-behavior-answers.json", "global-research-closure.json", "owner-authoring-policy.json"}
    if set(EVIDENCE_PINS) != expected:
        raise ValueError("supporting evidence pins are incomplete")
    packets = {}
    for filename, digest in EVIDENCE_PINS.items():
        raw = (HERE / "samples" / filename).read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError(f"supporting evidence digest changed: {filename}")
        packets[filename] = json.loads(raw)
        capture_bounds.validate(packets[filename], filename)
    access = packets["imbuement-access.json"]["by_name"]
    if set(access) != set(LAYOUT):
        raise ValueError("access profiles do not cover exactly the 24 types")
    if any(set(row["direct_shrine"]) != {"basic", "intricate", "powerful"} for row in access.values()):
        raise ValueError("access profiles must cover all 72 direct shrine routes")
    for family in access.values():
        for route in family["direct_shrine"].values():
            if route["all_of"].count({"predicate_ref": "unequipped_shrine_target"}) != 1:
                raise ValueError("direct shrine routes require an unequipped target")
        if ({"predicate_ref": "unequipped_shrine_target"} in family["scroll_apply"]["all_of"]
                or family["scroll_apply"]["equipped_target_permission"]["value"] is not None):
            raise ValueError("scroll equipped-target permission remains separately unconfirmed")
    rule_rows = packets["global-rules-evidence.json"]["rules"]
    if len({r["id"] for r in rule_rows}) != len(rule_rows):
        raise ValueError("duplicate Global rule id")
    slots = packets["imbuement-eligibility.json"]["items"]
    if len(slots) != 663 or len({r["client_id"] for r in slots}) != 663:
        raise ValueError("eligibility census must cover all 663 distinct primary items")
    import combat_evidence
    import newbranch_evidence
    combat_evidence.validate(packets["imbuement-combat.json"])
    newbranch_evidence.validate(packets["crystal-imbuements-evidence.json"])
    import research_closure
    research_closure.validate(packets["global-research-closure.json"])
    import owner_policy
    owner_policy.validate(packets["owner-authoring-policy.json"], packets)
    validate_observation_plan(packets["global-observation-plan.json"], packets)
    import behavior_answers
    errors = behavior_answers.validate(packets["current-behavior-answers.json"],
                                      global_evidence=packets["global-rules-evidence.json"],
                                      combat_evidence=packets["imbuement-combat.json"])
    if errors:
        raise ValueError("current behavior evidence: " + "; ".join(errors))
    proposals = packets["missing-item-definitions.json"]["proposals"]
    if {p["source_client_id"] for p in proposals} != {49160, 53192} or any(
            p["identity_state"] != "PROPOSED_NOT_REGISTERED" for p in proposals):
        raise ValueError("missing Item proposals cannot masquerade as canonical bindings")
    return packets

# IMBUE-FORGE-0 sections 3 and 5: 24 types, 20 exclusion categories.
LAYOUT = {
    "Scorch": ("elemental_conversion", "fire"),
    "Venom": ("elemental_conversion", "earth"),
    "Frost": ("elemental_conversion", "ice"),
    "Electrify": ("elemental_conversion", "energy"),
    "Reap": ("elemental_conversion", "death"),
    "Vampirism": ("leech", "health"), "Void": ("leech", "mana"),
    "Strike": ("critical", None),
    "Lich Shroud": ("protection", "death"),
    "Snake Skin": ("protection", "earth"),
    "Dragon Hide": ("protection", "fire"),
    "Quara Scale": ("protection", "ice"),
    "Cloud Fabric": ("protection", "energy"),
    "Demon Presence": ("protection", "holy"),
    "Chop": ("skill_bonus", "axe"), "Slash": ("skill_bonus", "sword"),
    "Bash": ("skill_bonus", "club"), "Blockade": ("skill_bonus", "shielding"),
    "Precision": ("skill_bonus", "distance"),
    "Epiphany": ("skill_bonus", "magic_level"), "Punch": ("skill_bonus", "fist"),
    "Swiftness": ("speed_bonus", None),
    "Featherweight": ("capacity_bonus", None),
    "Vibrancy": ("paralysis_recovery", None),
}


def key(name):
    return name.lower().replace(" ", "_")


def category(name):
    return "elemental_damage" if LAYOUT[name][0] == "elemental_conversion" else key(name)


def obj(properties):
    return {"type": "object", "properties": properties,
            "required": list(properties), "additionalProperties": False}


def integer(minimum=1, maximum=None):
    return {"type": "integer", "minimum": minimum, **({"maximum": maximum} if maximum is not None else {})}


def enum(*values):
    return {"enum": list(values)}


def array(items, minimum, maximum):
    return {"type": "array", "items": items, "minItems": minimum, "maxItems": maximum}


def schema():
    text = {"type": "string", "minLength": 1}
    percent = integer(1, 10000)  # basis points: 100 = 1 percent.
    element = enum("fire", "earth", "ice", "energy", "death", "holy")
    effects = [
        obj({"kind": {"const": "elemental_conversion"}, "element": element, "share_bps": percent}),
        obj({"kind": {"const": "protection"}, "element": element, "absorb_bps": percent}),
        obj({"kind": {"const": "leech"}, "resource": enum("health", "mana"), "share_bps": percent, "chance_bps": percent}),
        obj({"kind": {"const": "critical"}, "extra_damage_bps": percent, "chance_bps": percent,
             "value_semantics": {"const": "ADDITIVE_IMBUEMENT_MODIFIER"}}),
        obj({"kind": {"const": "skill_bonus"}, "skill": enum("axe", "sword", "club", "shielding", "distance", "magic_level", "fist"), "amount": integer()}),
        obj({"kind": {"const": "speed_bonus"}, "amount": integer()}),
        obj({"kind": {"const": "capacity_bonus"}, "increase_bps": percent}),
        obj({"kind": {"const": "paralysis_recovery"}, "remove_chance_bps": percent,
             "trigger": {"const": "additional_paralysis_attack_while_paralysed"},
             "sequence_profile": {"const": "imbuement-combat.json#vibrancy_sequence"}}),
    ]
    item_ref = obj({"family": {"const": "Item"},
                    "key": {"type": "string", "pattern": "^oteryn:item\\.tibia\\.i[0-9]+$"},
                    "revision": {"const": "definition-r1"}})
    provenance = obj({"effect": enum("wiki_br", "global-rules-evidence.json#strike_additive_modifiers"), "materials": {"const": "wiki_br"},
                      "duration": {"const": "manual"}, "access": {"const": "imbuement-access.json"},
                      "fees": {"const": "global-rules-evidence.json"}})
    tier = obj({"tier": integer(1, 3), "name": enum("Basic", "Intricate", "Powerful"),
                "duration_ms": {"const": 72000000}, "effect": {"oneOf": effects},
                "materials": array(obj({"source_name": text, "count": integer(), "item_binding": item_ref}), 1, 3),
                "scroll_item": item_ref,
                "apply_fee_gold": integer(), "clear_fee_gold": integer(),
                "access": obj({"direct_shrine_target_unequipped_required": {"const": True},
                               "premium_required": {"type": "boolean"},
                               "catalogue": {"const": "imbuement-access.json"},
                               "profile": enum(*sorted(LAYOUT)),
                               "tier": enum("basic", "intricate", "powerful"),
                               "runtime_quest_binding": {"const": "QUEST_FAMILY_NOT_POPULATED"}}),
                "provenance": provenance})
    definition = obj({"candidate_key": enum(*(key(n) for n in sorted(LAYOUT))),
                      "name": enum(*sorted(LAYOUT)),
                      "category": enum(*sorted({category(n) for n in LAYOUT})),
                      "parity": {"const": "AUDITED_WITH_EXPLICIT_GAPS"},
                      "item_eligibility_binding": obj({"catalogue": {"const": "imbuement-eligibility.json"},
                                                       "candidate_key": enum(*(key(n) for n in sorted(LAYOUT)))}),
                      "tiers": array(tier, 3, 3)})
    root = obj({"schema": {"const": "OTERYN_IMBUEMENT_AUTHORING_CATALOGUE/v1"},
                "activation": {"const": "DRAFT_NOT_RUNTIME_READY"},
                "target": {"const": "global-tibia-current-2026-10-01"},
                "source_facts_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                "supporting_catalogues": obj({name: {"type": "string", "pattern": "^[0-9a-f]{64}$"}
                                              for name in sorted(EVIDENCE_PINS)}),
                "global_rules_profile": {"const": "global-rules-evidence.json"},
                "observation_plan_profile": {"const": "global-observation-plan.json"},
                "combat_profile": {"const": "imbuement-combat.json"},
                "current_behavior_profile": {"const": "current-behavior-answers.json"},
                "global_research_profile": {"const": "global-research-closure.json"},
                "owner_authoring_policy_profile": {"const": "owner-authoring-policy.json"},
                "missing_item_proposals": {"const": "missing-item-definitions.json"},
                "engine_reference_profile": {"const": "crystal-imbuements-evidence.json"},
                "architecture_reconciliation": {"const": "GLOBAL_SOURCE_CONFLICTS_REQUIRE_ARCHITECTURE_UPDATE"},
                "definitions": array(definition, 24, 24)})
    return {"$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Oteryn imbuement authoring candidate v1", **root}


def source_facts():
    raw = FACTS.read_bytes()
    if hashlib.sha256(raw).hexdigest() != FACTS_SHA256:
        raise ValueError("source facts digest changed; refresh and review the evidence pin")
    return json.loads(raw)["sources"]


def effect(name, numbers):
    kind, detail = LAYOUT[name]
    expected = 2 if kind in ("critical", "leech") else 1
    if len(numbers) != expected:
        raise ValueError(f"unexpected wiki effect numbers for {name}")
    value = numbers[0]
    fields = {
        "elemental_conversion": {"element": detail, "share_bps": value * 100},
        "protection": {"element": detail, "absorb_bps": value * 100},
        "skill_bonus": {"skill": detail, "amount": value},
        "speed_bonus": {"amount": value},
        "capacity_bonus": {"increase_bps": value * 100},
        "paralysis_recovery": {"remove_chance_bps": value * 100,
                               "trigger": "additional_paralysis_attack_while_paralysed",
                               "sequence_profile": "imbuement-combat.json#vibrancy_sequence"},
    }
    if kind in ("critical", "leech"):
        fields[kind] = {"chance_bps": numbers[1] * 100,
                        "extra_damage_bps" if kind == "critical" else "share_bps": value * 100}
        if kind == "leech":
            fields[kind]["resource"] = detail
        else:
            fields[kind]["value_semantics"] = "ADDITIVE_IMBUEMENT_MODIFIER"
    return {"kind": kind, **fields[kind]}


def build():
    sources = source_facts()
    wiki = sources["wiki_br"]["records"]
    if {r["name"] for r in wiki} != set(LAYOUT) or len(wiki) != 24:
        raise ValueError("wiki type set differs from the specified 24 types")
    packets = supporting()
    bindings = packets["imbuement-bindings.json"]
    rules = {r["id"]: r for r in packets["global-rules-evidence.json"]["rules"]}
    fees = rules["apply_fee_gold"]["value"]
    strike = rules["strike_additive_modifiers"]["value"]
    if strike != {"chance_bps": 500, "extra_damage_bps_by_tier": [500, 1500, 4000],
                  "value_semantics": "ADDITIVE_IMBUEMENT_MODIFIER"}:
        raise ValueError("Strike modifiers differ from the qualified post-2025 source values")
    definitions = []
    for record in sorted(wiki, key=lambda r: r["name"]):
        name = record["name"]
        tiers = []
        for i, tier_name in enumerate(("Basic", "Intricate", "Powerful")):
            numbers = ([strike["extra_damage_bps_by_tier"][i] // 100, strike["chance_bps"] // 100]
                       if name == "Strike" else record["tier_effect_numbers"][i])
            tiers.append({"tier": i + 1, "name": tier_name, "duration_ms": 72000000,
                          "effect": effect(name, numbers),
                          "materials": [{"source_name": m["name"], "count": m["count"],
                                         "item_binding": bindings["material_bindings"][m["name"]]["item_ref"]}
                                        for m in record["incremental_materials"][:i + 1]],
                          "scroll_item": bindings["scroll_bindings"][name][tier_name.lower()]["item_ref"],
                          "apply_fee_gold": fees[tier_name.lower()], "clear_fee_gold": rules["clear_fee_gold"]["value"],
                          "access": {"direct_shrine_target_unequipped_required": True, "premium_required": i > 0, "catalogue": "imbuement-access.json",
                                     "profile": name, "tier": tier_name.lower(),
                                     "runtime_quest_binding": "QUEST_FAMILY_NOT_POPULATED"},
                          "provenance": {"effect": ("global-rules-evidence.json#strike_additive_modifiers"
                                                    if name == "Strike" else "wiki_br"),
                                         "materials": "wiki_br", "duration": "manual",
                                         "access": "imbuement-access.json", "fees": "global-rules-evidence.json"}})
        definitions.append({"candidate_key": key(name), "name": name, "category": category(name),
                            "parity": "AUDITED_WITH_EXPLICIT_GAPS",
                            "item_eligibility_binding": {"catalogue": "imbuement-eligibility.json", "candidate_key": key(name)},
                            "tiers": tiers})
    return {"schema": "OTERYN_IMBUEMENT_AUTHORING_CATALOGUE/v1", "activation": "DRAFT_NOT_RUNTIME_READY",
            "target": packets["global-rules-evidence.json"]["target"], "source_facts_sha256": FACTS_SHA256,
            "supporting_catalogues": dict(sorted(EVIDENCE_PINS.items())),
            "global_rules_profile": "global-rules-evidence.json",
            "observation_plan_profile": "global-observation-plan.json",
            "combat_profile": "imbuement-combat.json",
            "current_behavior_profile": "current-behavior-answers.json",
            "global_research_profile": "global-research-closure.json",
            "owner_authoring_policy_profile": "owner-authoring-policy.json",
            "missing_item_proposals": "missing-item-definitions.json",
            "engine_reference_profile": "crystal-imbuements-evidence.json",
            "architecture_reconciliation": "GLOBAL_SOURCE_CONFLICTS_REQUIRE_ARCHITECTURE_UPDATE", "definitions": definitions}


def comparison():
    sources = source_facts()
    packets = supporting()
    import newbranch_evidence
    corrected = newbranch_evidence.corrected_previous_references(packets["crystal-imbuements-evidence.json"])
    engines = {e: {(r["name"], r["tier"]): r for r in corrected[e]} for e in ("canary", "crystal")}
    differences = []
    for name, tier in sorted(engines["canary"]):
        a, b = engines["canary"][(name, tier)], engines["crystal"][(name, tier)]
        for field in ("effect", "materials", "premium", "storage", "scroll_item_ids"):
            if a[field] != b[field]:
                differences.append({"name": name, "tier": tier, "field": field, "canary": a[field], "crystal": b[field]})
    return {"schema": "OTERYN_IMBUEMENT_SOURCE_COMPARISON/v1", "source_facts_sha256": FACTS_SHA256,
            "engine_comparison_key": ["name", "tier"], "differences": differences,
            "comparison_scope": "Raw XML facts with independently pinned corrected scroll bindings, not final engine outcomes. Original immutable captures incorrectly read scrollid only from children and remain preserved as capture history. Strike candidate values are additive modifiers; intrinsic character critical baseline is a separate Global rule.",
            "selection": "Source-qualified post-2025 Strike additive modifiers; Wiki BR other effects and cumulative recipes; primary-plus-canonical Item identities; official Global fees; sourced quest predicates; direct per-item Tibiopedia types/tiers with explicit Wiki BR fallback and retained source conflicts",
            "global_fee_conflict": {"candidate_architecture": [5000, 30000, 200000],
                                    "global_since_2025": [7500, 60000, 250000]},
            "supporting_catalogues": dict(sorted(EVIDENCE_PINS.items())),
            "completion": {
                "definitions": 24, "tier_recipes": 72,
                "material_bindings": len(packets["imbuement-bindings.json"]["material_bindings"]),
                "scroll_bindings": sum(len(r) for r in packets["imbuement-bindings.json"]["scroll_bindings"].values()),
                "current_equipment_typed": packets["imbuement-eligibility.json"]["summary"]["typed_items"],
                "target_equipment_typed": packets["imbuement-eligibility.json"]["summary"]["target_candidate_typed_items"],
                "target_existing_item_refs": packets["imbuement-eligibility.json"]["summary"]["target_candidate_bound_items"],
                "basic_only_eligibility_profiles": sum(any(cap == 1 for cap in row["allowed_types"].values())
                                                      for row in packets["imbuement-eligibility.json"]["items"]),
                "validated_missing_item_proposals": len(packets["missing-item-definitions.json"]["proposals"]),
                "source_defined_shrine_routes": 72,
                "completed_scroll_loot_records": 48,
                "global_rules": len(packets["global-rules-evidence.json"]["rules"]),
                "combat_rule_profiles": len(packets["imbuement-combat.json"]["rules"]),
                "source_answered_engine_question_groups": packets["current-behavior-answers.json"]["counts"]["engine_answered_questions"],
                "owner_approved_operational_decisions": packets["owner-authoring-policy.json"]["counts"]["owner_approved_decisions"],
                "owner_resolved_scope_questions": packets["current-behavior-answers.json"]["counts"]["owner_scope_resolved"],
                "planned_not_observed_scenarios": packets["global-observation-plan.json"]["counts"]["planned_scenarios"],
                "crystal_imbuements_revision": packets["crystal-imbuements-evidence.json"]["revision"],
                "gold_token_exchange_bundles": len(packets["imbuement-access.json"]["material_acquisition"]["yana_gold_token_exchange"]["recipes"]),
            },
            "global_research_profile": "global-research-closure.json",
            "owner_authoring_policy_profile": "owner-authoring-policy.json",
            "remaining_global_public_fields": {g["id"]: g["remaining_public_fields"] for g in packets["global-research-closure.json"]["groups"] if g["remaining_public_fields"]},
            "observation_requirement_interpretation": "SUPPLEMENTAL_CAPTURE_ALTERNATIVES; LITERAL_PUBLIC_REFERENCES_ACCEPTED; RUNTIME_CONTRACTS_SEPARATE",
            "remaining_global_observation_requirements": packets["global-rules-evidence.json"]["unresolved"],
            "blocked": ["Candidate architecture reconciliation (fees, Basic inscription, Stash, Vibrancy)", "Canonical Quest runtime state",
                        "Canonical Imbuement ruleset remains READY_UNPOPULATED; the authoring schema is not installed in runtime",
                        "Canonical Item type/tier profiles remain UNKNOWN; ReferenceImbuementTier lacks tier1 lowering",
                        "Per-field evidence gaps listed in eligibility and Global rules packets",
                        "Runtime, persistence and wire implementation"]}


def validate(candidate):
    validator = Draft202012Validator(schema(), format_checker=FormatChecker())
    errors = sorted(validator.iter_errors(candidate), key=lambda e: str(list(e.path)))
    if errors:
        raise ValueError("schema: " + "; ".join(f"{list(e.path)}: {e.message}" for e in errors[:5]))
    names = [d["name"] for d in candidate["definitions"]]
    if len(set(names)) != 24:
        raise ValueError("duplicate or missing imbuement type")
    for d in candidate["definitions"]:
        if d["candidate_key"] != key(d["name"]) or d["category"] != category(d["name"]):
            raise ValueError("name, candidate key and category mismatch")
        if [t["tier"] for t in d["tiers"]] != [1, 2, 3]:
            raise ValueError("tiers must be exactly 1, 2, 3 in order")
        for i, t in enumerate(d["tiers"]):
            if len(t["materials"]) != i + 1:
                raise ValueError("materials must include all preceding tiers")
            if len({m["source_name"] for m in t["materials"]}) != i + 1:
                raise ValueError("duplicate material")
            if t["access"]["premium_required"] != (i > 0):
                raise ValueError("premium requirement differs from the sourced tier predicates")
    if candidate != build():
        raise ValueError("candidate differs from pinned Global evidence and identity bindings")


def write_or_check(path, data, check):
    encoded = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != encoded:
            raise ValueError(f"generated file drift: {path.name}")
    else:
        path.write_text(encoded, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("build").add_argument("--check", action="store_true")
    commands.add_parser("validate").add_argument("path", type=Path, nargs="?", default=CATALOGUE)
    args = parser.parse_args()
    try:
        if args.command == "build":
            candidate = build()
            validate(candidate)
            for path, data in ((SCHEMA, schema()), (CATALOGUE, candidate), (REPORT, comparison())):
                write_or_check(path, data, args.check)
        else:
            validate(json.loads(args.path.read_text(encoding="utf-8")))
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as error:
        parser.exit(1, f"ERROR: {error}\n")
    print("PASS: imbuement authoring candidate (24 types, 72 tiers; runtime admission blocked)")


if __name__ == "__main__":
    main()
