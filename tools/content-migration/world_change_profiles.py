"""Admit the closed, source-normalized world-change item family through the native provider."""
import hashlib
import json

PATH = "tools/content-schema/native-gameplay/world-change-item-profiles.json"
IDS = frozenset({2576, 3583, 3990, 4103, 4104, 5963, 12059, 12060, 12061, 12062, 12063, 12065, 12077, 12368, 12369, 13853, 13854, 13855, 13856, 13974, 13976, 14018, 14020, 14051, 14052, 14053, 14054, 14055, 14064, 14074, 14163, 14164, 14255, 21752, 21803, 21804, 21805, 21812, 21893, 21894, 21895, 21896})
IDS = frozenset({3583, 3990, 4103, 4104, 5963, 12059, 12060, 12061, 12062, 12063, 12065, 12077, 12368, 12369, 13853, 13854, 13855, 13856, 13974, 13976, 14018, 14020, 14051, 14052, 14053, 14054, 14055, 14064, 14074, 14163, 14164, 14255, 21752, 21803, 21804, 21805, 21812, 21893, 21894, 21895, 21896, 22059, 22074})
SCHEMA = "OTERYN_NATIVE_WORLD_CHANGE_ITEM_PROFILES/v1"
XML_SHA256 = "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"


def validate(overlay):
    records = overlay["records"]
    expected = {f"oteryn:item.tibia.i{number}" for number in IDS}
    keys = [row["production_definition"]["production_key"] for row in records]
    source_rows = overlay["source_rows"]
    if (overlay["schema"] != SCHEMA or len(keys) != len(IDS) or set(keys) != expected
            or overlay["reserved_ids_excluded"] != [12064]
            or len(source_rows) != len(IDS) or {row["id"] for row in source_rows} != IDS):
        raise ValueError("WORLD_CHANGE_ITEM_PROFILE_SET")
    sources = {row["role"]: row for row in overlay["sources"]}
    if (len(overlay["sources"]) != 2
            or sources.get("crystal_items", {}).get("sha256") != XML_SHA256
            or sources.get("official_appearances", {}).get("sha256") != APPEARANCES_SHA256):
        raise ValueError("WORLD_CHANGE_ITEM_PROFILE_SOURCE")
    for row in records:
        number = int(row["production_definition"]["production_key"].rsplit(".i", 1)[1])
        portable = number in {12077, 12369, 13974, 13976, 14018, 14020,
                              14051, 14052, 14053, 14054, 14055, 14255,
                              21752, 21803, 21804, 21805, 21812}
        portable = portable or number in {3583,3990,4103,4104,22074}
        movable = portable or number in {13854,13855,13856,21894,21895,21896}
        stack = "StackCapable" if number in {3583,14052,14053,14054} else "NonStackable"
        solid = number in {14074, 14163}
        height = number == 14164
        physical = row["semantics"]["physical"]
        if (physical["state"] != "KNOWN"
                or physical["value"]["movable"] != {"state": "KNOWN", "value": movable}
                or physical["value"]["pickupable"] != {"state": "KNOWN", "value": portable}
                or row["admission"] != {
                    "materializable": True, "stack_class": stack,
                    "legal_destinations": ["CharacterInventory", "Ground"] if portable else ["Ground"]}
                or row["attributes"]["blocks_movement"] is not solid
                or row["attributes"]["immovable_block_solid"] is not solid
                or row["attributes"]["blocks_projectile"] is not False
                or row["attributes"]["has_height"] is not height):
            raise ValueError("WORLD_CHANGE_ITEM_PROFILE_PHYSICAL")
        chains = {5963:(10,10000,3990),3990:(10,300000,4103),4103:(8,300000,4104),4104:(None,60000,None),
                  13853:(18,10000,13854),13854:(18,300000,13855),13855:(9,300000,13856),13856:(None,60000,None),
                  21893:(36,10000,21894),21894:(36,300000,21895),21895:(19,300000,21896),21896:(None,60000,None)}
        if number in chains:
            capacity, duration, target = chains[number]
            expected_container = ({"state":"KNOWN","value":{"capacity":{"state":"KNOWN","value":capacity}}}
                                  if capacity is not None else {"state":"NOT_APPLICABLE"})
            temporal = {"state":"KNOWN","value":{
                "consumption_mode":{"state":"KNOWN","value":"DURABLE_ABSOLUTE_DEADLINE"},
                "duration":{"state":"KNOWN","value":duration},
                "stop_duration":{"state":"KNOWN","value":False},
                "decay_target":({"state":"KNOWN","value":{"key":f"oteryn:item.tibia.i{target}","revision":"definition-r1"}}
                                if target is not None else {"state":"NOT_APPLICABLE"})}}
            if row["semantics"]["container"] != expected_container or row["semantics"]["temporal"] != temporal:
                raise ValueError("WORLD_CHANGE_ITEM_CORPSE_CHAIN")
    return records


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = validate(overlay)
    document = json.loads(generated[profile_path])
    present = {row["production_definition"]["production_key"]: row for row in document["records"]}
    additions = []
    for row in records:
        key = row["production_definition"]["production_key"]
        if key not in present:
            additions.append(row)
            continue
        # ITEMHOLD-Q already owns the captured wolf on main. Reuse its native
        # policy only when the World-change custody facts agree; retain its
        # presentation, trade facts and provenance unchanged.
        existing = present[key]
        if (key != "oteryn:item.tibia.i12369"
                or existing["production_definition"] != row["production_definition"]
                or existing["admission"] != row["admission"]
                or any(existing["semantics"][group] != row["semantics"][group]
                       for group in ("physical", "stack", "container", "temporal", "charges"))):
            raise ValueError("WORLD_CHANGE_ITEM_PROFILE_DUPLICATE")
    document["records"] = sorted(document["records"] + additions, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
