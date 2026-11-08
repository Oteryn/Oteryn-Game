"""Finite explicit armor, lifetime, fluid and partial text custody contracts."""

import grizzly_shop_profiles as owner

IDS = (8862, 21389, 5552, 9171, 21572)
ATTRS = {
    8862: {"primarytype", "absorbpercentdeath", "armor", "weight"},
    21389: {"description", "duration", "weight"},
    5552: {"primarytype", "allowpickupable", "weight"},
    9171: {"primarytype", "description", "maxtextlen", "weight"},
    21572: {"primarytype", "description"},
}
LEAVES = {
    8862: {
        "primarytype": "armors",
        "absorbpercentdeath": "3",
        "armor": "16",
        "weight": "7000",
    },
    21389: {"duration": "3600", "weight": "100"},
    5552: {"primarytype": "fluid containers", "allowpickupable": "1", "weight": "280"},
    9171: {
        "primarytype": "documents and papers",
        "maxtextlen": "1023",
        "weight": "200",
    },
    21572: {"primarytype": "rocks"},
}


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    if (
        item_id not in IDS
        or node.get("id") != str(item_id)
        or definition["identity"]["key"] != f"oteryn:item.tibia.i{item_id}"
    ):
        raise ValueError("SPECIAL_ITEM_IDENTITY")
    attributes = source.attrs(node)
    if set(attributes) != ATTRS[item_id] or any(
        attributes.get(k) != v for k, v in LEAVES[item_id].items()
    ):
        raise ValueError("SPECIAL_ITEM_SOURCE_ATTRIBUTES")
    scripts = node.findall("attribute[@key='script']")
    if item_id == 8862:
        if (
            len(scripts) != 1
            or scripts[0].get("value") != "moveevent"
            or [(c.get("key"), c.get("value")) for c in scripts[0]]
            != [
                ("level", "80"),
                ("slot", "armor"),
                ("vocation", "Knight;true, Elite Knight"),
            ]
        ):
            raise ValueError("SPECIAL_ARMOR_SCRIPT")
    elif scripts:
        raise ValueError("SPECIAL_ITEM_CUSTOM_SCRIPT")
    flags = appearance["flags"]
    if (
        flags.get("flags.cumulative")
        or flags.get("flags.container")
        or bool(flags.get("flags.liquidcontainer")) != (item_id == 5552)
        or flags.get("flags.liquidpool")
    ):
        raise ValueError("SPECIAL_ITEM_INSTANCE_FLAGS")
    binding = bindings.get(str(item_id))
    if (
        binding is None
        or binding["target"] != definition["identity"]
        or binding["disposition"] != "EXACT"
    ):
        raise ValueError("SPECIAL_ITEM_EXACT_BINDING")
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    if item_id == 21572 and (
        record["admission"]["legal_destinations"] != ["Ground"]
        or record["semantics"]["physical"]["value"]["movable"] != source.K(False)
        or record["semantics"]["physical"]["value"]["pickupable"] != source.K(False)
    ):
        raise ValueError("SPECIAL_CRYSTAL_GROUND_ONLY")
    for group in ("physical", "stack", "container", "temporal"):
        if owner.known_conflict(
            definition["semantics"][group], record["semantics"][group]
        ):
            raise ValueError("SPECIAL_ITEM_CANONICAL_CONFLICT:" + group)
    if item_id == 8862:
        expected = source.K(
            {
                "armor": source.K(16),
                "resistances": source.K(
                    [
                        {
                            "kind": "DEATH",
                            "percent": source.K({"numerator": 3, "denominator": 1}),
                        }
                    ]
                ),
            }
        )
        if definition["semantics"]["protection"] != expected:
            raise ValueError("SPECIAL_ARMOR_PROTECTION")
        accepted = definition["semantics"]["equipment"]["value"]["patterns"]["value"]
        observed = record["semantics"]["equipment"]["value"]["patterns"]["value"]
        if len(accepted) != 1 or owner.known_conflict(accepted[0], observed[0]):
            raise ValueError("SPECIAL_ARMOR_CANONICAL_PATTERN")
    if item_id == 5552:
        record["semantics"]["fluid"] = source.K({"fluid_type": source.NA})
    if item_id == 9171:
        text = {
            field: source.U
            for field in (
                "readable",
                "writeable",
                "distance_read",
                "max_text_length",
                "write_once_target",
            )
        }
        text["max_text_length"] = source.K(1023)
        record["semantics"]["readable_writeable"] = source.K(text)
    # XML21572 omits weight: retain the existing importer's verified loader
    # default0 (build_source_item_profiles.py:127), never infer from its name.
    record["semantics"] = owner.accepted_first(
        definition["semantics"], record["semantics"]
    )
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {item_id},
    )["records"][0]
