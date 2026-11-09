"""Admit the pinned M19 Item family without replacing any established native facts."""

import hashlib
import json
from pathlib import Path

PACKET = "tools/content-schema/native-gameplay/qready1-quest-item-profiles.json"
IDS = {5953, 11982, 12023, 25783}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PACKET).read_bytes())
    records = packet["records"]
    keys = {f"oteryn:item.tibia.i{i}" for i in IDS}
    if (
        packet["schema"] != "OTERYN_QREADY1_QUEST_ITEM_PROFILES/v1"
        or len(records) != len(keys)
        or {r["production_definition"]["production_key"] for r in records} != keys
        or packet["xml_sha256"]
        != "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"
        or packet["appearance_sha256"]
        != "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
        or packet["binding_sha256"]
        != hashlib.sha256(
            (root / "imports/crystalserver/bindings/items.json").read_bytes()
        ).hexdigest()
    ):
        raise ValueError("QREADY1_QUEST_ITEM_SOURCE_DRIFT")
    document = json.loads(generated[profile_path])
    existing = {
        r["production_definition"]["production_key"]: r for r in document["records"]
    }
    for record in records:
        key = record["production_definition"]["production_key"]
        if key in existing and existing[key] != record:
            raise ValueError("QREADY1_QUEST_ITEM_CONFLICT:" + key)
        existing[key] = record
    document["records"] = sorted(existing.values(), key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[profile_path]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)
