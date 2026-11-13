#!/usr/bin/env python3
"""Qualify Primal Pod 39176 for the existing physical Item owner.

Crystal 201189f items.xml:71991 has no ItemType overrides; appearances.dat
marks it nonstackable, immovable, nonpickupable, noncontainer and nonfield.
hazard_primal.lua:75-80 creates the Item; :25-54 and :60-70 remove it.
Its four-second encounter callback is not an ItemType decay duration.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ITEM = "oteryn:item.tibia.i39176"



def verify_source(directory):
    import build_source_world as source
    raw = (directory / "appearances.dat").read_bytes()
    if hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest() != "37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0":
        raise ValueError("Primal Pod appearance source differs")
    xml = (directory / "items.xml").read_bytes()
    if hashlib.sha1(f"blob {len(xml)}\0".encode() + xml).hexdigest() != "ea4196beae8f77be898f05de89db81de134886b6":
        raise ValueError("Primal Pod ItemType source differs")
    policy = source.policies(raw, xml, {39176})[0]
    if (policy["movable"] or policy["pickupable"] or policy["container"]
            or policy["ground"] or policy["magic_field"] or policy["xml_attributes"]):
        raise ValueError("Primal Pod source flags differ")
    for entry in source.fields(raw).get(1, []):
        fields = source.fields(entry)
        if fields.get(1) == [39176]:
            flags = source.fields(fields[3][0])
            if flags.get(6, [0]) != [0]:
                raise ValueError("Primal Pod source is stackable")
            return
    raise ValueError("Primal Pod appearance absent")


def native_profile(directory):
    """Use the same pinned Item parser as every existing physical source Item."""
    import build_source_item_profiles as source
    verify_source(directory)
    raw = (directory / "appearances.dat").read_bytes()
    xml = (directory / "items.xml").read_bytes()
    bindings = source.binding_index(source.BINDINGS.read_bytes())
    objects = source.engine_items.load_appearance_objects(raw)
    physical = source.policies(raw, xml, {39176})[0]
    row = source.profile(39176, source.xml_index(xml)[39176], objects[39176], physical, bindings, None)
    row = source.bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [row]},
                               bindings, {39176})["records"][0]
    return {"schema": "OTERYN_NATIVE_PRIMAL_POD_ITEM_PROFILE/v1",
            "source_revision": "201189f871d23597b82caf88fd611454a2ddefb8",
            "sources": {"data/items/appearances.dat": hashlib.sha256(raw).hexdigest(),
                        "data/items/items.xml": hashlib.sha256(xml).hexdigest()},
            "records": [row]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-dir", type=Path, required=True)
    args = parser.parse_args()
    output = ROOT / "tools/content-schema/native-gameplay/primal-pod-item-profile.json"
    raw = (json.dumps(native_profile(args.source_dir), sort_keys=True,
                      indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if output.read_bytes() != raw:
            raise SystemExit("stale Primal Pod native profile")
    else:
        output.write_bytes(raw)


if __name__ == "__main__":
    main()
