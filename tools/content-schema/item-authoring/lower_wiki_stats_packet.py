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
and blocked states, and do not admit behavior or materialization. Equipment (2b-2) follows
the wiki slot, and ammunition without a slot takes the Extra slot (2b-3); absent
requirements remain UNKNOWN, and unsupported vocations or hand claims hold the complete
pattern. `without` is the vocation `NONE` (2b-3). Use requirements (runes, ammunition, the
Extra slot and slotless Items) are lowered into `use_requirements` with
`enforcement_mode: ON_USE` (2b-3), and the record of written counts and held rows is
written next to the packet. Weight is in hundredths of an ounce, as in Tibia
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

from d289_holds import SEALED_STATE_DECISION, SEALED_STATE_HOLD, require_hits
from key_ring5801_source_selection import load_context, select

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
RECORD = (
    ROOT
    / "docs"
    / "agents"
    / "evidence"
    / "OTV2-20261003-item-equipment-requirements-v1.json"
)
SCHEMA = "OTERYN_ITEM_STATS_PROMOTION/v2"
RECORD_SCHEMA = "OTERYN_ITEM_EQUIPMENT_REQUIREMENTS_RECORD/v1"
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
    # ITEM-SEM-2b-3: the A13 key `none`, a character without a vocation.
    "without": "NONE",
}
VOCATION_ORDER = tuple(VOCATIONS.values())


def known(value):
    return {"state": "KNOWN", "value": value}


# ITEM-SEM-2b-2: the occupancy group of `domain/equipment.rs` (manual §3.4.1). Shields,
# spellbooks and two-handed distance weapons reserve it, so only a quiver may share the left
# hand with them.
NON_QUIVER_LEFT_HAND = "oteryn:equipment-group.non_quiver_left_hand"
HAND_SLOTS = ("Weapon Hand", "Both Hands", "Shield Hand", "Shield")
# Requirement facts that bind use, not equip (ITEM-SEM-2b-3).
REQUIREMENT_FIELDS = ("levelrequired", "vocrequired", "mlrequired")
# D310 (extends D289): main's sealed reward_stack_normalization.json pins i3450's definition
# digest, so its Extra pattern and use requirements are held, not written.
SEALED_STATE_REQUIREMENT_KEYS = frozenset({"oteryn:item.tibia.i3450"})
SEALED_STATE_REQUIREMENT_FIELDS = ("equipment.patterns", "use_requirements")


def vocations(raw):
    """Vocation keys (`without` is `NONE`), or None for an unsupported token.

    `None` is no restriction: the vocation list stays UNKNOWN. A promoted vocation is
    matched against its base key by the equip rule (ITEM-MOVE-2a), not here.
    """
    if raw.strip() == "None":
        return []
    parts = re.split(r",\s*|\s+and\s+", raw.strip().lower())
    values = [VOCATIONS.get(part.rstrip("s")) for part in parts]
    if None in values or len(set(values)) != len(values):
        return None
    return sorted(values, key=VOCATION_ORDER.index)


def use_requirements(fields):
    """Requirement facts that bind use, not equip (ITEM-SEM-2b-2 §2.4): runes, ammunition,
    the Extra slot and slotless Items."""
    slot = fields.get("slot")
    if slot is not None and slot != "Extra Slot" and "mlrequired" not in fields:
        return None
    raw = {
        name: fields[name]
        for name in REQUIREMENT_FIELDS
        if name in fields and not (name == "levelrequired" and fields[name] == "0")
    }
    return raw or None


def use_requirement_row(fields):
    """The use-requirements group (ITEM-SEM-2b-3): level, magic level and vocations that
    RUNE-USE-0 and RANGED-0 enforce at use. A `0` level or magic level writes nothing."""
    raw = use_requirements(fields)
    if raw is None:
        return None, None
    unknown = {"state": "UNKNOWN"}
    value = {
        "min_level": unknown,
        "min_magic_level": unknown,
        "vocations": unknown,
        "enforcement_mode": "ON_USE",
    }
    for name, target in (
        ("levelrequired", "min_level"),
        ("mlrequired", "min_magic_level"),
    ):
        if name in raw:
            parsed = unsigned(raw[name], 65535)
            if parsed is None:
                return raw, "MALFORMED"
            if parsed:
                value[target] = known(parsed)
    if "vocrequired" in raw:
        values = vocations(raw["vocrequired"])
        if values is None:
            return raw, "MALFORMED"
        if values:
            value["vocations"] = known(values)
    if all(
        value[name] == unknown for name in ("min_level", "min_magic_level", "vocations")
    ):
        return None, None
    return raw, {"kind": "USE_REQUIREMENTS", "value": value}


