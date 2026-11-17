"""Admit the Krailos lake tools and Water Elemental corpse decay chain."""

import hashlib
import json

import build_source_item_profiles as native
import build_world_change_item_profiles as sources

CARRIED_IDS = {22692, 22708}
CORPSE_IDS = {9582, 9583}
ITEM_IDS = CARRIED_IDS | CORPSE_IDS


def admit(root, generated, profile_path, manifest_path, order, canonical):
    xml_raw = sources.read_xml(root)
    appearance_raw = (root / sources.APPEARANCES).read_bytes()
    sources.verify_sources(xml_raw, appearance_raw)
    xml = native.xml_index(xml_raw)
    objects = native.engine_items.load_appearance_objects(appearance_raw)
    physical = {
        row["server_item_id"]: row
        for row in native.policies(appearance_raw, xml_raw, ITEM_IDS)
    }
    bindings = native.binding_index(
        (root / native.BINDINGS.relative_to(native.ROOT)).read_bytes()
    )
    candidates = [
        native.profile(item, xml[item], objects[item], physical[item], bindings, None)
        for item in sorted(ITEM_IDS)
    ]
    profiles = native.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": candidates},
        bindings,
        ITEM_IDS,
    )
    document = json.loads(generated[profile_path])
    by_key = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    for row in profiles["records"]:
        key = row["production_definition"]["production_key"]
        item = int(key.removeprefix("oteryn:item.tibia.i"))
        destinations = ["CharacterInventory", "Ground"] if item in CARRIED_IDS else ["Ground"]
        if row["admission"] != {
            "materializable": True,
            "stack_class": "NonStackable",
            "legal_destinations": destinations,
        } or (key in by_key and by_key[key] != row):
            raise ValueError(f"Krailos Item profile drift: {key}")
        by_key[key] = row
    document["records"] = sorted(by_key.values(), key=order)
    generated[profile_path] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[profile_path]).hexdigest()
    generated[manifest_path] = canonical(manifest)
