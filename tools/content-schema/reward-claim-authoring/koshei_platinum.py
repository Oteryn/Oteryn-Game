"""Exact Koshei platinum quantity overlay; immutable donor evidence stays unchanged.

The donor records 100 platinum coins. CP PR3259 decision #6956822717 accepts
the scalar 50 using these three community witnesses (not three independent
sources or a rechecked pinned raw Global source body):
- https://tibiopedia.pl/quests/Koshei_the_Deathless_Quest : the live reward
  section shows 50 linked to https://tibiopedia.pl/items/Platinum_Coin . Its
  "lub"/"or" reward wording remains OPEN-FACT; this overlay settles quantity only.
- https://tibia.fandom.com/wiki/Koshei_The_Deathless_Quest/Spoiler : indexed
  current text says "If you plan to complete the quest you will receive 5,000
  gold back." This witness is not a pinned raw-body recheck.
- https://forum.portaltibia.com.br/topic/7471-blue-legs-quest/ : the 2008 guide
  says "Voce ira pegar os 5000gp de volta no final da quest." The guide is
  copied wiki material, so it is not an independent source.
Apply only to the allocated binding; donor drift requires a fresh source ruling.
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
