"""Capture the pinned Stag sword profile; admission never enables custom use logic."""

from __future__ import annotations

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import grizzly_shop_profiles as source_owner
import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as batch
import questpc4_stag_item_profiles as scoped

ROOT = Path(__file__).resolve().parents[2]
PROOF = "tools/content-schema/native-gameplay/questpc4-stag-item-profiles.json"
SCHEMA = "OTERYN_QUESTPC4_STAG_ITEM_PROFILES/v1"
KEY = "oteryn:item.tibia.i52824"


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()
    ).hexdigest()


def definition(root):
    found = [
        row["definition"]
        for path in (root / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] == KEY
    ]
    if len(found) != 1:
        raise ValueError("STAG_ITEM_CANONICAL_SET")
    return found[0]


def capture(source_root, root=ROOT):
    xml_raw = (source_root / "items.xml").read_bytes()
    client_raw = (root / shared.CLIENT).read_bytes()
    if (
        hashlib.sha256(xml_raw).hexdigest() != batch.XML_SHA256
        or hashlib.sha256(client_raw).hexdigest() != shared.CLIENT_SHA
    ):
        raise ValueError("STAG_ITEM_SOURCE_PIN")
    native = source_owner.source
    node = native.xml_index(xml_raw)[52824]
    appearance = native.engine_items.load_appearance_objects(client_raw)[52824]
    physical = next(iter(native.policies(client_raw, xml_raw, {52824})))
    bindings = native.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    canonical = definition(root)
    record = scoped.qualify(52824, node, appearance, physical, bindings, canonical)
    return {
        "schema": SCHEMA,
        "producer": "tools/content-migration/build_questpc4_stag_item_profiles.py",
        "scope": "Physical custody and existing neutral ammo-slot semantics only; no custom action/script activation.",
        "sources": [
            {
                "revision": batch.REVISION,
                "path": "data/items/items.xml",
                "sha256": batch.XML_SHA256,
                "line": batch.xml_lines(xml_raw)[52824],
                "item_record_sha256": hashlib.sha256(
                    ET.tostring(node, encoding="utf-8")
                ).hexdigest(),
            },
            {"path": shared.CLIENT, "sha256": shared.CLIENT_SHA},
        ],
        "definition_sha256": digest(canonical),
        "binding_sha256": digest(bindings["52824"]),
        "record_sha256": digest(record),
        "records": [record],
    }


def admit(root, generated, profile_path, manifest_path, order, canonical_bytes):
    packet = json.loads((root / PROOF).read_bytes())
    bindings = source_owner.source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if (
        packet["schema"] != SCHEMA
        or len(packet["records"]) != 1
        or packet["records"][0]["production_definition"]["production_key"] != KEY
        or packet["definition_sha256"] != digest(definition(root))
        or packet["binding_sha256"] != digest(bindings["52824"])
        or packet["sources"][0]["sha256"] != batch.XML_SHA256
        or packet["sources"][1]["sha256"] != shared.CLIENT_SHA
    ):
        raise ValueError("STAG_ITEM_PROOF_DRIFT")
    record = packet["records"][0]
    if (
        packet["record_sha256"] != digest(record)
        or packet["sources"][0]["revision"] != batch.REVISION
        or record["semantics"]["protection"]["value"]["armor"]
        != source_owner.source.K(0)
        or record["admission"]["legal_destinations"] != ["CharacterInventory", "Ground"]
        or not record["admission"]["materializable"]
    ):
        raise ValueError("STAG_ITEM_CAPTURE_FACT_DRIFT")
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
    if args.check:
        if (ROOT / PROOF).read_bytes() != raw:
            raise SystemExit("Stag item capture is stale")
    else:
        (ROOT / PROOF).write_bytes(raw)
