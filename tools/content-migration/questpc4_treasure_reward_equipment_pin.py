"""Declare the finite five-identity Treasure Hunt reward cohort."""

import re

from build_questpc4_treasure_reward_profiles import IDS, PRODUCER, SCHEMA


def refresh(source, packet):
    keys = [row["production_definition"]["production_key"] for row in packet["records"]]
    if (
        packet["schema"] != SCHEMA
        or packet["producer"] != PRODUCER
        or keys != [f"oteryn:item.tibia.i{i}" for i in IDS]
    ):
        raise ValueError("TREASURE_REWARD_SOURCE_COHORT")
    name = "QUESTPC4-TREASURE-REWARDS"
    line = f'    ("{name}", {len(IDS)}),'
    pattern = rf'^\s*\("{name}",\s*\d+\),$'
    matches = list(re.finditer(pattern, source, re.MULTILINE))
    if len(matches) > 1:
        raise ValueError("TREASURE_REWARD_DUPLICATE_COHORT")
    if matches:
        return source[: matches[0].start()] + line + source[matches[0].end() :]
    anchor = re.search(r'^\s*\("QUESTPC4-STAG-ITEMS",\s*\d+\),$', source, re.MULTILINE)
    if anchor is None:
        raise ValueError("TREASURE_REWARD_PIN_ANCHOR")
    return source[: anchor.end()] + "\n" + line + source[anchor.end() :]
