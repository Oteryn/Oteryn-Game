"""Derive the Stag equipment cohort from its finite source capture, never runtime totals."""

import re


def refresh(source, packet):
    expected = {
        f"oteryn:item.tibia.i{i}" for i in (52586, 52635, 52743, 52758, 52759, 52824)
    }
    records = packet.get("records", [])
    keys = [record["production_definition"]["production_key"] for record in records]
    if (
        packet.get("schema") != "OTERYN_QUESTPC4_STAG_ITEM_PROFILES/v3"
        or packet.get("producer")
        != "tools/content-migration/build_questpc4_stag_item_profiles.py"
        or len(keys) != len(expected)
        or set(keys) != expected
    ):
        raise ValueError("STAG_EQUIPMENT_SOURCE_COHORT")
    pattern = r'(\("QUESTPC4-STAG-ITEMS",\s*)[0-9_]+(\),)'
    if len(re.findall(pattern, source)) != 1:
        raise ValueError("STAG_EQUIPMENT_PIN_SELECTOR")
    source = source.replace(
        "// QUESTPC4: source-neutral sword52824;",
        "// QUESTPC4: four plain materials plus neutral compass52759/sword52824;",
    ).replace(
        "// QUESTPC4: source-neutral compass52759 and sword52824;",
        "// QUESTPC4: four plain materials plus neutral compass52759/sword52824;",
    )
    return re.sub(pattern, lambda match: f"{match[1]}{len(keys)}{match[2]}", source)
