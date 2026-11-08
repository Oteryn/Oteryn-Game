"""Declare the finite captured owned recipe inventory-item cohort."""

import questpc4_yalahar_equipment_pin as shared
from build_questpc4_recipe_item_profiles import IDS, PRODUCER, SCHEMA


def refresh(source, packet):
    return shared.refresh(
        source,
        packet,
        ids=IDS,
        schema=SCHEMA,
        producer=PRODUCER,
        cohort="QUESTPC4-RECIPE-ITEMS",
        anchor_cohort="QUESTPC4-BEREGAR-ITEMS",
    )
