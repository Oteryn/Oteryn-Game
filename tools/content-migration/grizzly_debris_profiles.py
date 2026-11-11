"""Admit the closed Grizzly-room debris set as walkable native Item profiles."""

import hashlib
import json

from native_profile_composition import encode_native_profile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = "tools/content-schema/native-gameplay/grizzly-debris-item-profiles.json"
IDS = {6475, 6476, 6477, 6478, 6479, 6480, 6481, 6482, 6483, 6485, 6486}
SCHEMA = "OTERYN_NATIVE_GRIZZLY_DEBRIS_ITEM_PROFILES/v1"


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay["records"]
    keys = [row["production_definition"]["production_key"] for row in records]
    expected = {f"oteryn:item.tibia.i{identifier}" for identifier in IDS}
    if overlay["schema"] != SCHEMA or len(keys) != len(IDS) or set(keys) != expected:
        raise ValueError("GRIZZLY_DEBRIS_PROFILE_RECORD_SET")
    if any(row["attributes"]["blocks_movement"] is not False for row in records):
        raise ValueError("GRIZZLY_DEBRIS_PROFILE_NOT_WALKABLE")
    for row in records:
        physical = row["semantics"]["physical"]
        if physical["state"] != "KNOWN" or any(
            physical["value"][field] != {"state": "KNOWN", "value": False}
            for field in ("movable", "pickupable")
        ):
            raise ValueError("GRIZZLY_DEBRIS_PROFILE_NOT_FIXED")
    document = json.loads(generated[profile_path])
    present = {
        row["production_definition"]["production_key"]
        for row in document["records"]
    }
    if present & set(keys):
        raise ValueError("GRIZZLY_DEBRIS_PROFILE_RECORD_SET")
    document["records"] = sorted(document["records"] + records, key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
