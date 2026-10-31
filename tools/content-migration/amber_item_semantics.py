"""Amber's bounded, source-qualified non-container fact for the Item tree.

The retained wiki frame names Both Hands; Canary declares a two-handed fist
weapon with no container type/capacity. The pinned client container getter is
false, using the same default as build_source_item_profiles.profile. Keep all
other Item facts and admission unchanged.
"""
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KEY = "oteryn:item.tibia.i50239"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
XML_SHA = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
FRAME = "docs/agents/evidence/OTV2-20261001-item-stack-historical-source-frame-v1.json"
FRAME_SHA = "0ecaedfe954961a3a26859a2cb05ecfed9ca9a513610e44dfba1a0260f1eff29"


def checked(root, path, digest):
    raw = (root / path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError("AMBER_SOURCE_DRIFT:" + path)
    return raw


def apply_container_fact(definitions, root=ROOT):
    sys.path.insert(0, str(root / "tools/content-schema/item-authoring"))
    from engine_items import load_appearance_objects

    xml = checked(root, "imports/canary/items-xml/items.xml", XML_SHA)
    node = ET.fromstring(xml).find("item[@id='50239']")
    attrs = {a.get("key").lower(): a.get("value") for a in node.findall("attribute")}
    client = checked(root, f"content/assets/files/appearances-{CLIENT_SHA}.dat", CLIENT_SHA)
    appearance = load_appearance_objects(client)[50239]
    frames = json.loads(checked(root, FRAME, FRAME_SHA))["frames"]
    frame = next(f["historical"] for f in frames if f["historical"]["source_item_id"] == 50239)
    if (node.get("name") != "amber kusarigama"
            or attrs.get("weapontype") != "fist" or attrs.get("slottype") != "two-handed"
            or "containersize" in attrs or attrs.get("type") == "container"
            or appearance.get("name") != node.get("name")
            or appearance["flags"].get("flags.container", False)
            or frame["item_key"] != KEY or "| slot          = Both Hands\n" not in frame["content"]):
        raise ValueError("AMBER_SOURCE_SEMANTICS")
    matches = [d for d in definitions if d["identity"]["key"] == KEY]
    if len(matches) != 1 or matches[0]["identity"]["revision"] != "definition-r1":
        raise ValueError("AMBER_ITEM_IDENTITY")
    semantics = matches[0]["semantics"]
    target = {"state": "NOT_APPLICABLE"}
    if semantics["container"] not in ({"state": "UNKNOWN"}, target):
        raise ValueError("AMBER_CONTAINER_CONFLICT")
    semantics["container"] = target
