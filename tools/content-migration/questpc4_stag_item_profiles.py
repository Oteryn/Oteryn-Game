"""Finite Stag custody profiles; retain source equipment and plain-item distinctions."""

import grizzly_shop_profiles as owner

MATERIALS = (52586, 52635, 52743, 52758)
NEUTRAL_EQUIPMENT = (52759, 52824)


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    key = f"oteryn:item.tibia.i{item_id}"
    if (
        item_id not in MATERIALS + NEUTRAL_EQUIPMENT
        or node.get("id") != str(item_id)
        or definition["identity"]["key"] != key
    ):
        raise ValueError("STAG_ITEM_IDENTITY")
    attributes = source.attrs(node)
    if item_id in MATERIALS:
        if bool(appearance["flags"].get("flags.cumulative", False)) != (item_id != 52586):
            raise ValueError("STAG_MATERIAL_STACK_FLAG")
        if set(attributes) != {"description", "weight"} or node.findall(
            "attribute[@key='script']"
        ):
            raise ValueError("STAG_MATERIAL_SOURCE_ATTRIBUTES")
        return owner.qualify(item_id, node, appearance, physical, bindings, definition)
    if (
        set(attributes) != {"armor", "description", "weight"}
        or attributes["armor"] != "0"
    ):
        raise ValueError("STAG_ITEM_NON_NEUTRAL_ATTRIBUTES")
    scripts = node.findall("attribute[@key='script']")
    if (
        len(scripts) != 1
        or scripts[0].get("value") != "moveevent"
        or [(child.get("key"), child.get("value")) for child in scripts[0]]
        != [("slot", "ammo")]
    ):
        raise ValueError("STAG_ITEM_CUSTOM_SCRIPT")
    armor = definition["semantics"]["protection"].get("value", {}).get("armor")
    if armor != source.K(0):
        raise ValueError("STAG_ITEM_CANONICAL_ARMOR")
    binding = bindings.get(str(item_id))
    if (
        binding is None
        or binding["target"] != definition["identity"]
        or binding["disposition"] != "EXACT"
    ):
        raise ValueError("STAG_ITEM_EXACT_BINDING")
    flags = appearance["flags"]
    if flags.get("flags.liquidcontainer") or flags.get("flags.liquidpool"):
        raise ValueError("STAG_ITEM_FLUID")
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    if (
        record["admission"]["legal_destinations"] != ["CharacterInventory", "Ground"]
        or record["semantics"]["temporal"] != source.NA
        or record["semantics"]["charges"] != source.NA
    ):
        raise ValueError("STAG_ITEM_INSTANCE_OWNER")
    for group in ("physical", "stack", "container"):
        if owner.known_conflict(
            definition["semantics"][group], record["semantics"][group]
        ):
            raise ValueError("STAG_ITEM_CANONICAL_CONFLICT:" + group)
    if definition["stack_class"] not in ("Unknown", record["admission"]["stack_class"]):
        raise ValueError("STAG_ITEM_STACK_CONFLICT")
    record["semantics"] = owner.accepted_first(
        definition["semantics"], record["semantics"]
    )
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {item_id},
    )["records"][0]
