#!/usr/bin/env python3
"""Build the offline player-Might and creature/area-Threat guide dataset."""
from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from functools import cache
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CREATURES = ROOT / "content/creatures/definitions"
ATTACKS = ROOT / "content/abilities/source-imports/monster-attack-authoring.jsonl"
AREAS = ROOT / "content/world/areas/hunting-places/hunting-places-00000-00444.json"
OUTPUT = ROOT / "content/creatures/power-score.json"
VOCATIONS = ("knight", "paladin", "sorcerer", "druid")
GEAR_MULTIPLIER = {"normal": 1.0, "bis": 1.22}
AOE_SHARE = {"knight": 0.65, "paladin": 0.72, "sorcerer": 0.90, "druid": 0.88}


def _records(path: Path) -> list[dict[str, Any]]:
    return json.loads(path.read_text())["records"]


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
) -> dict[str, float | int]:
    """Return the documented 30-second representative solo player model."""
    if vocation not in (*VOCATIONS, "monk") or gear not in GEAR_MULTIPLIER or level < 1:
        raise ValueError("unsupported vocation, gear tier, or level")
    trained = typical_skill(vocation, level) if skill is None else skill
    attack = {"knight": 50, "paladin": 48, "sorcerer": 36, "druid": 36, "monk": 46}[vocation]
    cadence = {"knight": 2.0, "paladin": 2.0, "sorcerer": 2.4, "druid": 2.4, "monk": 1.8}[vocation]
    spell = {"knight": 0.34, "paladin": 0.48, "sorcerer": 0.82, "druid": 0.76, "monk": 0.44}[vocation]
    offence = ((attack * (trained + 4) / 28 + level * spell) / cadence) * GEAR_MULTIPLIER[gear]
    offence *= 1 + imbuement + wheel
    hp_gain = {"knight": 15, "paladin": 10, "sorcerer": 5, "druid": 5, "monk": 10}[vocation]
    mana_gain = {"knight": 5, "paladin": 15, "sorcerer": 30, "druid": 30, "monk": 10}[vocation]
    hp = 185 + max(0, level - 8) * hp_gain
    mana_shield = (90 + max(0, level - 8) * mana_gain) * (0.35 if vocation in ("sorcerer", "druid") else 0)
    mitigation = min(0.48, (trained / 650) + (0.06 if gear == "bis" else 0))
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
    return round(sum(values) / math.sqrt(1 + 0.12 * (len(values) - 1)), 2)


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
    pack_hp = float(creature["ehp"]) * (1 + (pack_size - 1) * (1 - AOE_SHARE[vocation]))
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


def _creature_rows() -> list[dict[str, Any]]:
    attacks = _attack_index()
    rows = []
    for path in sorted(CREATURES.glob("creatures-*.json")):
        for record in _records(path):
            profile = record.get("authoring", {}).get("profile", {})
            details = profile.get("details", {})
            name = details.get("display_name")
            if not name:
                continue
            missing = False
            parsed = []
            elemental: defaultdict[str, float] = defaultdict(float)
            max_melee = 0
            for ref in profile.get("abilities", []):
                key = ref["key"]
                if ".attack-" not in key:
                    continue
                source = attacks.get(key)
                if not source or source.get("status") != "authored":
                    missing = True
                    parsed.append({"key": key, "data_quality": "missing"})
                    continue
                schedule = source.get("schedules", [{}])[0]
                interval = schedule.get("interval_ms")
                chance = schedule.get("chance_ppm")
                effects = [e for e in source.get("effects", []) if e.get("operation") == "damage"]
                if not interval or chance is None or not effects:
                    missing = True
                    parsed.append({"key": key, "data_quality": "missing"})
                    continue
                for effect in effects:
                    low, high = effect.get("minimum"), effect.get("maximum")
                    if low is None or high is None:
                        missing = True
                        continue
                    element = effect.get("damage_type", "unknown")
                    dps = ((low + high) / 2) * chance / 1_000_000 / (interval / 1000)
                    elemental[element] += dps
                    if source.get("kind") == "melee":
                        max_melee = max(max_melee, high)
                    parsed.append({"key": key, "kind": source.get("kind"), "element": element,
                                   "minimum": low, "maximum": high, "interval_ms": interval,
                                   "chance_ppm": chance, "dps": round(dps, 2)})
            hp = profile.get("health", 0)
            armor = profile.get("armor", 0)
            defense = details.get("defense", 0)
            mitigation = min(0.65, (armor + defense) / 500)
            ehp = hp / (1 - mitigation) if hp else 0
            dps = sum(elemental.values())
            boss = bool(details.get("system_eligibility", {}).get("reward_boss")) or (hp >= 100_000 and profile.get("experience", 0) == 0)
            rows.append({"key": record["definition"]["identity"]["key"], "name": name,
                         "hp": hp, "ehp": round(ehp, 2), "armor": armor, "defence": defense,
                         "resistances": profile.get("resistances", []), "speed": profile.get("speed"),
                         "max_melee": max_melee, "attacks": parsed,
                         "dps_by_element": {k: round(v, 2) for k, v in sorted(elemental.items())},
                         "dps": round(dps, 2), "threat": round(math.sqrt(dps * ehp) / 10) if dps else None,
                         "boss": boss, "data_quality": "missing_attacks" if missing else "complete"})
    return rows


