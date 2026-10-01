"""Lower the pinned TibiaWiki Item stat snapshot into the Item stat promotion packet v2
(ITEM-SEM-2b-1) that `apps/game-server/src/content/item_stats_promotion.rs` applies.

Source policy: tibia.com, then TibiaWiki; Crystal and Canary are hypotheses. So every
value here comes from the English TibiaWiki snapshot
(`imports/tibiawiki/facts/items-stats.json`) and, when applied, replaces whatever an
earlier promotion (the Crystal lowering v1, the Wave 1 capture) put in that field.
Where the wiki is silent the field is left alone.

A field is emitted only when every page that lists the item agrees on one parseable
value; disagreement and unparseable values go to the report, never into a row. Only
ids that are Items in content (`content/items/index.json`) get rows.

Fields (2b-1): attack, defense, defense modifier, armor, range, weapon type, elemental
attacks, imbuement slots, equipment patterns, weight, positive charge counts and explicit
durations. Charges/durations require a single-ID source page, preserve known conflicts
and blocked states, and do not admit behavior or materialization. Equipment requires an explicit
wiki slot; absent requirements remain UNKNOWN and unsupported vocations/hand claims
hold the complete pattern. Weight is in hundredths of an ounce, as in Tibia
(owner decision 2026-09-30: 41.00 oz = 4100).

`--check` rebuilds the packet in memory and fails on any byte difference.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SNAPSHOT = ROOT / "imports" / "tibiawiki" / "facts" / "items-stats.json"
ITEM_INDEX = ROOT / "content" / "items" / "index.json"
SOURCE_BINDINGS = ROOT / "imports" / "crystalserver" / "bindings" / "items.json"
OUTPUT = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20260930-item-stats-promotion-v2.json"
)
COMPILER_PATH = "tools/content-schema/item-authoring/lower_wiki_stats_packet.py"
SOURCE_HOLD = (
    ROOT / "docs/agents/evidence/OTV2-20261001-item-resistance-source-hold-v1.json"
)
SOURCE_HOLD_SHA256 = "ddd48b4381dc4cdac02b56fe124827bc951377e6423bb88c1c2090f46386e134"
MODIFIER_SOURCE_HOLD = (
    ROOT / "docs/agents/evidence/OTV2-20261001-item-modifier-source-hold-v1.json"
)
MODIFIER_SOURCE_HOLD_SHA256 = (
    "7d2a7f2d35c6099066677d7c4728d9bb832926b686b19901ff5cce1e55f4ed07"
)
SCHEMA = "OTERYN_ITEM_STATS_PROMOTION/v2"
ITEM_KEY = "oteryn:item.tibia.i{}"

INTEGER = re.compile(r"^[+-]?[0-9]+$")
UNSIGNED = re.compile(r"^[0-9]+$")
WEIGHT = re.compile(r"^([0-9]+)(?:\.([0-9]{1,2}))?$")
# Wiki `weapontype` -> `ReferenceWeaponType`. Wands, rods, fist weapons and ammunition (and
# a few pages that omit it) have no weapontype and are typed from `primarytype`; exercise
# and training weapons stay untyped.
WEAPON_TYPES = {
    "Axe": "AXE",
    "Club": "CLUB",
    "Distance": "DISTANCE",
    "Fist Fighting": "FIST",
    "fist": "FIST",
    "Sword": "SWORD",
}
PRIMARYTYPE_WEAPON_TYPES = {
    "Ammunition": "AMMUNITION",
    "Axe Weapons": "AXE",
    "Club Weapons": "CLUB",
    "Distance Weapons": "DISTANCE",
    "Fist Fighting Weapons": "FIST",
    "Rods": "WAND",
    "Sword Weapons": "SWORD",
    "Wands": "WAND",
}
# `<element>_attack` -> `ReferenceWeaponElement`; holy has no variant and is reported.
ELEMENT_PARAMS = {
    "death_attack": "DEATH",
    "earth_attack": "EARTH",
    "energy_attack": "ENERGY",
    "fire_attack": "FIRE",
    "ice_attack": "ICE",
}
UNSUPPORTED_PARAMS = ("holy_attack",)
EQUIPMENT_SLOTS = {
    "Head": "HEAD",
    "Body": "TORSO",
    "Torso": "TORSO",
    "Legs": "LEGS",
    "Feet": "FEET",
    "Weapon Hand": "WEAPON",
    "Both Hands": "WEAPON",
    "Shield Hand": "SHIELD",
    "Shield": "SHIELD",
    "Neck": "AMULET",
    "Finger": "RING",
    "Container": "CONTAINER",
    "Extra Slot": "EXTRA",
}
VOCATIONS = {
    "druid": "DRUID",
    "knight": "KNIGHT",
    "monk": "MONK",
    "paladin": "PALADIN",
    "sorcerer": "SORCERER",
}
VOCATION_ORDER = tuple(VOCATIONS.values())


def known(value):
    return {"state": "KNOWN", "value": value}


def equipment(fields):
    """Equip-only requirements: explicit slot, closed vocations, complete hand claims.

    Rune use levels and ammunition have no accepted equipment lowering here. Missing
    requirements remain UNKNOWN; a vague vocation or contradictory hand claim holds
    the pattern rather than silently removing that restriction.
    """
    slot = fields.get("slot")
    if (
        slot is None
        or "Rune" in fields.get("primarytype", "")
        or fields.get("primarytype") == "Ammunition"
    ):
        return None, None
    raw = {
        name: fields[name]
        for name in ("primarytype", "slot", "hands", "levelrequired", "vocrequired")
        if name in fields
    }
    if slot not in EQUIPMENT_SLOTS:
        return raw, "MALFORMED"
    unknown = {"state": "UNKNOWN"}
    pattern = {
        "pattern_id": 1,
        "primary_slot": known(EQUIPMENT_SLOTS[slot]),
        "additional_reserved_slots": unknown,
        "mutually_exclusive_groups": unknown,
        "vocations": unknown,
        "level": unknown,
        "compatibility_rule": unknown,
    }
    hands = fields.get("hands")
    if hands is not None and (
        hands not in ("One", "Two")
        or slot not in ("Weapon Hand", "Both Hands", "Shield Hand", "Shield")
    ):
        return raw, "MALFORMED"
    if slot == "Both Hands":
        if hands == "One":
            return raw, "MALFORMED"
        pattern["additional_reserved_slots"] = known(["SHIELD"])
    elif slot in ("Weapon Hand", "Shield Hand", "Shield") and hands is not None:
        if hands == "Two":
            return raw, "MALFORMED"
        pattern["additional_reserved_slots"] = known([])
    if "levelrequired" in fields:
        level = unsigned(fields["levelrequired"], 65535)
        if level is None:
            return raw, "MALFORMED"
        pattern["level"] = known(level)
    if "vocrequired" in fields:
        parts = re.split(r",\s*|\s+and\s+", fields["vocrequired"].strip().lower())
        values = [VOCATIONS.get(part.rstrip("s")) for part in parts]
        if None in values or len(set(values)) != len(values):
            return raw, "MALFORMED"
        pattern["vocations"] = known(sorted(values, key=VOCATION_ORDER.index))
    return raw, {"kind": "EQUIPMENT_PATTERNS", "value": [pattern]}


def signed(raw):
    return int(raw) if INTEGER.match(raw) else None


def unsigned(raw, maximum):
    if not UNSIGNED.match(raw):
        return None
    value = int(raw)
    return value if value <= maximum else None


def weight(raw):
    match = WEIGHT.match(raw)
    if not match:
        return None
    # The source is ounces; omitted decimal digits are exact trailing zeroes.
    # Reject finer precision rather than rounding a source observation.
    return int(match.group(1)) * 100 + int((match.group(2) or "").ljust(2, "0"))


def duration(raw):
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?) (minute|hour|day)s?", raw)
    if not match:
        return None
    value = (
        Fraction(match[1])
        * {"minute": 60000, "hour": 3600000, "day": 86400000}[match[2]]
    )
    return int(value) if value.denominator == 1 and 1 <= value <= 2**64 - 1 else None


def positive_count(raw):
    value = unsigned(raw, 2**32 - 1)
    return value if value is not None and value > 0 else None


# Native discriminant order, not lexical order; percentages are percentage points.
RESISTANCE_KINDS = (
    "DEATH",
    "DROWN",
    "EARTH",
    "ENERGY",
    "FIRE",
    "HOLY",
    "ICE",
    "LIFE_DRAIN",
    "MANA_DRAIN",
    "PHYSICAL",
    "POISON",
    "FIRE_FIELD",
)
RESISTANCE_ALIASES = {kind.lower().replace("_", " "): kind for kind in RESISTANCE_KINDS}
RESISTANCE_ALIASES["drowning"] = "DROWN"


def resistances(fields):
    raw = fields.get("resist")
    if raw is None:
        return None, None
    evidence = {"resist": raw}
    values = {}
    for clause in raw.split(","):
        match = re.fullmatch(
            r"([a-z ]+) ([+-]?[0-9]+(?:\.[0-9]+)?)%", clause.strip().lower()
        )
        if not match or match[1] not in RESISTANCE_ALIASES:
            return evidence, "MALFORMED"
        kind = RESISTANCE_ALIASES[match[1]]
        try:
            percent = Fraction(match[2])
        except ValueError:
            return evidence, "MALFORMED"
        if (
            kind in values
            or not -100 <= percent <= 100
            or not -(2**63) <= percent.numerator <= 2**63 - 1
            or percent.denominator > 2**64 - 1
        ):
            return evidence, "MALFORMED"
        values[kind] = {
            "numerator": percent.numerator,
            "denominator": percent.denominator,
        }
    return evidence, {
        "kind": "RESISTANCES",
        "value": [
            {"kind": kind, "percent": values[kind]}
            for kind in RESISTANCE_KINDS
            if kind in values
        ],
    }


# Existing native kind order; context and activation bindings remain UNKNOWN.
MODIFIER_POINTS = {
    "magic level": "MAGIC_LEVEL_POINTS",
    "axe fighting": "SKILL_AXE",
    "club fighting": "SKILL_CLUB",
    "distance fighting": "SKILL_DISTANCE",
    "fist fighting": "SKILL_FIST",
    "shielding": "SKILL_SHIELD",
    "sword fighting": "SKILL_SWORD",
    "speed": "SPEED",
}
MODIFIER_PERCENTS = {
    "crithit_ch": "CRITICAL_HIT_CHANCE",
    "critextra_dmg": "CRITICAL_HIT_DAMAGE",
    "hpleech_am": "LIFE_LEECH_AMOUNT",
    "hpleech_ch": "LIFE_LEECH_CHANCE",
    "manaleech_am": "MANA_LEECH_AMOUNT",
    "manaleech_ch": "MANA_LEECH_CHANCE",
}
MODIFIER_ORDER = (
    "CRITICAL_HIT_CHANCE",
    "CRITICAL_HIT_DAMAGE",
    "LIFE_LEECH_AMOUNT",
    "LIFE_LEECH_CHANCE",
    "MAGIC_LEVEL_POINTS",
    "MANA_LEECH_AMOUNT",
    "MANA_LEECH_CHANCE",
    "SKILL_AXE",
    "SKILL_CLUB",
    "SKILL_DISTANCE",
    "SKILL_FIST",
    "SKILL_SHIELD",
    "SKILL_SWORD",
    "SPEED",
)


def modifiers(fields):
    # A known list must not drop explicitly observed facts from this same group.
    unsupported = ("mantra", "elementalbond")
    raw = {
        name: fields[name]
        for name in ("attrib", *MODIFIER_PERCENTS, *unsupported)
        if name in fields
    }
    if not raw:
        return None, None
    if any(name in raw for name in unsupported):
        return raw, "MALFORMED"
    values = {}
    if "attrib" in raw:
        for part in raw["attrib"].lower().split(","):
            match = re.fullmatch(r"([a-z ]+) ([+-]?[0-9]+)", part.strip())
            kind = MODIFIER_POINTS.get(match[1]) if match else None
            if kind is None or kind in values or not -(2**31) <= int(match[2]) < 2**31:
                return raw, "MALFORMED"
            values[kind] = {"kind": "SIGNED_POINTS", "value": int(match[2])}
    for field, kind in MODIFIER_PERCENTS.items():
        if field not in raw:
            continue
        match = re.fullmatch(r"([+]?[0-9]+(?:\.[0-9]+)?)%", raw[field])
        if not match:
            return raw, "MALFORMED"
        value = Fraction(match[1])
        # Explicit Wiki percentage points, not engine hundredths of a percent.
        if (
            not 0 <= value <= 100
            or value.numerator > 2**63 - 1
            or value.denominator > 2**64 - 1
        ):
            return raw, "MALFORMED"
        values[kind] = {
            "kind": "RATIONAL_PERCENT",
            "value": {"numerator": value.numerator, "denominator": value.denominator},
        }
    unknown = {"state": "UNKNOWN"}
    return raw, {
        "kind": "MODIFIERS",
        "value": [
            {
                "kind": kind,
                "target_domain": unknown,
                "evaluation_phase": unknown,
                "priority": unknown,
                "parameter": known(values[kind]),
            }
            for kind in MODIFIER_ORDER
            if kind in values
        ],
    }


QUALIFIED_PHYSICAL_FIELDS = {
    "skill_modifiers.modifiers",
    "charges.count",
    "temporal.duration",
    "protection.resistances",
}


def physical_field_inputs():
    definitions, routed = {}, set()
    for shard in json.loads(ITEM_INDEX.read_text())["shards"]:
        for row in json.loads((ROOT / shard).read_text())["records"]:
            definition = row["definition"]
            definitions[definition["identity"]["key"]] = definition
    for family in ("terrain", "objects"):
        for path in (ROOT / f"content/world/{family}").glob("*.json"):
            for row in json.loads(path.read_text()).get("records", []):
                pointer = row.get("provenance", {}).get("item_pointer")
                if pointer:
                    routed.add(pointer["key"])
    return definitions, routed


def field_precondition(definition, field_path, value):
    group, member = field_path.split(".")
    source = definition.get("semantics", {}).get(group, {"state": "UNKNOWN"})
    if source.get("state") in {"CONFLICT", "NOT_APPLICABLE"}:
        return "BLOCKED_EVIDENCE_STATE"
    leaf = source.get("value", {}).get(member, {"state": "UNKNOWN"})
    if leaf.get("state") in {"CONFLICT", "NOT_APPLICABLE"}:
        return "BLOCKED_EVIDENCE_STATE"
    if leaf.get("state") == "KNOWN":
        actual = leaf["value"]
        if field_path == "protection.resistances":
            if any(
                row["percent"].get("state") in {"CONFLICT", "NOT_APPLICABLE"}
                for row in actual
            ):
                return "BLOCKED_EVIDENCE_STATE"
            if any(row["percent"].get("state") != "KNOWN" for row in actual):
                return "KNOWN_PARTIAL_VECTOR_UNQUALIFIED"
            actual = [
                {"kind": row["kind"], "percent": row["percent"]["value"]}
                for row in actual
            ]
        if field_path == "skill_modifiers.modifiers" and any(
            entry[name].get("state") in {"CONFLICT", "NOT_APPLICABLE"}
            for entry in actual
            for name in ("target_domain", "evaluation_phase", "priority", "parameter")
        ):
            return "BLOCKED_EVIDENCE_STATE"
        if actual != value:
            return "KNOWN_FIELD_CONFLICT"
    return None


def scalar(parameter, kind, parse):
    """Field lowering from one wiki parameter to one typed value."""

    def lower(fields):
        raw = fields.get(parameter)
        if raw is None:
            return None, None
        value = parse(raw)
        if value is None:
            return {parameter: raw}, "MALFORMED"
        return {parameter: raw}, {"kind": kind, "value": value}

    return lower


def weapon_type(fields):
    raw = fields.get("weapontype")
    if raw is not None:
        value = WEAPON_TYPES.get(raw)
        if value is None:
            return {"weapontype": raw}, "MALFORMED"
        return {"weapontype": raw}, {"kind": "WEAPON_TYPE", "value": value}
    primary = fields.get("primarytype")
    if primary in PRIMARYTYPE_WEAPON_TYPES:
        value = PRIMARYTYPE_WEAPON_TYPES[primary]
        return {"primarytype": primary}, {"kind": "WEAPON_TYPE", "value": value}
    return None, None


def elemental(fields):
    present = {name: fields[name] for name in ELEMENT_PARAMS if name in fields}
    if not present:
        return None, None
    attacks = []
    for name, raw in sorted(present.items()):
        points = signed(raw)
        if points is None:
            return present, "MALFORMED"
        attacks.append({"element": ELEMENT_PARAMS[name], "points": points})
    attacks.sort(key=lambda row: row["element"])
    return present, {"kind": "ELEMENTAL_ATTACKS", "value": attacks}


FIELDS = {
    "skill_modifiers.modifiers": modifiers,
    "protection.resistances": resistances,
    "charges.count": scalar("charges", "COUNT_U32", positive_count),
    "temporal.duration": scalar("duration", "DURATION_MS", duration),
    "equipment.patterns": equipment,
    "weapon.attack": scalar("attack", "SIGNED_POINTS", signed),
    "weapon.defense": scalar("defense", "SIGNED_POINTS", signed),
    "weapon.extra_defense": scalar("defensemod", "SIGNED_POINTS", signed),
    "protection.armor": scalar("armor", "SIGNED_POINTS", signed),
    "weapon.range_cells": scalar("range", "CELLS", lambda raw: unsigned(raw, 65535)),
    "weapon.weapon_type": weapon_type,
    "weapon.elemental": elemental,
    "imbuement.slot_count": scalar(
        "imbueslots", "COUNT_U8", lambda raw: unsigned(raw, 255)
    ),
    "physical.weight": scalar("weight", "WEIGHT_CENTI_OZ", weight),
}


def content_item_ids():
    """Content Items with a source binding.

    ITEM-ADD-1 (owner decision 2a): an appearance-only Item has no source binding and keeps its
    semantics UNKNOWN, so it takes no TibiaWiki stats.
    """
    bindings = json.loads(SOURCE_BINDINGS.read_text(encoding="utf-8"))["bindings"]
    bound = {row["target"]["key"] for row in bindings if row["disposition"] == "EXACT"}
    index = json.loads(ITEM_INDEX.read_text(encoding="utf-8"))
    ids = set()
    for shard in index["shards"]:
        document = json.loads((ROOT / shard).read_text(encoding="utf-8"))
        for record in document["records"]:
            key = record["definition"]["identity"]["key"]
            if key in bound:
                ids.add(int(key.rsplit(".i", 1)[1]))
    return ids


def source_hold(path=None, digest=None):
    path = SOURCE_HOLD if path is None else path
    digest = SOURCE_HOLD_SHA256 if digest is None else digest
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError("resistance qualification manifest digest mismatch")
    return json.loads(data)


def build(snapshot, item_ids, definitions=None, routed_keys=()):
    source_holds = (
        source_hold(),
        source_hold(MODIFIER_SOURCE_HOLD, MODIFIER_SOURCE_HOLD_SHA256),
    )
    rows = []
    definitions = definitions or {}
    # Every retained page ID participates, including retired and unbound source IDs.
    page_ids = defaultdict(set)
    for record in snapshot["records"].values():
        for observation in record["observations"]:
            page_ids[observation["page_id"]].add(record["item_id"])
    report = {
        "conflict": Counter(),
        "malformed": Counter(),
        "unsupported": Counter(),
        "examples": defaultdict(list),
        "equipment_holds": [],
        "physical_field_holds": [],
    }
    # Records are keyed by Item key or, without an Item record, by the bare Tibia id (#1325).
    for record in sorted(
        snapshot["records"].values(), key=lambda record: record["item_id"]
    ):
        if record["item_id"] not in item_ids:
            continue
        observations = record["observations"]
        holds_by_field = {
            h["field_path"]: h
            for h in source_holds
            if ITEM_KEY.format(record["item_id"]) == h["item_key"]
        }
        for hold in holds_by_field.values():
            names = hold["expected_snapshot_sources"][0]["values"]
            actual = [
                {
                    "page_id": obs["page_id"],
                    "revision_id": obs["revision_id"],
                    "content_sha256": obs.get("content_sha256"),
                    "values": {name: obs["fields"].get(name) for name in names},
                }
                for obs in observations
            ]
            if actual != hold["expected_snapshot_sources"]:
                raise ValueError("resistance qualification snapshot facts drift")
        for field_path, lower in FIELDS.items():
            # STARTER-BACKPACK-0 supplies a separately qualified complete pattern.
            if field_path == "equipment.patterns" and record["item_id"] == 2854:
                continue
            results = [lower(row["fields"]) for row in observations]
            present = [
                (obs, res)
                for obs, res in zip(observations, results)
                if res[0] is not None
            ]
            if not present:
                continue
            if field_path in QUALIFIED_PHYSICAL_FIELDS:
                key = ITEM_KEY.format(record["item_id"])
                typed = [result[1] for _obs, result in present]
                hold = holds_by_field.get(field_path)
                external = hold is not None
                reason = None
                if external:
                    reason = hold["reason"]
                elif key not in definitions or "semantics" not in definitions[key]:
                    reason = "NO_CANONICAL_ITEM_SEMANTICS"
                elif key in routed_keys:
                    reason = "EXISTING_MAP_OWNER"
                elif any(len(page_ids[obs["page_id"]]) != 1 for obs, _ in present):
                    reason = "SHARED_PAGE_VARIANT_UNQUALIFIED"
                elif any(value == "MALFORMED" for value in typed):
                    reason = "MALFORMED_WIKI_VALUE"
                elif any(value != typed[0] for value in typed):
                    reason = "WIKI_PAGE_DISAGREEMENT"
                else:
                    reason = field_precondition(
                        definitions[key], field_path, typed[0]["value"]
                    )
                if reason:
                    report["physical_field_holds"].append(
                        {
                            "item_key": key,
                            "field_path": field_path,
                            "reason": reason,
                            "classification": "CONFLICT"
                            if reason
                            in {
                                "KNOWN_FIELD_CONFLICT",
                                "WIKI_PAGE_DISAGREEMENT",
                                "EXTERNAL_SOURCE_DISAGREEMENT",
                            }
                            else "UNKNOWN",
                            **({"external_qualification": hold} if external else {}),
                            "sources": [
                                {
                                    "page_id": obs["page_id"],
                                    "revision_id": obs["revision_id"],
                                    "values": result[0],
                                    "page_item_ids": sorted(page_ids[obs["page_id"]]),
                                }
                                for obs, result in present
                            ],
                        }
                    )
                    continue
            if field_path == "equipment.patterns" and any(
                "Rune" in obs["fields"].get("primarytype", "")
                or obs["fields"].get("primarytype") == "Ammunition"
                for obs in observations
            ):
                report["conflict"][field_path] += 1
                report["equipment_holds"].append(
                    {
                        "item_key": ITEM_KEY.format(record["item_id"]),
                        "classification": "CONFLICT",
                        "reason": "GEAR_AND_EXCLUDED_USE_CATEGORY_DISAGREE",
                        "sources": [
                            {
                                "page_id": obs["page_id"],
                                "revision_id": obs["revision_id"],
                                "values": obs["fields"],
                            }
                            for obs in observations
                        ],
                    }
                )
                continue
            typed = [res[1] for _obs, res in present]
            if any(value == "MALFORMED" for value in typed):
                report["malformed"][field_path] += 1
                if field_path == "equipment.patterns":
                    conflict = any(
                        res[0].get("hands") == "Two"
                        and res[0]["slot"] != "Both Hands"
                        or res[0].get("hands") == "One"
                        and res[0]["slot"] == "Both Hands"
                        for _obs, res in present
                    )
                    report["equipment_holds"].append(
                        {
                            "item_key": ITEM_KEY.format(record["item_id"]),
                            "classification": "CONFLICT" if conflict else "UNKNOWN",
                            "reason": "SLOT_HANDS_DISAGREE"
                            if conflict
                            else "UNSUPPORTED_OR_MALFORMED_EQUIPMENT_FACTS",
                            "sources": [
                                {
                                    "page_id": obs["page_id"],
                                    "revision_id": obs["revision_id"],
                                    "values": res[0],
                                }
                                for obs, res in present
                            ],
                        }
                    )
                if len(report["examples"][f"malformed:{field_path}"]) < 5:
                    report["examples"][f"malformed:{field_path}"].append(
                        [record["item_id"], present[0][1][0]]
                    )
                continue
            if any(value != typed[0] for value in typed):
                report["conflict"][field_path] += 1
                if field_path == "equipment.patterns":
                    report["equipment_holds"].append(
                        {
                            "item_key": ITEM_KEY.format(record["item_id"]),
                            "classification": "CONFLICT",
                            "reason": "WIKI_EQUIPMENT_OBSERVATIONS_DISAGREE",
                            "sources": [
                                {
                                    "page_id": obs["page_id"],
                                    "revision_id": obs["revision_id"],
                                    "values": res[0],
                                }
                                for obs, res in present
                            ],
                        }
                    )
                if len(report["examples"][f"conflict:{field_path}"]) < 5:
                    report["examples"][f"conflict:{field_path}"].append(
                        record["item_id"]
                    )
                continue
            rows.append(
                {
                    "field_path": field_path,
                    "item_key": ITEM_KEY.format(record["item_id"]),
                    "sources": [
                        {
                            "page_id": obs["page_id"],
                            "revision_id": obs["revision_id"],
                            "values": res[0],
                        }
                        | (
                            {
                                name: obs[name]
                                for name in ("content_sha256", "url")
                                if name in obs
                            }
                            if field_path in QUALIFIED_PHYSICAL_FIELDS
                            else {}
                        )
                        for obs, res in present
                    ],
                    "typed_value": typed[0],
                }
            )
        for name in UNSUPPORTED_PARAMS:
            if any(name in row["fields"] for row in observations):
                report["unsupported"][name] += 1
    counts = Counter(row["field_path"] for row in rows)
    return rows, report, counts


def packet_bytes(snapshot, item_ids, compiler_sha256):
    records_digest = hashlib.sha256(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    if records_digest != snapshot["snapshot_sha256"]:
        raise ValueError("wiki snapshot digest mismatch")
    rows, report, counts = build(snapshot, item_ids, *physical_field_inputs())
    packet = {
        "compiler": {"path": COMPILER_PATH, "sha256": compiler_sha256},
        "counts": {
            "fields": len(rows),
            "items": len({row["item_key"] for row in rows}),
            "by_field": dict(sorted(counts.items())),
        },
        "policy": {
            "precedence": "TIBIAWIKI_REPLACES_EARLIER_PROMOTION",
            "agreement": "ALL_PAGES_AGREE_ELSE_REPORT",
            "weight_unit": "HUNDREDTHS_OF_OUNCE",
        },
        "promotions": rows,
        "report": {
            "equipment_holds": report["equipment_holds"],
            "physical_field_holds": report["physical_field_holds"],
            "conflict": dict(sorted(report["conflict"].items())),
            "malformed": dict(sorted(report["malformed"].items())),
            "unsupported": dict(sorted(report["unsupported"].items())),
            "examples": {k: v for k, v in sorted(report["examples"].items())},
        },
        "schema": SCHEMA,
        "source": {
            "batch_id": snapshot["batch_id"],
            "path": "imports/tibiawiki/facts/items-stats.json",
            "snapshot_sha256": snapshot["snapshot_sha256"],
            "modifier_external_qualification": {
                "path": str(MODIFIER_SOURCE_HOLD.relative_to(ROOT)),
                "sha256": MODIFIER_SOURCE_HOLD_SHA256,
            },
            "external_qualification": {
                "path": str(SOURCE_HOLD.relative_to(ROOT)),
                "sha256": SOURCE_HOLD_SHA256,
            },
            "bindings_path": str(SOURCE_BINDINGS.relative_to(ROOT)),
            "bindings_sha256": hashlib.sha256(SOURCE_BINDINGS.read_bytes()).hexdigest(),
            "map_owner_inputs": {
                str(path.relative_to(ROOT)): hashlib.sha256(
                    path.read_bytes()
                ).hexdigest()
                for family in ("terrain", "objects")
                for path in sorted((ROOT / f"content/world/{family}").glob("*.json"))
            },
        },
    }
    return (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args(argv)
    snapshot = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    compiler_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    data = packet_bytes(snapshot, content_item_ids(), compiler_sha256)
    if args.check:
        if args.output.read_bytes() != data:
            print(f"packet drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    packet = json.loads(data)
    print(
        json.dumps(
            {
                "counts": packet["counts"],
                "report": {
                    k: packet["report"][k]
                    for k in ("conflict", "malformed", "unsupported")
                },
                "bytes": len(data),
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
