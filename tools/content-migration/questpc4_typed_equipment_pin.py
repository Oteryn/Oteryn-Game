"""Declare the finite captured typed quest inventory-item cohort."""

import questpc4_yalahar_equipment_pin as shared
from build_questpc4_typed_item_profiles import IDS, PRODUCER, SCHEMA


def refresh(source, packet):
    return shared.refresh(
        source,
        packet,
        ids=IDS,
        schema=SCHEMA,
        producer=PRODUCER,
        cohort="QUESTPC4-TYPED-ITEMS",
        anchor_cohort="QUESTPC4-RECIPE-ITEMS",
    )