def equipment(fields):
    """One compact equip pattern from the wiki slot (ITEM-SEM-2b-2 §2).

    The slot fixes the hands and the occupancy of `domain/equipment.rs`. Level and
    vocations are equip requirements only outside the Extra slot; there they bind use and
    go to `use_requirements`. Ammunition without a slot takes the Extra slot (2b-3); runes
    get no equipment block. Missing requirements remain UNKNOWN; a vocation token that is
    not a known vocation or a slot/hands disagreement holds the whole pattern.
    """
    slot = fields.get("slot")
    primary = fields.get("primarytype", "")
    if slot is None and primary == "Ammunition":
        slot = "Extra Slot"
    # Runes bind use, not equip.
    if slot is None or "Rune" in primary:
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
        "additional_reserved_slots": known([]),
        "mutually_exclusive_groups": known([]),
        "vocations": unknown,
        "level": unknown,
        "compatibility_rule": unknown,
    }
    hands = fields.get("hands")
    if slot in HAND_SLOTS:
        if hands is not None and (
            hands not in ("One", "Two") or (hands == "Two") != (slot == "Both Hands")
        ):
            return raw, "MALFORMED"
        if slot == "Both Hands" and primary == "Distance Weapons":
            pattern["mutually_exclusive_groups"] = known([NON_QUIVER_LEFT_HAND])
        elif slot == "Both Hands":
            pattern["additional_reserved_slots"] = known(["SHIELD"])
        elif slot != "Weapon Hand":
            if primary in ("Shields", "Spellbooks"):
                pattern["mutually_exclusive_groups"] = known([NON_QUIVER_LEFT_HAND])
            elif primary != "Quivers":
                # The left-hand occupancy of anything but a shield, spellbook or quiver
                # is not fixed by `domain/equipment.rs`.
                pattern["mutually_exclusive_groups"] = unknown
    elif hands is not None and slot != "Extra Slot":
        return raw, "MALFORMED"
    if slot == "Extra Slot":
        # The Extra slot runs no level or vocation check (ITEM-MOVE-WIRE-1 §4).
        return raw, {"kind": "EQUIPMENT_PATTERNS", "value": [pattern]}
    if "levelrequired" in fields:
        level = unsigned(fields["levelrequired"], 65535)
        if level is None:
            return raw, "MALFORMED"
        if level:
            pattern["level"] = known(level)
    if "vocrequired" in fields:
        values = vocations(fields["vocrequired"])
        if values is None:
            return raw, "MALFORMED"
        if values:
            pattern["vocations"] = known(values)
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
    "use_requirements": use_requirement_row,
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


