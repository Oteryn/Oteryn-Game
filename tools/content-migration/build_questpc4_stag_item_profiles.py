"""Capture finite Stag custody profiles; admission never enables custom use logic."""

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
SCHEMA = "OTERYN_QUESTPC4_STAG_ITEM_PROFILES/v3"
ITEMS = scoped.MATERIALS + scoped.NEUTRAL_EQUIPMENT
KEYS = {i: f"oteryn:item.tibia.i{i}" for i in ITEMS}


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
        ).encode()
    ).hexdigest()


def definition(root, items=ITEMS):
    keys = {f"oteryn:item.tibia.i{i}" for i in items}
    found = [
        row["definition"]
        for path in (root / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    ]
    if len(found) != len(items):
        raise ValueError("STAG_ITEM_CANONICAL_SET")
    return {row["identity"]["key"]: row for row in found}


def capture(
    source_root,
    root=ROOT,
    *,
    items=ITEMS,
    qualify=scoped.qualify,
    schema=SCHEMA,
    producer="tools/content-migration/build_questpc4_stag_item_profiles.py",
    scope="Physical custody and existing neutral ammo-slot semantics only; no custom action/script activation.",
):
    keys = {i: f"oteryn:item.tibia.i{i}" for i in items}
    xml_raw = (source_root / "items.xml").read_bytes()
    client_raw = (root / shared.CLIENT).read_bytes()
    if (
        hashlib.sha256(xml_raw).hexdigest() != batch.XML_SHA256
        or hashlib.sha256(client_raw).hexdigest() != shared.CLIENT_SHA
    ):
        raise ValueError("STAG_ITEM_SOURCE_PIN")
    native = source_owner.source
    nodes = native.xml_index(xml_raw)
    appearances = native.engine_items.load_appearance_objects(client_raw)
    physical = {
        row["server_item_id"]: row
        for row in native.policies(client_raw, xml_raw, set(items))
    }
    bindings = native.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    canonical = definition(root, items)
    records = [
        qualify(
            item_id,
            nodes[item_id],
            appearances[item_id],
            physical[item_id],
            bindings,
            canonical[keys[item_id]],
        )
        for item_id in items
    ]
    lines = batch.xml_lines(xml_raw)
    return {
        "schema": schema,
        "producer": producer,
        "scope": scope,
        "sources": [
            {
                "revision": batch.REVISION,
                "path": "data/items/items.xml",
                "sha256": batch.XML_SHA256,
                "items": [
                    {
                        "id": item_id,
                        "line": lines[item_id],
                        "item_record_sha256": hashlib.sha256(
                            ET.tostring(nodes[item_id], encoding="utf-8")
                        ).hexdigest(),
                    }
                    for item_id in items
                ],
            },
            {"path": shared.CLIENT, "sha256": shared.CLIENT_SHA},
        ],
        "definition_sha256": {key: digest(value) for key, value in canonical.items()},
        "binding_sha256": {str(i): digest(bindings[str(i)]) for i in items},
        "record_sha256": {
            r["production_definition"]["production_key"]: digest(r) for r in records
        },
        "records": records,
    }


def admit(
    root,
    generated,
    profile_path,
    manifest_path,
    order,
    canonical_bytes,
    *,
    proof=PROOF,
    items=ITEMS,
    schema=SCHEMA,
    neutral=scoped.NEUTRAL_EQUIPMENT,
    equipment=(),
    ground_only=(),
    producer="tools/content-migration/build_questpc4_stag_item_profiles.py",
):
    keys = {i: f"oteryn:item.tibia.i{i}" for i in items}
    packet = json.loads((root / proof).read_bytes())
    bindings = source_owner.source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    if (
        packet["schema"] != schema
        or packet["producer"] != producer
        or len(packet["records"]) != len(items)
        or [r["production_definition"]["production_key"] for r in packet["records"]]
        != list(keys.values())
        or packet["definition_sha256"]
        != {key: digest(value) for key, value in definition(root, items).items()}
        or packet["binding_sha256"] != {str(i): digest(bindings[str(i)]) for i in items}
        or packet["sources"][0]["sha256"] != batch.XML_SHA256
        or packet["sources"][1]["sha256"] != shared.CLIENT_SHA
    ):
        raise ValueError("STAG_ITEM_PROOF_DRIFT")
    for record in packet["records"]:
        key = record["production_definition"]["production_key"]
        if (
            packet["record_sha256"][key] != digest(record)
            or packet["sources"][0]["revision"] != batch.REVISION
            or (
                key in {keys[i] for i in neutral}
                and record["semantics"]["protection"]["value"]["armor"]
                != source_owner.source.K(0)
            )
            or (
                key
                in {keys[i] for i in items if i not in neutral and i not in equipment}
                and record["semantics"]["equipment"] != source_owner.source.NA
            )
            or (
                key in {keys[i] for i in equipment}
                and (
                    record["semantics"]["equipment"]["state"] != "KNOWN"
                    or record["semantics"]["protection"]
                    != definition(root, equipment)[key]["semantics"]["protection"]
                )
            )
            or record["admission"]["legal_destinations"]
            != (
                ["Ground"]
                if key in {keys[i] for i in ground_only}
                else ["CharacterInventory", "Ground"]
            )
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
