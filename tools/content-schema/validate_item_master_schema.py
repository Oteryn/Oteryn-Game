#!/usr/bin/env python3
"""Validate the closed TibiaWiki Item Master Schema v1 census.

This validator checks schema ownership/completeness only. It does not fetch TibiaWiki,
promote Reference truth, mutate Item content, or validate runtime gameplay behavior.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA = "OTERYN_TIBIAWIKI_ITEM_MASTER_FIELD_CENSUS/v1"

EXPECTED_TEMPLATE_PARAMETERS = {
    "GetValue", "List", "DropList", "hidetemplate",
    "name", "mana", "skillboost", "resist", "modificadores", "volume", "armor",
    "attack", "elementattack", "range", "hit", "defense", "defensemod", "charges",
    "mantra", "vocrequired", "levelrequired", "augments", "imbuement", "classificacao",
    "weight", "elemental_bond", "flavortext", "attrib", "hands", "enchantable",
    "enchanted", "stackable", "duration", "damagetype", "damage", "destructible",
    "writable", "readable", "edible", "sounds", "droppedby", "droppedRaidby",
    "droppedEventby", "taskitem", "implemented", "removed", "mercado", "notes",
    "perk1", "perk2", "perk3", "perk4", "perk5", "perk6", "perk7", "regenseconds",
    "dromevalue", "htaskvalue", "storevalue", "tournamentvalue", "buyfrom", "sellto",
    "primarytype", "secondarytype", "tertiarytype", "itemclass", "type", "writechars",
    "value", "npcvalue", "npcprice",
}

EXPECTED_NAVIGATION_FAMILIES = {
    "Capacetes", "Botas", "Armaduras", "Escudos", "Calças", "Spellbooks", "Aljavas",
    "Extra Slot", "Machados", "Distância", "Clavas", "Espadas", "Wands", "Rods",
    "Munição", "Punhos", "Antigas Wands e Rods", "Réplicas de Armas", "Livros",
    "Recipientes", "Prêmios de Eventos", "Decorações", "Documentos e Papéis",
    "Dolls e Bears", "Troféus", "Instrumentos Musicais", "Itens de Fansites",
    "Runas de Decoração", "Itens de Addons", "Itens de Imbuements", "Itens Encantados",
    "Jogos e Diversão", "Itens de Quest", "Itens de Festa", "Cristais (Itens)",
    "Valiosos", "Delivery Tasks", "Runas", "Lixos", "Amuletos e Colares", "Anéis",
    "Chaves", "Fontes de Luz", "Ferramentas", "Ferramentas de Cozinha", "Itens de Domar",
    "Produtos de Criaturas", "Comidas", "Líquidos", "Plantas e Ervas",
}

ALLOWED_DISPOSITIONS = {
    "ITEM_TYPED",
    "ITEM_AUTHORING",
    "RELATIONSHIP",
    "PRESENTATION_EDITOR",
    "PROVENANCE",
    "EXTERNAL_DOMAIN",
    "SOURCE_TEXT_PRESERVE_AND_PARSE",
    "TEMPLATE_CONTROL",
}

REQUIRED_MASTER_GROUPS = {
    "identity", "presentation", "taxonomy", "physical", "stack", "requirements",
    "equipment", "weapon", "protection", "modifiers", "charges", "temporal",
    "container", "imbuement", "forge", "proficiency", "consumable", "fluid",
    "readable", "light", "bed", "use", "lifecycle", "trade", "source_observations",
    "editor",
}


class ValidationError(RuntimeError):
    pass


def load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"INVALID_JSON:{path}") from exc
    if not isinstance(value, dict):
        raise ValidationError("ROOT_NOT_OBJECT")
    return value


def _require(condition: bool, code: str) -> None:
    if not condition:
        raise ValidationError(code)


def validate(value: dict[str, Any]) -> None:
    _require(value.get("schema") == SCHEMA, "SCHEMA_MISMATCH")
    _require(value.get("status") == "FIELD_CENSUS_CLOSED_UNASSIGNED_ZERO", "STATUS_MISMATCH")

    mappings = value.get("field_mappings")
    _require(isinstance(mappings, list), "FIELD_MAPPINGS_NOT_LIST")

    names: list[str] = []
    for index, mapping in enumerate(mappings):
        _require(isinstance(mapping, dict), f"FIELD_MAPPING_NOT_OBJECT:{index}")
        name = mapping.get("source_parameter")
        disposition = mapping.get("disposition")
        _require(isinstance(name, str) and name, f"FIELD_NAME_INVALID:{index}")
        _require(disposition in ALLOWED_DISPOSITIONS, f"FIELD_DISPOSITION_INVALID:{name}")
        _require(isinstance(mapping.get("value_shape"), str), f"FIELD_VALUE_SHAPE_INVALID:{name}")
        _require(isinstance(mapping.get("notes"), str), f"FIELD_NOTES_INVALID:{name}")
        if disposition not in {"TEMPLATE_CONTROL", "EXTERNAL_DOMAIN"}:
            _require(
                isinstance(mapping.get("canonical_path"), str) and mapping["canonical_path"],
                f"FIELD_CANONICAL_PATH_MISSING:{name}",
            )
        names.append(name)

    _require(len(names) == len(set(names)), "DUPLICATE_FIELD_MAPPING")
    actual_parameters = set(names)
    missing = sorted(EXPECTED_TEMPLATE_PARAMETERS - actual_parameters)
    extra = sorted(actual_parameters - EXPECTED_TEMPLATE_PARAMETERS)
    _require(not missing, "MISSING_TEMPLATE_PARAMETERS:" + ",".join(missing))
    _require(not extra, "UNEXPECTED_TEMPLATE_PARAMETERS:" + ",".join(extra))

    nav = value.get("navigation_families")
    _require(isinstance(nav, list), "NAV_FAMILIES_NOT_LIST")
    _require(len(nav) == len(set(nav)), "DUPLICATE_NAV_FAMILY")
    actual_nav = set(nav)
    missing_nav = sorted(EXPECTED_NAVIGATION_FAMILIES - actual_nav)
    extra_nav = sorted(actual_nav - EXPECTED_NAVIGATION_FAMILIES)
    _require(not missing_nav, "MISSING_NAV_FAMILIES:" + ",".join(missing_nav))
    _require(not extra_nav, "UNEXPECTED_NAV_FAMILIES:" + ",".join(extra_nav))

    profiles = value.get("family_profiles")
    _require(isinstance(profiles, list) and profiles, "FAMILY_PROFILES_INVALID")
    assigned: dict[str, str] = {}
    profile_ids: set[str] = set()
    for index, profile in enumerate(profiles):
        _require(isinstance(profile, dict), f"FAMILY_PROFILE_NOT_OBJECT:{index}")
        profile_id = profile.get("profile_id")
        families = profile.get("navigation_families")
        _require(isinstance(profile_id, str) and profile_id, f"PROFILE_ID_INVALID:{index}")
        _require(profile_id not in profile_ids, f"DUPLICATE_PROFILE_ID:{profile_id}")
        profile_ids.add(profile_id)
        _require(isinstance(families, list) and families, f"PROFILE_FAMILIES_INVALID:{profile_id}")
        _require(profile.get("additional_capabilities_allowed") is True, f"PROFILE_NOT_COMPOSITIONAL:{profile_id}")
        for family in families:
            _require(family in EXPECTED_NAVIGATION_FAMILIES, f"PROFILE_UNKNOWN_FAMILY:{family}")
            _require(family not in assigned, f"FAMILY_ASSIGNED_TWICE:{family}")
            assigned[family] = profile_id

    _require(set(assigned) == EXPECTED_NAVIGATION_FAMILIES, "FAMILY_PROFILE_COVERAGE_INCOMPLETE")

    explicit_assignments = value.get("family_assignments")
    _require(isinstance(explicit_assignments, dict), "FAMILY_ASSIGNMENTS_NOT_OBJECT")
    _require(explicit_assignments == assigned, "FAMILY_ASSIGNMENTS_MISMATCH")

    master = value.get("canonical_master_schema")
    _require(isinstance(master, dict), "MASTER_SCHEMA_NOT_OBJECT")
    missing_groups = sorted(REQUIRED_MASTER_GROUPS - set(master))
    _require(not missing_groups, "MASTER_SCHEMA_GROUPS_MISSING:" + ",".join(missing_groups))

    attrib_candidates = value.get("attrib_promotion_candidates")
    _require(isinstance(attrib_candidates, list) and attrib_candidates, "ATTRIB_PROMOTION_CANDIDATES_MISSING")
    concepts = {entry.get("concept") for entry in attrib_candidates if isinstance(entry, dict)}
    for required in {"light emission", "usable/use-with", "sleepable bed", "toggleable light/object"}:
        _require(required in concepts, f"ATTRIB_PROMOTION_CONCEPT_MISSING:{required}")

    summary = value.get("summary")
    _require(isinstance(summary, dict), "SUMMARY_NOT_OBJECT")
    _require(summary.get("template_parameters") == len(EXPECTED_TEMPLATE_PARAMETERS), "SUMMARY_TEMPLATE_COUNT")
    _require(summary.get("navigation_families") == len(EXPECTED_NAVIGATION_FAMILIES), "SUMMARY_FAMILY_COUNT")
    _require(summary.get("unassigned_fields") == 0, "UNASSIGNED_FIELDS_NONZERO")
    _require(summary.get("unassigned_families") == 0, "UNASSIGNED_FAMILIES_NONZERO")

    source_policy = value.get("source_text_policy")
    _require(isinstance(source_policy, dict), "SOURCE_TEXT_POLICY_MISSING")
    _require(
        "never execute raw text" in source_policy.get("attributes_text", ""),
        "ATTRIB_RAW_TEXT_EXECUTION_GUARD_MISSING",
    )
    _require(
        "gameplay content uses stable Oteryn identity" in source_policy.get("no_source_id_in_gameplay", ""),
        "SOURCE_ID_IDENTITY_GUARD_MISSING",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "path",
        nargs="?",
        type=Path,
        default=Path("docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json"),
    )
    args = parser.parse_args()
    value = load(args.path)
    validate(value)
    print(
        "PASS "
        f"fields={len(value['field_mappings'])} "
        f"families={len(value['navigation_families'])} "
        f"profiles={len(value['family_profiles'])} "
        "unassigned_fields=0 unassigned_families=0"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
