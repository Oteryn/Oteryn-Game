#!/usr/bin/env python3
"""Lower the accepted CRAFT-1 reference facts; never activate conversion runtime.

`content --check` is called by the existing content validation/CI entry point.
Source capture is separate: generation is offline and verifies the extracted snapshot.
"""
from __future__ import annotations

import argparse
from collections import Counter
from fractions import Fraction
from functools import reduce
import hashlib
import math
from pathlib import Path
import sys

import jsonschema

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-migration"))
sys.path.insert(0, str(ROOT / "tools/content-schema/item-authoring"))
from world_project_v2_to_tree import canonical_bytes, load, write, write_registry
from lower_item_consumption_packet import items_xml_lines
sys.path.insert(0, str(ROOT / "tools/content-schema/reward-claim-authoring"))
from reward_claim_authoring import item_ref
from digging import append_rows, map_facts

FAMILY = "item_conversion"
DIRECTORY = "content/craft/"
INDEX = DIRECTORY + "index.json"
CATALOGUE = DIRECTORY + "recipes.json"
SOURCES = DIRECTORY + "sources.json"
SNAPSHOT_SHA256 = "a503fa38c9f22fd9695433d97dfe9e2528cf61f368e54ed059851cbf82b58cde"
XML_SHA256 = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
CURVE = {"base": 10000, "per_skill": 597, "skill_offset": 10,
         "minimum": 10000, "maximum": 50000, "roll_scale": 1000, "roll_max": 100}
MESSAGES = {"success": "", "no_room": "not enough room", "too_heavy": "too heavy",
            "no_fish": "no fish", "not_owner": "You are not the owner.",
            "empty": "There was just rubbish in it."}


def source_snapshot():
    raw = (ROOT / SOURCES).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SNAPSHOT_SHA256:
        raise ValueError("craft source snapshot digest mismatch")
    if hashlib.sha256((ROOT / "imports/canary/items-xml/items.xml").read_bytes()).hexdigest() != XML_SHA256:
        raise ValueError("craft XML source digest mismatch")
    return load(ROOT / SOURCES)


def item(item_id):
    return item_ref(f"oteryn:item.tibia.i{item_id}")


def operand(role, item_id, effect, *, count=1, sub_type=None, output=None, lifecycle=None):
    result = {"role": role, "item_id": item(item_id), "count": count, "effect": effect}
    if sub_type is not None:
        result["sub_type"] = sub_type
    if role == "INGREDIENT":
        result["consume_on"] = "SUCCESS"
    if output is not None:
        result["output_item_id"] = item(output)
        result["lifecycle"] = lifecycle or "REPLACE_INSTANCE"
        if sub_type is not None:
            result["output_sub_type"] = 0
    return result


def output(item_id, count=1):
    return {"item_id": item(item_id), "count": count}


def fixed(*entries):
    return {"kind": "fixed", "entries": list(entries)}


def weighted(entries):
    """Flatten disjoint source groups exactly, combining repeated output identities."""
    combined = Counter()
    for item_id, probability in entries:
        combined[item_id] += probability
    denominator = reduce(math.lcm, (p.denominator for p in combined.values()), 1)
    return {"kind": "weighted", "alternatives": [
        {"weight": int(combined[i] * denominator), "entries": [] if i is None else [output(i)]}
        for i in sorted(combined, key=lambda i: -1 if i is None else i)]}


