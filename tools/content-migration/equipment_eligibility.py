"""Complete unknown native equipment eligibility from pinned Crystal item facts."""

import argparse
import hashlib
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

FACTS = "tools/content-schema/native-gameplay/equipment-eligibility-facts.json"
SCHEMA = "OTERYN_NATIVE_EQUIPMENT_ELIGIBILITY_FACTS/v1"
FIELDS = (
    "level",
    "vocations",
    "additional_reserved_slots",
    "mutually_exclusive_groups",
)
CLAW_KEY = "oteryn:item.tibia.i9393"
CLAW_MOVEEVENT = "data/scripts/movements/claw_of_the_noxious_spawn.lua"
CLAW_MOVEEVENT_SHA256 = (
    "5f29d7440198f2d1a9657a2fb3891c09a58c129dc2d54482ebe9b9cdd77b452c"
)


def load_facts(root):
    document = json.loads((root / FACTS).read_bytes())
    if document["schema"] != SCHEMA:
        raise ValueError("EQUIPMENT_ELIGIBILITY_FACTS_SCHEMA")
    return document["records"]


def complete(document, facts):
    """Fill only captured fields, requiring their exact native slot and pattern."""
    by_key = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    for fact in facts:
        key = fact["key"]
        row = by_key.get(key)
        if row is None:
            raise ValueError(f"EQUIPMENT_ELIGIBILITY_UNKNOWN_ITEM:{key}")
        equipment = row["semantics"].get("equipment", {})
        patterns = equipment.get("value", {}).get("patterns", {}).get("value", [])
        matching = [
            pattern
            for pattern in patterns
            if pattern["pattern_id"] == fact["pattern_id"]
            and pattern["primary_slot"] == {"state": "KNOWN", "value": fact["slot"]}
        ]
        if len(matching) != 1:
            raise ValueError(
                f"EQUIPMENT_ELIGIBILITY_PATTERN:{key}:{fact['pattern_id']}"
            )
        pattern = matching[0]
        for field in FIELDS:
            if field not in fact:
                continue
            expected = {"state": "KNOWN", "value": fact[field]}
            if pattern[field]["state"] == "UNKNOWN":
                pattern[field] = expected
            elif pattern[field] != expected:
                raise ValueError(f"EQUIPMENT_ELIGIBILITY_KNOWN_CONFLICT:{key}:{field}")


def read_pinned(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"EQUIPMENT_ELIGIBILITY_SOURCE_DIGEST:{path}")
    return data


def refresh_claw(root, crystal):
    path = root / FACTS
    document = json.loads(path.read_bytes())
    source = document["source"]
    items = ET.fromstring(read_pinned(crystal / source["path"], source["sha256"]))
    callback = read_pinned(crystal / CLAW_MOVEEVENT, CLAW_MOVEEVENT_SHA256).decode()
    equip = re.search(
        r':type\("equip"\)\n\w+:slot\("ring"\)\n\w+:id\((\d+)\)\n\w+:level\((\d+)\)',
        callback,
    )
    if equip is None:
        raise ValueError("EQUIPMENT_ELIGIBILITY_CLAW_MOVEEVENT")
    item_id, level = equip.groups()
    claw = items.find(f"./item[@id='{item_id}']")
    active = None if claw is None else claw.find("./attribute[@key='transformequipto']")
    if active is None:
        raise ValueError("EQUIPMENT_ELIGIBILITY_CLAW_ITEM")
    worn = items.find(f"./item[@id='{active.attrib['value']}']")
    if (
        worn is None
        or worn.find("./attribute[@key='script']/attribute[@key='slot'][@value='ring']")
        is None
    ):
        raise ValueError("EQUIPMENT_ELIGIBILITY_CLAW_RING")
    [fact] = [row for row in document["records"] if row["key"] == CLAW_KEY]
    # The verified MoveEvent and the XML's worn form both occupy just the ring slot.
    fact.update(
        level=int(level), additional_reserved_slots=[], mutually_exclusive_groups=[]
    )
    rows = document.pop("records")
    output = json.dumps(document, indent=2).removesuffix("\n}")
    path.write_text(
        output
        + ',\n  "records": [\n'
        + ",\n".join("    " + json.dumps(row) for row in rows)
        + "\n  ]\n}\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--refresh-claw", type=Path, required=True, metavar="CRYSTAL_ROOT"
    )
    args = parser.parse_args()
    refresh_claw(Path(__file__).resolve().parents[2], args.refresh_claw)
