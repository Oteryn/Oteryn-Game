"""Complete unknown native equipment eligibility from pinned Crystal item facts."""

import json

FACTS = "tools/content-schema/native-gameplay/equipment-eligibility-facts.json"
SCHEMA = "OTERYN_NATIVE_EQUIPMENT_ELIGIBILITY_FACTS/v1"
FIELDS = ("level", "vocations")


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
