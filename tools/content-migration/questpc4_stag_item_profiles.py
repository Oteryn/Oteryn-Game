"""Scoped source qualification for Stag's inert armour-zero reward sword."""

import grizzly_shop_profiles as owner


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    key = "oteryn:item.tibia.i52824"
    if item_id != 52824 or node.get("id") != "52824" or definition["identity"]["key"] != key:
        raise ValueError("STAG_ITEM_IDENTITY")
    attributes = source.attrs(node)
    if set(attributes) != {"armor", "description", "weight"} or attributes["armor"] != "0":
        raise ValueError("STAG_ITEM_NON_NEUTRAL_ATTRIBUTES")
    scripts = node.findall("attribute[@key='script']")
    if (
        len(scripts) != 1
        or scripts[0].get("value") != "moveevent"
        or [(child.get("key"), child.get("value")) for child in scripts[0]] != [("slot", "ammo")]
    ):
        raise ValueError("STAG_ITEM_CUSTOM_SCRIPT")
    armor = definition["semantics"]["protection"].get("value", {}).get("armor")
    if armor != source.K(0):
        raise ValueError("STAG_ITEM_CANONICAL_ARMOR")
    binding = bindings.get(str(item_id))
    if binding is None or binding["target"] != definition["identity"] or binding["disposition"] != "EXACT":
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
        if owner.known_conflict(definition["semantics"][group], record["semantics"][group]):
            raise ValueError("STAG_ITEM_CANONICAL_CONFLICT:" + group)
    if definition["stack_class"] not in ("Unknown", record["admission"]["stack_class"]):
        raise ValueError("STAG_ITEM_STACK_CONFLICT")
    record["semantics"] = owner.accepted_first(definition["semantics"], record["semantics"])
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {item_id},
    )["records"][0]
