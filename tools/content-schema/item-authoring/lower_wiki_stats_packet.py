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
attacks, imbuement slots, equipment patterns and weight. Equipment requires an explicit
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
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SNAPSHOT = ROOT / "imports" / "tibiawiki" / "facts" / "items-stats.json"
ITEM_INDEX = ROOT / "content" / "items" / "index.json"
SOURCE_BINDINGS = ROOT / "imports" / "crystalserver" / "bindings" / "items.json"
OUTPUT = (
    ROOT / "docs" / "agents" / "evidence" / "OTV2-20260930-item-stats-promotion-v2.json"
)
COMPILER_PATH = "tools/content-schema/item-authoring/lower_wiki_stats_packet.py"
SCHEMA = "OTERYN_ITEM_STATS_PROMOTION/v2"
ITEM_KEY = "oteryn:item.tibia.i{}"

INTEGER = re.compile(r"^[+-]?[0-9]+$")
UNSIGNED = re.compile(r"^[0-9]+$")
WEIGHT = re.compile(r"^([0-9]+)\.([0-9]{2})$")
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
        hands not in ("One", "Two") or slot not in ("Weapon Hand", "Both Hands")
    ):
        return raw, "MALFORMED"
    if slot == "Both Hands":
        if hands == "One":
            return raw, "MALFORMED"
        pattern["additional_reserved_slots"] = known(["SHIELD"])
    elif slot == "Weapon Hand" and hands is not None:
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
    return int(match.group(1)) * 100 + int(match.group(2)) if match else None


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
    bound = {row["target"]["key"] for row in bindings}
    index = json.loads(ITEM_INDEX.read_text(encoding="utf-8"))
    ids = set()
    for shard in index["shards"]:
        document = json.loads((ROOT / shard).read_text(encoding="utf-8"))
        for record in document["records"]:
            key = record["definition"]["identity"]["key"]
            if key in bound:
                ids.add(int(key.rsplit(".i", 1)[1]))
    return ids


def build(snapshot, item_ids):
    rows = []
    report = {
        "conflict": Counter(),
        "malformed": Counter(),
        "unsupported": Counter(),
        "examples": defaultdict(list),
        "equipment_holds": [],
    }
    # Records are keyed by Item key or, without an Item record, by the bare Tibia id (#1325).
    for record in sorted(
        snapshot["records"].values(), key=lambda record: record["item_id"]
    ):
        if record["item_id"] not in item_ids:
            continue
        observations = record["observations"]
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
    rows, report, counts = build(snapshot, item_ids)
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
