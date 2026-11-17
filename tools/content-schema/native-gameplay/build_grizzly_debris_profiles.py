#!/usr/bin/env python3
"""Derive the closed Grizzly-room debris profiles from pinned source facts."""

import argparse
import hashlib
import json
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
from lower_item_consumption_packet import fetch

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
from client_appearance_reader import read_appearances

OUTPUT = Path(__file__).with_name("grizzly-debris-item-profiles.json")
IDS = {6475, 6476, 6477, 6478, 6479, 6480, 6481, 6482, 6483, 6485, 6486}
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
XML_SHA256 = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
SCHEMA = "OTERYN_NATIVE_GRIZZLY_DEBRIS_ITEM_PROFILES/v1"


def capture():
    url = (
        "https://raw.githubusercontent.com/zimbadev/crystalserver/"
        f"{REVISION}/data/items/items.xml"
    )
    xml_raw = fetch(url)
    if hashlib.sha256(xml_raw).hexdigest() != XML_SHA256:
        raise ValueError("GRIZZLY_DEBRIS_XML_DIGEST")
    matching = [
        node
        for node in ET.fromstring(xml_raw)
        if node.get("fromid") == "6475" and node.get("toid") == "6487"
    ]
    if len(matching) != 1:
        raise ValueError("GRIZZLY_DEBRIS_XML_RANGE")
    node = matching[0]
    attributes = {
        child.get("key"): child.get("value") for child in node.findall("attribute")
    }
    if node.get("name") != "debris" or attributes != {"primarytype": "rocks"}:
        raise ValueError("GRIZZLY_DEBRIS_XML_ROW")

    manifest = json.loads(
        (ROOT / "imports/official/client-assets/15.30/manifest.json").read_bytes()
    )
    file = next(
        row
        for row in manifest["files"]
        if row["name"].startswith("appearances-")
        and row["name"].endswith(".dat")
    )
    appearance_raw = (ROOT / "content/assets/files" / file["name"]).read_bytes()
    if (
        file["sha256"] != APPEARANCES_SHA256
        or hashlib.sha256(appearance_raw).hexdigest() != APPEARANCES_SHA256
    ):
        raise ValueError("GRIZZLY_DEBRIS_APPEARANCE_DIGEST")
    appearances = read_appearances(appearance_raw)
    if any(identifier not in appearances for identifier in IDS):
        raise ValueError("GRIZZLY_DEBRIS_APPEARANCE_MISSING")
    if any("unpass" in appearances[identifier].flags for identifier in IDS):
        raise ValueError("GRIZZLY_DEBRIS_APPEARANCE_BLOCKS_MOVEMENT")

    xml = source.xml_index(xml_raw)
    bindings = binding_index(BINDINGS.read_bytes())
    records = []
    for identifier in sorted(IDS):
        flags = appearances[identifier].flags
        appearance = {
            "flags": {
                "flags.cumulative": False,
                "flags.container": False,
                "flags.clothes": False,
            }
        }
        physical = {
            "movable": False,
            "pickupable": False,
            "block_solid": False,
            "block_projectile": "clip" in flags,
            "has_height": False,
        }
        records.append(
            source.profile(
                identifier, xml[identifier], appearance, physical, bindings, None
            )
        )
    bound = bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records},
        bindings,
        IDS,
    )
    return {
        "schema": SCHEMA,
        "producer": (
            "tools/content-schema/native-gameplay/"
            "build_grizzly_debris_profiles.py"
        ),
        "sources": [
            {
                "path": "data/items/items.xml",
                "revision": REVISION,
                "sha256": XML_SHA256,
            },
            {
                "path": "official/client-assets/15.30/" + file["name"],
                "sha256": APPEARANCES_SHA256,
            },
        ],
        "records": bound["records"],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", action="store_true", required=True)
    parser.parse_args()
    OUTPUT.write_text(json.dumps(capture(), indent=2, ensure_ascii=False) + "\n")
