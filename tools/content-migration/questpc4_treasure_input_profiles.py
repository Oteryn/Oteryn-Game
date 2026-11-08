"""Finite physical inputs for the owned, selected Treasure Hunt route."""

import grizzly_shop_profiles as owner
import questpc4_plain_item_profiles as plain

IDS = (9181, 9182, 9183, 9184, 9186, 9177)


def qualify(item_id, node, appearance, physical, bindings, definition):
    # The chosen recipe permits one fixed successful code. Keep this exact
    # source identity; a different doll is not interchangeable evidence.
    if item_id == 9177 and owner.source.attrs(node).get("description") != (
        "You read the numbers 3613 on the back of the doll"
    ):
        raise ValueError("TREASURE_INPUT_SELECTED_CODE")
    return plain.qualify(IDS, item_id, node, appearance, physical, bindings, definition)
