"""Qualify physical custody for the Roshamuul dry mortar recipe."""

import hashlib
import json

import authored_quest_item_custody as custody
from native_profile_composition import encode_native_profile


def admit(root, generated, profile_path, manifest_path, order, canonical):
    # Crystal 201189f8 actions_mixture.lua:1-20 joins the two filled buckets.
    # The empty bucket already has a native fluid owner.
    keys = {f"oteryn:item.tibia.i{item}" for item in (20053, 20054, 20169)}
    shard = json.loads((root / "content/items/definitions/items-09000-09499.json").read_bytes())
    definitions = [
        row["definition"] for row in shard["records"]
        if row["definition"]["identity"]["key"] in keys
    ]
    if len(definitions) != len(keys) or {d["identity"]["key"] for d in definitions} != keys:
        raise ValueError("ROSHAMUUL_MIXTURE_ITEM_IDENTITIES")
    xml = (root / "imports/canary/items-xml/items.xml").read_bytes()
    profiles = json.loads(generated[profile_path])
    if any(row["production_definition"]["production_key"] in keys for row in profiles["records"]):
        raise ValueError("ROSHAMUUL_MIXTURE_ITEM_OWNER_CONFLICT")
    profiles["records"].extend(custody.qualify_many(root, definitions, xml))
    profiles["records"].sort(key=order)
    raw = encode_native_profile(profiles)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
