"""Canonical Store catalog vocabulary shared by the schema, converter, validator and
census. Every entry is read from `GameStore.OfferTypes`/`GameStore.States` in the pinned
Canary `data/libs/gamestore/constants.lua` (`47dfd51f`, version 2.0, 27 offer types) and
Crystal `data/modules/scripts/gamestore/init.lua` (`ff7ede5`, version 1.1, 28 offer
types); nothing here is invented. Two engines never agree on offer-type *numbering* for
one name (`weekly_task_expansion` is Canary id 25 / Crystal id 28), and Crystal alone
defines an unused `hunting_slot` (id 25) with no Canary equivalent — both are recorded
explicitly rather than silently reconciled.

`kind` is the shape of the offer's product reference payload (see `store-offer.schema.json`
`$defs.Product`): `service` offers carry none.
"""

from __future__ import annotations

# canonical_name -> (canary constant id or None, crystal constant id or None, kind, note)
OFFER_TYPES: dict[str, tuple[int | None, int | None, str, str]] = {
    "none": (
        0,
        0,
        "service",
        "OFFER_TYPE_NONE; the engine's own default for a missing `type` field.",
    ),
    "item": (
        1,
        1,
        "item",
        "OFFER_TYPE_ITEM; defined by both engines, used by neither catalog's actual data.",
    ),
    "stackable": (
        2,
        2,
        "item",
        "OFFER_TYPE_STACKABLE; itemtype + count (stack quantity).",
    ),
    "charges": (
        3,
        3,
        "item",
        (
            "OFFER_TYPE_CHARGES; itemtype + charges (use count). Canary's own catalog is "
            'inconsistent: one offer ("Ultimate Health Keg", itemtype 25906) uses `count` '
            "for the same meaning; the converter accepts either source field."
        ),
    ),
    "outfit": (
        4,
        4,
        "outfit",
        "OFFER_TYPE_OUTFIT; sexId{female,male} look types + addon mask.",
    ),
    "outfit_addon": (
        5,
        5,
        "outfit",
        "OFFER_TYPE_OUTFIT_ADDON; defined by both engines, unused by either catalog.",
    ),
    "mount": (
        6,
        6,
        "mount",
        "OFFER_TYPE_MOUNT; the engine `id` field is the mount look id.",
    ),
    "namechange": (7, 7, "service", "OFFER_TYPE_NAMECHANGE."),
    "sexchange": (8, 8, "service", "OFFER_TYPE_SEXCHANGE."),
    "house": (
        9,
        9,
        "item",
        "OFFER_TYPE_HOUSE; itemtype + count, delivered into the house.",
    ),
    "expboost": (
        10,
        10,
        "service",
        "OFFER_TYPE_EXPBOOST; value comes from GameStore.ExpBoostValues, not the offer.",
    ),
    "preyslot": (11, 11, "service", "OFFER_TYPE_PREYSLOT."),
    "preybonus": (12, 12, "service", "OFFER_TYPE_PREYBONUS."),
    "temple": (13, 13, "service", "OFFER_TYPE_TEMPLE."),
    "blessings": (
        14,
        14,
        "blessing",
        "OFFER_TYPE_BLESSINGS; blessid + count (blessing charges granted).",
    ),
    "premium": (
        15,
        15,
        "premium_time",
        "OFFER_TYPE_PREMIUM; validUntil is the day count.",
    ),
    "allblessings": (
        17,
        17,
        "blessing_bundle",
        "OFFER_TYPE_ALLBLESSINGS; count only, no single blessid.",
    ),
    "instant_reward_access": (18, 18, "service", "OFFER_TYPE_INSTANT_REWARD_ACCESS."),
    "charms": (19, 19, "service", "OFFER_TYPE_CHARMS."),
    "hireling": (
        20,
        20,
        "hireling",
        "OFFER_TYPE_HIRELING; sexId{female,male} hireling look types.",
    ),
    "hireling_namechange": (21, 21, "service", "OFFER_TYPE_HIRELING_NAMECHANGE."),
    "hireling_sexchange": (22, 22, "service", "OFFER_TYPE_HIRELING_SEXCHANGE."),
    "hireling_skill": (
        23,
        23,
        "hireling_typed",
        (
            "OFFER_TYPE_HIRELING_SKILL; source `id` is `HIRELING_SKILLS.<NAME>[1]`, a lookup "
            "table defined outside the GameStore catalog. Not statically resolvable here; "
            "`typed_id` is omitted (null) rather than guessed."
        ),
    ),
    "hireling_outfit": (
        24,
        24,
        "hireling_typed",
        (
            "OFFER_TYPE_HIRELING_OUTFIT; source `id` is `HIRELING_OUTFITS.<NAME>[1]`, same "
            "disposition as hireling_skill."
        ),
    ),
    "weekly_task_expansion": (
        25,
        28,
        "service",
        (
            "OFFER_TYPE_WEEKLY_TASK_EXPANSION (Canary) / OFFER_TYPE_WEEKLYTASKEXPANSION "
            "(Crystal); same offer, different numeric id and constant spelling per engine."
        ),
    ),
    "item_bed": (
        26,
        26,
        "item",
        "OFFER_TYPE_ITEM_BED; itemtype is a 2-element list (two boxes -> one bed).",
    ),
    "item_unique": (
        27,
        27,
        "item",
        "OFFER_TYPE_ITEM_UNIQUE; itemtype + count, defined by both, used once.",
    ),
    "hunting_slot": (
        None,
        25,
        "service",
        (
            "OFFER_TYPE_HUNTINGSLOT; Crystal-only, no Canary equivalent, and unused by "
            "Crystal's own catalog data (0 offers) — kept for completeness, never emitted "
            "by the converter."
        ),
    ),
}

STATES = {"none": 0, "new": 1, "sale": 2, "timed": 3}
COIN_TYPES = {"coin": 0, "transferable": 1}

# Constant-name -> canonical-name spelling differences between the two engines
# (`flatten_constants` keys are the literal source spelling, e.g.
# `GameStore.OfferTypes.OFFER_TYPE_WEEKLYTASKEXPANSION`).
_ALIASES = {
    "weeklytaskexpansion": "weekly_task_expansion",
    "huntingslot": "hunting_slot",
}


def canonical_offer_type_name(raw_constant_name: str) -> str:
    """`OFFER_TYPE_ALLBLESSINGS` -> `allblessings`; `OFFER_TYPE_WEEKLYTASKEXPANSION` -> `weekly_task_expansion`."""
    stripped = raw_constant_name.removeprefix("OFFER_TYPE_").lower()
    return _ALIASES.get(stripped, stripped)


def offer_type_id_by_engine(engine: str) -> dict[int, str]:
    index = 0 if engine == "canary" else 1
    return {
        ids[index]: name for name, ids in OFFER_TYPES.items() if ids[index] is not None
    }


def slug(text: str) -> str:
    import re

    return re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")
