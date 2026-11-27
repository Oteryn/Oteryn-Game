"""Admit source-derived M06/M01c/Panpipe keys, doors and Koshei catacomb route items."""
import hashlib
import json

from native_profile_composition import encode_native_profile

PATH = "tools/content-schema/native-gameplay/key-door-item-profiles.json"
CLASSIC_QUEST_DOORS = {5106, 6251, 9565, 9566}
IDS = {
    1628,
    1631,
    1646,
    1647,
    1648,
    1649,
    1678,
    1679,
    1680,
    1681,
    1892,
    1895,
    1898,
    1899,
    2967,
    2968,
    2969,
    3110,
    3115,
    3116,
    5097,
    5102,
    5103,
    5120,
    5121,
    5291,
    5292,
    6248,
    6262,
    6263,
    6264,
    6265,
    6298,
    7711,
    7714,
} | CLASSIC_QUEST_DOORS


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay["records"]
    keys = [row["production_definition"]["production_key"] for row in records]
    if (
        overlay["schema"] != "OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1"
        or overlay["source_revision"] != "99902524e052f37574194466c2949c576e4ab269"
        or len(keys) != len(IDS)
        or set(keys) != {f"oteryn:item.tibia.i{i}" for i in IDS}
    ):
        raise ValueError("KEY_DOOR_ITEM_PROFILE_SET")
    document = json.loads(generated[profile_path])
    if {
        row["production_definition"]["production_key"] for row in document["records"]
    } & set(keys):
        raise ValueError("KEY_DOOR_ITEM_PROFILE_DUPLICATE")
    document["records"] = sorted(document["records"] + records, key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
