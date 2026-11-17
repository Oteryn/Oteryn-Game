"""Admit only pinned non-corpse Soul War source Items through the native provider."""
import hashlib
import json

SOURCE = "tools/content-schema/native-gameplay/soul-war-source-item-profiles.json"
ITEMS = {33793, 33877, 34005, 33950, 33854, 34006, 34007}
CARRIED = {33793, 33950}
DECAY = {33854: (600_000, 34006), 34006: (300_000, 34007), 34007: (300_000, None)}


def admit(root, generated, destination, manifest_path, order, canonical):
    source = json.loads((root / SOURCE).read_bytes())
    if (source["schema"] != "OTERYN_NATIVE_SOUL_WAR_SOURCE_ITEMS/v1" or
            source["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8" or
            len(source["records"]) != len(ITEMS)):
        raise ValueError("SOUL_WAR_SOURCE_ITEM_PROFILE")
    seen = set()
    for row in source["records"]:
        target = row["production_definition"]
        key = target["production_key"]
        if not key.startswith("oteryn:item.tibia.i"):
            raise ValueError("SOUL_WAR_SOURCE_ITEM_IDENTITY")
        item = int(key.removeprefix("oteryn:item.tibia.i"))
        if item not in ITEMS or item in seen or target != {"family": "Item", "production_key": key, "revision_ref": "definition-r1"}:
            raise ValueError("SOUL_WAR_SOURCE_ITEM_IDENTITY")
        seen.add(item)
        if row["admission"] != {"materializable": True, "stack_class": "NonStackable", "legal_destinations": ["CharacterInventory", "Ground"] if item in CARRIED else ["Ground"]}:
            raise ValueError("SOUL_WAR_SOURCE_ITEM_ADMISSION")
        for name in ("movable", "pickupable"):
            if row["semantics"]["physical"]["value"][name] != {"state": "KNOWN", "value": item in CARRIED}:
                raise ValueError("SOUL_WAR_SOURCE_ITEM_PHYSICAL")
        for name in ("blocks_movement", "immovable_block_solid"):
            if row["attributes"][name] is not (item == 33890):
                raise ValueError("SOUL_WAR_SOURCE_ITEM_COLLISION")
        if row["attributes"]["blocks_projectile"] is not False:
            raise ValueError("SOUL_WAR_SOURCE_ITEM_COLLISION")
        temporal = row["semantics"]["temporal"]
        if item not in DECAY:
            if temporal != {"state": "NOT_APPLICABLE"}:
                raise ValueError("SOUL_WAR_SOURCE_ITEM_CALLBACK_IS_NOT_DECAY")
        else:
            duration, target_id = DECAY[item]
            expected_target = {"state": "KNOWN", "value": {"key": f"oteryn:item.tibia.i{target_id}", "revision": "definition-r1"}} if target_id else {"state": "NOT_APPLICABLE"}
            if temporal != {"state": "KNOWN", "value": {"consumption_mode": {"state": "KNOWN", "value": "DURABLE_ABSOLUTE_DEADLINE"}, "duration": {"state": "KNOWN", "value": duration}, "stop_duration": {"state": "KNOWN", "value": False}, "decay_target": expected_target}}:
                raise ValueError("SOUL_WAR_SOURCE_ITEM_DECAY_CHAIN")
    document = json.loads(generated[destination])
    by_key = {row["production_definition"]["production_key"]: row for row in document["records"]}
    for row in source["records"]:
        key = row["production_definition"]["production_key"]
        if key in by_key and by_key[key] != row:
            raise ValueError("SOUL_WAR_SOURCE_ITEM_PROFILE_CONFLICT")
        if key not in by_key:
            document["records"].append(row)
    document["records"].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)
