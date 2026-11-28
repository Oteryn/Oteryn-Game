"""Qualify the personal Asura Palace defender from its observed Global species facts."""

import json

from engenc_elemental_profiles import variant

KEY = "oteryn:creature.cave_hydra"
SOURCE = "imports/tibiawiki/facts/creatures/creatures-01500-01999.json"


def profile(root, records):
    (fact,) = (
        row
        for row in json.loads((root / SOURCE).read_bytes())["records"]
        if row["source_page_id"] == "55741"
    )
    fields = fact["observed_fields"]
    if fact["source_revision_id"] != 399031 or fields["name"] != "Cave Hydra":
        raise ValueError("Cave Hydra source revision or identity")
    (hydra,) = (
        row
        for row in records
        if row["profile"]["target"]["key"] == "oteryn:creature.hydra"
    )
    return variant(
        hydra,
        KEY,
        fields["name"],
        {
            "health": int(fields["hp"]),
            "experience": int(fields["exp"]),
        },
    )


def admit(root, document):
    records = document["records"]
    if any(row["profile"]["target"]["key"] == KEY for row in records):
        raise ValueError("duplicate Cave Hydra")
    records.append(profile(root, records))


def inventory(root, records):
    (actual,) = (row for row in records if row["profile"]["target"]["key"] == KEY)
    if actual != profile(root, records):
        raise ValueError("Cave Hydra source/profile mismatch")
    behavior = actual["behavior"]["data"]["profile"]
    return sum(len(behavior.get(kind, ())) for kind in ("attacks", "defenses"))
