"""Qualify the exact Item closure for the two newly admitted Crystal raid bosses."""

import argparse
import copy
import hashlib
import json
from pathlib import Path

from boss_corpse_profiles import (
    PIN,
    ROOT,
    SOURCE_HASHES,
    load_source_blobs,
    source,
    xml_source_lines,
)
from build_spell_item_identities import binding_index
from imbuement_item_profiles import ORDER, TIERS, slots

PATH = "imports/crystalserver/raid-items-201189f.json"
CHAINS = ((9607, 9608, 9609, 9610), (10337, 10334, 10335, 10332))
BOW = 8029
CORPSES = {number for chain in CHAINS for number in chain}
IDS = CORPSES | {BOW}
CARRIED = {BOW, 10332, 10334, 10335}


def item_bindings(root):
    return binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )


def capture(cache, root):
    blobs = load_source_blobs(cache)
    xml = source.xml_index(blobs["data/items/items.xml"])
    for chain in CHAINS:
        for number, target in zip(chain, (*chain[1:], 0), strict=True):
            if int(source.attrs(xml[number]).get("decayto", "0")) != target:
                raise ValueError(f"Crystal raid corpse decay target drift: {number}")
    appearances = source.engine_items.load_appearance_objects(
        blobs["data/items/appearances.dat"]
    )
    physics = {
        row["server_item_id"]: row
        for row in source.policies(
            blobs["data/items/appearances.dat"], blobs["data/items/items.xml"], IDS
        )
    }
    bindings = item_bindings(root)
    bow_slots = slots(blobs["data/items/items.xml"], {BOW})[BOW]
    records = []
    for number in sorted(IDS):
        binding = bindings[str(number)]
        record = source.profile(
            number, xml[number], appearances[number], physics[number], bindings, None
        )
        if number == BOW:
            record["semantics"]["imbuement"] = {
                "state": "KNOWN",
                "value": {
                    "slot_count": {"state": "KNOWN", "value": bow_slots["slot_count"]},
                    "allowed_family_tiers": {
                        "state": "KNOWN",
                        "value": [
                            {
                                "family": family,
                                "tier": TIERS[bow_slots["families"][family]],
                            }
                            for family in ORDER
                            if family in bow_slots["families"]
                        ],
                    },
                    "excluded_families": {"state": "KNOWN", "value": []},
                },
            }
        record["authoring"]["item"]["revision"] = "crystal-raid-items-201189f"
        record["production_binding"] = copy.deepcopy(binding)
        record["production_binding_qualification"] = None
        record["production_definition"] = {
            "family": "Item",
            "production_key": binding["target"]["key"],
            "revision_ref": binding["target"]["revision"],
        }
        records.append(record)
    return {
        "revision": PIN,
        "sources": SOURCE_HASHES,
        "xml_lines": xml_source_lines(blobs["data/items/items.xml"], IDS),
        "records": records,
    }


def admit(root, generated, destination, manifest_path, order, canonical):
    data = json.loads((root / PATH).read_bytes())
    if data["revision"] != PIN or data["sources"] != SOURCE_HASHES:
        raise ValueError("Crystal raid Item source drift")
    bindings = item_bindings(root)
    document = json.loads(generated[destination])
    existing = {
        record["production_definition"]["production_key"]: record
        for record in document["records"]
    }
    seen = set()
    for record in data["records"]:
        binding = record["production_binding"]
        number = int(binding["external_id"])
        key = binding["target"]["key"]
        destinations = (
            ["CharacterInventory", "Ground"] if number in CARRIED else ["Ground"]
        )
        if (
            number not in IDS
            or number in seen
            or binding != bindings[str(number)]
            or binding["disposition"] != "EXACT"
            or record["production_definition"]
            != {
                "family": "Item",
                "production_key": key,
                "revision_ref": binding["target"]["revision"],
            }
            or record["admission"]
            != {
                "materializable": True,
                "stack_class": "NonStackable",
                "legal_destinations": destinations,
            }
        ):
            raise ValueError(f"Crystal raid Item admission conflict: {number}")
        seen.add(number)
        if key not in existing:
            document["records"].append(record)
            continue
        expected = copy.deepcopy(record)
        expected["authoring"]["item"]["revision"] = "srx3-boss-corpses-crystal-201189f"
        if number not in CHAINS[0] or existing[key] != expected:
            raise ValueError(f"Crystal raid Item admission conflict: {number}")
    if seen != IDS:
        raise ValueError("Crystal raid Item closure differs")
    document["records"].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[destination]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-cache", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    path = ROOT / PATH
    text = (
        json.dumps(
            capture(args.source_cache, ROOT),
            indent=2,
            sort_keys=True,
            ensure_ascii=False,
        )
        + "\n"
    )
    if args.check:
        if path.read_text() != text:
            raise SystemExit("stale Crystal raid Item source capture")
    else:
        path.write_text(text)
