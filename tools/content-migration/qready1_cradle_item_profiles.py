"""Admit only M20's independently corroborated Wiki/official Cradle material cohort."""

import hashlib
import json

PACKET = "tools/content-schema/native-gameplay/qready1-cradle-item-profiles.json"
IDS = {40016, 40598, 40601, 40603}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PACKET).read_bytes())
    keys = {f"oteryn:item.tibia.i{i}" for i in IDS}
    records = packet["records"]
    membership = (
        "imports/official/appearance-membership/appearances-"
        + packet["appearance_sha256"]
        + ".json"
    )
    if (
        packet["schema"] != "OTERYN_QREADY1_CRADLE_ITEM_PROFILES/v1"
        or packet["binding_basis"]
        != "EXPLICIT_WIKI_ITEM_ID_CORROBORATED_BY_OFFICIAL_NAMED_APPEARANCE"
        or len(records) != len(keys)
        or {row["production_definition"]["production_key"] for row in records} != keys
        or packet["official_membership_sha256"]
        != hashlib.sha256((root / membership).read_bytes()).hexdigest()
        or packet["wiki_stats_sha256"]
        != hashlib.sha256((root / "imports/tibiawiki/facts/items-stats.json").read_bytes()).hexdigest()
    ):
        raise ValueError("QREADY1_CRADLE_SOURCE_DRIFT")
    document = json.loads(generated[profile_path])
    existing = {
        row["production_definition"]["production_key"]: row for row in document["records"]
    }
    for record in records:
        key = record["production_definition"]["production_key"]
        if key in existing and existing[key] != record:
            raise ValueError("QREADY1_CRADLE_PROFILE_CONFLICT:" + key)
        existing[key] = record
    document["records"] = sorted(existing.values(), key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[profile_path]).hexdigest()
    generated[manifest_path] = canonical(manifest)
