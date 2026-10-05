#!/usr/bin/env python3
"""Inventory existing player cast routes; never grant runtime admission."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = "tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
SELECTION = "docs/reference/spells/r21-local-candidate/active-artifact/source-selection.json"
WIKI = "tools/content-schema/spell-authoring/samples/spell-verify-3-sources-2026-09-28.json"
FRESH = "docs/reference/spells/r24-candidate/research/source-access-and-findings.json"
ROUTES = {
    "ability": "ordinary_combat.rs", "party_buff": "ordinary_combat.rs",
    "conjure": "world_item_cast.rs", "random_item_grant": "world_item_cast.rs",
    "familiar_summon": "familiar_cast.rs", "stance_toggle": "stance_cast.rs",
    "monk_focus": "native_combat_cast.rs", "companion_haste": "native_combat_cast.rs",
    "wheel_combat": "native_combat_cast.rs", "avatar_state": "native_combat_cast.rs",
    "monster_ai_override": "native_combat_cast.rs", "mass_spirit_mend": "native_combat_cast.rs",
    "mana_shield_capacity": "native_combat_cast.rs",
    "acquire_summon": "native_companion_item_cast.rs",
    "tile_item_operation": "native_world_item_cast.rs",
    "barrier": "native_world_item_cast.rs", "vertical_move": "parameter_cast.rs",
    "locate_message": "parameter_cast.rs", "house_access": "parameter_cast.rs",
}
MISSING = {
    "equipment_attack": "No Channel dispatcher for this qualified native profile; equipment planner is data/component-only.",
    "delayed_strike": "No Channel dispatcher installs this profile's initial delayed strike; timer infrastructure alone is insufficient.",
    "owned_field_buff": "No Channel dispatcher for owned field buff installation and ownership.",
    "creature_appearance": "No Channel dispatcher for creature-name appearance acquisition/installation.",
}


def evidence(path):
    return {"path": path, "sha256": hashlib.sha256((ROOT / path).read_bytes()).hexdigest()}


def build():
    catalog = json.loads((ROOT / CATALOG).read_text())
    selection = json.loads((ROOT / SELECTION).read_text())
    wiki = json.loads((ROOT / WIKI).read_text())
    fresh = json.loads((ROOT / FRESH).read_text())
    aliases = {alternative["key"] for row in selection["selections"] for alternative in row["alternatives"]}
    groups = {}
    for entry in catalog["bundles"]:
        spell = entry["bundle"]["spell"]
        if spell["carrier"] == "instant":
            groups.setdefault(spell["words"].lower(), []).append(entry)
    # executable_catalog::equivalent_body removes only identity/name and compares
    # the whole remaining spell and dependencies. Equivalent groups select first.
    for entries in groups.values():
        bodies = [({key: value for key, value in entry["bundle"]["spell"].items()
                    if key not in ("identity", "name")}, entry["dependencies"]) for entry in entries]
        if len(entries) > 1 and all(body == bodies[0] for body in bodies):
            aliases.update(entry["bundle"]["spell"]["identity"]["key"] for entry in entries[1:])
    rows = []
    for entry in sorted(catalog["bundles"], key=lambda value: value["bundle"]["spell"]["identity"]["key"]):
        spell = entry["bundle"]["spell"]
        native = spell["execution"].get("native_behavior")
        mechanic = native["key"] if native else next(iter(spell["execution"]))
        if any(effect.get("pvp_safe_item") for effect in entry["dependencies"]["effects"]):
            mechanic = "barrier"
        route = ROUTES.get(mechanic)
        status = "existing_route_requires_owner_facts"
        reason = "Existing dispatcher only; each cast still requires current source/access/physical/durable owners and scenario qualification."
        if mechanic in MISSING:
            status, reason = "missing_runtime_composition", MISSING[mechanic]
        elif mechanic == "locate_message" and native["parameters"]["source"] != "online_player":
            status, reason = "missing_runtime_composition", "Find Fiend nearest_fiendish owner reader/dispatcher is absent; Find Person route does not implement it."
        elif mechanic == "house_access":
            status, reason = "owner_unavailable", "Aleta parameter route exists but real HouseInteriorOwner returns HouseInteriorOwnerUnavailable."
        elif route is None:
            raise ValueError("unclassified mechanic: " + mechanic)
        if spell["identity"]["key"] in aliases:
            status, reason = "unselected_source_alternative", "Duplicate-incantation policy (explicit or equivalent-body first selection) preserves this definition but refuses its source index."
        historical = [row for row in wiki["rows"] if row["name"] == spell["name"].lower() and row["spell_type"] == spell["carrier"]]
        fresh_checks = []
        if spell["carrier"] == "instant":
            for observation in fresh["fresh_facts"]:
                if observation["url"].rsplit("/", 1)[-1].replace("_", " ").lower() != spell["name"].lower():
                    continue
                facts = observation.get("facts", {})
                values = {"words": spell["words"], "minimum_level": spell["requirements"]["level"],
                          "mana": spell["costs"].get("mana"), "base_power": spell.get("base_power"),
                          "individual_cooldown_ms": spell["cooldown_ms"],
                          "group_cooldown_ms": spell["groups"][0]["cooldown_ms"],
                          "premium_required": spell["requirements"].get("premium")}
                fresh_checks.append({"url": observation["url"], "method": observation["method"],
                                     "header_comparison": [{"field": field, "authored": value, "wiki": facts[field],
                                                            "verdict": "agree" if value == facts[field] else "differs"}
                                                           for field, value in values.items() if field in facts],
                                     "mechanics_qualification": "Fresh factual evidence does not prove gameplay execution or a damage formula."})
        rows.append({"identity": spell["identity"], "name": spell["name"], "carrier": spell["carrier"],
                     "mechanic": mechanic, "import_status": "catalog_present", "route_status": status,
                     "route": "apps/game-server/src/gameplay_transport/" + route if route else None,
                     "reason": reason, "premium_required": spell["requirements"].get("premium", False),
                     "premium_live_limit": "Node premium_coordinator is None; no fabricated entitlement" if spell["requirements"].get("premium") else None,
                     "source_identities": entry["source_identities"],
                     "historical_wiki_difference_fields": [{"field": row["field"], "verdict": row["verdict"]} for row in historical],
                     "historical_wiki_note": "2026-09-28 comparison against its original authoring revision; differences are not current-candidate defect claims.",
                     "fresh_wiki_header_checks": fresh_checks,
                     "gameplay_qualification": "not_established_by_this_inventory"})
    if len(rows) != 246 or len({row["identity"]["key"] for row in rows}) != 246:
        raise ValueError("incomplete player identity closure")
    paths = [CATALOG, SELECTION, WIKI, FRESH, "apps/game-server/src/spell/authoring.rs",
             "apps/game-server/src/spell/mod.rs", "apps/game-server/src/spell/house_execution.rs",
             "apps/game-server/src/spell/executable_catalog.rs",
             "apps/game-server/src/node/serve.rs"]
    paths += ["apps/game-server/src/gameplay_transport/" + route for route in sorted(set(ROUTES.values()))]
    return {"schema": "OTERYN_PLAYER_SPELL_ROUTE_INVENTORY/v1", "sources": [evidence(path) for path in paths],
            "summary": {"catalog_records": len(rows), "route_status": dict(sorted(Counter(row["route_status"] for row in rows).items()))},
            "scope": "Static, exact-file-bound route inventory; import completeness is separate from successful cast or production admission.",
            "source_access": "Pinned Canary/Crystal Git source and preserved wiki captures; three player spell headers additionally compared with r24 fresh Remote Desktop + Chrome/CDP public wiki observations.",
            "rows": rows}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(build(), ensure_ascii=False, indent=2) + "\n")
