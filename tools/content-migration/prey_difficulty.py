#!/usr/bin/env python3
"""Generate the PREY-DIFF classification, model inputs and pinned runtime values.

Thresholds, prices and sizes are owner-authored data. Combat rates are produced by
the Rust model through real combat rules; a missing/stale model never becomes a
fabricated rate. register_spell_families calls apply_native before hashing its
Creature provider, so the active generation owns every runtime value.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tempfile
from collections import defaultdict
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = "rulesets/prey/difficulty.json"
PROFILES = "content/creatures/definitions/spell-native-profiles.json"
LOOT = "tools/content-schema/native-gameplay/loot-tables.json"
TIERS = "content/creatures/prey-tiers.json"
VALUES = "content/creatures/prey-slot-values.json"
RESULTS = "tools/content-schema/native-gameplay/prey-model-results.json"
PARITY = "content/combat/parity/tibiapal_auto_attack_v1.json"
MICROS = 1_000_000


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def digest(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def read(root, path):
    return json.loads((root / path).read_bytes())


def authoring(root):
    config = read(root, CONFIG)
    thresholds = config["experience_thresholds"]
    levels = config["minimum_levels"]
    if (config["schema"] != "OTERYN_PREY_DIFFICULTY_AUTHORING/v1"
            or len(thresholds) != 4 or thresholds != sorted(set(thresholds))
            or any(type(value) is not int or value <= 0 for value in thresholds)
            or len(levels) != 5 or levels != sorted(levels) or levels[0] != 1
            or any(type(value) is not int or not 1 <= value <= 2**32 - 1 for value in levels)
            or any(type(config[key]) is not int for key in ("list_size", "expanded_list_size"))
            or not 1 <= config["list_size"] <= config["expanded_list_size"] <= 64
            or any(type(config[key]) is not int or not 0 <= config[key] <= 2**32 - 1
                   for key in ("pick_again_list_multiplier_ppm", "pick_again_value_factor_ppm"))
            or config["defense_mode"] not in ("received_damage", "dealt_or_attacked")
            or config["npc_price_policy"] not in ("maximum_catalogue_offer", "minimum_catalogue_offer")):
        raise ValueError("PREY_DIFFICULTY_AUTHORING_INVALID")
    model = config["model"]
    if (sorted(row["tier"] for row in model["presets"]) != [1, 2, 3, 4, 5]
            or any(type(model[key]) is not int or not 1 <= model[key] <= 2**32 - 1
                   for key in ("horizon_hours", "duel_horizon_seconds", "duel_samples"))
            or type(model["seed"]) is not int or not 0 <= model["seed"] <= 2**64 - 1):
        raise ValueError("PREY_MODEL_AUTHORING_INVALID")
    return config


def eligible_profiles(document):
    """The same membership as durability/prey/draw.rs, before draw exclusions."""
    rows = {}
    for record in document["records"]:
        profile = record["profile"]
        if profile["data"]["kind"] != "Creature":
            continue
        value = profile["data"]["profile"]
        details = value.get("details")
        key = profile["target"]["key"]
        if (details is None or value.get("bestiary") is None
                or value.get("experience", 0) == 0
                or not details["system_eligibility"]["prey"]
                or not key.startswith("oteryn:")):
            continue
        if key in rows:
            raise ValueError(f"PREY_DUPLICATE_CREATURE:{key}")
        rows[key] = record
    return dict(sorted(rows.items()))


def classify(document, config):
    records = []
    for key, record in eligible_profiles(document).items():
        value = record["profile"]["data"]["profile"]
        details = value["details"]
        records.append({
            "creature": key,
            "display_name": details["display_name"],
            "experience": value["experience"],
            "max_health": value["health"],
            "tier": 1 + sum(value["experience"] >= cut for cut in config["experience_thresholds"]),
            "exclusive": details["system_eligibility"]["exclusive_prey"],
        })
    summaries = []
    for tier in range(1, 6):
        group = sorted((row for row in records if row["tier"] == tier and not row["exclusive"]),
                       key=lambda row: (row["experience"], row["max_health"], row["creature"]))
        summaries.append({
            "tier": tier,
            "minimum_level": config["minimum_levels"][tier - 1],
            "experience_min": 1 if tier == 1 else config["experience_thresholds"][tier - 2],
            "experience_max": None if tier == 5 else config["experience_thresholds"][tier - 1] - 1,
            "nonexclusive_count": len(group),
            "normal_three_slot_capacity": len(group) >= 3 * config["list_size"],
            "expanded_three_slot_capacity": len(group) >= 3 * config["expanded_list_size"],
            "examples": [group[(len(group) - 1) * i // 7]["display_name"] for i in range(8)] if group else [],
        })
    return {"schema": "OTERYN_PREY_TIERS/v1", "score": "experience; max_health and canonical key break ties",
            "proposal_status": config["proposal_status"], "tiers": summaries, "records": records}


def npc_prices(root, policy):
    prices = defaultdict(list)
    for path in sorted((root / "content/services/trade").glob("trade-*.json")):
        for row in json.loads(path.read_bytes())["records"]:
            for offer in row["declaration"].get("offers", []):
                if offer["direction"] == "BuyFromPlayer":
                    prices[offer["item"]["key"]].append(offer["unit_price"])
    choose = max if policy == "maximum_catalogue_offer" else min
    values = {key: choose(offers) for key, offers in sorted(prices.items())}
    # Read the closed denomination owner instead of a second hardcoded gold table.
    source = (root / "crates/gameplay/domain/src/domain/currency.rs").read_text()
    keys = dict(re.findall(r'Self::(Gold|Platinum|Crystal) => "([^"]+)"', source))
    amounts = dict(re.findall(r'Self::(Gold|Platinum|Crystal) => ([\d_]+),', source))
    if keys.keys() != {"Gold", "Platinum", "Crystal"} or amounts.keys() != keys.keys():
        raise ValueError("PREY_COIN_OWNER_SHAPE_CHANGED")
    values.update({keys[k]: int(amounts[k].replace("_", "")) for k in keys})
    return dict(sorted(values.items()))


def expected_loot(entries, prices, parent=Fraction(1)):
    """Independent source chance x average count x best NPC sell price, in gp."""
    total = Fraction(0)
    for entry in entries:
        chance = Fraction(entry["probability_ppm"], MICROS)
        count = Fraction(entry["min_count"] + entry["max_count"], 2)
        total += parent * chance * count * prices.get(entry["item"]["key"], 0)
        # Children roll once within the spawned container; container count is not
        # an extra independent child loot roll in the existing nested owner.
        total += expected_loot(entry.get("children", []), prices, parent * chance)
    return total


def loot_values(loot, prices, keys):
    tables = {row["identity"]["key"]: row for row in loot["tables"]}
    bindings = {row["creature"]["key"]: row["loot"] for row in loot["creature_loot"]}
    values = {}
    for key in keys:
        ref = bindings.get(key)
        table = tables[ref["key"]] if ref is not None else None
        if table is not None and table["algorithm"] not in ("IndependentBernoulliPpm", "NestedGroups"):
            raise ValueError(f"PREY_LOOT_ALGORITHM_REQUIRES_VALUATION:{key}")
        amount = expected_loot(table["entries"], prices) if table else Fraction(0)
        values[key] = amount.numerator * MICROS // amount.denominator
    return values


def source_points(root):
    points = defaultdict(list)
    bosses = []
    for path in sorted((root / "content/world/spawns").glob("spawns-*.json")):
        for row in json.loads(path.read_bytes())["records"]:
            group = row["declaration"]
            for index, point in enumerate(group["points"]):
                fact = {"creature": point["creature"], "source_path": path.relative_to(root).as_posix(),
                        "source_group": group["identity"]["key"], "source_point": index,
                        "cell": point["cell"], "respawn_ms": point.get("respawn_ms")}
                if "boss_policy" in point:
                    fact["boss_policy"] = point["boss_policy"]
                    bosses.append(fact)
                else:
                    points[point["creature"]].append(fact)
    return points, bosses


def item_facts(root, keys):
    found = {}
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            if key in keys:
                found[key] = {"definition": definition, "source_path": path.relative_to(root).as_posix()}
    if found.keys() != keys:
        raise ValueError(f"PREY_PRESET_ITEM_MISSING:{sorted(keys - found.keys())}")
    return found


def known(value):
    if value.get("state") != "KNOWN":
        raise ValueError("PREY_PRESET_ITEM_FACT_UNKNOWN")
    return value["value"]


def model_input(root, profiles=None):
    config = authoring(root)
    profiles = read(root, PROFILES) if profiles is None else profiles
    tiers = classify(profiles, config)
    points, bosses = source_points(root)
    by_key = {row["profile"]["target"]["key"]: row for row in profiles["records"]}
    eligible = eligible_profiles(profiles)
    presets = []
    requested = {row["weapon"] for row in config["model"]["presets"]}
    requested.update(item for row in config["model"]["presets"] for item in row.get("equipment", []))
    requested.update(row["shield_item"] for row in config["model"]["presets"] if row.get("shield_item"))
    items = item_facts(root, requested)
    grid = read(root, PARITY)["grid"]
    for preset in config["model"]["presets"]:
        weapon = known(items[preset["weapon"]]["definition"]["semantics"]["weapon"])
        attack = known(weapon["attack"])
        if preset["level"] not in grid["levels"] or preset["skill"] not in grid["skills"]:
            raise ValueError("PREY_PRESET_NOT_IN_EXISTING_PARITY_GRID")
        armor = 0
        defense = known(weapon["defense"])
        equipment = list(preset.get("equipment", []))
        shield = bool(preset.get("shield_item"))
        if shield:
            equipment.append(preset["shield_item"])
            shield_weapon = known(items[preset["shield_item"]]["definition"]["semantics"]["weapon"])
            defense = known(shield_weapon["defense"])
        # These are effective-stat benchmark inputs, not a claim of complete
        # runtime Item admission. Unknown extra defence is not credited.
        if weapon.get("extra_defense", {}).get("state") == "KNOWN":
            defense += known(weapon["extra_defense"])
        for key in [preset["weapon"], *equipment]:
            semantics = items[key]["definition"]["semantics"]
            patterns = known(known(semantics["equipment"])["patterns"])
            if not any((pattern.get("level", {}).get("state") != "KNOWN"
                        or known(pattern["level"]) <= preset["level"])
                       and (pattern.get("vocations", {}).get("state") != "KNOWN"
                            or "KNIGHT" in known(pattern["vocations"])) for pattern in patterns):
                raise ValueError(f"PREY_PRESET_EQUIPMENT_NOT_ALLOWED:{key}")
        for key in equipment:
            semantics = items[key]["definition"]["semantics"]
            protection = semantics.get("protection", {})
            if protection.get("state") == "KNOWN":
                armor += known(protection["value"]["armor"])
        presets.append({**preset, "equipment": equipment, "attack": attack, "armor": armor,
                        "defense": defense, "shield": shield, "source": PARITY,
                        "item_sources": {k: items[k]["source_path"] for k in [preset["weapon"], *equipment]}})
    models = []
    used = set(eligible)
    for tier in range(1, 6):
        def physical_target(key):
            value = by_key[key]["profile"]["data"]["profile"]
            return "physical" not in value.get("immunities", []) and all(
                resistance["damage_type"] != "physical"
                or resistance["percent"]["numerator"] < 100 * resistance["percent"]["denominator"]
                for resistance in value.get("resistances", []))

        candidates = sorted((row for row in tiers["records"] if row["tier"] == tier
                             and not row["exclusive"] and points.get(row["creature"])
                             and by_key[row["creature"]].get("monster_melee")
                             and physical_target(row["creature"])),
                            key=lambda row: (row["experience"], row["max_health"], row["creature"]))
        if len(candidates) < 3:
            raise ValueError(f"PREY_MODEL_TIER_NEEDS_THREE_SOURCE_CREATURES:{tier}")
        start = max(0, (len(candidates) - 3) // 2)
        chosen = candidates[start:start + 3]
        route = [points[row["creature"]][0] for row in chosen]
        boss_options = [p for p in bosses if p["creature"] in by_key and by_key[p["creature"]].get("monster_melee")]
        if not boss_options:
            raise ValueError("PREY_MODEL_BOSS_SOURCE_MISSING")
        health = chosen[0]["max_health"]
        boss = min(boss_options, key=lambda p: (abs(by_key[p["creature"]]["profile"]["data"]["profile"]["health"] - health), p["creature"]))
        used.add(boss["creature"])
        for name, selection in (("single", route[:1]), ("mixed", route), ("boss", [route[0], boss])):
            models.append({"name": name, "tier": tier, "chosen": route[0]["creature"], "points": selection})
    creatures = {}
    for key in sorted(used):
        record = by_key[key]
        value = record["profile"]["data"]["profile"]
        details = value["details"]
        creatures[key] = {"name": details["display_name"], "health": value["health"],
                          "defense": details["defense"], "armor": value.get("armor", 0),
                          "mitigation": value.get("mitigation"), "resistances": value.get("resistances", []),
                          "damage_immunities": value.get("immunities", []),
                          "melee": record.get("monster_melee")}
    result = {"schema_version": 1, "seed": config["model"]["seed"],
              "horizon_hours": config["model"]["horizon_hours"], "duel_samples": config["model"]["duel_samples"],
              "duel_horizon_seconds": config["model"]["duel_horizon_seconds"],
              "presets": presets, "creatures": creatures, "tiers": {r["creature"]: r["tier"] for r in tiers["records"]},
              "models": models,
              "authoring_sha256": digest({"experience_thresholds": config["experience_thresholds"], "model": config["model"]}),
              "creature_source_sha256": digest({key: value for key, value in profiles.items() if key != "prey_rules"}),
              "loot_admission_sha256": hashlib.sha256((root / LOOT).read_bytes()).hexdigest(),
              "combat_sources": {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in (
                  "crates/gameplay/dispatch/examples/prey_time_sim.rs",
                  "crates/gameplay/combat-rules/src/attack/formulas.rs",
                  "crates/gameplay/combat-rules/src/attack/constants.rs",
                  "crates/gameplay/combat-rules/src/attack/block.rs",
                  "crates/gameplay/combat-rules/src/resolution.rs",
                  "crates/gameplay/combat-rules/src/resolution/player_defence.rs",
                  "crates/gameplay/characters/src/stamina.rs",
                  "crates/simulation-determinism/src/lib.rs",
                  "crates/gameplay/progression/src/prey.rs",
                  "crates/gameplay/progression/src/prey/difficulty.rs",
                  "crates/gameplay/content/src/content/creature_reward.rs",
                  "crates/gameplay/content/src/content/creature_policy.rs",
                  "crates/gameplay/content/src/content/spell_data/formula.rs",
                  "content/combat/attack_constants_v1.json")}}
    result["input_sha256"] = digest(result)
    return result


def report_files(config, tiers, values, results):
    date = config["model"]["date"]
    names = {row["creature"]: row["display_name"] for row in tiers["records"]}
    ordinary = {row["creature"] for row in tiers["records"] if not row["exclusive"]}
    available = ordinary & set(results["available_creatures"])
    unsettled = ordinary & set(results["unsettleable_creatures"])
    table = ["| Spawn model | Tier / level | Chosen race | Global | Offensive | Defense V1 | Defense V2 |",
             "|---|---|---|---:|---:|---:|---:|"]
    presets = {row["tier"]: row for row in results["presets"]}
    for case in results["cases"]:
        def duration(key, index):
            value = case[key]
            if value is not None:
                return f"{value / (60 * MICROS):.2f}"
            observed = case["observed_micros"] / (60 * MICROS)
            return f"not exhausted in {observed:.2f} ({case['consumed_seconds'][index]} s consumed)"
        columns = [duration(key, i) for i, key in enumerate(("global_micros", "offensive_micros", "defense_v1_micros", "defense_v2_micros"))]
        table.append(f"| {case['name']} | {case['tier']} / {presets[case['tier']]['level']} | {names[case['chosen']]} | " + " | ".join(columns) + " |")
    heading = [f"# PREY-DIFF time simulation — {date}", "",
               "Generated by `python3 tools/content-migration/prey_difficulty.py --simulate`; do not edit.", "",
               f"Input SHA-256: `{results['input_sha256']}`. Seed: `{results['seed']}`. Prey duration: `{results['prey_seconds']}` seconds, read from `progression::prey::HUNTING_SECONDS`.", "",
               f"Sampling: up to {config['model']['duel_samples']} seeded duels per creature; each duel is bounded to {config['model']['duel_horizon_seconds']} seconds. Sampling stops at the first duel without a kill; a positive throughput estimate requires all {config['model']['duel_samples']} duels to finish with kills. Each route is observed for at most {config['model']['horizon_hours']} hours, or until all four time policies exhaust the Prey.", "",
               "## Model and interpretation", "",
               "The simulator calls the production `player_melee_formula`, `creature_mitigated`, `creature_physical_response`, `swing_mitigated`, `BlockBudget`, `PreyDefenseMode::consumes` and `HuntingActivityClock`. It does not divide elapsed time by a kill share. A short gap between chosen-race hits (at most 60 seconds) still advances the same activity clock, so a one-third kill share need not triple the lifetime.", "",
               "Each tier uses a knight benchmark drawn from the existing auto-attack parity fixture grid, plus sourced Item equipment. Effective skill is the fixture input. These are reproducible model presets proposed as the initial typical-build approximation; owner approval is pending. They are not measurements of player builds or proof that every Item is currently admitted. Only known source armor/defense and known weapon extra defense are credited; unknown extra defense contributes no benchmark bonus. Tier thresholds and minimum levels remain the proposal recorded in the PR until owner approval.", "",
               "Single-race and mixed models use actual source spawn points and their respawn delays. The mixed model cycles three distinct races; the observed kill share below makes departures from one-third visible. The boss model combines the chosen point with one source-qualified boss point, conditioned on that boss being present; the boss is attempted once and does not respawn. An attempt which reaches the duel bound without a kill still contributes its observed combat time. These are synthetic model routes assembled from named source points, not a claim that the points form one walkable hunt route.", "",
               "The model is sustained physical auto-attack combat with identical unboosted damage for the four time-policy comparisons. Travel, healing costs, death, area spells, kiting, critical hits and non-melee monster spells are excluded. A supplied native monster-melee profile is used exactly; missing melee is reported, never invented. Coin and NPC values use content, not market prices. Per-creature throughput is an estimate for continuous access to that race, measured over the configured seeded duels; real spawn travel and waiting lower it. It is intentionally replaceable by the post-release counters.", "",
               "Global receives positive actual damage dealt/received from any race. Offensive receives positive damage dealt to the chosen race. Defense V1 receives positive incoming chosen-race damage. Defense V2 receives outgoing chosen-race damage or a chosen-race attack attempt, including a zero-damage attack. No Magic Shield is equipped in these presets. Both variants share the unchanged minute clock, including its idle/stale-event behavior.", "",
               "All table values are wall-clock minutes in the model. A cell marked not exhausted is a bounded observation, not an invented finite lifetime.", "", *table, "", "## Presets", "",
               "| Tier | Level | Effective skill | Weapon | Armor | Defense | Equipment |", "|---|---:|---:|---|---:|---:|---|"]
    for preset in results["presets"]:
        heading.append(f"| {preset['tier']} | {preset['level']} | {preset['skill']} | `{preset['weapon']}` | {preset['armor']} | {preset['defense']} | " + ", ".join(f"`{item}`" for item in preset["equipment"]) + " |")
    heading.extend(["", "Equipment source paths (canonical keys are shown above): " + ", ".join(
        f"`{path}`" for path in sorted({path for preset in results["presets"] for path in preset["item_sources"].values()})) + ".", "",
        "## Source spawn points and observed kill share", ""])
    by_case = {(case["name"], case["tier"]): case for case in results["cases"]}
    for model in results["models"]:
        case = by_case[(model["name"], model["tier"])]
        heading.append(f"- {model['name']}, tier {model['tier']}: chosen kills {case['chosen_kills']}/{case['kills']}; " + "; ".join(
            f"`{point['source_path']}` / `{point['source_group']}` point {point['source_point']} (`{point['creature']}`, respawn {point['respawn_ms']} ms)" for point in model["points"]))
    heading.extend(["", "## Limits of the current content", "",
                    f"- {len(results['no_source_melee'])} eligible creatures have no native melee profile. Their throughput estimates still use their real defensive statistics, but an incoming melee attack is not synthesized.",
                    f"- {len(results['no_physical_kill'])} eligible creatures did not complete all {config['model']['duel_samples']} required bounded duels with kills. Sampling stopped at their first failed duel, and their generated physical throughput is zero, explicitly listed in the value report.",
                    f"- The compiled reward owner admits {len(ordinary) - len(unsettled)} settleable ordinary rows out of {len(ordinary)}; {len(unsettled)} ordinary rows currently refuse settlement. The existing >7 admitted-loot-entry guard excludes {len(ordinary) - len(available)} ordinary candidates. These counts come from `CreatureRewardTable::from_pin`, not raw source entry counts.",
                    "- Current reward admission can refuse settlement even when a creature is Prey-eligible. The existing unsupported-loot guard asks the compiled reward owner; an unadmitted row is not equivalent to a large admitted loot table. This task preserves that rule. The value estimate reads source loot potential; post-release telemetry measures only committed loot, so current missing admissions can make those differ.", ""])
    value_lines = [f"# PREY-DIFF slot value estimates — {date}", "",
                   "Generated by the same deterministic model and content generator; do not edit.", "",
                   f"Model input SHA-256: `{results['input_sha256']}`. NPC policy: `{values['npc_price_policy']}`. Source: `content/services/trade/trade-*.json`, `BuyFromPlayer` offers; coin worth from `domain/currency.rs`. Items without an NPC offer contribute zero NPC sell value. Loot expectation sums exact source chance × average count × NPC value, with exact rational arithmetic until micro-gold truncation.", "",
                   f"Coverage: {len(values['creatures'])} eligible creatures, {len(values['npc_sell_prices'])} whole-gold NPC/coin prices. {len(results['no_physical_kill'])} creatures have zero physical throughput because they did not complete all {config['model']['duel_samples']} required bounded duels with kills; sampling stopped at the first failed duel. The zero is specific to this preset and bound, not a claim about other builds.", "",
                   "Rates are estimates from real physical combat rules under the tier preset, not measured player kills. The source loot expectation is not a promise that the current runtime reward admission can settle that loot. Replace the estimates from the control-plane measured counters after release.", "",
                   "| Creature | Tier | NPC gold / kill | Estimated kills / hour | Slot value (gold) | All required bounded duels completed with kills |", "|---|---:|---:|---:|---:|---|"]
    no_kill = set(results["no_physical_kill"])
    names = {row["creature"]: row["display_name"] for row in tiers["records"]}
    for key, row in values["creatures"].items():
        gold = Fraction(row["loot_per_kill_micros"], MICROS)
        rate = Fraction(row["kills_per_hour_micros"], MICROS)
        worth = gold * rate * Fraction(results["prey_seconds"], 3600)
        value_lines.append(f"| {names[key]} (`{key}`) | {row['tier']} | {float(gold):.6f} | {float(rate):.6f} | {float(worth):.6f} | {'no — zero estimate' if key in no_kill else 'yes'} |")
    return {f"docs/balance/PREY-TIME-SIM-{date}.md": ("\n".join(heading)).encode(),
            f"docs/balance/PREY-SLOT-VALUE-{date}.md": ("\n".join(value_lines) + "\n").encode()}


def outputs(root, profiles=None):
    config = authoring(root)
    profiles = read(root, PROFILES) if profiles is None else profiles
    tiers = classify(profiles, config)
    model = model_input(root, profiles)
    results = read(root, RESULTS)
    if results["input_sha256"] != model["input_sha256"]:
        raise ValueError("PREY_MODEL_STALE: run prey_difficulty.py --simulate")
    available = set(results["available_creatures"])
    unsettleable = set(results["unsettleable_creatures"])
    for summary in tiers["tiers"]:
        group = {row["creature"] for row in tiers["records"] if row["tier"] == summary["tier"] and not row["exclusive"]}
        summary["runtime_available_count"] = len(group & available)
        summary["runtime_unsettleable_count"] = len(group & unsettleable)
        summary["normal_three_slot_capacity"] = len(group & available) >= 3 * config["list_size"]
        summary["expanded_three_slot_capacity"] = len(group & available) >= 3 * config["expanded_list_size"]
    prices = npc_prices(root, config["npc_price_policy"])
    loot = loot_values(read(root, LOOT), prices, model["tiers"])
    if results["kills_per_hour_micros"].keys() != model["tiers"].keys():
        raise ValueError("PREY_MODEL_CREATURE_COVERAGE")
    creatures = {key: {"tier": tier, "loot_per_kill_micros": loot[key],
                       "kills_per_hour_micros": results["kills_per_hour_micros"][key]}
                 for key, tier in model["tiers"].items()}
    rules = {key: config[key] for key in ("list_size", "expanded_list_size", "minimum_levels",
                                         "pick_again_list_multiplier_ppm", "pick_again_value_factor_ppm", "defense_mode")}
    rules.update(schema_version=1, creatures=creatures, npc_sell_prices=prices)
    values = {"schema": "OTERYN_PREY_SLOT_VALUES/v1", "model_input_sha256": model["input_sha256"],
              "npc_price_policy": config["npc_price_policy"], "prey_seconds": results["prey_seconds"],
              "creatures": creatures, "npc_sell_prices": prices}
    return {TIERS: encoded(tiers), VALUES: encoded(values), **report_files(config, tiers, values, results)}, rules


def apply_native(root, generated, canonical):
    """Called by the existing registration owner, before family/manifest hashes."""
    profiles = json.loads(generated[PROFILES])
    derived, rules = outputs(root, profiles)
    profiles["prey_rules"] = rules
    # Reports are generated by the CLI but do not become game-content manifest
    # members. The two content tables and the embedded provider are managed.
    generated.update({path: data for path, data in derived.items() if path.startswith("content/")})
    generated[PROFILES] = canonical(profiles)
    manifest = json.loads(generated["content/spells.manifest.json"])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(generated[PROFILES]).hexdigest()
    generated["content/spells.manifest.json"] = canonical(manifest)


def fresh_profiles(root):
    """Rebuild through the existing producer; never bootstrap from its old output."""
    import register_spell_families
    generated = register_spell_families.outputs(root, include_prey=False)
    return json.loads(generated[PROFILES])


def run_simulation(root, profiles, model):
    with tempfile.TemporaryDirectory(prefix="oteryn-prey-model-") as directory:
        input_path = Path(directory) / "input.json"
        creature_path = Path(directory) / "creature-profiles.json"
        input_path.write_bytes(encoded(model))
        creature_path.write_bytes(encoded(profiles))
        subprocess.run(["cargo", "run", "--locked", "-p", "oteryn-gameplay-dispatch", "--example", "prey_time_sim", "--",
                        "--input", str(input_path), "--output", str(root / RESULTS),
                        "--creature-profiles", str(creature_path)], cwd=root, check=True)


def regenerate(root):
    """World-tree registration calls this after rebuilding its source shards."""
    profiles = fresh_profiles(root)
    model = model_input(root, profiles)
    result = root / RESULTS
    if not result.is_file() or read(root, RESULTS).get("input_sha256") != model["input_sha256"]:
        run_simulation(root, profiles, model)
    files, _ = outputs(root, profiles)
    write_files(root, files)


def write_files(root, files, check=False):
    for path, data in files.items():
        target = root / path
        if check:
            if not target.is_file() or target.read_bytes() != data:
                raise ValueError(f"PREY_GENERATED_DRIFT:{path}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--classify", action="store_true")
    parser.add_argument("--simulate", action="store_true")
    parser.add_argument("--model-input", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    if args.check and (args.simulate or args.model_input):
        parser.error("--check cannot write model inputs or simulation results")
    profiles = fresh_profiles(root)
    if args.model_input:
        args.model_input.write_bytes(encoded(model_input(root, profiles)))
        return
    if args.simulate:
        run_simulation(root, profiles, model_input(root, profiles))
    if args.classify:
        files = {TIERS: encoded(classify(profiles, authoring(root)))}
    else:
        files, _ = outputs(root, profiles)
    write_files(root, files, args.check)
    print("PREY_DIFF_CONTENT_GENERATED", "checked" if args.check else "written", len(files))


if __name__ == "__main__":
    main()
