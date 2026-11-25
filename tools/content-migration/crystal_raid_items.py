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
CORPSE_SQL = "apps/game-server/migrations/3161_crystal_raid_corpse_chains.sql"


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


def admit_ranged_equipment(root, document, parity):
    """Qualify carried ranged loot against the verified Global equip requirements."""
    from import_cyclopedia_global import load_capture

    admissions = json.loads((root / PATH).read_bytes())["records"] + [
        row["record"] for row in parity["item_admissions"]
    ]
    ranged = {
        row["production_definition"]["production_key"]
        for row in admissions
        if row["semantics"]["weapon"].get("value", {}).get("weapon_type")
        == source.K("DISTANCE")
    }
    capture = load_capture(root)
    for row in document["records"]:
        key = row["production_definition"]["production_key"]
        if key not in ranged or row["semantics"]["equipment"] != source.U:
            continue
        facts = capture[int(key.rsplit(".i", 1)[1])]["basic"]
        if facts["Body Position"] != "both hands" or facts["Vocations"] != "paladins":
            raise ValueError(f"Crystal ranged loot equipment is unqualified: {key}")
        row["semantics"]["equipment"] = source.K(
            {
                "patterns": source.K(
                    [
                        {
                            "pattern_id": 1,
                            "primary_slot": source.K("WEAPON"),
                            "additional_reserved_slots": source.K(["SHIELD"]),
                            "mutually_exclusive_groups": source.K([]),
                            "vocations": source.K(["PALADIN"]),
                            "level": source.K(int(facts["Required Level"])),
                            "compatibility_rule": source.NA,
                        }
                    ]
                ),
            }
        )


def corpse_sql(root):
    from quest_corpse_profiles import XML, XML_SHA, decay_closure
    from quest_shared_boss_corpses import BODIES, apply_arbaziloth_corpse_facts

    raid = json.loads((root / PATH).read_bytes())
    key_prefix = "oteryn:item.tibia.i"
    records = {}
    for row in raid["records"]:
        temporal = row["semantics"]["temporal"]
        if temporal["state"] != "KNOWN":
            continue
        values = temporal["value"]
        records[row["production_definition"]["production_key"]] = (
            values["duration"]["value"],
            values["decay_target"].get("value", {}).get("key"),
        )
    # The shared Grave Danger and Arbaziloth forms already use the pinned
    # Quest corpse source and have reachable durable death callers.
    quest_roots = (31599, BODIES["arbaziloth"])
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA:
        raise ValueError("Quest corpse Item source digest differs")
    xml = source.xml_index(raw)
    apply_arbaziloth_corpse_facts(xml)
    for number in decay_closure(quest_roots, xml):
        attrs = source.attrs(xml[number])
        target = int(attrs["decayto"])
        records[f"{key_prefix}{number}"] = (
            int(attrs["duration"]) * 1000,
            f"{key_prefix}{target}" if target else None,
        )
    chain_cases = []
    duration_cases = []
    for number in sorted({*(chain[0] for chain in CHAINS), *quest_roots}):
        root_key = f"{key_prefix}{number}"
        key = root_key
        chain = []
        durations = []
        while key:
            if not key.startswith(key_prefix) or len(chain) >= 4:
                raise ValueError(f"Invalid durable Crystal corpse chain: {root_key}")
            chain.append(int(key.removeprefix(key_prefix)))
            duration, key = records[key]
            durations.append(duration)
        chain_cases.append(
            f"  WHEN '{root_key}' THEN ARRAY[{','.join(map(str, chain))}]"
        )
        if durations != [10_000, 300_000, 300_000, 60_000]:
            duration_cases.append(
                f"  WHEN p_key='{root_key}' THEN ARRAY[{','.join(map(str, durations))}]::BIGINT[]"
            )

    def extend_sql(function, anchor, cases):
        return (
            f"DO $$ DECLARE body TEXT; anchor TEXT := '{anchor}'; BEGIN\n"
            f" body := pg_get_functiondef('{function}(text)'::regprocedure);\n"
            " IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN\n"
            "  RAISE EXCEPTION 'Crystal corpse registry anchor changed';\n"
            " END IF;\n"
            " EXECUTE replace(body,anchor,anchor || $branches$\n"
            + "\n".join(cases)
            + "$branches$);\nEND $$;\n"
        )

    return (
        "-- Generated by crystal_raid_items.corpse_sql from the pinned raid and Quest Item sources.\n"
        + extend_sql("game_summer_corpse_chain", " SELECT CASE p_key", chain_cases)
        + extend_sql("game_source_corpse_durations_v1", " SELECT CASE", duration_cases)
    )


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
