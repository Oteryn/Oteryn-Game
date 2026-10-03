"""Current own-Object weapon routes; the immutable legacy84 census is unchanged."""

FIELDS = {
    "atk_mod": ("weapon_attack_modifier_points", "/item/weapon/attack_modifier"),
    "hit_chance": (
        "weapon_absolute_hit_chance_percent",
        "/item/weapon/hit_chance_percent",
    ),
}


def build_alias_catalog():
    return {
        "schema": "OTERYN_FANDOM_WEAPON_METADATA_ALIAS_SUPPLEMENT/v1",
        "accepted_owner": "ProjectV2ItemAuthoring",
        "purpose": "SOURCE_ONLY_NONEXECUTABLE_INTRINSIC_WEAPON_METADATA",
        "contract": "docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md",
        "fields": [
            {
                "source_field": field,
                "authoring_property": owner,
                "allowed_destinations": [formal],
                "disposition": "ITEM_AUTHORING",
                "unit": "SIGNED_I32_POINTS"
                if field == "atk_mod"
                else "ABSOLUTE_PERCENTAGE_POINTS_0_TO_100",
            }
            for field, (owner, formal) in FIELDS.items()
        ],
    }


def formal_weapon(authoring):
    """Explicit scalar source projection; never passes through native relative hit."""
    return {
        path.rsplit("/", 1)[1]: authoring[owner]
        for owner, path in FIELDS.values()
        if authoring.get(owner) is not None
    }
