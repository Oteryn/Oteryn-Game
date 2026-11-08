"""Capture five finite Treasure Hunt reward identities without activating their use."""

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import build_questpc4_stag_item_profiles as owner
import grizzly_shop_profiles as source_owner
import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as batch
import questpc4_treasure_reward_profiles as scoped

ROOT = owner.ROOT
IDS = scoped.IDS
SCHEMA = "OTERYN_QUESTPC4_TREASURE_REWARD_PROFILES/v1"
PROOF = "tools/content-schema/native-gameplay/questpc4-treasure-reward-profiles.json"
PRODUCER = "tools/content-migration/build_questpc4_treasure_reward_profiles.py"


def definitions(root):
    keys = {f"oteryn:item.tibia.i{i}" for i in IDS}
    rows = [
        row["definition"]
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    ]
    if len(rows) != len(IDS):
        raise ValueError("TREASURE_REWARD_CANONICAL_SET")
    return {
        row["identity"]["key"]: row
        for row in sorted(rows, key=lambda row: row["identity"]["key"])
    }


def capture(source_root, root=ROOT):
    xml_raw = (source_root / "items.xml").read_bytes()
    client_raw = (root / shared.CLIENT).read_bytes()
    if (
        hashlib.sha256(xml_raw).hexdigest() != batch.XML_SHA256
        or hashlib.sha256(client_raw).hexdigest() != shared.CLIENT_SHA
    ):
        raise ValueError("TREASURE_REWARD_SOURCE_PIN")
    source = source_owner.source
    nodes = source.xml_index(xml_raw)
    appearances = source.engine_items.load_appearance_objects(client_raw)
    physical = {
        row["server_item_id"]: row
        for row in source.policies(client_raw, xml_raw, set(IDS))
    }
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    canonical = definitions(root)
    records = [
        scoped.qualify(
            i,
            nodes[i],
            appearances[i],
            physical[i],
            bindings,
            canonical[f"oteryn:item.tibia.i{i}"],
        )
        for i in IDS
    ]
    lines = batch.xml_lines(xml_raw)
    return {
        "schema": SCHEMA,
        "producer": PRODUCER,
        "scope": "Poem readability and exact four-state telescope rotation facts; no default text, grant, rotation execution or whole-quest inference.",
        "sources": [
            {
                "revision": batch.REVISION,
                "path": "data/items/items.xml",
                "sha256": batch.XML_SHA256,
                "items": [
                    {
                        "id": i,
                        "line": lines[i],
                        "item_record_sha256": hashlib.sha256(
                            ET.tostring(nodes[i], encoding="utf-8")
                        ).hexdigest(),
                    }
                    for i in IDS
                ],
            },
            {"path": shared.CLIENT, "sha256": shared.CLIENT_SHA},
        ],
        "definition_sha256": {key: owner.digest(row) for key, row in canonical.items()},
        "binding_sha256": {str(i): owner.digest(bindings[str(i)]) for i in IDS},
        "record_sha256": {
            row["production_definition"]["production_key"]: owner.digest(row)
            for row in records
        },
        "records": records,
    }


def admit(root, generated, profile_path, manifest_path, order, canonical_bytes):
    packet = json.loads((root / PROOF).read_bytes())
    bindings = source_owner.source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    keys = [f"oteryn:item.tibia.i{i}" for i in IDS]
    if (
        packet["schema"] != SCHEMA
        or packet["producer"] != PRODUCER
        or [row["production_definition"]["production_key"] for row in packet["records"]]
        != keys
        or packet["definition_sha256"]
        != {key: owner.digest(row) for key, row in definitions(root).items()}
        or packet["binding_sha256"]
        != {str(i): owner.digest(bindings[str(i)]) for i in IDS}
        or packet["sources"][0]["sha256"] != batch.XML_SHA256
        or packet["sources"][0]["revision"] != batch.REVISION
        or packet["sources"][1]["sha256"] != shared.CLIENT_SHA
    ):
        raise ValueError("TREASURE_REWARD_PROOF_DRIFT")
    for row in packet["records"]:
        key = row["production_definition"]["production_key"]
        if (
            packet["record_sha256"][key] != owner.digest(row)
            or row["semantics"]["equipment"] != source_owner.source.NA
            or row["admission"]["legal_destinations"]
            != ["CharacterInventory", "Ground"]
            or not row["admission"]["materializable"]
        ):
            raise ValueError("TREASURE_REWARD_CAPTURE_FACT_DRIFT")
    shared.admit_records(
        root,
        generated,
        profile_path,
        manifest_path,
        order,
        canonical_bytes,
        packet["records"],
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = source_owner.encode(capture(args.source_root))
    target = ROOT / PROOF
    if args.check:
        if target.read_bytes() != raw:
            raise SystemExit("Treasure reward capture is stale")
    else:
        target.write_bytes(raw)
