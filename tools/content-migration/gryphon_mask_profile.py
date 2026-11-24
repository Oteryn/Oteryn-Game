"""Admit the source-qualified Gryphon taming prerequisite as headgear."""

import argparse
import hashlib
import json
from pathlib import Path

import grizzly_shop_profiles as item_import
import qengine_reward_item_profiles as shared
import quest_batch3_item_profiles as crystal

ROOT = Path(__file__).resolve().parents[2]
PACKET = "tools/content-schema/native-gameplay/gryphon-mask-profile.json"
SCHEMA = "OTERYN_GRYPHON_MASK_PROFILE/v1"
ITEM_ID = 31369
KEY = f"oteryn:item.tibia.i{ITEM_ID}"
WIKI_REVISIONS = {"Gryphon Mask": 388130, "Gryphon": 441290}


def known(value):
    return {"state": "KNOWN", "value": value}


def canonical_definition(root):
    shard = root / "content/items/definitions/items-18500-18999.json"
    return next(row["definition"] for row in json.loads(shard.read_bytes())["records"]
                if row["definition"]["identity"]["key"] == KEY)


def capture(xml, root=ROOT):
    appearances = (root / shared.CLIENT).read_bytes()
    if (hashlib.sha256(xml).hexdigest() != crystal.XML_SHA256
            or hashlib.sha256(appearances).hexdigest() != shared.CLIENT_SHA):
        raise ValueError("Gryphon Mask donor source drift")
    native = item_import.source
    node = native.xml_index(xml)[ITEM_ID]
    if node.get("name") != "gryphon mask" or native.attrs(node).get("primarytype") != "helmets":
        raise ValueError("Gryphon Mask donor identity drift")
    obj = native.engine_items.load_appearance_objects(appearances)[ITEM_ID]
    physical = next(iter(native.policies(appearances, xml, {ITEM_ID})))
    bindings = native.binding_index((root / "imports/crystalserver/bindings/items.json").read_bytes())
    record = item_import.qualify(ITEM_ID, node, obj, physical, bindings, canonical_definition(root))
    pattern, = record["semantics"]["equipment"]["value"]["patterns"]["value"]
    if (pattern["primary_slot"] != known("HEAD")
            or pattern["level"]["state"] != "UNKNOWN"
            or pattern["vocations"]["state"] != "UNKNOWN"):
        raise ValueError("Gryphon Mask canonical equipment facts drift")
    # Crystal's helmet has no level or vocation restriction. TibiaWiki confirms
    # the mask is worn on the head while using the Regalia of Suon.
    pattern.update(level=known(0), vocations=known([]))
    return {
        "schema": SCHEMA,
        "source": {"path": "data/items/items.xml", "revision": crystal.REVISION,
                   "sha256": crystal.XML_SHA256, "line": crystal.xml_lines(xml)[ITEM_ID]},
        "appearance_sha256": shared.CLIENT_SHA,
        "tibiawiki_revisions": WIKI_REVISIONS,
        "records": [record],
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PACKET).read_bytes())
    records = packet["records"]
    record, = records
    pattern, = record["semantics"]["equipment"]["value"]["patterns"]["value"]
    if (packet["schema"] != SCHEMA
            or packet["source"]["revision"] != crystal.REVISION
            or packet["source"]["sha256"] != crystal.XML_SHA256
            or packet["appearance_sha256"] != shared.CLIENT_SHA
            or packet["tibiawiki_revisions"] != WIKI_REVISIONS
            or record["production_definition"] != {
                "family": "Item",
                "production_key": KEY,
                "revision_ref": canonical_definition(root)["identity"]["revision"],
            }
            or (pattern["primary_slot"], pattern["level"], pattern["vocations"])
            != (known("HEAD"), known(0), known([]))):
        raise ValueError("Gryphon Mask production policy drift")
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, records)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--items-xml", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = item_import.encode(capture(args.items_xml.read_bytes()))
    packet = ROOT / PACKET
    if args.check:
        if packet.read_bytes() != expected:
            raise SystemExit("Gryphon Mask profile needs regeneration")
    else:
        packet.write_bytes(expected)
