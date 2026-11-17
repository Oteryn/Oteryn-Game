"""Global healing-potion facts that win over the immutable legacy World package (HEALPAR).

Each value names its measurement source or TibiaWiki revision (docs/reference/parity/healing.md, section B).
The legacy package stays unchanged; the producers apply these after it, the way ITEMGLOB
replays the Global client weights.
"""

# key -> (measurement source or TibiaWiki revision, [(resource, min, max)]) for the shard definitions.
POTION_RESTORES = {
    # Global mana n=12, min=124, max=160 contradicts the Wiki minimum 125.
    # Retain the imported range; HP is unchanged.
    "oteryn:item.tibia.i7642": (
        "docs/reference/parity/evidence/2026-11-15-global-client/ct-potions-summary.txt",
        [("HEALTH", 250, 350), ("MANA", 100, 200)],
    ),
    "oteryn:item.tibia.i23374": (1183261, [("HEALTH", 420, 580), ("MANA", 150, 250)]),
    "oteryn:item.tibia.i23375": (1183258, [("HEALTH", 875, 1175)]),
}
# Great Mana Potion, all vocations since 2026-06-20 (TibiaWiki rev 1190635).
ALL_VOCATIONS = {"oteryn:item.tibia.i238": 1190635}


def apply_definitions(item_records):
    """Replay the Global potion facts on canonical Item definitions."""
    for definition in item_records:
        key = definition.get("identity", {}).get("key")
        semantics = definition.get("semantics", {})
        if key in POTION_RESTORES:
            potion = semantics["consumption"]["value"]["value"]
            potion["restores"] = [{"resource": r, "min": lo, "max": hi}
                                  for r, lo, hi in POTION_RESTORES[key][1]]
        if key in ALL_VOCATIONS:
            semantics["use_requirements"]["value"]["vocations"] = {"state": "NOT_APPLICABLE"}


def apply_native_profiles(profiles):
    """Replay the all-vocations fact on the native Item provider records."""
    for record in profiles["records"]:
        if record["production_definition"]["production_key"] in ALL_VOCATIONS:
            record["semantics"]["use_requirements"]["value"]["vocations"] = {"state": "NOT_APPLICABLE"}
