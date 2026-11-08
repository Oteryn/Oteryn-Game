"""Exact dwarven-legs source facts; reuse the existing equipment importer."""

import grizzly_shop_profiles as owner


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    if (
        item_id != 3398
        or node.get("id") != "3398"
        or definition["identity"]["key"] != "oteryn:item.tibia.i3398"
    ):
        raise ValueError("BEREGAR_LEGS_IDENTITY")
    if source.attrs(node) != {
        "primarytype": "legs",
        "absorbpercentphysical": "3",
        "armor": "7",
        "weight": "4900",
    }:
        raise ValueError("BEREGAR_LEGS_SOURCE_ATTRIBUTES")
    if any(
        appearance["flags"].get("flags." + flag)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool")
    ):
        raise ValueError("BEREGAR_LEGS_INSTANCE_FLAGS")
    scripts = node.findall("attribute[@key='script']")
    if (
        len(scripts) != 1
        or scripts[0].get("value") != "moveevent"
        or [(child.get("key"), child.get("value")) for child in scripts[0]]
        != [("slot", "legs")]
    ):
        raise ValueError("BEREGAR_LEGS_SOURCE_SCRIPT")
    expected = source.K(
        {
            "armor": source.K(7),
            "resistances": source.K(
                [
                    {
                        "kind": "PHYSICAL",
                        "percent": source.K({"denominator": 1, "numerator": 3}),
                    }
                ]
            ),
        }
    )
    if definition["semantics"]["protection"] != expected:
        raise ValueError("BEREGAR_LEGS_CANONICAL_PROTECTION")
    binding = bindings.get("3398")
    if (
        binding is None
        or binding["target"] != definition["identity"]
        or binding["disposition"] != "EXACT"
    ):
        raise ValueError("BEREGAR_LEGS_EXACT_BINDING")
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    for group in ("physical", "stack", "container"):
        if owner.known_conflict(
            definition["semantics"][group], record["semantics"][group]
        ):
            raise ValueError("BEREGAR_LEGS_CANONICAL_CONFLICT:" + group)
    accepted = definition["semantics"]["equipment"]["value"]["patterns"]["value"]
    observed = record["semantics"]["equipment"]["value"]["patterns"]["value"]
    if len(accepted) != 1 or owner.known_conflict(accepted[0], observed[0]):
        raise ValueError("BEREGAR_LEGS_CANONICAL_EQUIPMENT")
    record["semantics"] = owner.accepted_first(
        definition["semantics"], record["semantics"]
    )
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {3398},
    )["records"][0]
