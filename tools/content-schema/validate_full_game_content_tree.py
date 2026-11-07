#!/usr/bin/env python3
"""Validate the Oteryn Full Game Content & Ruleset Tree v1 contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA = "OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE/v1"

EXPECTED_DOMAINS = {
    "items", "documents", "geography", "creatures", "bosses", "familiars",
    "npc", "quests", "abilities", "achievements", "charms", "cosmetics",
    "events", "houses", "player_systems", "economy", "presentation",
    "provenance", "wiki",
}

EXPECTED_SYSTEMS = {
    "Store", "Market", "Outfitter", "Raids/Invasions", "Marriage", "PvP Arena",
    "Skull System", "Guild War System", "Loyalty System", "Imbuements",
    "Exaltation Forge", "Prey", "Wheel of Destiny", "Gem Atelier", "Bestiary",
    "Bosstiary", "Charms", "Weapon Proficiency", "Stamina", "Soul", "Blessings",
    "World Changes", "Mini World Changes", "World Quests", "Tibiadrome",
    "Hazard System", "Soulpit", "Doomforging", "Charging System", "Crystal Shield",
    "Enchanting", "Holy Shrines", "Bakragore's Taints", "Goshnar's Taints",
    "Umbral Creation", "Hunting Tasks", "Daily Tasks", "Premium gating",
    "Tibia Client",
}

REQUIRED_STATE_SCOPES = {"ItemInstance", "Character", "Account", "World/Channel", "House", "Market"}


class ValidationError(RuntimeError):
    pass


def _require(condition: bool, code: str) -> None:
    if not condition:
        raise ValidationError(code)


def load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"INVALID_JSON:{path}") from exc
    if not isinstance(value, dict):
        raise ValidationError("ROOT_NOT_OBJECT")
    return value


def validate(value: dict[str, Any]) -> None:
    _require(value.get("schema") == SCHEMA, "SCHEMA_MISMATCH")
    _require(value.get("status") == "TREE_COVERAGE_CLOSED_UNASSIGNED_ZERO", "STATUS_MISMATCH")

    domains = value.get("protected_domain_groups")
    _require(isinstance(domains, list), "DOMAINS_NOT_LIST")
    _require(len(domains) == len(set(domains)), "DUPLICATE_DOMAIN")
    actual_domains = set(domains)
    _require(actual_domains == EXPECTED_DOMAINS, "DOMAIN_SET_MISMATCH")

    domain_assignments = value.get("domain_assignments")
    _require(isinstance(domain_assignments, dict), "DOMAIN_ASSIGNMENTS_NOT_OBJECT")
    _require(set(domain_assignments) == EXPECTED_DOMAINS, "DOMAIN_ASSIGNMENT_COVERAGE")
    for domain, paths in domain_assignments.items():
        _require(isinstance(paths, list) and paths, f"DOMAIN_PATHS_EMPTY:{domain}")
        _require(all(isinstance(path, str) and path for path in paths), f"DOMAIN_PATH_INVALID:{domain}")

    systems = value.get("selected_system_surfaces")
    _require(isinstance(systems, list), "SYSTEMS_NOT_LIST")
    _require(len(systems) == len(set(systems)), "DUPLICATE_SYSTEM")
    actual_systems = set(systems)
    _require(actual_systems == EXPECTED_SYSTEMS, "SYSTEM_SET_MISMATCH")

    assignments = value.get("system_assignments")
    _require(isinstance(assignments, list), "SYSTEM_ASSIGNMENTS_NOT_LIST")
    names: list[str] = []
    for index, assignment in enumerate(assignments):
        _require(isinstance(assignment, dict), f"SYSTEM_ASSIGNMENT_NOT_OBJECT:{index}")
        surface = assignment.get("surface")
        _require(isinstance(surface, str) and surface, f"SYSTEM_SURFACE_INVALID:{index}")
        _require(isinstance(assignment.get("disposition"), str) and assignment["disposition"], f"SYSTEM_DISPOSITION_INVALID:{surface}")
        _require(isinstance(assignment.get("state_owner"), str) and assignment["state_owner"], f"SYSTEM_STATE_OWNER_INVALID:{surface}")
        _require(isinstance(assignment.get("repository"), str) and assignment["repository"], f"SYSTEM_REPOSITORY_INVALID:{surface}")
        _require(isinstance(assignment.get("notes"), str) and assignment["notes"], f"SYSTEM_NOTES_INVALID:{surface}")
        names.append(surface)

    _require(len(names) == len(set(names)), "DUPLICATE_SYSTEM_ASSIGNMENT")
    _require(set(names) == EXPECTED_SYSTEMS, "SYSTEM_ASSIGNMENT_COVERAGE")

    by_name = {assignment["surface"]: assignment for assignment in assignments}
    store = by_name["Store"]
    _require(store["disposition"] == "PLATFORM_COMMERCIAL", "STORE_DISPOSITION")
    _require(store["repository"] == "Oteryn/Oteryn-Platform", "STORE_OWNER")
    _require(store.get("static_path") is None and store.get("ruleset_path") is None, "STORE_GAME_PATH_LEAK")

    premium = by_name["Premium gating"]
    _require("Oteryn/Oteryn-Platform" in premium["repository"], "PREMIUM_PLATFORM_OWNER_MISSING")
    _require("Oteryn/Oteryn-Game" in premium["repository"], "PREMIUM_GAME_CONSUMER_MISSING")

    client = by_name["Tibia Client"]
    _require(client["disposition"] == "EXTERNAL_PRODUCT_SURFACE", "CLIENT_NOT_EXTERNAL_PRODUCT")

    nodes = value.get("target_tree_nodes")
    _require(isinstance(nodes, list) and nodes, "TREE_NODES_INVALID")
    paths: list[str] = []
    for index, node in enumerate(nodes):
        _require(isinstance(node, dict), f"TREE_NODE_NOT_OBJECT:{index}")
        path = node.get("path")
        _require(isinstance(path, str) and path, f"TREE_PATH_INVALID:{index}")
        _require(isinstance(node.get("kind"), str) and node["kind"], f"TREE_KIND_INVALID:{path}")
        _require(isinstance(node.get("owner"), str) and node["owner"], f"TREE_OWNER_INVALID:{path}")
        _require(node.get("repository") == "Oteryn/Oteryn-Game", f"TREE_REPOSITORY_INVALID:{path}")
        paths.append(path)
    _require(len(paths) == len(set(paths)), "DUPLICATE_TREE_PATH")
    _require(any(path.startswith("content/") for path in paths), "CONTENT_TREE_MISSING")
    _require(any(path.startswith("rulesets/") for path in paths), "RULESET_TREE_MISSING")
    _require(any(path.startswith("imports/") for path in paths), "IMPORT_TREE_MISSING")

    states = value.get("durable_state_ownership")
    _require(isinstance(states, list), "STATE_OWNERSHIP_NOT_LIST")
    scopes = {entry.get("scope") for entry in states if isinstance(entry, dict)}
    _require(scopes == REQUIRED_STATE_SCOPES, "STATE_SCOPE_COVERAGE")
    for entry in states:
        _require(isinstance(entry.get("fields"), list) and entry["fields"], f"STATE_FIELDS_EMPTY:{entry.get('scope')}")
        _require(isinstance(entry.get("owner"), str) and entry["owner"], f"STATE_OWNER_EMPTY:{entry.get('scope')}")

    commercial = value.get("commercial_logical_surface")
    _require(isinstance(commercial, dict), "COMMERCIAL_SURFACE_MISSING")
    _require(commercial.get("owner_repository") == "Oteryn/Oteryn-Platform", "COMMERCIAL_OWNER_INVALID")
    _require(commercial.get("physical_path_status") == "DEFER_TO_PLATFORM_CONTRACT", "PLATFORM_PATH_OVERCLAIM")
    _require(isinstance(commercial.get("logical_tree"), list) and commercial["logical_tree"], "COMMERCIAL_TREE_EMPTY")

    principles = value.get("principles")
    _require(isinstance(principles, dict), "PRINCIPLES_MISSING")
    _require("source id" in principles.get("source_ids", "").casefold(), "SOURCE_ID_BOUNDARY_MISSING")
    _require("platform" in principles.get("platform_boundary", "").casefold(), "PLATFORM_BOUNDARY_MISSING")

    summary = value.get("summary")
    _require(isinstance(summary, dict), "SUMMARY_NOT_OBJECT")
    _require(summary.get("protected_domain_groups") == len(EXPECTED_DOMAINS), "SUMMARY_DOMAIN_COUNT")
    _require(summary.get("assigned_domain_groups") == len(EXPECTED_DOMAINS), "SUMMARY_ASSIGNED_DOMAIN_COUNT")
    _require(summary.get("selected_system_surfaces") == len(EXPECTED_SYSTEMS), "SUMMARY_SYSTEM_COUNT")
    _require(summary.get("assigned_system_surfaces") == len(EXPECTED_SYSTEMS), "SUMMARY_ASSIGNED_SYSTEM_COUNT")
    _require(summary.get("target_tree_nodes") == len(nodes), "SUMMARY_NODE_COUNT")
    _require(summary.get("unassigned_domains") == 0, "UNASSIGNED_DOMAINS_NONZERO")
    _require(summary.get("unassigned_systems") == 0, "UNASSIGNED_SYSTEMS_NONZERO")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "path",
        nargs="?",
        type=Path,
        default=Path("docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"),
    )
    args = parser.parse_args()
    value = load(args.path)
    validate(value)
    print(
        "PASS "
        f"domains={len(value['protected_domain_groups'])} "
        f"systems={len(value['selected_system_surfaces'])} "
        f"nodes={len(value['target_tree_nodes'])} "
        "unassigned_domains=0 unassigned_systems=0"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
