#!/usr/bin/env python3
"""Build the offline player-Might and creature/area-Threat guide dataset."""
from __future__ import annotations

import argparse
import json
import math
from bisect import bisect_left
from collections import Counter, defaultdict
from functools import cache
from pathlib import Path
from statistics import median
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CREATURES = ROOT / "content/creatures/definitions"
ATTACKS = ROOT / "content/abilities/source-imports/monster-attack-authoring.jsonl"
AREAS = ROOT / "content/world/areas/hunting-places/hunting-places-00000-00444.json"
OUTPUT = ROOT / "content/creatures/power-score.json"
VOCATIONS = ("knight", "paladin", "sorcerer", "druid")
GEAR_MULTIPLIER = {"normal": 1.0, "bis": 1.22}
SPELL = {"knight": 0.34, "paladin": 0.48, "sorcerer": 0.82, "druid": 0.76, "monk": 0.44}
AOE_SHARE = {"knight": 0.65, "paladin": 0.72, "sorcerer": 0.90, "druid": 0.88}


def _records(path: Path) -> list[dict[str, Any]]:
    return json.loads(path.read_text()).get("records", [])


def typical_skill(vocation: str, level: int) -> int:
    """Typical trained primary skill/magic level, logarithmic after early training."""
    base = {"knight": 55, "paladin": 50, "sorcerer": 30, "druid": 30}.get(vocation, 45)
    slope = {"knight": 18, "paladin": 17, "sorcerer": 13, "druid": 13}.get(vocation, 15)
    return round(base + slope * math.log2(max(level, 8) / 8))


def player_might(
    vocation: str,
    level: int,
    gear: str = "normal",
    *,
    skill: int | None = None,
    imbuement: float = 0.0,
    wheel: float = 0.0,
    model: dict[str, float] | None = None,
) -> dict[str, float | int]:
    """Return the documented 30-second representative solo player model."""
    if vocation not in (*VOCATIONS, "monk") or gear not in GEAR_MULTIPLIER or level < 1:
        raise ValueError("unsupported vocation, gear tier, or level")
    model = _calibrated_model() if model is None else model
    trained = typical_skill(vocation, level) if skill is None else skill
    attack = {"knight": 50, "paladin": 48, "sorcerer": 36, "druid": 36, "monk": 46}[vocation]
    cadence = {"knight": 2.0, "paladin": 2.0, "sorcerer": 2.4, "druid": 2.4, "monk": 1.8}[vocation]
    spell = model.get("spell_" + vocation, SPELL[vocation])
    offence = ((attack * (trained + 4) / 28 + level * spell) / cadence) * (model["bis_multiplier"] if gear == "bis" else 1)
    offence *= 1 + imbuement + wheel
    hp_gain = {"knight": 15, "paladin": 10, "sorcerer": 5, "druid": 5, "monk": 10}[vocation]
    mana_gain = {"knight": 5, "paladin": 15, "sorcerer": 30, "druid": 30, "monk": 10}[vocation]
    hp = 185 + max(0, level - 8) * hp_gain
    mana_shield = (90 + max(0, level - 8) * mana_gain) * (0.35 if vocation in ("sorcerer", "druid") else 0)
    mitigation = min(model["player_mitigation_cap"], (trained / 650) + (0.06 if gear == "bis" else 0))
    healing = (4 + level * {"knight": .14, "paladin": .22, "sorcerer": .30, "druid": .34, "monk": .20}[vocation]) * 30
    ehp = (hp + mana_shield + healing) / (1 - mitigation)
    might = round(math.sqrt(offence * ehp) / 10)
    return {"skill": trained, "offence": round(offence, 2), "ehp": round(ehp, 2), "might": might}


def party_might(values: list[int], mode: str = "per_player") -> float:
    if not values:
        raise ValueError("party must have a member")
    if mode == "lowest_member":
        return float(min(values))
    if mode != "per_player":
        raise ValueError("unknown party mode")
    return round(sum(values) / math.sqrt(1 + _calibrated_model()["party_constant"] * (len(values) - 1)), 2)


def verdict(might: float, threat: float) -> str:
    ratio = threat / max(might, 1e-9)
    if ratio <= 0.25:
        return "trivial"
    if ratio <= 0.60:
        return "easy"
    if ratio <= 1.00:
        return "fair"
    if ratio <= 1.60:
        return "hard"
    return "deadly"


