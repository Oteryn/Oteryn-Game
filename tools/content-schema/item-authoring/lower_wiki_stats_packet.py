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
attacks, imbuement slots and weight. Weight is in hundredths of an ounce, as in Tibia
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
    index = json.loads(ITEM_INDEX.read_text(encoding="utf-8"))
    ids = set()
    for shard in index["shards"]:
        document = json.loads((ROOT / shard).read_text(encoding="utf-8"))
        for record in document["records"]:
            key = record["definition"]["identity"]["key"]
            ids.add(int(key.rsplit(".i", 1)[1]))
    return ids


def build(snapshot, item_ids):
    rows = []
    report = {
        "conflict": Counter(),
        "malformed": Counter(),
        "unsupported": Counter(),
        "examples": defaultdict(list),
    }
    # Records are keyed by Item key or, without an Item record, by the bare Tibia id (#1325).
    for record in sorted(
        snapshot["records"].values(), key=lambda record: record["item_id"]
    ):
        if record["item_id"] not in item_ids:
            continue
        observations = record["observations"]
        for field_path, lower in FIELDS.items():
            results = [lower(row["fields"]) for row in observations]
            present = [
                (obs, res)
                for obs, res in zip(observations, results)
                if res[0] is not None
            ]
            if not present:
                continue
            typed = [res[1] for _obs, res in present]
            if any(value == "MALFORMED" for value in typed):
                report["malformed"][field_path] += 1
                if len(report["examples"][f"malformed:{field_path}"]) < 5:
                    report["examples"][f"malformed:{field_path}"].append(
                        [record["item_id"], present[0][1][0]]
                    )
                continue
            if any(value != typed[0] for value in typed):
                report["conflict"][field_path] += 1
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
