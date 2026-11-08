"""Declare the six captured Treasure Hunt input identities."""

import questpc4_yalahar_equipment_pin as shared
from build_questpc4_treasure_input_profiles import IDS, PRODUCER, SCHEMA


def refresh(source, packet):
    return shared.refresh(
        source,
        packet,
        ids=IDS,
        schema=SCHEMA,
        producer=PRODUCER,
        cohort="QUESTPC4-TREASURE-INPUTS",
        anchor_cohort="QUESTPC4-SPECIAL-ITEMS",
    )
