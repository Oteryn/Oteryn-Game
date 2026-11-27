"""Admit the source-backed Ground custody of declared creature corpses.

Use the same exact WorldObject/Item join as the Bug corpse fix. The pinned
Canary XML supplies physical capacity and decay; the captured Crystal rows
cover IDs absent from that import. Existing known Item facts remain binding.
"""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import re

import grizzly_shop_profiles as grizzly
import quest_corpse_profiles as quest

ROOT = Path(__file__).resolve().parents[2]
ITEM = re.compile(r"oteryn:item\.tibia\.i([1-9][0-9]*)\Z")
BINDINGS = "imports/crystalserver/bindings/items.json"
CRYSTAL_CAPTURE = "tools/content-schema/native-gameplay/verified-loot-corpse-crystal.json"
CRYSTAL_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CRYSTAL_XML_SHA = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
GROUPS = ("physical", "stack", "container", "temporal")
MAX_CHAIN = 8
COOKBOOK = "content/quests/hot-cuisine-batch3.json"


def read(root, path):
    return json.loads((root / path).read_bytes())


def key(number):
    return f"oteryn:item.tibia.i{number}"


def creature_roots(creatures):
    roots = set()
    for record in creatures["records"]:
        details = record["profile"]["data"]["profile"].get("details") or {}
        corpse = details.get("corpse_item")
        if corpse and corpse["family"] == "Item" and corpse["revision"] == "definition-r1":
            if match := ITEM.fullmatch(corpse["key"]):
                roots.add(int(match[1]))
    return roots


def xml_attributes(raw):
    return {number: grizzly.source.attrs(node)
            for number, node in grizzly.source.xml_index(raw).items()}


