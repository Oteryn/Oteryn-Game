"""Pin the QENGINE witness to its independently declared, source-qualified cohort."""
import re

import qengine_reward_item_profiles as producer


def refresh(source, packet):
    rows = packet.get("records", [])
    keys = [r["production_definition"]["production_key"] for r in rows]
    expected = {f"oteryn:item.tibia.i{i}" for i in producer.IDS}
    if (
        packet.get("schema") != "OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1"
        or packet.get("producer") != "tools/content-migration/qengine_reward_item_profiles.py"
        or packet.get("sources") != [
            {"path": producer.XML, "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446", "sha256": producer.XML_SHA},
            {"path": producer.CLIENT, "sha256": producer.CLIENT_SHA},
        ]
        or len(keys) != len(expected)
        or set(keys) != expected
    ):
        raise ValueError("QENGINE_EQUIPMENT_SOURCE_COHORT")
    pattern = r"assert_eq!\(added_keys\.len\(\),\s*[0-9_]+\);"
    if len(re.findall(pattern, source)) != 1:
        raise ValueError("QENGINE_EQUIPMENT_PIN_ANCHOR")
    return re.sub(pattern, f"assert_eq!(added_keys.len(), {len(producer.IDS)});", source)
