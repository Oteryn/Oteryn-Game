"""Finite source-cohort validation for plain, nonstackable quest inventory items."""

import grizzly_shop_profiles as owner


def qualify(ids, item_id, node, appearance, physical, bindings, definition):
    low = int(node.get("id", node.get("fromid", "0")))
    high = int(node.get("id", node.get("toid", "0")))
    if item_id not in ids or not low <= item_id <= high:
        raise ValueError("PLAIN_ITEM_IDENTITY")
    if set(owner.source.attrs(node)) - {
        "primarytype",
        "description",
        "weight",
    } or node.findall("attribute[@key='script']"):
        raise ValueError("PLAIN_ITEM_SOURCE_ATTRIBUTES")
    if any(
        appearance["flags"].get("flags." + flag)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool")
    ):
        raise ValueError("PLAIN_ITEM_INSTANCE_FLAGS")
    record = owner.qualify(item_id, node, appearance, physical, bindings, definition)
    if record["semantics"]["equipment"] != owner.source.NA or record["semantics"][
        "protection"
    ] != {"state": "UNKNOWN"}:
        raise ValueError("PLAIN_ITEM_NON_EQUIPMENT_FACTS")
    return record
