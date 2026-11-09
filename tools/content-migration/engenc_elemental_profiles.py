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
        for field in ["encyclopedia_document", "soul_core_item", "bestiary"]:
            details.pop(field, None)
        details["system_eligibility"] = {
            k: False for k in details["system_eligibility"]
        }
        additions.append(row)
    records.extend(additions)
