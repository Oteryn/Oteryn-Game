"""Finite poem readability and the four source telescope rotation identities."""

import grizzly_shop_profiles as owner
from item_transform_profiles import KINDS

IDS = (9204, 9616, 9617, 9618, 9619)


def qualify(item_id, node, appearance, physical, bindings, definition):
    source = owner.source
    if item_id not in IDS or node.get("id") != str(item_id):
        raise ValueError("TREASURE_REWARD_IDENTITY")
    attributes = source.attrs(node)
    if node.findall("attribute[@key='script']"):
        raise ValueError("TREASURE_REWARD_CUSTOM_SCRIPT")
    if item_id == 9204:
        if attributes != {
            "primarytype": "documents and papers",
            "readable": "1",
            "weight": "200",
        }:
            raise ValueError("TREASURE_POEM_SOURCE_ATTRIBUTES")
    else:
        target = item_id + 1 if item_id < 9619 else 9616
        if attributes != {
            "description": "It's the only left-over from a famous pirate treasure",
            "rotateto": str(target),
            "weight": "4500",
        }:
            raise ValueError("TREASURE_TELESCOPE_SOURCE_CYCLE")
    if any(
        appearance["flags"].get("flags." + flag)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool")
    ):
        raise ValueError("TREASURE_REWARD_INSTANCE_FLAGS")
    binding = bindings.get(str(item_id))
    if (
        binding is None
        or binding["target"] != definition["identity"]
        or binding["disposition"] != "EXACT"
    ):
        raise ValueError("TREASURE_REWARD_EXACT_BINDING")
    record = source.profile(item_id, node, appearance, physical, bindings, None)
    if item_id == 9204:
        text = {
            key: source.U
            for key in (
                "readable",
                "writeable",
                "distance_read",
                "max_text_length",
                "write_once_target",
            )
        }
        text["readable"] = source.K(True)
        record["semantics"]["readable_writeable"] = source.K(text)
    else:
        record["semantics"]["use_transform"] = source.K(
            {
                "targets": [
                    {
                        "kind": kind,
                        "target": source.K(
                            {
                                key: bindings[str(target)]["target"][key]
                                for key in ("key", "revision")
                            }
                        )
                        if kind == "ROTATE"
                        else source.U,
                    }
                    for kind in KINDS
                ]
            }
        )
    for group in (
        "physical",
        "stack",
        "container",
        "temporal",
        "readable_writeable",
        "use_transform",
    ):
        if owner.known_conflict(
            definition["semantics"][group], record["semantics"][group]
        ):
            raise ValueError("TREASURE_REWARD_CANONICAL_CONFLICT:" + group)
    # Importer defaults are not evidence overriding accepted trade/compatibility facts.
    record["semantics"] = owner.accepted_first(
        definition["semantics"], record["semantics"]
    )
    return source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
        bindings,
        {item_id},
    )["records"][0]