def build(snapshot, item_ids, definitions=None, routed_keys=(), temporal_context=None):
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
        "requirement_holds": [],
        "sealed_state_holds": [],
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
            field_observations = select(record, field_path, temporal_context)
            results = [lower(row["fields"]) for row in field_observations]
            present = [
                (obs, res)
                for obs, res in zip(field_observations, results)
                if res[0] is not None
            ]
            if not present:
                continue
            key = ITEM_KEY.format(record["item_id"])
            if (
                key in SEALED_STATE_REQUIREMENT_KEYS
                and field_path in SEALED_STATE_REQUIREMENT_FIELDS
            ):
                report["sealed_state_holds"].append(
                    {
                        "item_key": key,
                        "field_path": field_path,
                        "decision": SEALED_STATE_DECISION,
                        "reason": SEALED_STATE_HOLD,
                        "held_typed_values": [res[1] for _obs, res in present],
                    }
                )
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
                "Rune" in obs["fields"].get("primarytype", "") for obs in observations
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
                        and res[0].get("slot") != "Both Hands"
                        or res[0].get("hands") == "One"
                        and res[0].get("slot") == "Both Hands"
                        for _obs, res in present
                    )
                    without = any(
                        "without" in res[0].get("vocrequired", "")
                        for _obs, res in present
                    )
                    report["equipment_holds"].append(
                        {
                            "item_key": ITEM_KEY.format(record["item_id"]),
                            "classification": "CONFLICT" if conflict else "UNKNOWN",
                            "reason": "SLOT_HANDS_DISAGREE"
                            if conflict
                            else "UNSUPPORTED_VOCATION_TOKEN"
                            if without
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
                if field_path == "use_requirements":
                    report["requirement_holds"].append(
                        {
                            "item_key": ITEM_KEY.format(record["item_id"]),
                            "classification": "UNKNOWN",
                            "reason": "UNSUPPORTED_OR_MALFORMED_USE_REQUIREMENT",
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
                if field_path == "use_requirements":
                    report["requirement_holds"].append(
                        {
                            "item_key": ITEM_KEY.format(record["item_id"]),
                            "classification": "CONFLICT",
                            "reason": "WIKI_USE_REQUIREMENT_OBSERVATIONS_DISAGREE",
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
                            or (
                                record["item_id"] == 5801
                                and field_path == "physical.weight"
                                and temporal_context
                            )
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
    # Every D310 hold must hit exactly once whenever its Item is in scope.
    require_hits(
        [
            (key, field)
            for key in sorted(SEALED_STATE_REQUIREMENT_KEYS)
            if int(key.rsplit(".i", 1)[1]) in item_ids
            for field in SEALED_STATE_REQUIREMENT_FIELDS
        ],
        [
            (hold["item_key"], hold["field_path"])
            for hold in report["sealed_state_holds"]
        ],
        "item stat promotion",
    )
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
    definitions, routed_keys = physical_field_inputs()
    temporal_context = load_context(ROOT, snapshot, definitions, routed_keys)
    rows, report, counts = build(
        snapshot, item_ids, definitions, routed_keys, temporal_context
    )
    packet = {
        "compiler": {"path": COMPILER_PATH, "sha256": compiler_sha256},
        "counts": {
            "fields": len(rows),
            "items": len({row["item_key"] for row in rows}),
            "by_field": dict(sorted(counts.items())),
        },
        "policy": {
            "precedence": "TIBIAWIKI_REPLACES_EARLIER_PROMOTION",
            "agreement": "ALL_PAGES_AGREE_ELSE_REPORT_EXCEPT_EXACT_ITEM5801_WEIGHT_CAPACITY_QUALIFIER",
            "weight_unit": "HUNDREDTHS_OF_OUNCE",
        },
        "promotions": rows,
        "report": {
            "equipment_holds": report["equipment_holds"],
            "physical_field_holds": report["physical_field_holds"],
            "requirement_holds": report["requirement_holds"],
            "sealed_state_holds": report["sealed_state_holds"],
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
            "temporal_qualification": temporal_context["qualification"],
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


def record_bytes(packet_data):
    """The ITEM-SEM-2b-2 record: written equipment counts and every reported row."""
    packet = json.loads(packet_data)
    counts = Counter()
    for row in packet["promotions"]:
        if row["field_path"] == "use_requirements":
            value = row["typed_value"]["value"]
            counts["use_requirements"] += 1
            for name in ("min_level", "min_magic_level", "vocations"):
                if value[name]["state"] == "KNOWN":
                    counts[f"use_requirements_with_{name}"] += 1
            if "NONE" in value["vocations"].get("value", []):
                counts["use_requirements_vocation:NONE"] += 1
            continue
        if row["field_path"] != "equipment.patterns":
            continue
        pattern = row["typed_value"]["value"][0]
        counts["patterns"] += 1
        counts[f"slot:{pattern['primary_slot']['value']}"] += 1
        for name in ("level", "vocations"):
            if pattern[name]["state"] == "KNOWN":
                counts[f"with_{name}"] += 1
        if "NONE" in pattern["vocations"].get("value", []):
            counts["vocation:NONE"] += 1
        if pattern["additional_reserved_slots"].get("value"):
            counts["reserved:SHIELD"] += 1
        if pattern["mutually_exclusive_groups"].get("value"):
            counts["group:non_quiver_left_hand"] += 1
    report = packet["report"]
    return (
        json.dumps(
            {
                "schema": RECORD_SCHEMA,
                "packet": {
                    "path": str(OUTPUT.relative_to(ROOT)),
                    "sha256": hashlib.sha256(packet_data).hexdigest(),
                },
                "written": dict(sorted(counts.items())),
                "reported": {
                    "equipment_holds": report["equipment_holds"],
                    "requirement_holds": report["requirement_holds"],
                    "sealed_state_holds": report["sealed_state_holds"],
                },
            },
            sort_keys=True,
            ensure_ascii=False,
            indent=1,
        )
        + "\n"
    ).encode("utf-8")



# EQUIP-CONTENT-1 (EQUIP-0 §3.1-§3.2, architect bundle §2.9; control-plane ruling a): the typed
# abilities are a derived view over the existing `skill_modifiers` and `protection` groups
# (`apps/game-server/src/content/item_abilities.rs`). This record lists every source per Item
# and the Canary fallback rows; TibiaWiki first, Canary `items.xml` (D384 pin,
# OTS_HYPOTHESIS_ONLY) only where every wiki page of the Item is silent on the group.
ABILITIES = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20261003-equip-abilities-v1.json"
)
ABILITIES_SCHEMA = "OTERYN_EQUIP_ABILITIES/v1"
CANARY = ROOT / "imports" / "canary" / "items-xml" / "items.xml"
CANARY_MANIFEST = ROOT / "imports" / "canary" / "items-xml" / "manifest.json"
# Every wiki parameter the stats lowering reads into each group: one present means not silent.
WIKI_MODIFIER_FIELDS = ("attrib", *MODIFIER_PERCENTS, "mantra", "elementalbond")
WIKI_RESISTANCE_FIELDS = ("resist",)
CANARY_MODIFIER_POINTS = {
    "magiclevelpoints": "MAGIC_LEVEL_POINTS",
    "skillaxe": "SKILL_AXE",
    "skillclub": "SKILL_CLUB",
    "skilldist": "SKILL_DISTANCE",
    "skillfist": "SKILL_FIST",
    "skillshield": "SKILL_SHIELD",
    "skillsword": "SKILL_SWORD",
    "speed": "SPEED",
}
CANARY_SUPPRESSIONS = {
    "suppressdrown": "SUPPRESS_DROWN",
    "suppressdrunk": "SUPPRESS_DRUNK",
}
# Canary parses `absorbpercentpoison` and `absorbpercentearth` as the same earth damage.
CANARY_ABSORB = {
    "absorbpercentdeath": "DEATH",
    "absorbpercentdrown": "DROWN",
    "absorbpercentearth": "EARTH",
    "absorbpercentenergy": "ENERGY",
    "absorbpercentfire": "FIRE",
    "absorbpercentholy": "HOLY",
    "absorbpercentice": "ICE",
    "absorbpercentlifedrain": "LIFE_DRAIN",
    "absorbpercentmanadrain": "MANA_DRAIN",
    "absorbpercentphysical": "PHYSICAL",
    "absorbpercentpoison": "EARTH",
    "fieldabsorbpercentfire": "FIRE_FIELD",
}
# Other top-level Canary keys of the same two groups. A Known list never drops an observed fact
# of its group, so an Item carrying one of these is held, not written.
CANARY_MODIFIER_OTHER = re.compile(
    r"^(?:skill|suppress|speed|magicshield|manashield$|invisible$|criticalhit|reflect"
    r"|perfectshot|cleave|lifeleech|manaleech|(?:health|mana)(?:gain|ticks)$)"
    r"|magiclevelpoints$"
)
CANARY_ABSORB_OTHER = re.compile(r"absorbpercent")
NO_SOURCE = (
    "no source in the TibiaWiki stats snapshot nor in Canary items.xml (D384 pin); "
    "a follow-up question for EQUIP-RT-1"
)


def load_canary_top_level():
    """Canary Item id -> top-level attribute map (lower-cased keys); ranges expand.

    Nested attributes (imbuement slots, script requirements) are not Item abilities.
    """
    import xml.etree.ElementTree as ET

    out = {}
    for item in ET.parse(CANARY).getroot().iter("item"):
        attrs = {
            node.get("key").lower(): node.get("value")
            for node in item.findall("attribute")
        }
        if item.get("id"):
            ids = [int(item.get("id"))]
        else:
            ids = range(int(item.get("fromid")), int(item.get("toid")) + 1)
        for item_id in ids:
            out[item_id] = attrs
    return out


def canary_pin():
    manifest = json.loads(CANARY_MANIFEST.read_text(encoding="utf-8"))
    pinned = next(row for row in manifest["files"] if row["path"] == "items.xml")
    actual = hashlib.sha256(CANARY.read_bytes()).hexdigest()
    if actual != pinned["sha256"]:
        raise ValueError("Canary items.xml digest differs from its D384 pin")
    return {
        "path": str(CANARY.relative_to(ROOT)),
        "sha256": actual,
        "revision": manifest["source"]["revision"],
        "evidence": "OTS_HYPOTHESIS_ONLY",
    }


def canary_points(raw):
    if raw is None or not INTEGER.match(raw):
        return None
    value = int(raw)
    return value if value != 0 and -(2**31) <= value < 2**31 else None


def canary_modifiers(attrs):
    """(observed keys, typed modifier list | "MALFORMED" | "UNMAPPED" | None)."""
    mapped = {k: attrs[k] for k in (*CANARY_MODIFIER_POINTS, *CANARY_SUPPRESSIONS) if k in attrs}
    other = sorted(
        k
        for k in attrs
        if CANARY_MODIFIER_OTHER.search(k)
        and k not in CANARY_MODIFIER_POINTS
        and k not in CANARY_SUPPRESSIONS
    )
    if not mapped and not other:
        return None, None
    observed = {k: attrs[k] for k in sorted({*mapped, *other})}
    if other:
        return observed, "UNMAPPED"
    values = {}
    for key, kind in CANARY_MODIFIER_POINTS.items():
        if key in mapped:
            points = canary_points(mapped[key])
            if points is None:
                return observed, "MALFORMED"
            # The pinned items.xml states speed in displayed units (ability_sources speed_unit).
            values[kind] = {"kind": "SIGNED_POINTS", "value": points}
    for key, kind in CANARY_SUPPRESSIONS.items():
        if key in mapped:
            if mapped[key] != "1":
                return observed, "MALFORMED"
            values[kind] = {"kind": "BOOLEAN", "value": True}
    unknown = {"state": "UNKNOWN"}
    return observed, [
        {
            "evaluation_phase": unknown,
            "kind": kind,
            "parameter": known(values[kind]),
            "priority": unknown,
            "target_domain": unknown,
        }
        for kind in sorted(values)
    ]


def canary_resistances(attrs):
    """(observed keys, typed resistance list | "MALFORMED" | "UNMAPPED" | None)."""
    observed = {k: attrs[k] for k in sorted(attrs) if CANARY_ABSORB_OTHER.search(k)}
    if not observed:
        return None, None
    if any(k not in CANARY_ABSORB for k in observed):
        return observed, "UNMAPPED"
    values = {}
    for key, raw in observed.items():
        kind = CANARY_ABSORB[key]
        if raw is None or not INTEGER.match(raw) or not -100 <= int(raw) <= 100:
            return observed, "MALFORMED"
        if int(raw) == 0 or values.get(kind, int(raw)) != int(raw):
            return observed, "MALFORMED"
        values[kind] = int(raw)
    return observed, [
        {
            "kind": kind,
            "percent": known({"denominator": 1, "numerator": values[kind]}),
        }
        for kind in RESISTANCE_KINDS
        if kind in values
    ]


def wiki_speed(fields):
    match = re.search(r"(?:^|,)\s*speed ([+-]?[0-9]+)\s*(?:,|$)", fields.get("attrib", "").lower())
    return int(match[1]) if match else None


def timed(definition):
    """EQUIP-0 §3.2: `charges.count` or `temporal.duration` present."""
    semantics = definition.get("semantics", {})
    for group, leaf in (("charges", "count"), ("temporal", "duration")):
        source = semantics.get(group, {})
        if source.get("state") == "KNOWN":
            if source["value"].get(leaf, {}).get("state") == "KNOWN":
                return True
    return False


def ability_sources(snapshot, canary, item_ids, definitions):
    """The EQUIP-CONTENT-1 record: every source per Item, the timed flag and fallback rows."""
    wiki = {record["item_id"]: record["observations"] for record in snapshot["records"].values()}
    items, holds, speed_pairs = [], [], []
    for item_id in sorted(item_ids):
        key = ITEM_KEY.format(item_id)
        observations = wiki.get(item_id, [])
        attrs = canary.get(item_id, {})
        groups = (
            (
                "skill_modifiers.modifiers",
                WIKI_MODIFIER_FIELDS,
                canary_modifiers(attrs),
            ),
            (
                "protection.resistances",
                WIKI_RESISTANCE_FIELDS,
                canary_resistances(attrs),
            ),
        )
        wiki_sources = [
            {
                "class": "TIBIAWIKI",
                "page_id": obs["page_id"],
                "revision_id": obs["revision_id"],
                "values": {
                    name: obs["fields"][name]
                    for name in (*WIKI_MODIFIER_FIELDS, *WIKI_RESISTANCE_FIELDS)
                    if name in obs["fields"]
                },
            }
            for obs in observations
            if any(
                name in obs["fields"]
                for name in (*WIKI_MODIFIER_FIELDS, *WIKI_RESISTANCE_FIELDS)
            )
        ]
        canary_observed = {}
        fallback = []
        for field_path, wiki_fields, (observed, typed) in groups:
            if observed is None:
                continue
            canary_observed |= observed
            if any(name in obs["fields"] for obs in observations for name in wiki_fields):
                continue
            reason = None
            if "semantics" not in definitions.get(key, {}):
                reason = "NO_CANONICAL_ITEM_SEMANTICS"
            elif typed in ("MALFORMED", "UNMAPPED"):
                reason = f"{typed}_CANARY_VALUE"
            if reason:
                holds.append(
                    {
                        "item_key": key,
                        "field_path": field_path,
                        "reason": reason,
                        "canary_attributes": observed,
                    }
                )
                continue
            fallback.append({"field_path": field_path, "value": typed})
        for obs in observations:
            speed = wiki_speed(obs["fields"])
            if speed is not None and "speed" in attrs:
                speed_pairs.append((key, speed, attrs["speed"]))
        if not wiki_sources and not canary_observed:
            continue
        if "semantics" not in definitions.get(key, {}):
            continue
        sources = list(wiki_sources)
        if canary_observed:
            sources.append(
                {
                    "class": "OTS_HYPOTHESIS_ONLY",
                    "canary_item_id": item_id,
                    "attributes": canary_observed,
                }
            )
        items.append(
            {
                "item_key": key,
                "timed": timed(definitions[key]),
                "sources": sources,
                "fallback": fallback,
            }
        )
    # EQUIP-0 §2 expected Canary speed in a doubled unit; the pinned items.xml agrees 1:1 with
    # the wiki on every Item that has both, so the conversion factor is 1. Any other pair fails.
    disagree = [pair for pair in speed_pairs if str(pair[1]) != pair[2].lstrip("+")]
    if disagree or not speed_pairs:
        raise ValueError(f"Canary speed unit is not the displayed unit: {disagree[:5]}")
    return items, holds, len(speed_pairs)


def abilities_bytes(snapshot, item_ids, definitions, packet_data, canary=None):
    canary = load_canary_top_level() if canary is None else canary
    items, holds, speed_pairs = ability_sources(snapshot, canary, item_ids, definitions)
    by_field = Counter(row["field_path"] for item in items for row in item["fallback"])
    return (
        json.dumps(
            {
                "schema": ABILITIES_SCHEMA,
                "policy": {
                    "precedence": "TIBIAWIKI_THEN_CANARY_WHERE_EVERY_WIKI_PAGE_IS_SILENT_ON_THE_GROUP",
                    "derived_view": "apps/game-server/src/content/item_abilities.rs",
                    "speed_unit": {
                        "canary_to_displayed": "1:1",
                        "agreeing_wiki_canary_items": speed_pairs,
                    },
                    "timed": "charges.count or temporal.duration KNOWN (EQUIP-0 §3.2)",
                    "abilities_without_source": {
                        "LIGHT": NO_SOURCE,
                        "STAT_BOOST": NO_SOURCE,
                    },
                },
                "source": {
                    "canary": canary_pin(),
                    "stats_packet": {
                        "path": str(OUTPUT.relative_to(ROOT)),
                        "sha256": hashlib.sha256(packet_data).hexdigest(),
                    },
                    "wiki_snapshot_sha256": snapshot["snapshot_sha256"],
                },
                "counts": {
                    "items": len(items),
                    "timed_items": sum(item["timed"] for item in items),
                    "fallback_items": sum(bool(item["fallback"]) for item in items),
                    "fallback_by_field": dict(sorted(by_field.items())),
                    "holds": len(holds),
                },
                "items": items,
                "holds": holds,
            },
            sort_keys=True,
            ensure_ascii=False,
            indent=1,
        )
        + "\n"
    ).encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--record", type=Path, default=RECORD)
    parser.add_argument("--abilities", type=Path, default=ABILITIES)
    args = parser.parse_args(argv)
    snapshot = json.loads(SNAPSHOT.read_text(encoding="utf-8"))
    compiler_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    item_ids = content_item_ids()
    data = packet_bytes(snapshot, item_ids, compiler_sha256)
    record = record_bytes(data)
    abilities = abilities_bytes(snapshot, item_ids, physical_field_inputs()[0], data)
    if args.check:
        if (
            args.output.read_bytes() != data
            or args.record.read_bytes() != record
            or args.abilities.read_bytes() != abilities
        ):
            print(f"packet drift against {args.output}", file=sys.stderr)
            return 1
        print(json.dumps({"check": "PASS", "bytes": len(data)}))
        return 0
    args.output.write_bytes(data)
    args.record.write_bytes(record)
    args.abilities.write_bytes(abilities)
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
