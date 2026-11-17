"""Refresh only three source loot closures from admitted native Item facts; never edit drops."""
import hashlib
import json

PATH = "tools/content-schema/native-gameplay/loot-tables.json"
TABLES = {
    "demon_skeleton": "6edd00c3532af603eaae98151e664ead3eaa182c39ca864cdae568467571aa0b",
    "calamary": "aa80ad8b7b5520e7c38e7440e4b5032595688921355e7e6155a3fbb8994ad64c",
    "seacrest_serpent": "7b62d0d1c332fb7b55da1b6146c7268cf8049876384010bd6809b951d99c91d8",
}
CORPSES = {5963: 10, 13853: 18, 21893: 36}


def apply(root, generated, profile_path, manifest_path, canonical):
    document = json.loads(generated.get(PATH, (root / PATH).read_bytes()))
    profiles = json.loads(generated[profile_path])["records"]
    indexed = {row["production_definition"]["production_key"]: row for row in profiles}
    if len(indexed) != len(profiles):
        raise ValueError("WORLD_CHANGE_LOOT_PROFILE_DUPLICATE")
    outputs = set()
    for name, expected in TABLES.items():
        key = f"oteryn:loot.creature.{name}"
        matches = [row for row in document["tables"] if row["identity"]["key"] == key]
        if len(matches) != 1:
            raise ValueError("WORLD_CHANGE_LOOT_TABLE_MISSING")
        table, = matches
        digest = hashlib.sha256(json.dumps(table, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        if digest != expected or any(entry.get("children") for entry in table["entries"]):
            raise ValueError("WORLD_CHANGE_LOOT_DROP_FACTS_CHANGED")
        outputs.update(entry["item"]["key"] for entry in table["entries"])
        bindings = [row for row in document["creature_loot"]
                    if row["creature"]["key"] == f"oteryn:creature.{name}"]
        if len(bindings) != 1 or bindings[0]["loot"]["key"] != key:
            raise ValueError("WORLD_CHANGE_LOOT_SOURCE_BINDING")
    if len(outputs) != 35:
        raise ValueError("WORLD_CHANGE_LOOT_CLOSURE_BOUND")
    selected = outputs | {f"oteryn:item.tibia.i{number}" for number in CORPSES}
    found = set()
    for facts in document["items"]:
        key = facts["item"]["key"]
        if key not in selected:
            continue
        if key in found:
            raise ValueError("WORLD_CHANGE_LOOT_FACT_DUPLICATE")
        found.add(key)
        profile = indexed.get(key)
        if (profile is None or profile["admission"]["materializable"] is not True
                or profile["production_definition"]["revision_ref"] != facts["item"]["revision"]):
            raise ValueError("WORLD_CHANGE_LOOT_ITEM_NOT_ADMITTED")
        container = profile["semantics"]["container"]
        capacity = None
        if container["state"] == "KNOWN":
            value = container["value"]["capacity"]
            if value["state"] != "KNOWN" or value["value"] <= 0:
                raise ValueError("WORLD_CHANGE_LOOT_CONTAINER_UNKNOWN")
            capacity = value["value"]
        if int(key.rsplit(".i", 1)[1]) in CORPSES and capacity != CORPSES[int(key.rsplit(".i", 1)[1])]:
            raise ValueError("WORLD_CHANGE_LOOT_CORPSE_SOURCE_CAPACITY")
        facts["materializable"] = True
        facts["container_capacity"] = capacity
        facts["stackable"] = profile["admission"]["stack_class"] == "StackCapable"
    if found != selected:
        raise ValueError("WORLD_CHANGE_LOOT_FACT_MISSING")
    data = canonical(document)
    generated[PATH] = data
    manifest = json.loads(generated[manifest_path])
    manifest["loot_tables"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