@cache
def _might_at(vocation: str, level: int, gear: str) -> int:
    return int(player_might(vocation, level, gear)["might"])


def _green_level(threat: float, vocation: str, gear: str) -> int | None:
    for level in range(8, 2001):
        if threat / max(_might_at(vocation, level, gear), 1e-9) <= 0.60:
            return level
    return None


def build() -> dict[str, Any]:
    creatures = _creature_rows()
    by_name = {row["name"].casefold(): row for row in creatures}
    areas = []
    calibration_errors = []
    for record in _records(AREAS):
        declaration = record["declaration"]
        members = [by_name[n.casefold()] for n in declaration["source_facts"].get("creature_names", []) if n.casefold() in by_name]
        usable = [m for m in members if m["threat"] is not None and not m["boss"] and m["data_quality"] == "complete"]
        base = sum(m["threat"] for m in usable)
        pack_pressure = 1 + 0.18 * max(0, len(usable) - 1)
        threat = round(base * pack_pressure)
        green = {v: {g: _green_level(threat, v, g) for g in GEAR_MULTIPLIER} for v in VOCATIONS}
        areas.append({"key": declaration["identity"]["key"], "name": declaration["name"],
                      "creatures": [m["name"] for m in members], "pack_size": len(members),
                      "pack_pressure": round(pack_pressure, 2), "threat": threat,
                      "green_level": green, "source_recommended_levels": declaration.get("recommended_levels", {})})
        source = declaration.get("recommended_levels", {})
        for vocation in ("knight", "paladin"):
            if vocation in source and green[vocation]["normal"] is not None:
                calibration_errors.append(abs(green[vocation]["normal"] - source[vocation]))
        for vocation in ("sorcerer", "druid"):
            if "mage" in source and green[vocation]["normal"] is not None:
                calibration_errors.append(abs(green[vocation]["normal"] - source["mage"]))
    for row in creatures:
        row["green_level"] = ({v: {g: _green_level(row["threat"], v, g) for g in GEAR_MULTIPLIER} for v in VOCATIONS}
                              if row["threat"] is not None and not row["boss"] and row["data_quality"] == "complete" else None)
    return {"schema_version": 1, "formula": "round(sqrt(offence_or_dps * ehp) / 10)",
            "constants": {"fight_seconds": 30, "attack_constants": "content/combat/attack_constants_v1.json",
                          "gear_multiplier": GEAR_MULTIPLIER, "aoe_share": AOE_SHARE,
                          "party_diminishing_factor": "sum(might)/sqrt(1+0.12*(members-1))",
                          "verdict_threat_to_might_boundaries": [0.25, 0.60, 1.00, 1.60]},
            "creatures": creatures, "areas": areas,
            "calibration": {"source": "content/world/areas/hunting-places (pinned TibiaWiki reference)",
                            "excluded": ["boss", "missing_attacks"], "observations": len(calibration_errors),
                            "mean_absolute_error_levels": round(sum(calibration_errors) / len(calibration_errors), 2) if calibration_errors else None}}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    data = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
