"""Admit the pinned M20 crafting Item family without replacing any established native facts."""

import hashlib
import json
import copy

import qengine_reward_item_profiles as shared

PACKET = "tools/content-schema/native-gameplay/qready1-craft-item-profiles.json"
IDS = {8997, 9108, 9109, 21506, 21507, 21573}


def merge_existing(existing, observed):
    candidate = copy.deepcopy(observed)
    old = existing['semantics'].get('presentation', {}).get('value', {}).get('description')
    new = candidate['semantics'].get('presentation', {}).get('value', {}).get('description')
    # The retained XML snapshot omits the terminal period on the oil/dust
    # descriptions. Keep the current owner's text; all substantive known facts
    # still pass through the existing strict native-profile join.
    if (isinstance(old, dict) and isinstance(new, dict)
            and old.keys() == new.keys() == {'state', 'value'}
            and old['state'] == new['state'] == 'KNOWN'
            and isinstance(old['value'], str) and isinstance(new['value'], str)
            and old['value'].removesuffix('.') == new['value'].removesuffix('.')):
        candidate['semantics']['presentation']['value']['description'] = copy.deepcopy(old)
    return shared.reconcile(existing, candidate)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PACKET).read_bytes())
    records = packet["records"]
    keys = {f"oteryn:item.tibia.i{i}" for i in IDS}
    if (
        packet["schema"] != "OTERYN_QREADY1_CRAFT_ITEM_PROFILES/v1"
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
        raise ValueError("QREADY1_CRAFT_ITEM_SOURCE_DRIFT")
    document = json.loads(generated[profile_path])
    existing = {
        r["production_definition"]["production_key"]: r for r in document["records"]
    }
    for record in records:
        key = record["production_definition"]["production_key"]
        if key in existing:
            try:
                record = merge_existing(existing[key], record)
            except ValueError as error:
                raise ValueError("QREADY1_CRAFT_ITEM_CONFLICT:" + key) from error
        existing[key] = record
    document["records"] = sorted(existing.values(), key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(
        generated[profile_path]
    ).hexdigest()
    generated[manifest_path] = canonical(manifest)
