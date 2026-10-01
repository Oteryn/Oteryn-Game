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
FACTS_SHA256 = "33f2c1e26beb9fe0e20bd00901023510eca19f6a388a0d5610123cb6a317a9b0"
CATALOGUE = HERE / "samples/imbuements-candidate.json"
REPORT = HERE / "samples/imbuement-source-comparison.json"
SCHEMA = HERE / "imbuement.schema.json"
EVIDENCE_PINS = {
    "imbuement-bindings.json": "e9b3c4355a5db835af150c125fa3204f4bd6e674ef9e3b2d52383bac81f21ebc",
    "imbuement-access.json": "7b18978f9c2b31d98bc42dba5a9a799d8845f974beb51299efd92bac7fc00334",
    "imbuement-eligibility.json": "f39b7a1c1593907e424f70ed2db6d27ba9b8c4835346d30cb3a35e4112f0e677",
    "global-rules-evidence.json": "051d86fbbe7f3fd48cc4361bc1dcea41f9d8912fe6c3db75af29da72308e40ea",
}


def supporting():
    expected = {"imbuement-bindings.json", "imbuement-access.json",
                "imbuement-eligibility.json", "global-rules-evidence.json"}
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
    "Vibrancy": ("paralysis_deflection", None),
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
        obj({"kind": {"const": "paralysis_deflection"}, "chance_bps": percent}),
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
        "paralysis_deflection": {"chance_bps": value * 100},
    }
    if kind in ("critical", "leech"):
        fields[kind] = {"chance_bps": numbers[1] * 100,
                        "extra_damage_bps" if kind == "critical" else "share_bps": value * 100}
        if kind == "leech":
            fields[kind]["resource"] = detail
    return {"kind": kind, **fields[kind]}


def build():
    sources = source_facts()
    packets = supporting()
    bindings = packets["imbuement-bindings.json"]
    rules = {r["id"]: r for r in packets["global-rules-evidence.json"]["rules"]}
    wiki = sources["wiki_br"]["records"]
    if {r["name"] for r in wiki} != set(LAYOUT) or len(wiki) != 24:
        raise ValueError("wiki type set differs from the accepted 24 types")
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
            "architecture_reconciliation": "GLOBAL_SOURCE_CONFLICTS_REQUIRE_ARCHITECTURE_UPDATE", "definitions": definitions}


def comparison():
    sources = source_facts()
    engines = {e: {(r["name"], r["tier"]): r for r in sources[e]["records"]} for e in ("canary", "crystal")}
    differences = []
    for name, tier in sorted(engines["canary"]):
        a, b = engines["canary"][(name, tier)], engines["crystal"][(name, tier)]
        for field in ("effect", "materials", "premium", "storage", "scroll_item_ids"):
            if a[field] != b[field]:
                differences.append({"name": name, "tier": tier, "field": field, "canary": a[field], "crystal": b[field]})
    return {"schema": "OTERYN_IMBUEMENT_SOURCE_COMPARISON/v1", "source_facts_sha256": FACTS_SHA256,
            "engine_comparison_key": ["name", "tier"], "differences": differences,
            "selection": "Wiki BR effects and cumulative recipes; primary-plus-canonical Item identities; official Global fees; sourced quest predicates; direct per-item Tibiopedia types/tiers with explicit Wiki BR fallback and retained source conflicts",
            "global_fee_conflict": {"accepted_decision": [5000, 30000, 200000],
                                    "global_since_2025": [7500, 60000, 250000]},
            "supporting_catalogues": dict(sorted(EVIDENCE_PINS.items())),
            "blocked": ["Accepted architecture fee correction", "Canonical Quest runtime state",
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
                raise ValueError("premium requirement differs from the accepted decision")
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
