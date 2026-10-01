#!/usr/bin/env python3
"""Offline preparation validation; no runtime evaluator or external writes."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlparse

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
SOURCES = ROOT / "samples/charm-mechanics-sources-2026-10-01.json"
SCHEMA = ROOT / "mechanics.schema.json"
MECHANICS = REPO / "rulesets/progression/charms/mechanics.json"
PROGRESSION = REPO / "rulesets/progression/charms/progression.json"
CATALOGUE = REPO / "content/charms/charms-00000-00024.json"
INDEX = REPO / "rulesets/progression/charms/index.json"
SOURCE_SHA256 = "ee168313cd61d6fea4d91e5de434116006a239bf05425f6e646a2ebf34c31405"
RESOLUTION = ROOT / "samples/charm-source-resolution-2026-10-01.json"
RESOLUTION_SHA256 = "dd4e4d3689b42d6395ca7e017ac772e28c2c4179b9ed6069d59d3e6485cf9955"
GLOBAL = ROOT / "samples/charm-global-parity-2026-10-01.json"
GLOBAL_SHA256 = "20bbdb465e573133324ba9756da628b0ccb5948864655b36a8d5872cb076b66f"
ORIGINAL_UNKNOWN_SHA256 = (
    "d1fc314b809eae4678b6fedcd0161985932e4fc75649d032dfe9adf83c21b994"
)
OTS_REPOS = {"opentibiabr/canary", "zimbadev/crystalserver"}
HOOKS = {
    "attack_proc_damage": "after_player_health_reduction",
    "attack_proc_resource_damage": "after_player_health_reduction",
    "kill_area_damage": "committed_creature_death",
    "reflect_damage_taken": "incoming_monster_damage",
    "dodge_attack": "before_incoming_monster_damage",
    "haste_after_hit": "after_incoming_monster_hit",
    "paralyse_creature_on_attack": "after_player_health_reduction",
    "paralyse_creature_after_its_attack": "after_incoming_monster_hit",
    "cleanse_after_hit": "incoming_monster_condition",
    "prevent_creature_flee": "player_health_combat",
    "mana_drain_inversion": "incoming_monster_mana_drain",
    "critical_hit_chance": "critical_chance_resolution",
    "critical_extra_damage": "critical_damage_resolution",
    "life_leech": "leech_amount_resolution",
    "mana_leech": "leech_amount_resolution",
    "death_loss_reduction": "character_death_penalty",
    "creature_product_bonus": "creature_product_loot_roll",
    "skinning_chance_bonus": "corpse_skin_or_dust_attempt",
}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dumps(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def build(sample: dict) -> tuple[dict, dict]:
    shared = {
        "catalogue": sample["catalogue"],
        "status": "PREPARATION_ONLY",
        "runtime_connected": False,
        "evidence": "tools/content-schema/charm-authoring/samples/charm-mechanics-sources-2026-10-01.json",
    }
    mechanics = {
        **shared,
        "schema": "OTERYN_CHARM_MECHANICS_PREPARATION/v1",
        "ruleset_key": "oteryn:ruleset.charms.mechanics",
        "common_rules": sample["common_rules"],
        "charms": sample["mechanics"],
    }
    progression = {
        **shared,
        "schema": "OTERYN_CHARM_PROGRESSION_PREPARATION/v1",
        "ruleset_key": "oteryn:ruleset.charms.progression",
        "rules": sample["progression_rules"],
        "excluded_reference_only": sample["excluded_reference_only"],
        "conflicts": sample["progression_conflicts"],
        "unknowns": sample["progression_unknowns"],
    }
    return copy.deepcopy(mechanics), copy.deepcopy(progression)


def evidence_values(value: object):
    if isinstance(value, dict):
        if {"value", "status", "references", "activation"} <= value.keys():
            yield value
        for child in value.values():
            yield from evidence_values(child)
    elif isinstance(value, list):
        for child in value:
            yield from evidence_values(child)


def validate_resolution(package: dict, sample: dict) -> list[str]:
    """Keep source tracing and proposed consumer choices separate from activation."""
    errors = []
    if (
        package.get("schema") != "OTERYN_CHARM_SOURCE_RESOLUTION/v1"
        or package.get("repository") != "Oteryn/Oteryn-Game"
        or package.get("pr_number") != 1434
        or package.get("predecessor_sha") != "ca991aa1fb12fb656ea3d39f5736d9745769027e"
    ):
        errors.append("resolution target drift")
    for field in ["runtime_connected", "official_parity_proven", "activation"]:
        if package.get(field) is not False:
            errors.append(f"resolution {field} must remain false")
    origin = package.get("original_unknowns", [])
    pin = digest(json.dumps(origin, sort_keys=True, separators=(",", ":")).encode())
    if len(origin) != 32 or pin != ORIGINAL_UNKNOWN_SHA256:
        errors.append("resolution original unknown inventory drift")
    expected = {(row["key"], row["text"]) for row in origin}
    coverage = package.get("coverage", [])
    observed = [(row.get("key"), row.get("original_unknown")) for row in coverage]
    if len(observed) != 32 or len(set(observed)) != 32 or set(observed) != expected:
        errors.append("resolution coverage must include each original entry once")
    lanes = package.get("lanes", {})
    if set(lanes) != {
        "conditions",
        "damage",
        "defensive",
        "critical-leech",
        "passives",
    }:
        errors.append("resolution lane coverage drift")
    for row in coverage:
        if row.get("lane") not in lanes:
            errors.append("resolution unknown lane")
        if row.get("official_parity_proven") is not False:
            errors.append("resolution row cannot claim official parity")
        if row.get("runtime_activation") is not False:
            errors.append("resolution row cannot activate runtime")
        if row.get("result") not in {
            "STALE_HANDOFF_CORRECTED",
            "SOURCE_TRACE_AND_CONCRETE_RECOMMENDATION",
        }:
            errors.append("resolution outcome drift")
    sources = package.get("sources", {})
    for sid, source in sources.items():
        if sid != source.get("repository", "") + ":" + source.get("path", ""):
            errors.append("resolution source identity drift")
        if not re.fullmatch(r"[a-f0-9]{40}", source.get("revision", "")):
            errors.append("resolution source revision must be pinned")
        if not re.fullmatch(r"[a-f0-9]{64}", source.get("sha256", "")):
            errors.append("resolution source hash must be pinned")
        if (
            source.get("repository") in OTS_REPOS
            and source.get("evidence_class") != "OTS_HYPOTHESIS_ONLY"
        ):
            errors.append("resolution OTS source promotion")
    for sid, source in sample["sources"].items():
        if sources.get(sid) != source:
            errors.append("resolution captured source binding drift")
    if not package.get("adoption_boundary"):
        errors.append("resolution must state consumer adoption boundary")
    if digest(dumps(package).encode()) != RESOLUTION_SHA256:
        errors.append("resolution captured packet drift; review before repinning")
    return errors


def validate_global(package: dict, catalogue: dict) -> list[str]:
    """Bind public evidence without claiming connected or official server proof."""
    errors = []
    if (
        package.get("schema") != "OTERYN_CHARM_GLOBAL_PARITY_RESEARCH/v1"
        or package.get("repository") != "Oteryn/Oteryn-Game"
        or package.get("pr_number") != 1434
        or package.get("as_of") != "2026-10-01"
        or package.get("predecessor_sha") != "b588cd46413c18837643a593002609b02a03fd11"
    ):
        errors.append("global research target drift")
    for field in ["activation", "runtime_connected", "official_runtime_parity_proven"]:
        if package.get(field) is not False:
            errors.append("global research cannot activate or claim server parity")
    if package.get("catalogue_sha256") != digest(
        (ROOT / "samples/charms-candidate.json").read_bytes()
    ):
        errors.append("global research catalogue binding drift")
    expected = {row["key"]: row for row in catalogue["charms"]}
    profiles = package.get("profiles", [])
    if len(profiles) != 25 or {p.get("key") for p in profiles} != set(expected):
        errors.append("global research requires all25 unique profiles")
    sources = package.get("sources", {})
    for sid, source in sources.items():
        if source.get("id") != sid or not source.get("version_scope"):
            errors.append("global source identity or version scope missing")
        if source.get("authority") not in {
            "OFFICIAL_PUBLISHER",
            "COMMUNITY",
            "OTS_REFERENCE",
        }:
            errors.append("global source authority missing")
        if source.get("authority") == "OFFICIAL_PUBLISHER" and urlparse(
            source.get("url", "")
        ).hostname not in {"tibia.com", "www.tibia.com"}:
            errors.append("global unofficial source promoted")
        if source.get("access_kind") not in {"INDEXED_SNIPPET", "CAPTURED_CONTENT"}:
            errors.append("global source access kind missing")
        if not re.fullmatch(r"[a-f0-9]{64}", source.get("raw_artifact_sha256", "")):
            errors.append("global raw capture hash missing")
    claims = package.get("claims", [])
    ids = {c.get("id") for c in claims}
    if len(ids) != len(claims):
        errors.append("global duplicate claim")
    for claim in claims:
        refs = claim.get("evidence_ids", [])
        if not refs or not set(refs) <= set(sources):
            errors.append("global claim has unresolved evidence")
        if not claim.get("limitations"):
            errors.append("global claim limitations missing")
        if (
            claim.get("field") == "mitigation"
            and claim.get("value", {}).get("elemental_resistances_apply") is True
            and set(claim.get("applies_to", []))
            & {"oteryn:charm.overpower", "oteryn:charm.overflux"}
        ):
            errors.append("global resource damage cannot inherit elemental resistance")
        if (
            claim.get("activation") is not False
            or claim.get("official_runtime_parity_proven") is not False
        ):
            errors.append("global claim cannot activate or claim server parity")
    for profile in profiles:
        row = expected.get(profile.get("key"))
        if row and (
            profile.get("catalogue_stages") != row["stages"]
            or profile.get("community_costs") != [s["cost"] for s in row["stages"]]
            or profile.get("community_stage_values")
            != [s["value"] for s in row["stages"]]
        ):
            errors.append("global profile numeric drift")
        if (
            profile.get("source_id") not in sources
            or not set(profile.get("claim_ids", [])) <= ids
        ):
            errors.append("global profile unresolved evidence")
        if (
            profile.get("activation") is not False
            or profile.get("official_runtime_parity_proven") is not False
        ):
            errors.append("global profile cannot activate or claim server parity")
    if {r.get("id") for r in package.get("supersedes_global_recommendations", [])} != {
        "gut_probability_only",
        "leech_overkill_cap",
        "parry_after_player_resistance",
        "dodge_independent_status_delivery",
    }:
        errors.append("global supersession coverage drift")
    assets = package.get("client_assets", {})
    if assets.get("official_mechanics_parity_proven") is not False or any(
        row.get("bestiary_charm_record_present") is not False
        for row in assets.get("charms", [])
    ):
        errors.append("global asset corpus cannot supply absent charm proof")
    if not package.get("owner_rule_conflicts") or not package.get("adoption_boundary"):
        errors.append("global owner boundary missing")
    if digest(dumps(package).encode()) != GLOBAL_SHA256:
        errors.append("global research captured packet drift; review before repinning")
    return errors


def validate(
    sample: dict,
    mechanics: dict,
    progression: dict,
    catalogue_bytes: bytes | None = None,
) -> list[str]:
    errors = []
    schema = load(SCHEMA)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    for name, document in [
        ("sources", sample),
        ("mechanics", mechanics),
        ("progression", progression),
    ]:
        for error in validator.iter_errors(document):
            errors.append(f"{name}: schema: {error.message[:200]}")
    if errors:
        return errors
    catalogue_bytes = (
        CATALOGUE.read_bytes() if catalogue_bytes is None else catalogue_bytes
    )
    catalogue = json.loads(catalogue_bytes)
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for r in catalogue["records"]
    }
    if sample["catalogue"]["sha256"] != digest(catalogue_bytes):
        errors.append("catalogue SHA256 drift")
    expected_keys = set(definitions)
    for name, rows in [
        ("sources", sample["mechanics"]),
        ("mechanics", mechanics["charms"]),
    ]:
        keys = [r["key"] for r in rows]
        if len(keys) != 25 or len(set(keys)) != 25 or set(keys) != expected_keys:
            errors.append(f"{name}: catalogue key coverage")
        for row in rows:
            if row["key"] not in definitions:
                continue
            effect = definitions[row["key"]]["effect"]["type"]
            if row["effect_type"] != effect or row["hook"] != HOOKS[effect]:
                errors.append(f"{row['key']}: effect type/hook drift")
            if not set(row["common_rules"]) <= set(sample["common_rules"]):
                errors.append(f"{row['key']}: undefined common rule")
    for sid, source in sample["sources"].items():
        repo = source["repository"]
        expected_class = (
            "OTS_HYPOTHESIS_ONLY"
            if repo in OTS_REPOS
            else "STRUCTURED_REFERENCE"
            if repo == "PawelKusnierek/TibiaPal"
            else "CIPSOFT_OFFICIAL_CAPTURE"
            if source["path"].startswith("docs/reference/tibia-manual/")
            else "PROJECT_ACCEPTED_RECORD"
            if any(
                k in source["path"]
                for k in ["OTERYN_GAME_CHARM0_", "OWNER_DECISION_BATCH_"]
            )
            else "PROJECT_IMPLEMENTATION_REFERENCE"
        )
        if source["evidence_class"] != expected_class:
            errors.append(f"{sid}: source class promotion")
    for fact in evidence_values(sample):
        classes = []
        for reference in fact["references"]:
            source = sample["sources"].get(reference["source"])
            if source is None:
                errors.append("unknown source reference")
                continue
            classes.append(source["evidence_class"])
            if reference["lines"][0] > reference["lines"][1]:
                errors.append("reversed source line bounds")
        if fact["activation"]:
            errors.append("preparation fact activation prohibited")
        if (
            fact["status"] == "OWNER_ACCEPTED"
            and "PROJECT_ACCEPTED_RECORD" not in classes
        ):
            errors.append("owner status promotion without accepted record")
        if fact["status"] == "STRUCTURED_REFERENCE" and not set(classes) & {
            "STRUCTURED_REFERENCE",
            "PROJECT_ACCEPTED_RECORD",
            "CIPSOFT_OFFICIAL_CAPTURE",
        }:
            errors.append("OTS hypothesis promoted to structured fact")
    comparison = sample["catalogue_comparison"]
    if (
        len(comparison["rows"]) != 25
        or {r["key"] for r in comparison["rows"]} != expected_keys
    ):
        errors.append("comparison catalogue coverage")
    for row in comparison["rows"]:
        c = definitions.get(row["key"])
        if c is None:
            continue
        expected = {
            "category": c["category"],
            "costs": [s["cost"] for s in c["stages"]],
            "values": [s["value"] for s in c["stages"]],
        }
        if row["expected"] != expected or any(
            v != expected for v in row["observed"].values()
        ):
            errors.append(f"{row['key']}: independent numeric comparison drift")
    expected_mechanics, expected_progression = build(sample)
    if mechanics != expected_mechanics:
        errors.append(
            "mechanics differs from captured facts (including unresolved statuses)"
        )
    if progression != expected_progression:
        errors.append(
            "progression differs from captured facts (including excluded references)"
        )
    # Facts are immutable captured inputs. A researched refresh requires an explicit new pin.
    if digest(dumps(sample).encode()) != SOURCE_SHA256:
        errors.append(
            "captured facts SHA256 drift; review evidence refresh before repinning"
        )
    errors += validate_resolution(load(RESOLUTION), sample)
    errors += validate_global(
        load(GLOBAL), load(ROOT / "samples/charms-candidate.json")
    )
    return errors


def verify_sources(sample: dict, checkouts: dict[str, Path]) -> list[str]:
    """Verify raw-file hashes offline; never download or redistribute original sources."""
    errors = []
    for sid, source in sample["sources"].items():
        root = checkouts.get(source["repository"])
        if root is None:
            continue
        path = root / source["path"]
        if not path.is_file() or digest(path.read_bytes()) != source["sha256"]:
            errors.append(f"{sid}: source SHA256 mismatch")
    return errors


def source_catalogue(data: bytes, planner: bool = False) -> dict:
    """Independent numeric extraction from pinned reference files."""
    text = data.decode("utf-8")
    rows = {}
    if planner:
        descriptions = re.search(
            r"var charm_description_dict = \{(.*?)\n\}", text, re.DOTALL
        )
        costs_block = re.search(
            r"var major_charms_costs = \{(.*?)\n\}", text, re.DOTALL
        )
        if not descriptions or not costs_block:
            raise ValueError("planner numeric dictionaries missing")
        minor_block = re.search(
            r"var minor_charms_costs = \{(.*?)\n\}", text, re.DOTALL
        )
        if not minor_block:
            raise ValueError("planner minor costs missing")
        minor_costs = [int(v) for _, v in re.findall(r"(\d): (\d+)", minor_block[1])]
        if len(minor_costs) != 3:
            raise ValueError("planner minor stage costs missing")
        costs = {
            k: [int(v) for v in nums.split(",")]
            for k, nums in re.findall(r'"([a-z_]+)": \[([^]]+)\]', costs_block[1])
        }
        for key, prose in re.findall(r'"([a-z_]+)": \'([^\']+)\'', descriptions[1]):
            triple = re.search(
                r"(\d+(?:\.\d+)?)/(\d+(?:\.\d+)?)/(\d+(?:\.\d+)?)", prose
            )
            if not triple:
                raise ValueError(f"planner stage triple missing: {key}")
            rows["oteryn:charm." + key] = {
                "category": "major" if key in costs else "minor",
                "costs": costs.get(key, minor_costs),
                "values": [float(v) for v in triple.groups()],
            }
    else:
        for _, body in re.findall(
            r"^\t\[(\d+)\] = \{\n(.*?)^\t\},$", text, re.MULTILINE | re.DOTALL
        ):
            name = re.search(r'name = "([^"]+)"', body)[1]
            key = name.lower().replace("'", "").replace(" ", "_")

            def triple(field, body=body, name=name):
                match = re.search(
                    rf"{field} = \{{ ([\d.]+), ([\d.]+), ([\d.]+) \}}", body
                )
                if not match:
                    raise ValueError(f"{name}: missing {field}")
                return [float(v) for v in match.groups()]

            rows["oteryn:charm." + key] = {
                "category": re.search(r"category = CHARM_(MAJOR|MINOR)", body)[
                    1
                ].lower(),
                "costs": triple("points"),
                "values": triple("chance"),
            }
    if len(rows) != 25:
        raise ValueError("reference catalogue must contain 25 distinct charms")
    return rows


def validate_index(index: dict) -> list[str]:
    expected = {
        "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
        "path": "rulesets/progression/charms/",
        "kind": "ruleset",
        "owner": "Character progression",
        "repository": "Oteryn/Oteryn-Game",
        "population_state": "POPULATED",
        "contract": "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md",
    }
    if set(index) != set(expected) | {"notes"} or any(
        index.get(k) != v for k, v in expected.items()
    ):
        return ["charm directory index shape/population drift"]
    if (
        "tools/content-schema/charm-authoring/mechanics.schema.json"
        not in index["notes"]
        or "not qualified" not in index["notes"]
    ):
        return ["charm index must identify schema and preparation limit"]
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check", "validate", "verify-sources"])
    parser.add_argument("--canary", type=Path)
    parser.add_argument("--crystal", type=Path)
    parser.add_argument("--tibiapal", type=Path)
    args = parser.parse_args(argv)
    sample = load(SOURCES)
    errors = validate(sample, load(MECHANICS), load(PROGRESSION)) + validate_index(
        load(INDEX)
    )
    if args.command == "verify-sources":
        roots = {
            repo: p
            for repo, p in [
                ("Oteryn/Oteryn-Game", REPO),
                ("opentibiabr/canary", args.canary),
                ("zimbadev/crystalserver", args.crystal),
                ("PawelKusnierek/TibiaPal", args.tibiapal),
            ]
            if p is not None
        }
        errors += verify_sources(sample, roots)
        for label, reference in sample["catalogue_comparison"]["sources"].items():
            source = sample["sources"][reference["source"]]
            root = roots.get(source["repository"])
            if root is None:
                continue
            path = root / source["path"]
            if not path.is_file() or digest(path.read_bytes()) != source["sha256"]:
                continue
            try:
                extracted = source_catalogue(path.read_bytes(), label == "tibiapal")
            except (ValueError, TypeError, IndexError) as error:
                errors.append(f"{label}: extraction failed: {error}")
                continue
            for row in sample["catalogue_comparison"]["rows"]:
                if extracted.get(row["key"]) != row["observed"][label]:
                    errors.append(f"{label}: captured extraction differs: {row['key']}")
        print(
            f"verified repositories={len(roots)} (omitted checkout roots remain unverified)"
        )
    if errors:
        print("\n".join(errors))
        return 1
    print(
        "PASS charm preparation: 25 keys; 75 source numeric rows; reference activation disabled"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