def encounter(player: dict[str, float | int], creature: dict[str, Any], *, pack_size: int = 1, vocation: str = "knight") -> dict[str, float | str]:
    """Compute comparison times and tier; AoE share discounts additional pack HP."""
    if pack_size < 1 or vocation not in AOE_SHARE:
        raise ValueError("invalid pack size or vocation")
    offence = float(player["offence"])
    ehp = float(player["ehp"])
    pack_hp = float(creature["ehp"]) * (1 + (pack_size - 1) * (1 - _calibrated_model()["aoe_" + vocation]))
    incoming = float(creature["dps"]) * pack_size
    return {"time_to_kill_seconds": round(pack_hp / max(offence, 1e-9), 2),
            "time_to_die_seconds": round(ehp / max(incoming, 1e-9), 2),
            "verdict": verdict(float(player["might"]), float(creature["threat"]) * pack_size)}


def _attack_index() -> dict[str, dict[str, Any]]:
    result = {}
    for line in ATTACKS.read_text().splitlines():
        attack = json.loads(line)
        result[attack["ability"]["key"]] = attack
    return result


def _graph_index(folder: str) -> dict[str, tuple[dict[str, Any], str]]:
    return {r["definition"]["identity"]["key"]: (r, str(p.relative_to(ROOT)))
            for p in sorted((ROOT / folder).glob("*.json")) for r in _records(p)
            if "definition" in r}