def source_rows(root, crystal_xml=None):
    raw = (root / quest.XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != quest.XML_SHA:
        raise ValueError("VERIFIED_LOOT_CORPSE_CANARY_PIN")
    primary = xml_attributes(raw)
    if crystal_xml is not None:
        if hashlib.sha256(crystal_xml).hexdigest() != CRYSTAL_XML_SHA:
            raise ValueError("VERIFIED_LOOT_CORPSE_CRYSTAL_PIN")
        secondary = xml_attributes(crystal_xml)
    else:
        capture = read(root, CRYSTAL_CAPTURE)
        if (capture["revision"], capture["source_sha256"]) != (CRYSTAL_REVISION, CRYSTAL_XML_SHA):
            raise ValueError("VERIFIED_LOOT_CORPSE_CRYSTAL_CAPTURE")
        secondary = {int(number): attributes for number, attributes in capture["records"].items()}
    return primary, secondary


def selected_source(number, primary, secondary):
    return primary.get(number) or secondary.get(number) or {}


def source_numbers(attributes):
    if (attributes.keys() & {"type", "field", "script", "blocksolid", "blockprojectile"}
            or attributes.get("stopduration", "0") not in ("0", "false")):
        return None
    try:
        duration = int(attributes["duration"])
        successor = int(attributes["decayto"])
        capacity = int(attributes["containersize"]) if "containersize" in attributes else None
        weight = int(attributes.get("weight", 0))
    except (ValueError, KeyError):
        return None
    if (not 0 < duration <= 366 * 86400 or not 0 <= successor <= 65535
            or not 0 <= weight <= 0xffffffff or capacity is not None and not 0 < capacity <= 255):
        return None
    return capacity, duration * 1000, successor or None, weight


def canonical_definitions(root):
    return [row["definition"]
            for path in sorted((root / "content/items/definitions").glob("items-*.json"))
            for row in json.loads(path.read_bytes())["records"]]


def plan(root, definitions, creatures, *, holds=None):
    from primal_corpse_items import corpse_definition

    primary, secondary = source_rows(root)
    roots = creature_roots(creatures)
    sources = {number: source_numbers(attributes)
               for number in primary.keys() | secondary.keys()
               if (attributes := selected_source(number, primary, secondary))}
    candidates = {number for number, value in sources.items() if value is not None}
    worlds = quest.world_objects(root, candidates)
    definitions = {row["identity"]["key"]: row for row in definitions}
    bindings = grizzly.source.binding_index((root / BINDINGS).read_bytes())
    # The cooking owner's exact, playable source ingredients/rewards cannot
    # be reclassified as Ground-only through a legacy corpse pointer.
    cookbook = read(root, COOKBOOK)
    cooked = {row["item"] for recipe in cookbook["recipes"]
              for row in [*recipe["ingredients"], recipe["reward"]]}
    eligible, rejected = {}, {}
    for number, world in worlds.items():
        item = definitions.get(key(number))
        binding = bindings.get(str(number))
        movable = world["movable"]
        if (item is None or key(number) in cooked or binding is None or binding["disposition"] != "EXACT"
                or binding["target"] != item["identity"]
                or world["provenance"].get("item_pointer") != item["identity"]
                or world["kind"] != grizzly.source.K("corpse")
                or world["corpse"]["corpse"] != grizzly.source.K(True)
                or world["corpse"]["player_corpse"] != grizzly.source.K(False)
                or world["client_projection"] != grizzly.source.K({"appearance_id": number})
                or movable.get("state") != "KNOWN" or type(movable.get("value")) is not bool):
            rejected[number] = "canonical Item/WorldObject corpse identity or movement proof is absent"
            if key(number) in cooked:
                rejected[number] = "the cooking catalogue already owns this Item"
            continue
        capacity, duration, successor, weight = sources[number]
        observed = corpse_definition(item, capacity, duration, successor, movable["value"], weight)
        semantics = item.get("semantics", {})
        if (item["stack_class"] not in ("Unknown", "NonStackable")
                or any(grizzly.known_conflict(semantics.get(group, grizzly.source.U),
                                              observed["semantics"][group]) for group in GROUPS)):
            rejected[number] = "pinned donor custody facts conflict with a known canonical Item fact"
            continue
        # Preserve known canonical facts beyond the four custody groups.
        observed["semantics"].update(
            (name, value) for name, value in semantics.items() if name not in GROUPS
        )
        item_type = grizzly.source.K("CONTAINER") if capacity is not None else grizzly.source.NA
        classification = grizzly.source.K({"item_type": item_type, "capabilities": grizzly.source.U})
        prior_classification = semantics.get("classification", grizzly.source.U)
        if grizzly.known_conflict(prior_classification, classification):
            rejected[number] = "donor corpse classification conflicts with the canonical Item type"
            continue
        observed["semantics"]["classification"] = grizzly.accepted_first(
            prior_classification, classification
        )
        collision = world["collision"]
        solid, sight = collision["blocks_movement"], collision["blocks_sight"]
        if (solid["state"] != "KNOWN" or sight["state"] != "KNOWN"
                or collision["blocks_projectile"] not in (grizzly.source.U, sight)):
            rejected[number] = "WorldObject collision proof is incomplete or conflicting"
            continue
        attributes = dict(speed_bonus=0, blocks_movement=solid["value"],
                          blocks_projectile=sight["value"],
                          immovable_block_solid=solid["value"] and not movable["value"])
        eligible[number] = (observed, binding, attributes, successor)

    plans = {}
    for number in sorted(roots):
        if number in eligible and sources[number][0] is None:
            rejected[number] = "donor row supplies no loot-container size"
        chain = {}
        stage = number
        while stage and stage not in chain and stage in eligible and len(chain) < MAX_CHAIN:
            chain[stage] = eligible[stage]
            stage = eligible[stage][-1]
        if not stage and number not in rejected:
            plans.update(chain)
        elif holds is not None:
            if stage in chain or len(chain) == MAX_CHAIN:
                evidence = "the donor decay chain cycles or exceeds the eight-stage bound"
            else:
                failed = stage or number
                evidence = rejected.get(failed) or (
                    "pinned donor duration/decay proof is missing" if sources.get(failed) is None
                    else "no exact corpse WorldObject witness")
                if failed != number:
                    evidence = f"decay stage i{failed}: {evidence}"
            holds[number] = evidence
    return plans


def admit_native(root, items, creatures, order):
    definitions = canonical_definitions(root)
    plans = plan(root, definitions, creatures)
    by_key = {record["production_definition"]["production_key"]: record for record in items["records"]}
    added = []
    for number, (observed, binding, attributes, _) in sorted(plans.items()):
        name = key(number)
        # Specialized quest, corpse and portable Item owners retain precedence.
        if name in by_key:
            continue
        added.append({
            "authoring": {"item": observed["identity"]},
            "semantics": copy.deepcopy(observed["semantics"]),
            "attributes": attributes,
            "production_definition": {
                "family": "Item", "production_key": name, "revision_ref": "definition-r1",
            },
            "production_binding": binding,
            "production_binding_qualification": None,
            "admission": {"materializable": True, "stack_class": "NonStackable",
                          "legal_destinations": ["Ground"]},
        })
    items["records"].extend(added)
    items["records"].sort(key=order)
    return added


def capture(crystal_path, root=ROOT):
    primary, secondary = source_rows(root, Path(crystal_path).read_bytes())
    selected = set()
    for number in creature_roots(read(root, quest.CREATURES)):
        chain = set()
        while number and number not in chain and len(chain) < MAX_CHAIN:
            chain.add(number)
            source = source_numbers(selected_source(number, primary, secondary))
            if not source:
                break
            if not primary.get(number):
                selected.add(number)
            number = source[2]
    output = {"revision": CRYSTAL_REVISION, "source_sha256": CRYSTAL_XML_SHA,
              "records": {str(number): secondary[number] for number in sorted(selected)}}
    (root / CRYSTAL_CAPTURE).write_text(json.dumps(output, indent=2) + "\n")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-items", required=True)
    capture(parser.parse_args().crystal_items)