def build():
    snapshot = source_snapshot()
    recipes = []

    def row(name, source_name, lines, used, target, operands, outputs, **extras):
        source = snapshot["sources"][source_name]
        recipes.append({"kind": FAMILY,
                        "identity": {"key": f"oteryn:item-conversion.{name}", "revision": "craft-r1"},
                        "recipe": {"input_item": item(used), "target": target,
                                   "operands": operands, "outputs": outputs, "tool": "none",
                                   "capacity_check": True, "result_message": MESSAGES,
                                   "source": {**source, "line_start": lines[0], "line_end": lines[1],
                                              "evidence": "OtsHypothesisOnly", "differences": []},
                                   **extras}, "fields": []})

    tables = snapshot["facts"]["baking"]["tables"]
    for vessel in tables["liquidContainers"]:
        for fluid in (1, 9):
            result = (8195 if vessel == 133 else 3604) if fluid == 1 else 6276
            row(f"flour-{vessel}-{fluid}", "baking", (8, 23), 3603, {"kind": "carried"},
                [operand("USED", 3603, "CONSUME"),
                 operand("TARGET", vessel, "TRANSFORM_TO", sub_type=fluid,
                         output=2874 if vessel == 133 and fluid == 1 else vessel,
                         lifecycle="PRESERVE_INSTANCE")], fixed(output(result)))
    for name, used, target, result in [("chocolate-dough", 6276, 6574, 8018),
                                       ("garlic-dough", 8195, 8197, 8196)]:
        row(name, "baking", (24, 31), used, {"kind": "carried"},
            [operand("USED", used, "CONSUME"), operand("TARGET", target, "CONSUME")], fixed(output(result)))
    row("garlic-tray", "baking", (32, 34), 8196, {"kind": "carried"},
        [operand("USED", 8196, "CONSUME"), operand("TARGET", 3464, "TRANSFORM_TO", output=8198)], fixed())
    for oven in tables["oven"]:
        for used, result in [(3604, 3600), (6276, 6277), (8018, 8019), (8196, 8194)]:
            row(f"bake-{used}-{oven}", "baking", (35, 48), used,
                {"kind": "map_item", "definition": item(oven)},
                [operand("USED", used, "TRANSFORM_TO", output=result)], fixed())
        row(f"cookies-{oven}", "baking", (42, 44), 8198,
            {"kind": "map_item", "definition": item(oven)},
            [operand("USED", 8198, "TRANSFORM_TO", output=3464)], fixed(output(8199, 12)))
    row("sugar-oat", "baking", (49, 51), 5466, {"kind": "carried"},
        [operand("USED", 5466, "TRANSFORM_TO", output=12802), operand("TARGET", 3605, "CONSUME")], fixed())
    # The accepted C1 explicitly adopts this branch although the upstream action id list omits 5466.
    recipes[-1]["recipe"]["source"]["differences"].append("C1 adopts sugar cane despite its omitted upstream action registration")
    for mill in tables["millstones"]:
        for used, result in [(3605, 3603), (30975, 31686)]:
            row(f"mill-{used}-{mill}", "baking", (52, 59), used,
                {"kind": "map_item", "definition": item(mill)},
                [operand("USED", used, "TRANSFORM_TO", output=result)], fixed())

    ordinary = list(range(4597, 4603)) + list(range(4609, 4615)) + list(range(629, 635)) + [12561, 12563, 21414]
    for rod, bait, catch, source_name in [(3483, 3492, 3578, "fishing"), (9306, 953, 9307, "mechanical_fishing")]:
        for water in ordinary + [7236, 13988]:
            entries = fixed(output(catch))
            cooldown = 1200000
            if water == 7236:
                entries = weighted([(i, Fraction(w, 100)) for i, w in [(7158, 1), (3580, 3), (7159, 6), (catch, 90)]])
                cooldown = 900000
            elif water == 13988:
                entries = weighted([(13992, Fraction(4, 100)), (catch, Fraction(96, 100))])
                cooldown = 3600000
            row(f"fish-{rod}-{water}", source_name, (1, 141) if rod == 3483 else (1, 84), rod,
                {"kind": "tile_kind", "definition": item(water)},
                [operand("USED", rod, "KEEP"), operand("INGREDIENT", bait, "CONSUME")], entries,
                tool="rod", skill="Fishing", chance_curve=CURVE, tile_cooldown_ms=cooldown)
            differences = recipes[-1]["recipe"]["source"]["differences"]
            if water == 13988:
                differences.append("PARITY_PENDING: C4a bounds the once-per-map-load source at 3600000 ms")
            if rod == 9306:
                differences.append("PARITY_PENDING: C4 refuses missing bait before granting a skill try")
                if water not in snapshot["facts"][source_name]["tables"]["waterIds"]:
                    differences.append("C4a adopts the ordinary rod's complete water set")
            if water == 21414:
                differences.append("Dawnport quest catch belongs to DAWNPORT-1; only the plain catch is a conversion")

    fish = snapshot["facts"]["fishing"]["tables"]
    dirty = []
    for table, percent in [("lootVeryRare1", 1), ("lootRare1", 2), ("lootCommon1", 7), ("lootTrash", 90)]:
        dirty += [(i, Fraction(percent, 100 * len(fish[table]))) for i in fish[table]]
    row("dirty-water", "fishing", (61, 74), 3483,
        {"kind": "tile_kind", "definition": item(12560)},
        [operand("USED", 3483, "KEEP")], weighted(dirty), tool="rod")
    row("corpse-ordinary", "fishing", (10, 58), 3483,
        {"kind": "map_item", "definition": item(9582)}, [operand("USED", 3483, "KEEP")],
        weighted([(i, Fraction(w, 10000)) for i, w in
                  [(3026, 500), (3029, 301), (3032, 201), (281, 51), (282, 51), (9303, 11), (None, 8885)]]),
        tool="rod", target_effect={"effect": "TRANSFORM_TO", "output_item_id": item(9583)})
    mechanical = snapshot["facts"]["mechanical_fishing"]["tables"]
    entries = []
    for table, percent in [("lootVeryRare", 1), ("lootRare", 2), ("lootCommon", 7), ("lootTrash", 90)]:
        entries += [(i, Fraction(percent, 100 * len(mechanical[table]))) for i in mechanical[table]]
    row("corpse-mechanical", "mechanical_fishing", (1, 36), 9306,
        {"kind": "map_item", "definition": item(9582)}, [operand("USED", 9306, "KEEP")],
        weighted(entries), tool="rod", target_effect={"effect": "RETIRE"})
    append_rows(row, snapshot, items_xml_lines(), item, operand, fixed)
    return sorted(recipes, key=lambda r: r["identity"]["key"])


