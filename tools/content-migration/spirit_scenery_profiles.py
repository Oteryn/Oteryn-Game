"""Qualify the fixed walkable Items on the approaches to the Spirit wills."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

from native_profile_composition import encode_native_profile

ROOT = Path(__file__).resolve().parents[2]
PATH = "tools/content-schema/native-gameplay/spirit-scenery-item-profiles.json"
SCHEMA = "OTERYN_NATIVE_SPIRIT_SCENERY_ITEM_PROFILES/v1"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
IDS = {8703, 10538}
SOURCES = {
    "items.xml": "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509",
    "appearances.dat": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
}


def capture(source_root):
    sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as source

    raw = {path: (source_root / path).read_bytes() for path in SOURCES}
    if {path: hashlib.sha256(data).hexdigest() for path, data in raw.items()} != SOURCES:
        raise ValueError("Spirit scenery source digest differs")
    xml = source.xml_index(raw["items.xml"])
    objects = source.engine_items.load_appearance_objects(raw["appearances.dat"])
    bindings = source.binding_index(source.BINDINGS.read_bytes())
    records = []
    for policy in source.policies(raw["appearances.dat"], raw["items.xml"], IDS):
        item = policy["server_item_id"]
        row = source.profile(item, xml[item], objects[item], policy, bindings, None)
        records.append(row)
    return {
        "schema": SCHEMA,
        "source_revision": REVISION,
        "sources": SOURCES,
        "records": source.bind_profiles(
            {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records}, bindings, IDS
        )["records"],
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    keys = {f"oteryn:item.tibia.i{item}" for item in IDS}
    records = overlay["records"]
    if (overlay["schema"], overlay["source_revision"], overlay["sources"]) != (SCHEMA, REVISION, SOURCES):
        raise ValueError("Spirit scenery source declaration differs")
    if len(records) != len(keys) or {r["production_definition"]["production_key"] for r in records} != keys:
        raise ValueError("Spirit scenery Item keys differ")
    for row in records:
        physical = row["semantics"]["physical"]["value"]
        if (row["admission"]["legal_destinations"] != ["Ground"]
                or any(physical[field] != {"state": "KNOWN", "value": False}
                       for field in ("movable", "pickupable"))
                or row["attributes"]["blocks_movement"] is not False
                or row["attributes"]["blocks_projectile"] is not False):
            raise ValueError("Spirit scenery must retain fixed walkable custody")
    document = json.loads(generated[profile_path])
    if any(r["production_definition"]["production_key"] in keys for r in document["records"]):
        raise ValueError("Spirit scenery duplicates a native Item profile")
    document["records"] = sorted(document["records"] + records, key=order)
    generated[profile_path] = encode_native_profile(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[profile_path]).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    args = parser.parse_args()
    (ROOT / PATH).write_text(json.dumps(capture(args.source_dir), indent=2) + "\n")