def _resolve_attack(key, schedule, abilities, effects, formulas):
    """Resolve only executable Damage; never turn an unknown graph edge into zero."""
    ability, ability_path = abilities.get(key, ({}, ""))
    provenance = {"ability": ability_path, "behavior": schedule.get("source", "")}
    result = []
    if not ability:
        return [{"key": key, "classification": "unresolved", "reason": "missing_ability", "source": provenance}]
    refs = ability["definition"].get("effects", [])
    for ref in refs:
        effect, effect_path = effects.get(ref["key"], ({}, ""))
        source = {**provenance, "effect": effect_path}
        base = {"key": key, "effect": ref["key"], "source": source}
        if not effect:
            result.append({**base, "classification": "unresolved", "reason": "missing_effect"})
            continue
        if effect["definition"].get("effect_family") != "Damage":
            result.append({**base, "classification": "non_damage"})
            continue
        fkey = effect["definition"].get("formula", {}).get("key")
        formula, formula_path = formulas.get(fkey, ({}, ""))
        source.update({"formula": formula_path, "formula_key": fkey})
        profile = formula.get("authoring", {}).get("profile", {})
        kind = profile.get("formula")
        bounds = {}
        if kind == "Range":
            bounds = profile
        elif kind == "CasterMagnitude":
            bounds = schedule.get("magnitude", {})
        elif kind == "MeleeAttackSkill":
            # Same pinned arithmetic as combat-rules/src/attack/formulas.rs.
            attack, skill = profile.get("attack"), profile.get("skill")
            if attack is not None and skill is not None:
                bounds = {"minimum": 0, "maximum": (attack * skill + 10 * attack + 19) // 20}
        low, high = bounds.get("minimum"), bounds.get("maximum")
        interval, chance = schedule.get("interval_ms"), schedule.get("chance_ppm")
        if low is None or high is None or low < 0 or high < low:
            result.append({**base, "classification": "unresolved", "reason": "damage_without_numbers"})
        elif not interval or chance is None:
            result.append({**base, "classification": "unresolved", "reason": "missing_schedule"})
        else:
            element = effect.get("authoring", {}).get("profile", {}).get("damage_type", "unknown")
            result.append({**base, "classification": "damage", "kind": ability.get("authoring", {}).get("profile", {}).get("details", {}).get("kind", "").lower(),
                           "element": element, "minimum": low, "maximum": high,
                           "interval_ms": interval, "chance_ppm": chance,
                           "dps": (low + high) / 2 * chance / 1_000_000 / (interval / 1000)})
    return result or [{"key": key, "classification": "non_damage", "source": provenance}]


def _creature_rows() -> list[dict[str, Any]]:
    abilities = _graph_index("content/abilities/definitions")
    effects = _graph_index("content/abilities/effects")
    formulas = _graph_index("content/abilities/formulas")
    behaviors = _graph_index("content/behaviors")
    rows = []
    for path in sorted(CREATURES.glob("creatures-*.json")):
        for record in _records(path):
            profile = record.get("authoring", {}).get("profile", {})
            details = profile.get("details", {})
            name = details.get("display_name")
            if not name:
                continue
            behavior_key = record["definition"].get("behavior", {}).get("key")
            behavior, behavior_path = behaviors.get(behavior_key, ({}, ""))
            schedules = {a["ability"]["key"]: {**a, "source": behavior_path}
                         for a in behavior.get("authoring", {}).get("profile", {}).get("attacks", [])}
            keys = sorted(set(schedules) | {r["key"] for r in profile.get("abilities", []) if ".attack-" in r["key"]})
            parsed = [a for key in keys for a in _resolve_attack(key, schedules.get(key, {}), abilities, effects, formulas)]
            missing = any(a["classification"] == "unresolved" for a in parsed)
            elemental: defaultdict[str, float] = defaultdict(float)
            max_melee = 0
            for attack in parsed:
                if attack["classification"] == "damage":
                    elemental[attack["element"]] += attack["dps"]
                    if attack["kind"] == "melee":
                        max_melee = max(max_melee, attack["maximum"])
                    attack["dps"] = round(attack["dps"], 2)
            hp, armor, defense = profile.get("health", 0), profile.get("armor", 0), details.get("defense", 0)
            dps = sum(elemental.values())
            boss = bool(details.get("system_eligibility", {}).get("reward_boss")) or (hp >= 100_000 and profile.get("experience", 0) == 0)
            rows.append({"key": record["definition"]["identity"]["key"], "name": name,
                         "hp": hp, "armor": armor, "defence": defense,
                         "resistances": profile.get("resistances", []), "speed": profile.get("speed"),
                         "max_melee": max_melee, "attacks": parsed,
                         "dps_by_element": {k: round(v, 2) for k, v in sorted(elemental.items())},
                         "dps": round(dps, 2), "boss": boss,
                         "data_quality": "missing_attacks" if missing else "complete"})
    _apply_creature_model(rows, _initial_model())
    return sorted(rows, key=lambda row: row["key"])


def _apply_creature_model(rows, model):
    for row in rows:
        mitigation = min(model["creature_mitigation_cap"], (row["armor"] + row["defence"]) / 500)
        row["ehp"] = round(row["hp"] / (1 - mitigation), 2)
        row["threat"] = (round(math.sqrt(row["dps"] * row["ehp"]) / 10)
                         if row["data_quality"] == "complete" and row["dps"] else None)


def _initial_model():
    return {**{"spell_" + v: SPELL[v] for v in VOCATIONS},
            **{"aoe_" + v: AOE_SHARE[v] for v in VOCATIONS},
            "player_mitigation_cap": .48, "creature_mitigation_cap": .65,
            "pack_pressure": .18, "party_constant": .12, "bis_multiplier": 1.22}


def _spawn_weights(declarations, members, points):
    """Nearest compatible anchor on the same floor; key breaks distance ties."""
    candidates = defaultdict(list)
    for area in declarations:
        pos = area.get("position")
        if pos and pos.get("coordinate_frame") == "global-target-2026-09-27":
            key = area["identity"]["key"]
            for creature in members[key]:
                candidates[(creature, pos["floor"])].append((key, pos))
    counts = {a["identity"]["key"]: Counter() for a in declarations}
    for point in points:
        pos = point["cell"]
        options = [(key, anchor) for key, anchor in candidates.get((point["creature"], pos["floor"]), [])
                   if (anchor["x"] - pos["x"]) ** 2 + (anchor["y"] - pos["y"]) ** 2 <= 128 ** 2]
        if options:
            key, _ = min(options, key=lambda item: ((item[1]["x"] - pos["x"]) ** 2 + (item[1]["y"] - pos["y"]) ** 2, item[0]))
            counts[key][point["creature"]] += 1
    return {key: (dict(count) if count else {m: 1 for m in members[key]}, "spawns" if count else "uniform")
            for key, count in counts.items()}


def _area_inputs(creatures):
    declarations = sorted((r["declaration"] for r in _records(AREAS)), key=lambda a: a["identity"]["key"])
    by_name = {c["name"].casefold(): c["key"] for c in creatures}
    members = {a["identity"]["key"]: [by_name[n.casefold()] for n in a["source_facts"].get("creature_names", []) if n.casefold() in by_name] for a in declarations}
    index = json.loads((ROOT / "content/world/spawns/index.json").read_text())
    points = (point for shard in index["shards"] for r in _records(ROOT / shard) for point in r["declaration"]["points"])
    weights = _spawn_weights(declarations, members, points)
    return [{"key": a["identity"]["key"], "name": a["name"],
             "creatures": members[a["identity"]["key"]],
             "spawn_weights": weights[a["identity"]["key"]][0],
             "spawn_weight_source": weights[a["identity"]["key"]][1],
             "split": "holdout" if (i + 1) % 5 == 0 else "train",
             "source_recommended_levels": a.get("recommended_levels", {})} for i, a in enumerate(declarations)]


@cache
def _might_table(vocation, gear, parameters):
    model = dict(parameters)
    return [int(player_might(vocation, level, gear, model=model)["might"]) for level in range(8, 2001)]


def _green_level(threat, vocation, gear, model):
    # Censor above 2000 as 2001 in metrics, never silently discard difficult places.
    values = _might_table(vocation, gear, tuple((key, model[key]) for key in ("spell_" + vocation, "player_mitigation_cap", "bis_multiplier")))
    level = bisect_left(values, threat / .60) + 8
    return level if level <= 2000 else None


def _area_predictions(areas, creatures, model, *, all_gears=False):
    by_key = {c["key"]: c for c in creatures}
    result = []
    for area in areas:
        usable = [(by_key[key], weight) for key, weight in area["spawn_weights"].items()
                  if by_key[key]["threat"] is not None and not by_key[key]["boss"]]
        count = sum(w for _, w in usable)
        pressure = 1 + model["pack_pressure"] * math.log2(max(1, count))
        threat = sum(c["threat"] * w for c, w in usable) / count * pressure if count else None
        encounter_threat = {v: (threat * math.sqrt(1 + (min(count, 8) - 1) * (1 - model["aoe_" + v])) if threat is not None else None) for v in VOCATIONS}
        green = {v: {g: (_green_level(encounter_threat[v], v, g, model) if threat is not None else None)
                     for g in (GEAR_MULTIPLIER if all_gears else ("normal",))} for v in VOCATIONS}
        result.append({**area, "spawn_count": count, "pack_size": min(count, 8), "pack_pressure": round(pressure, 4),
                       "threat_by_vocation": {v: round(t, 2) if t is not None else None for v, t in encounter_threat.items()},
                       "threat": round(threat, 2) if threat is not None else None, "green_level": green})
    return result


def _metrics(areas):
    errors = {split: {v: [] for v in VOCATIONS} for split in ("train", "holdout", "overall")}
    for area in areas:
        if area["threat"] is None:
            continue
        for v in VOCATIONS:
            expected = area["source_recommended_levels"].get("mage" if v in ("sorcerer", "druid") else v)
            if expected is not None:
                predicted = area["green_level"][v]["normal"]
                error = abs((2001 if predicted is None else predicted) - expected)
                errors[area["split"]][v].append(error)
                errors["overall"][v].append(error)
    def summary(values):
        return {"observations": len(values), "mean_absolute_error_levels": round(sum(values) / len(values), 2) if values else None,
                "median_absolute_error_levels": median(values) if values else None}
    return {split: {**summary([e for values in vocations.values() for e in values]),
                    "per_vocation": {v: summary(values) for v, values in vocations.items()}}
            for split, vocations in errors.items()}


def _fit(areas, creatures):
    # Train only. Identical initial point, grids, order and four sweeps on every run.
    model = _initial_model()
    training = [a for a in areas if a["split"] == "train"]
    grids = {**{"spell_" + v: (.1, .25, .5, 1., 2., 4., 8., 16., 32., 64., 128.) for v in VOCATIONS},
             "player_mitigation_cap": (.1, .25, .48, .65, .8),
             "creature_mitigation_cap": (.1, .25, .4, .65, .8),
             "pack_pressure": (0., .05, .1, .18, .3, .5),
             **{"aoe_" + v: (0., .25, .5, .65, .72, .88, .9, .95) for v in VOCATIONS}}
    def loss(candidate):
        _apply_creature_model(creatures, candidate)
        return _metrics(_area_predictions(training, creatures, candidate))["train"]["mean_absolute_error_levels"]
    best = loss(model)
    for _ in range(4):
        for key, values in grids.items():
            chosen = model[key]
            for value in values:
                candidate = {**model, key: value}
                error = loss(candidate)
                if error < best:
                    best, chosen = error, value
            model[key] = chosen
    return model


@cache
def _calibrated_model():
    creatures = _creature_rows()
    return _fit(_area_inputs(creatures), creatures)


def build() -> dict[str, Any]:
    creatures = _creature_rows()
    model = _calibrated_model()
    _apply_creature_model(creatures, model)
    areas = _area_predictions(_area_inputs(creatures), creatures, model, all_gears=True)
    for row in creatures:
        row["green_level"] = ({v: {g: _green_level(row["threat"], v, g, model) for g in GEAR_MULTIPLIER} for v in VOCATIONS}
                              if row["threat"] is not None and not row["boss"] else None)
    metrics = _metrics(areas)
    reasons = Counter(a["reason"] for c in creatures for a in c["attacks"] if a["classification"] == "unresolved")
    imports = list(_attack_index().values())
    before_reasons = Counter(reason for a in imports if a.get("status") != "authored" for reason in a["reasons"])
    classifications = defaultdict(set)
    for creature in creatures:
        for attack in creature["attacks"]:
            classifications[attack["key"]].add(attack["classification"])
    recovery = {}
    for reason in sorted(before_reasons):
        counts = Counter()
        for attack in imports:
            if attack.get("status") == "authored" or reason not in attack.get("reasons", []):
                continue
            kinds = classifications[attack["ability"]["key"]]
            status = ("unreferenced" if not kinds else "unresolved" if "unresolved" in kinds
                      else "damage" if "damage" in kinds else "non_damage")
            counts[status] += 1
        recovery[reason] = dict(sorted(counts.items()))
    return {"schema_version": 2, "formula": "round(sqrt(offence_or_dps * ehp) / 10)",
            "constants": {"fight_seconds": 30, "model": model,
                          "attack_constants": "content/combat/attack_constants_v1.json",
                          "verdict_threat_to_might_boundaries": [0.25, 0.60, 1.00, 1.60]},
            "creatures": creatures, "areas": areas,
            "calibration": {"source": "content/world/areas/hunting-places (pinned TibiaWiki reference)",
                            "excluded": ["boss", "missing_attacks", "areas_without_usable_creatures"],
                            "method": "four coordinate sweeps; fixed grids; training MAE; every fifth sorted area held out",
                            "unidentifiable_constants": ["bis_multiplier", "party_constant"],
                            "censored_level": 2001,
                            "observations": metrics["overall"]["observations"],
                            "mean_absolute_error_levels": metrics["overall"]["mean_absolute_error_levels"],
                            "metrics": metrics,
                            "missing_attacks_before": 547,
                            "missing_attacks_after": sum(c["data_quality"] == "missing_attacks" for c in creatures),
                            "unavailable_import_attacks_by_reason": dict(sorted(before_reasons.items())),
                            "unresolved_attacks_by_reason": dict(sorted(reasons.items())),
                            "import_attack_recovery_by_reason": recovery,
                            "attack_classifications": dict(sorted(Counter(a["classification"] for c in creatures for a in c["attacks"]).items())),
                            "areas_with_spawn_weights": sum(a["spawn_weight_source"] == "spawns" for a in areas)}}


def serialize(data):
    """Sorted top level and compact individual records, with a final newline."""
    lines = ["{"]
    for i, key in enumerate(sorted(data)):
        value = data[key]
        prefix = "  " + json.dumps(key) + ": "
        suffix = "," if i < len(data) - 1 else ""
        if key in ("creatures", "areas"):
            lines.append(prefix + "[")
            lines.extend("    " + json.dumps(row, sort_keys=True, separators=(",", ":")) + ("," if j < len(value) - 1 else "") for j, row in enumerate(value))
            lines.append("  ]" + suffix)
        else:
            block = json.dumps(value, indent=2, sort_keys=True).splitlines()
            lines.append(prefix + block[0])
            lines.extend("  " + line for line in block[1:])
            lines[-1] += suffix
    return "\n".join([*lines, "}", ""])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    data = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(serialize(data))


if __name__ == "__main__":
    main()
