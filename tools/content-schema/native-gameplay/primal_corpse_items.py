#!/usr/bin/env python3
"""Qualify the four Primal Menace corpse Items on existing Item owners.

zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
data/items/items.xml:73328-73347 supplies durations, targets and 30/30/15 capacities.
data/items/appearances.dat supplies exact object flags (blob SHA-256 below).
src/protobuf/appearances.proto:152-190 gives container/cumulative/unmove/corpse
field numbers; src/items/items.cpp:172 selects the container type from that flag.
The terminal object39527 is not a container; no capacity is inferred for it.
items.cpp:236/244/251 selects pickup/movement/stack flags. items.hpp:300/384
supplies the zero-weight/non-pausing defaults; this XML has no overrides.
The existing Oteryn Item lifetime owner uses DURABLE_ABSOLUTE_DEADLINE for
non-pausing decay (durability/native_world_items.rs:143-184). No other Item's
facts are used as a template; all remaining semantic groups stay UNKNOWN.

WorldObject bindings, collision, fluid-source ownership and encounter admission
are unchanged. This overlay qualifies Item data, not a playable boss instance.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
APPEARANCES_SHA256 = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
# item, container capacity, duration milliseconds, decay target
CHAIN = ((39530, 30, 10_000, 39529), (39529, 30, 300_000, 39528),
         (39528, 15, 300_000, 39527), (39527, None, 60_000, None))
OWNED = frozenset(row[0] for row in CHAIN)
OBJECT_SHARD = "content/world/objects/objects-11500-11999.json"
UNKNOWN_GROUPS = ("presentation", "classification", "equipment", "weapon",
                  "protection", "skill_modifiers", "charges", "imbuement",
                  "use_transform", "trade_restrictions", "fluid", "readable_writeable")


def verify_source(crystal: Path) -> None:
    """Recheck the pinned XML and protobuf flags; never execute donor code."""
    def blob(path):
        return subprocess.run(
            ["git", "-C", str(crystal), "show", f"{PIN}:{path}"],
            check=True, capture_output=True,
        ).stdout

    xml = ET.fromstring(blob("data/items/items.xml"))
    item_defaults = blob("src/items/items.hpp").decode()
    if "int32_t weight = 0;" not in item_defaults or "bool stopTime = false;" not in item_defaults:
        raise ValueError("Primal source Item defaults differ")
    for item, capacity, duration, target in CHAIN:
        nodes = xml.findall(f"item[@id='{item}']")
        if len(nodes) != 1:
            raise ValueError(f"Primal source Item is absent or duplicated: {item}")
        attributes = {}
        for child in nodes[0].findall("attribute"):
            key = child.attrib["key"].lower()
            if key in attributes:
                raise ValueError(f"Primal source Item attribute is duplicated: {item}/{key}")
            attributes[key] = child.attrib["value"]
        expected = {"duration": str(duration // 1000), "decayto": str(target or 0)}
        if capacity is not None:
            expected["containersize"] = str(capacity)
        if {key: value for key, value in attributes.items()
                if key in {"duration", "decayto", "containersize"}} != expected:
            raise ValueError(f"Primal source Item lifecycle differs: {item}")
        if any(key in attributes for key in ("weight", "stopduration", "pickupable", "allowpickupable")):
            raise ValueError(f"Primal source Item default is overridden: {item}")

    raw = blob("data/items/appearances.dat")
    if hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256:
        raise ValueError("Primal source appearance blob differs")
    sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
    import client_map_reader as wire

    def selected(payload, wanted):
        grouped = {}
        for number, kind, value in wire.fields(payload):
            if number in wanted:
                grouped.setdefault(number, []).append((kind, value))
        return grouped

    found = set()
    for number, kind, payload in wire.fields(raw):
        if number != 1:
            continue
        if kind != 2:
            raise ValueError("Primal appearance object is not a protobuf message")
        fields = selected(payload, {1, 3})
        item = wire.single(fields, 1, 0, "Primal object")
        if item not in OWNED:
            continue
        if item in found:
            raise ValueError("Primal appearance identity duplicated")
        found.add(item)
        flags = selected(wire.single(fields, 3, 2, "Primal flags"), {5, 6, 14, 18, 42, 43})
        actual = {flag: wire.single(flags, flag, 0, "Primal flag", required=False)
                  for flag in (5, 6, 14, 18, 42, 43)}
        if any(value not in (None, 0, 1) for value in actual.values()):
            raise ValueError("Primal appearance flag is not boolean")
        if {flag: bool(value) for flag, value in actual.items()} != {
                5: item != 39527, 6: False, 14: item == 39530, 18: False, 42: True, 43: False}:
            raise ValueError(f"Primal appearance flags differ: {item}")
    if found != OWNED:
        raise ValueError("Primal source appearance closure incomplete")


def corpse_definition(definition, capacity, duration, target, movable, weight=0):
    """Existing non-pickupable corpse Item policy; keep unrelated groups unknown."""
    replacement = {
        "kind": "Item", "identity": copy.deepcopy(definition["identity"]),
        "client_projection": definition["client_projection"],
        "materializable": True, "stack_class": "NonStackable",
        "semantics": {name: {"state": "UNKNOWN"} for name in UNKNOWN_GROUPS},
    }
    replacement["semantics"]["physical"] = {"state": "KNOWN", "value": {
        "weight": {"state": "KNOWN", "value": weight},
        "movable": {"state": "KNOWN", "value": movable},
        "pickupable": {"state": "KNOWN", "value": False},
    }}
    replacement["semantics"]["stack"] = {"state": "KNOWN", "value": {
        "stackable": {"state": "KNOWN", "value": False},
        "stack_max": {"state": "NOT_APPLICABLE"},
    }}
    replacement["semantics"]["container"] = (
        {"state": "KNOWN", "value": {"capacity": {"state": "KNOWN", "value": capacity}}}
        if capacity is not None else {"state": "NOT_APPLICABLE"})
    target_field = (
        {"state": "KNOWN", "value": {
            "key": f"oteryn:item.tibia.i{target}", "revision": "definition-r1"}}
        if target is not None else {"state": "NOT_APPLICABLE"})
    replacement["semantics"]["temporal"] = {"state": "KNOWN", "value": {
        "consumption_mode": {"state": "KNOWN", "value": "DURABLE_ABSOLUTE_DEADLINE"},
        "duration": {"state": "KNOWN", "value": duration},
        "stop_duration": {"state": "KNOWN", "value": False},
        "decay_target": target_field,
    }}
    return replacement


def apply_admissions(definitions, root=ROOT):
    definitions = list(definitions)
    indexed = {definition["identity"]["key"]: definition for definition in definitions}
    if len(indexed) != len(definitions):
        raise ValueError("Primal Item input contains duplicate identities")
    objects = json.loads((root / OBJECT_SHARD).read_bytes())["records"]
    pointers = {record["provenance"]["item_pointer"]["key"]: record for record in objects
                if record.get("provenance", {}).get("item_pointer")}
    replacements = []
    for item, capacity, duration, target in CHAIN:
        key = f"oteryn:item.tibia.i{item}"
        definition, world_object = indexed[key], pointers[key]
        if (world_object["kind"] != {"state": "KNOWN", "value": "corpse"}
                or world_object["corpse"]["corpse"] != {"state": "KNOWN", "value": True}
                or world_object["corpse"]["player_corpse"] != {"state": "KNOWN", "value": False}
                or world_object["movable"] != {"state": "KNOWN", "value": item != 39530}
                or world_object["provenance"]["item_pointer"]["revision"] != "definition-r1"):
            raise ValueError("Primal Item/WorldObject binding differs: " + key)
        replacement = corpse_definition(definition, capacity, duration, target, item != 39530)
        replacements.append((definition, replacement))
    for definition, replacement in replacements:
        definition.clear()
        definition.update(replacement)


def outputs(root=ROOT):
    shards = {path: json.loads(path.read_bytes())
              for path in (root / "content/items/definitions").glob("items-*.json")}
    apply_admissions([row["definition"] for shard in shards.values() for row in shard["records"]], root)
    return {path: (json.dumps(shard, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
            for path, shard in shards.items()
            if any(int(row["definition"]["identity"]["key"].rsplit("i", 1)[1]) in OWNED
                   for row in shard["records"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--crystal", type=Path)
    args = parser.parse_args()
    if args.crystal is not None:
        verify_source(args.crystal)
    for path, raw in outputs().items():
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit(f"stale Primal corpse Items: {path.relative_to(ROOT)}")
        else:
            path.write_bytes(raw)


if __name__ == "__main__":
    main()
