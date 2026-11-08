"""Source-derived Yalahar cohort declaration; never infer it from runtime totals."""

import re

from build_questpc4_yalahar_item_profiles import IDS, PRODUCER, SCHEMA


def refresh(source, packet):
    keys = [
        r["production_definition"]["production_key"] for r in packet.get("records", [])
    ]
    expected = {f"oteryn:item.tibia.i{i}" for i in IDS}
    if (
        packet.get("schema") != SCHEMA
        or packet.get("producer") != PRODUCER
        or len(keys) != len(expected)
        or set(keys) != expected
    ):
        raise ValueError("YALAHAR_EQUIPMENT_SOURCE_COHORT")
    pattern = r'(\("QUESTPC4-YALAHAR-ITEMS",\s*)[0-9_]+(\),)'
    matches = re.findall(pattern, source)
    if len(matches) > 1:
        raise ValueError("YALAHAR_EQUIPMENT_PIN_SELECTOR")
    if matches:
        return re.sub(pattern, lambda m: f"{m[1]}{len(keys)}{m[2]}", source)
    anchor = r'(?m)^(\s*\("QUESTPC4-STAG-ITEMS",\s*[0-9_]+\),)$'
    if len(re.findall(anchor, source)) != 1:
        raise ValueError("YALAHAR_EQUIPMENT_PIN_ANCHOR")
    return re.sub(
        anchor,
        lambda m: f'{m[1]}\n    ("QUESTPC4-YALAHAR-ITEMS", {len(keys)}),',
        source,
    )
