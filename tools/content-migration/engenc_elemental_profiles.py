"""Owner-authored Greater Elementals reuse admitted Massive Elemental combat facts."""

import copy

PAIRS = (
    ("massive_fire_elemental", "greater_fire_elemental", "Greater Fire Elemental"),
    (
        "massive_energy_elemental",
        "greater_energy_elemental",
        "Greater Energy Elemental",
    ),
)


def apply(document):
    records = document["records"]
    by_key = {r["profile"]["target"]["key"]: r for r in records}
    if len(by_key) != len(records):
        raise ValueError("duplicate native creature")
    additions = []
    for source, target, name in PAIRS:
        key = "oteryn:creature." + target
        if key in by_key:
            raise ValueError("duplicate authored elemental")
        row = copy.deepcopy(by_key["oteryn:creature." + source])
        row["profile"]["target"]["key"] = key
        profile = row["profile"]["data"]["profile"]
        profile["experience"] = 0
        profile.pop("bestiary", None)
        details = profile["details"]
        details["display_name"] = name
        details["inspection"] = "a " + name.lower()
        for field in [
            "encyclopedia_document",
            "soul_core_item",
            "bestiary",
            "corpse_item",
        ]:
            details.pop(field, None)
        details["system_eligibility"] = {
            k: False for k in details["system_eligibility"]
        }
        closure = row.get("attack_definitions")
        if closure is not None:
            for definition in closure["records"]:
                if definition["identity"]["family"] == "Creature":
                    if definition["identity"]["key"] != "oteryn:creature." + source:
                        raise ValueError("unexpected elemental attack Creature")
                    definition["identity"] = copy.deepcopy(row["profile"]["target"])
                    definition["loot"] = None
            for index, bound in enumerate(closure["profiles"]):
                if bound["target"]["family"] == "Creature":
                    closure["profiles"][index] = copy.deepcopy(row["profile"])
        additions.append(row)
    records.extend(additions)


def inventory_extension(rows):
    """Qualify both complete authored copies before extending native-only pins."""
    keys = {"oteryn:creature." + target for _, target, _ in PAIRS}
    selected = [row for row in rows if row["profile"]["target"]["key"] in keys]
    if not selected:
        return 0, 0
    original = {"records": [row for row in rows if row["profile"]["target"]["key"] not in keys]}
    apply(original)
    expected = original["records"][-len(PAIRS):]
    if selected != expected:
        raise ValueError("ENGENC_ELEMENTAL_INVENTORY_SOURCE_MISMATCH")
    schedules = sum(len(row["behavior"]["data"]["profile"].get(kind, []))
                    for row in expected for kind in ("attacks", "defenses"))
    return len(PAIRS), schedules