def outputs():
    records = build()
    jsonschema.Draft202012Validator(load(Path(__file__).with_name("schema.json"))).validate(records)
    timer_facts = map_facts(source_snapshot(), items_xml_lines(), XML_SHA256,
                           load(ROOT / "imports/canary/items-xml/manifest.json")["source"]["revision"])
    return {DIRECTORY + "map-facts.json": timer_facts,
            CATALOGUE: {"schema": "OTERYN_ITEM_CONVERSION_SHARD/v1", "family": FAMILY, "records": records},
            INDEX: {"schema": "OTERYN_FAMILY_INDEX/v1", "family": FAMILY, "record_count": len(records),
                    "shards": [CATALOGUE], "map_facts": DIRECTORY + "map-facts.json", "sources": {"path": SOURCES, "sha256": SNAPSHOT_SHA256},
                    "runtime_switch_authorized": False,
                    "contract": "docs/architecture/reviews/OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md"}}


def content_command(check=False):
    generated = outputs()
    # Use the existing content-tree serialization and writes, preserving every other family.
    project, manifest, lock = [load(ROOT / f"content/{n}.json") for n in ("project", "manifest", "content.lock")]
    project["migrated_families"] = list(dict.fromkeys([*project["migrated_families"], FAMILY]))
    manifest["families"][FAMILY] = {"records": generated[INDEX]["record_count"], "index": INDEX}
    paths = {r["path"] for r in manifest["managed_files"]} | set(generated) | {SOURCES}
    manifest["managed_files"] = [{"path": p} for p in sorted(paths)]
    lock["family_counts"][FAMILY] = generated[INDEX]["record_count"]
    registries = {f"content/{name}.json": doc for name, doc in zip(("project", "manifest", "content.lock"), (project, manifest, lock), strict=True)}
    for path, expected in generated.items():
        if check:
            if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != canonical_bytes(expected):
                raise ValueError(f"craft content drift: {path}")
        else:
            write(path, expected)
    for path, expected in registries.items():
        if check:
            if load(ROOT / path) != expected:
                raise ValueError(f"craft registration drift: {path}")
        else:
            write_registry(path, expected)
    print(f"PASS item_conversion records={generated[INDEX]['record_count']} check={check}")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["content"])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raise SystemExit(content_command(args.check))
