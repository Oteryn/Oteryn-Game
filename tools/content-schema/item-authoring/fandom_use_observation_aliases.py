"""Separate current Use aliases; preserve immutable shared catalog bytes."""

from source_field_catalogs import _mapped


def build_fandom_use_observation_alias_supplement():
    """Current own-Object aliases; never change the pinned legacy census."""
    return {
        "schema": "OTERYN_FANDOM_USE_OBSERVATION_ALIAS_SUPPLEMENT/v1",
        "accepted_owner": "ProjectV2ItemAuthoring.use_observation",
        "purpose": "SOURCE_ONLY_NONEXECUTABLE_OBSERVATIONS",
        "contract": "OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md:65",
        "fields": [
            {
                "source_field": "damagerange",
                "canonical_source_property": "damage",
                **_mapped(
                    "/item/use/damage",
                    reason="Exact Integer/Range/Text observation; Text remains unevaluated",
                ),
            },
            {
                "source_field": "manacost",
                "canonical_source_property": "mana_cost",
                **_mapped(
                    "/item/use/mana_cost",
                    reason="Exact u32 observation; cost purpose/consumption is not inferred",
                ),
            },
        ],
    }
