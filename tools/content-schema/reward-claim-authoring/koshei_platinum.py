"""Exact Koshei platinum quantity overlay; immutable donor evidence stays unchanged.

The donor records 100 platinum coins. The coordinator's live community
cross-check of Tibiopedia's Koshei reward section identifies 50 platinum coins
(the linked item is Platinum Coin). This is community corroboration, not a
rechecked pinned raw Global source body. Apply only to the allocated binding;
any donor drift requires a fresh source ruling.
"""

PILOT = "canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_the_deathless_platinum"
POSITION = {"x": 33261, "y": 32448, "z": 12}
ITEM = "oteryn:item.tibia.i3035"
LEGACY_IDS = [{"server": "canary", "unique_id": 6241},
              {"server": "crystalserver", "unique_id": 6241}]


def quantity(pilot_key: str, position: dict, item_key: str, source_count: int,
             legacy_unique_ids: list) -> int:
    """Normalize the allocated source tuple, refusing drift in that pilot."""
    if pilot_key != PILOT:
        return source_count
    if (position != POSITION or item_key != ITEM or type(source_count) is not int
            or source_count != 100 or legacy_unique_ids != LEGACY_IDS):
        raise ValueError("KOSHEI_PLATINUM_SOURCE_DRIFT")
    return 50
