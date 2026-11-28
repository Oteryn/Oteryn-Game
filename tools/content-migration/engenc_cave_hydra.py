"""Qualify the personal Asura Palace defender from its observed Global species facts."""

import copy
import hashlib
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


def admit_appearance(generated, destination, manifest_path):
    """The illusionable Cave Hydra shares the Hydra's observed sprite and colours."""
    document = json.loads(generated[destination])
    records = document["records"]
    (ordinary,) = (
        row
        for row in records
        if (row.get("creature") or {}).get("key") == "oteryn:creature.hydra"
    )
    if ordinary["look_type"] != 121:
        raise ValueError("Hydra appearance source changed")
    appearance = copy.deepcopy(ordinary)
    appearance["creature"]["key"] = KEY
    del appearance["qualification_sha256"]
    appearance["qualification_sha256"] = hashlib.sha256(
        json.dumps(appearance, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    document["records"] = [
        row for row in records if (row.get("creature") or {}).get("key") != KEY
    ] + [appearance]
    generated[destination] = (
        json.dumps(document, ensure_ascii=False, indent=2) + "\n"
    ).encode()
    manifest = json.loads(generated[manifest_path])
    manifest["spell_appearances"]["sha256"] = hashlib.sha256(
        generated[destination]
    ).hexdigest()
    generated[manifest_path] = (
        json.dumps(manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode()
