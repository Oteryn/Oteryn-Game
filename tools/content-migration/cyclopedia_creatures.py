"""Carry the canonical Crystal Cyclopedia fields into the sealed native providers."""

import json

from bind_monster_source_definitions import CREATURES
from cake_native_successor import (
    PRESENTATIONS,
    read_derived_shard,
    read_project_document,
    source_profile,
)
from import_spell_families import canonical_bytes

SOURCE = "imports/crystalserver/cyclopedia-creatures-201189f.json"


def apply(root, generated):
    from build_canonical_monster_profiles import union_profiles

    source = json.loads((root / SOURCE).read_bytes())
    keys = {row["creature"] for row in source["sources"]}
    added_keys = {row["target"]["key"] for row in source["bindings"]}
    canonical = {}
    for path in sorted((root / "content/creatures/definitions").glob("creatures-*.json")):
        for row in read_derived_shard(path)["records"]:
            key = row["definition"]["identity"]["key"]
            if key in keys:
                if key in canonical:
                    raise ValueError(f"Duplicate canonical Cyclopedia Creature: {key}")
                canonical[key] = row
    if canonical.keys() != keys:
        raise ValueError("The canonical Cyclopedia Creature cohort is incomplete")

    declarations = read_project_document(root, "definitions/declarations.json")
    active, presentations, appended = union_profiles(
        json.loads(generated[CREATURES]),
        json.loads(generated[PRESENTATIONS]),
        [canonical[key] for key in sorted(added_keys)],
        declarations["authoring_profiles"],
        preserve_existing_display_aliases=True,
    )
    if {row["key"] for row in appended} != added_keys:
        raise ValueError("Crystal Cyclopedia Creature identity or display name conflict")
    profiles = {row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]["details"]
                for row in active["records"] if row["profile"]["target"]["key"] in keys}
    if profiles.keys() != keys:
        raise ValueError("The native Cyclopedia Creature cohort is incomplete")
    for row in source["sources"]:
        key = row["creature"]
        details = canonical[key]["authoring"]["profile"]["details"]
        if "bosstiary" in row:
            if details["bosstiary"]["category"] != row["bosstiary"]:
                raise ValueError(f"Canonical Bosstiary category mismatch: {key}")
            profiles[key]["bosstiary"] = details["bosstiary"]
        else:
            exclusive = details["system_eligibility"]["exclusive_prey"]
            if exclusive != row["exclusive_prey"]:
                raise ValueError(f"Canonical Prey pool mismatch: {key}")
            profiles[key]["system_eligibility"]["exclusive_prey"] = exclusive
    generated[CREATURES] = canonical_bytes(active)
    generated[PRESENTATIONS] = canonical_bytes(presentations)


def inventory(root, rows, native):
    """Count only the exact, source-staged additions to the native Creature cohort."""
    source = json.loads((root / SOURCE).read_bytes())
    profiles = {row["target"]["key"]: row for row in source["profiles"]}
    bindings = {row["target"]["key"]: row for row in source["bindings"]}
    canonical = {row["definition"]["identity"]["key"]: row for row in rows}
    active = {row["profile"]["target"]["key"]: row for row in native}
    melee = spells = schedules = 0
    for record in source["records"]:
        if record["kind"] != "Creature":
            continue
        key = record["identity"]["key"]
        profile = source_profile(profiles[key])
        profile["data"]["profile"]["details"].setdefault("reference_level", None)
        behavior = profiles[record["behavior"]["key"]]
        if (canonical[key]["definition"] != record
                or canonical[key]["authoring"] != profile["data"]
                or canonical[key]["source_bindings"] != [bindings[key]]
                or active[key]["profile"] != profile or active[key]["behavior"] != behavior):
            raise ValueError(f"Crystal Cyclopedia Creature inventory mismatch: {key}")
        actions = behavior["data"]["profile"]
        attacks = actions.get("attacks", [])
        schedules += len(attacks) + len(actions.get("defenses", []))
        for attack in attacks:
            kind = profiles[attack["ability"]["key"]]["data"]["profile"]["details"]["kind"]
            melee += kind == "Melee"
            spells += kind == "Spell"
    return len(bindings), schedules, melee, spells
