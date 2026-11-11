#!/usr/bin/env python3
"""Pinned physical profiles for Spite fire and Hatred's two source items.

Only converter admission installs these records. This tool never changes source
map appearance policies or invents ItemType decay for delayed Lua callbacks.
True corpses33792/33876/33891/33949/33984 stay with the existing native corpse admission owner.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name("soul-war-source-item-profiles.json")
IDS = {33793, 33877, 34005, 33854, 34006, 34007, 33950}
CARRIED = {33793, 33950}
XML_ATTRIBUTES = {
    33793: [("primarytype", "quest items"), ("weight", "50")],
    33950: [("description", "Can be used to douse Goshnar's madness"), ("weight", "101")],
    33854: [("duration", "600"), ("decayTo", "34006")],
    34006: [("duration", "300"), ("decayTo", "34007")],
    34007: [("duration", "300"), ("decayTo", "0")],
}


def native_profiles(directory):
    import build_source_item_profiles as source
    raw = (directory / "appearances.dat").read_bytes()
    xml = (directory / "items.xml").read_bytes()
    for body, expected in ((raw, "37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0"),
                           (xml, "ea4196beae8f77be898f05de89db81de134886b6")):
        if hashlib.sha1(f"blob {len(body)}\0".encode() + body).hexdigest() != expected:
            raise ValueError("Soul War pinned Item source blob changed")
    physical = {row["server_item_id"]: row for row in source.policies(raw, xml, IDS)}
    objects = source.engine_items.load_appearance_objects(raw)
    bindings = source.binding_index(source.BINDINGS.read_bytes())
    xml_rows = source.xml_index(xml)
    rows = []
    for item in sorted(IDS):
        flags = physical[item]
        carried = item in CARRIED
        if (flags["movable"] != carried or flags["pickupable"] != carried or
                any(flags[key] for key in ("ground", "container", "magic_field", "block_projectile")) or
                flags["block_solid"] != (item == 33890) or flags["block_pathfind"] != (item == 33890)):
            raise ValueError("Soul War source Item physical flags changed")
        expected = [{"key": key, "value": value} for key, value in XML_ATTRIBUTES.get(item, [])]
        if flags["xml_attributes"] != expected:
            raise ValueError("Soul War source ItemType attributes changed")
        rows.append(source.profile(item, xml_rows[item], objects[item], flags, bindings, None))
    document = source.bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": rows}, bindings, IDS)
    return {"schema": "OTERYN_NATIVE_SOUL_WAR_SOURCE_ITEMS/v1",
            "source_revision": "201189f871d23597b82caf88fd611454a2ddefb8",
            "sources": {"data/items/appearances.dat": hashlib.sha256(raw).hexdigest(),
                        "data/items/items.xml": hashlib.sha256(xml).hexdigest()},
            "records": document["records"]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (json.dumps(native_profiles(args.source_dir), sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("Soul War source Item profiles need repository regeneration")
    else:
        OUTPUT.write_bytes(data)
