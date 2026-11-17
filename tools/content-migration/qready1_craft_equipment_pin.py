"""Declare M20's closed, independently source-qualified six-identity cohort."""

import re

IDS = {8997, 9108, 9109, 21506, 21507, 21573}
SCHEMA = "OTERYN_QREADY1_CRAFT_ITEM_PROFILES/v1"
NAME = "QREADY1-CRAFT"


def refresh(source, packet):
    keys = [r["production_definition"]["production_key"] for r in packet["records"]]
    if (
        packet["schema"] != SCHEMA
        or len(keys) != len(IDS)
        or set(keys) != {f"oteryn:item.tibia.i{i}" for i in IDS}
    ):
        raise ValueError("M20_CRAFT_SOURCE_COHORT")
    for row in packet["records"]:
        if row["attributes"]["speed_bonus"] != 0:
            raise ValueError("M20_CRAFT_SPEED_SOURCE_CHANGED")
    line = f'    ("{NAME}", {len(IDS)}),'
    matches = list(re.finditer(rf'^    \("{NAME}", \d+\),$', source, re.MULTILINE))
    if len(matches) > 1:
        raise ValueError("M20_DUPLICATE_COHORT")
    if matches:
        m = matches[0]
        return source[: m.start()] + line + source[m.end() :]
    anchor = "    // BEGIN GENERATED QREADY PROFILE ADDITIONS"
    if source.count(anchor) != 1:
        raise ValueError("M20_COHORT_ANCHOR")
    return source.replace(
        anchor,
        "    // M20: six exact Items in qready1-craft-item-profiles.json.\n"
        + line
        + "\n"
        + anchor,
    )
