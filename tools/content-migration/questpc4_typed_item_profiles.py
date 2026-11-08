"""Finite stack and empty-backpack source contracts; no use effects inferred."""

import grizzly_shop_profiles as owner

STACK_IDS = (24382, 21506, 7785, 52632)
IDS = (5949,) + STACK_IDS


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    if (
        item_id not in IDS
        or node.get("id") != str(item_id)
        or definition["identity"]["key"] != f"oteryn:item.tibia.i{item_id}"
    ):
        raise ValueError("TYPED_ITEM_IDENTITY")
    flags = appearance["flags"]
    if flags.get("flags.liquidcontainer") or flags.get("flags.liquidpool"):
        raise ValueError("TYPED_ITEM_FLUID")
    attributes = source.attrs(node)
    if item_id in STACK_IDS:
        if not flags.get("flags.cumulative") or flags.get("flags.container"):
            raise ValueError("TYPED_ITEM_STACK_FLAGS")
        if set(attributes) - {
            "primarytype",
            "description",
            "weight",
            "showcount",
        } or node.findall("attribute[@key='script']"):
            raise ValueError("TYPED_ITEM_SOURCE_ATTRIBUTES")
        record = owner.qualify(
            item_id, node, appearance, physical, bindings, definition
        )
        if (
            record["semantics"]["equipment"] != source.NA
            or record["semantics"]["protection"] != source.U
        ):
            raise ValueError("TYPED_ITEM_NON_EQUIPMENT")
        return record
    if attributes != {
        "primarytype": "containers",
        "containersize": "20",
        "weight": "1800",
        "imbuementslot": "1",
    }:
        raise ValueError("BACKPACK_SOURCE_ATTRIBUTES")
    scripts = node.findall("attribute[@key='script']")
    if (
        len(scripts) != 1
        or scripts[0].get("value") != "moveevent"
        or [(c.get("key"), c.get("value")) for c in scripts[0]]
        != [("slot", "backpack")]
    ):
        raise ValueError("BACKPACK_SOURCE_SCRIPT")
    if not flags.get("flags.container") or flags.get("flags.cumulative"):
        raise ValueError("BACKPACK_INSTANCE_FLAGS")
    binding = bindings.get("5949")
    if (
        binding is None
        or binding["target"] != definition["identity"]
        or binding["disposition"] != "EXACT"
    ):
        raise ValueError("BACKPACK_EXACT_BINDING")
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    for group in ("physical", "stack", "container"):
        if owner.known_conflict(
            definition["semantics"][group], record["semantics"][group]
        ):
            raise ValueError("BACKPACK_CANONICAL_CONFLICT:" + group)
    accepted = definition["semantics"]["equipment"]["value"]["patterns"]["value"]
    observed = record["semantics"]["equipment"]["value"]["patterns"]["value"]
    if len(accepted) != 1 or owner.known_conflict(accepted[0], observed[0]):
        raise ValueError("BACKPACK_CANONICAL_EQUIPMENT")
    record["semantics"] = owner.accepted_first(
        definition["semantics"], record["semantics"]
    )
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {5949},
    )["records"][0]
