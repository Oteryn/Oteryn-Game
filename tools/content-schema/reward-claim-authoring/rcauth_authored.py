"""RCAUTH: authored written text on plain donor reward claims.

The donor chests grant the maps without text; the text is TibiaWiki's (fetched
2026-11-17), cited per binding. The overlay applies after the donor rules have
validated the unmodified record, and only onto the exact donor reward it names.
"""
import copy

ITEM_REVISION = "definition-r1"
BINDINGS = {
    "oteryn:reward-claim.kv.purpletome1": (
        "oteryn:item.tibia.i2822",
        "You see a map of our world Tibia",
        "TibiaWiki Tibia Map (Book) rev 1022778 (fetched 2026-11-17)",
    ),
    "oteryn:reward-claim.kv.purpletome2": (
        "oteryn:item.tibia.i2823",
        "You see a map of the surface of the Fields of Glory. There are many red lines and "
        "two golden points on it. You wonder what their meaning is.",
        "TibiaWiki Fields of Glory Map (Book) rev 1022275 (fetched 2026-11-17)",
    ),
}


def _donor_reward(item: str) -> dict:
    return {"items": [{"count": 1, "item": {"family": "Item", "key": item,
                                            "revision": ITEM_REVISION}}]}


def _bound(records: list) -> dict:
    found = {}
    for row in records:
        key = row["definition"]["identity"]["key"]
        if key in BINDINGS:
            if key in found:
                raise ValueError(f"{key}: RCAUTH binding names a duplicated claim")
            found[key] = row["definition"]
    missing = sorted(set(BINDINGS) - set(found))
    if missing:
        raise ValueError(f"RCAUTH binding names a missing claim: {missing}")
    return found


def apply(records: list) -> list:
    """Records with each authored text bound onto its exact donor reward."""
    records = copy.deepcopy(records)
    for key, definition in _bound(records).items():
        item, text, _ = BINDINGS[key]
        placements = definition["placements"]
        if len(placements) != 1 or placements[0]["reward"] != _donor_reward(item):
            raise ValueError(f"{key}: RCAUTH donor reward changed; re-check the authored text")
        placements[0]["reward"]["written_text"] = {"text": text}
    return records


def strip(records: list) -> tuple[list, list[str]]:
    """The donor records under the committed overlay, and any overlay drift."""
    records = copy.deepcopy(records)
    errors = []
    try:
        bound = _bound(records)
    except ValueError as error:
        return records, [str(error)]
    for key, definition in bound.items():
        item, text, _ = BINDINGS[key]
        reward = definition["placements"][0]["reward"]
        if reward.pop("written_text", None) != {"text": text} or reward != _donor_reward(item):
            errors.append(f"{key}: RCAUTH authored text is missing or stale")
    return records, errors
