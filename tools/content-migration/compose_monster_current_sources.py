"""Compose two independently qualified overlays without changing either receipt."""

import copy
import json

from bind_monster_source_definitions import CREATURES, bind_definitions
from import_spell_families import canonical_bytes
from monster_seven_spell_overlay import ALLOWED_PATHS, apply_overlay


def identity(row):
    return canonical_bytes(row["profile"]["target"])


def indexed(document):
    rows = {identity(row): row for row in document["records"]}
    if len(rows) != len(document["records"]):
        raise ValueError("COMPOSE_MONSTER_DUPLICATE_IDENTITY")
    return rows


def compose(root, baseline, current):
    # Both stages retain their original full-file and per-record source guards.
    adopted = apply_overlay(root, baseline)
    base_doc = json.loads(baseline[CREATURES])
    current_doc = json.loads(current[CREATURES])
    result_doc = json.loads(adopted[CREATURES])
    base_rows, current_rows, result_rows = map(
        indexed, (base_doc, current_doc, result_doc)
    )
    if (
        {k: v for k, v in base_doc.items() if k != "records"}
        != {k: v for k, v in current_doc.items() if k != "records"}
        or base_rows.keys() != current_rows.keys()
        or not base_rows.keys() <= result_rows.keys()
    ):
        raise ValueError("COMPOSE_MONSTER_CURRENT_SCOPE_CHANGED")
    for key, before in base_rows.items():
        incoming, retained = current_rows[key], result_rows[key]
        if {k: v for k, v in before.items() if k != "monster_melee"} != {
            k: v for k, v in incoming.items() if k != "monster_melee"
        }:
            raise ValueError("COMPOSE_MONSTER_NON_MELEE_CHANGE")
        old_melee, new_melee = before.get("monster_melee"), incoming.get("monster_melee")
        if old_melee == new_melee:
            continue
        if new_melee is None or retained.get("monster_melee") not in (
            old_melee,
            new_melee,
        ):
            raise ValueError("COMPOSE_MONSTER_MELEE_CONFLICT")
        retained["monster_melee"] = copy.deepcopy(new_melee)
    result = dict(current)
    # Retain every adopted appearance, variant, Wiki field and source mechanic.
    for path in ALLOWED_PATHS:
        result[path] = adopted[path]
    result[CREATURES] = canonical_bytes(result_doc)
    # Player catalog/selection pins belong to the separately qualified current
    # source stage; Monster/presentation pins belong to the adopted stage.
    manifest_path = "content/spells.manifest.json"
    manifest = json.loads(result[manifest_path])
    current_manifest = json.loads(current[manifest_path])
    for section in ("catalog", "source_selection"):
        manifest[section] = current_manifest[section]
    result[manifest_path] = canonical_bytes(manifest)
    return bind_definitions(root, result)
