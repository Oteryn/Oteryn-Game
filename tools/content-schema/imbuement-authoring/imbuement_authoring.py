#!/usr/bin/env python3
"""Build and validate an offline imbuement authoring candidate; never activate it."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

HERE = Path(__file__).resolve().parent
FACTS = HERE / "samples/imbuement-sources-2026-10-01.json"
FACTS_SHA256 = "7ee2221886abdc80d27ce5cd13e58bdecab8b3c5210a7e0e7a36efc52771070e"
CATALOGUE = HERE / "samples/imbuements-candidate.json"
REPORT = HERE / "samples/imbuement-source-comparison.json"
SCHEMA = HERE / "imbuement.schema.json"
EVIDENCE_PINS = {
    "imbuement-bindings.json": "e9b3c4355a5db835af150c125fa3204f4bd6e674ef9e3b2d52383bac81f21ebc",
    "imbuement-access.json": "29f43745a3d0ecc74c183f9a031456a3254f7ca9bb29acbe6323dfa7162ab25e",
    "imbuement-eligibility.json": "3b0debbcb32d604464af9381c6f2bd14fa802d57d12dea6f72c64d3e926fddcf",
    "global-rules-evidence.json": "6167ec05f5ee0fc153203fb2372997a0ed9f29a17fae359fe7fccd25a95cc40a",
    "imbuement-combat.json": "18c5cc3fe0b7087eff23faa393a453a11ae869630109e523dd5c88612824c7f9",
    "crystal-imbuements-evidence.json": "3bc60e6c95f2e371530f9145d9fd6f382db373b08b80b34590471a2f822d5b4d",
    "missing-item-definitions.json": "f9d676c2e671d171f6a503e19466bebcd6d000bf9a5cd5fc2cc85889829d04c8",
    "missing-item-source-facts.json": "4dc218e74a559d7b92d3ca7915fc02d57a2dac3f980d11e734f45ee6c9edbf1d",
}


def supporting():
    expected = {"imbuement-bindings.json", "imbuement-access.json",
                "imbuement-eligibility.json", "global-rules-evidence.json",
                "imbuement-combat.json", "crystal-imbuements-evidence.json",
                "missing-item-definitions.json", "missing-item-source-facts.json"}
    if set(EVIDENCE_PINS) != expected:
        raise ValueError("supporting evidence pins are incomplete")
    packets = {}
    for filename, digest in EVIDENCE_PINS.items():
        raw = (HERE / "samples" / filename).read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError(f"supporting evidence digest changed: {filename}")
        packets[filename] = json.loads(raw)
    access = packets["imbuement-access.json"]["by_name"]
    if set(access) != set(LAYOUT):
        raise ValueError("access profiles do not cover exactly the 24 types")
    if any(set(row["direct_shrine"]) != {"basic", "intricate", "powerful"} for row in access.values()):
        raise ValueError("access profiles must cover all 72 direct shrine routes")
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
        obj({"kind": {"const": "critical"}, "extra_damage_bps": percent, "chance_bps": percent}),
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
    provenance = obj({"effect": {"const": "wiki_br"}, "materials": {"const": "wiki_br"},
                      "duration": {"const": "manual"}, "access": {"const": "imbuement-access.json"},
                      "fees": {"const": "global-rules-evidence.json"}})
    tier = obj({"tier": integer(1, 3), "name": enum("Basic", "Intricate", "Powerful"),
                "duration_ms": {"const": 72000000}, "effect": {"oneOf": effects},
                "materials": array(obj({"source_name": text, "count": integer(), "item_binding": item_ref}), 1, 3),
                "scroll_item": item_ref,
                "apply_fee_gold": integer(), "clear_fee_gold": integer(),
                "access": obj({"premium_required": {"type": "boolean"},
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
                "target": {"const": "global-tibia-observable-2026-07-28-post-server-save"},
                "source_facts_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                "supporting_catalogues": obj({name: {"type": "string", "pattern": "^[0-9a-f]{64}$"}
                                              for name in sorted(EVIDENCE_PINS)}),
                "global_rules_profile": {"const": "global-rules-evidence.json"},
                "combat_profile": {"const": "imbuement-combat.json"},
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
    definitions = []
    for record in sorted(wiki, key=lambda r: r["name"]):
        name = record["name"]
        tiers = []
        for i, tier_name in enumerate(("Basic", "Intricate", "Powerful")):
            tiers.append({"tier": i + 1, "name": tier_name, "duration_ms": 72000000,
                          "effect": effect(name, record["tier_effect_numbers"][i]),
                          "materials": [{"source_name": m["name"], "count": m["count"],
                                         "item_binding": bindings["material_bindings"][m["name"]]["item_ref"]}
                                        for m in record["incremental_materials"][:i + 1]],
                          "scroll_item": bindings["scroll_bindings"][name][tier_name.lower()]["item_ref"],
                          "apply_fee_gold": fees[tier_name.lower()], "clear_fee_gold": rules["clear_fee_gold"]["value"],
                          "access": {"premium_required": i > 0, "catalogue": "imbuement-access.json",
                                     "profile": name, "tier": tier_name.lower(),
                                     "runtime_quest_binding": "QUEST_FAMILY_NOT_POPULATED"},
                          "provenance": {"effect": "wiki_br", "materials": "wiki_br", "duration": "manual",
                                         "access": "imbuement-access.json", "fees": "global-rules-evidence.json"}})
        definitions.append({"candidate_key": key(name), "name": name, "category": category(name),
                            "parity": "AUDITED_WITH_EXPLICIT_GAPS",
                            "item_eligibility_binding": {"catalogue": "imbuement-eligibility.json", "candidate_key": key(name)},
                            "tiers": tiers})
    return {"schema": "OTERYN_IMBUEMENT_AUTHORING_CATALOGUE/v1", "activation": "DRAFT_NOT_RUNTIME_READY",
            "target": sources["decision"]["findings"]["target"], "source_facts_sha256": FACTS_SHA256,
            "supporting_catalogues": dict(sorted(EVIDENCE_PINS.items())),
            "global_rules_profile": "global-rules-evidence.json",
            "combat_profile": "imbuement-combat.json",
            "missing_item_proposals": "missing-item-definitions.json",
            "engine_reference_profile": "crystal-imbuements-evidence.json",
            "architecture_reconciliation": "GLOBAL_SOURCE_CONFLICTS_REQUIRE_ARCHITECTURE_UPDATE", "definitions": definitions}


def comparison():
    sources = source_facts()
    packets = supporting()
    engines = {e: {(r["name"], r["tier"]): r for r in sources[e]["records"]} for e in ("canary", "crystal")}
    differences = []
    for name, tier in sorted(engines["canary"]):
        a, b = engines["canary"][(name, tier)], engines["crystal"][(name, tier)]
        for field in ("effect", "materials", "premium", "storage", "scroll_item_ids"):
            if a[field] != b[field]:
                differences.append({"name": name, "tier": tier, "field": field, "canary": a[field], "crystal": b[field]})
    return {"schema": "OTERYN_IMBUEMENT_SOURCE_COMPARISON/v1", "source_facts_sha256": FACTS_SHA256,
            "engine_comparison_key": ["name", "tier"], "differences": differences,
            "comparison_scope": "Raw XML facts, not final engine outcomes. Crystal imbuements branch includes player baselines; use the qualified effective-strength comparison in its separate packet.",
            "selection": "Wiki BR effects and cumulative recipes; primary-plus-canonical Item identities; official Global fees; sourced quest predicates; direct per-item Tibiopedia types/tiers with explicit Wiki BR fallback and retained source conflicts",
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
                "validated_missing_item_proposals": len(packets["missing-item-definitions.json"]["proposals"]),
                "source_defined_shrine_routes": 72,
                "completed_scroll_loot_records": 48,
                "global_rules": len(packets["global-rules-evidence.json"]["rules"]),
                "combat_rule_profiles": len(packets["imbuement-combat.json"]["rules"]),
                "crystal_imbuements_revision": packets["crystal-imbuements-evidence.json"]["revision"],
                "gold_token_exchange_bundles": len(packets["imbuement-access.json"]["material_acquisition"]["yana_gold_token_exchange"]["recipes"]),
            },
            "remaining_global_observation_requirements": packets["global-rules-evidence.json"]["unresolved"],
            "blocked": ["Candidate architecture reconciliation (fees, Basic inscription, Stash, Vibrancy)", "Canonical Quest runtime state",
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
