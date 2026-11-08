"""Source-derived Yalahar cohort declaration; never infer it from runtime totals."""

import re

from build_questpc4_yalahar_item_profiles import IDS, PRODUCER, SCHEMA


def refresh(
    source,
    packet,
    *,
    ids=IDS,
    schema=SCHEMA,
    producer=PRODUCER,
    cohort="QUESTPC4-YALAHAR-ITEMS",
    anchor_cohort="QUESTPC4-STAG-ITEMS",
):
    cohort_pattern = re.escape(cohort)
    anchor_cohort = re.escape(anchor_cohort)
    keys = [
        r["production_definition"]["production_key"] for r in packet.get("records", [])
    ]
    expected = {f"oteryn:item.tibia.i{i}" for i in ids}
    if (
        packet.get("schema") != schema
        or packet.get("producer") != producer
        or len(keys) != len(expected)
        or set(keys) != expected
    ):
        raise ValueError("YALAHAR_EQUIPMENT_SOURCE_COHORT")
    pattern = rf'(\("{cohort_pattern}",\s*)[0-9_]+(\),)'
    matches = re.findall(pattern, source)
    if len(matches) > 1:
        raise ValueError("YALAHAR_EQUIPMENT_PIN_SELECTOR")
    if matches:
        return re.sub(pattern, lambda m: f"{m[1]}{len(keys)}{m[2]}", source)
    anchor = rf'(?m)^(\s*\("{anchor_cohort}",\s*[0-9_]+\),)$'
    if len(re.findall(anchor, source)) != 1:
        raise ValueError("YALAHAR_EQUIPMENT_PIN_ANCHOR")
    return re.sub(
        anchor,
        lambda m: f'{m[1]}\n    ("{cohort}", {len(keys)}),',
        source,
    )
