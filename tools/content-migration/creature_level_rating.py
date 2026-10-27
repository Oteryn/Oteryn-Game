#!/usr/bin/env python3
"""Build the deterministic, advisory Oteryn creature level-rating report."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REPORT = ROOT / "content/creatures/level-rating.json"
LABEL = "Oteryn rating (not an official Tibia formula)"
PLAYER_DAMAGE_TYPES = ("physical", "fire", "ice", "earth", "energy", "death", "holy")
CONSTANTS = {
    "pack_size": 3,
    "party_size": 4,
    "area_weight_milli": 1500,
    "incoming_exposure_milli": 350,
    "base_hp": {"druid": 150, "sorcerer": 150, "paladin": 155, "knight": 185},
    "hp_per_level": {"druid": 5, "sorcerer": 5, "paladin": 10, "knight": 15},
    "player_dps_base": 20,
    "player_dps_per_level_milli": 800,
    "reasonable_kill_seconds": 35,
    "trivial_kill_seconds": 8,
    "exp_fit_a_milli": 1100,
    "exp_fit_b_milli": 550,
    # Fit to the three published MRL calibration areas.  Integer milli-units
    # keep report generation identical on every supported Python platform.
    "area_cap_intercept_milli": -327413,
    "area_cap_sqrt_exp_milli": 13797,
    "area_cap_burst_milli": -563,
}


def _load(path: Path) -> object:
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def _ratio(value: object) -> float:
    if not isinstance(value, dict):
        return 0.0
    denominator = value.get("denominator", 1)
    return float(value.get("numerator", 0)) / float(denominator or 1)


def _is_area(attack: dict[str, object]) -> bool:
    shape = attack.get("area") or attack.get("shape") or attack.get("target_shape")
    return bool(shape and str(shape).lower() not in {"single", "target", "none"})


def _damage(attack: dict[str, object]) -> tuple[int, int]:
    magnitude = attack.get("magnitude", attack)
    if not isinstance(magnitude, dict):
        return 0, 0
    low = magnitude.get("minimum", magnitude.get("min", 0))
    high = magnitude.get("maximum", magnitude.get("max", low))
    return abs(int(low or 0)), abs(int(high or 0))


def attack_metrics(attacks: object) -> tuple[int, int]:
    """Return weighted burst and DPS milli-units; missing attacks are harmless."""
    burst = 0
    dps_milli = 0
    for attack in attacks if isinstance(attacks, list) else []:
        if not isinstance(attack, dict):
            continue
        low, high = _damage(attack)
        weight = CONSTANTS["area_weight_milli"] if _is_area(attack) else 1000
        burst = max(burst, (high * weight + 999) // 1000)
        chance = int(attack.get("chance_ppm", 1_000_000) or 0)
        interval = max(1, int(attack.get("interval_ms", 2000) or 2000))
        dps_milli += ((low + high) * chance * weight) // (2 * interval * 1000)
        condition = attack.get("condition")
        if isinstance(condition, dict):
            tick, _ = _damage(condition)
            tick_ms = max(1, int(condition.get("interval_ms", 1000) or 1000))
            dps_milli += tick * 1_000_000 // tick_ms
    return burst, dps_milli


def _level_for_hp(required_hp: int, vocation: str) -> int:
    base = CONSTANTS["base_hp"][vocation]
    growth = CONSTANTS["hp_per_level"][vocation]
    return max(1, 8 + max(0, required_hp - base + growth - 1) // growth)


def survival_levels(burst: int, pack_size: int = 3, split: int = 1) -> dict[str, int]:
    exposed = (burst * pack_size * CONSTANTS["incoming_exposure_milli"] + 999) // 1000
    required = (2 * exposed + split - 1) // split
    return {
        vocation: _level_for_hp(required, vocation)
        for vocation in sorted(CONSTANTS["base_hp"])
    }


def _percentile75(values: list[int]) -> int:
    if not values:
        return 1
    values = sorted(values)
    return values[(3 * (len(values) - 1) + 3) // 4]


def _profiles() -> tuple[
    dict[str, dict[str, object]], dict[str, list[dict[str, object]]]
]:
    profiles: dict[str, dict[str, object]] = {}
    for path in sorted(
        (ROOT / "content/creatures/definitions").glob("creatures-*.json")
    ):
        for record in _load(path)["records"]:  # type: ignore[index]
            profile = record["authoring"]["profile"]
            key = record["definition"]["identity"]["key"]
            profiles[key] = profile
    attacks: dict[str, list[dict[str, object]]] = {}
    native = _load(ROOT / "content/creatures/definitions/spell-native-profiles.json")
    for record in native["records"]:  # type: ignore[index]
        key = record["profile"]["target"]["key"].replace(
            "canary:creature/", "oteryn:creature."
        )
        rows = (
            record.get("behavior", {})
            .get("data", {})
            .get("profile", {})
            .get("attacks", [])
        )
        rows = list(rows) if isinstance(rows, list) else []
        melee = record.get("monster_melee")
        if isinstance(melee, dict):
            rows.append(melee)
        attacks[key] = rows
    return profiles, attacks


def rate_creature(
    key: str, profile: dict[str, object], attacks: object
) -> dict[str, object]:
    burst, dps_milli = attack_metrics(attacks)
    resistances = {
        row["damage_type"]: _ratio(row.get("percent"))
        for row in profile.get("resistances", [])
        if isinstance(row, dict)
    }
    mean_resistance = sum(
        resistances.get(kind, 0.0) for kind in PLAYER_DAMAGE_TYPES
    ) / len(PLAYER_DAMAGE_TYPES)
    armor = int(profile.get("armor", 0) or 0)
    mitigation = _ratio(profile.get("mitigation"))
    health = int(profile.get("health", 0) or 0)
    ehp = round(
        health
        * (1 + armor / 1000 + mitigation / 100)
        / max(0.1, 1 - mean_resistance / 100)
    )
    survival = survival_levels(burst)
    target_dps = max(
        1,
        (ehp + CONSTANTS["reasonable_kill_seconds"] - 1)
        // CONSTANTS["reasonable_kill_seconds"],
    )
    pace = max(
        1,
        1
        + max(0, target_dps - CONSTANTS["player_dps_base"])
        * 1000
        // CONSTANTS["player_dps_per_level_milli"],
    )
    experience = int(profile.get("experience", 0) or 0)
    exp_level = max(
        1,
        round(
            CONSTANTS["exp_fit_a_milli"]
            / 1000
            * experience ** (CONSTANTS["exp_fit_b_milli"] / 1000)
        ),
    )
    suggested = max(max(survival.values()), pace)
    details = profile.get("details", {})
    return {
        "key": key,
        "name": details.get("display_name", key) if isinstance(details, dict) else key,
        "burst": burst,
        "dps_milli": dps_milli,
        "ehp": ehp,
        "survival_level": survival,
        "experience": experience,
        "pace_level": pace,
        "exp_level": exp_level,
        "suggested_level": suggested,
        "review": abs(suggested - exp_level) * 10 > max(1, exp_level) * 3,
    }


def _area(
    name: str,
    area_id: str,
    creature_keys: list[str],
    ratings: dict[str, dict[str, object]],
    kind: str,
) -> dict[str, object]:
    rows = [ratings[key] for key in sorted(set(creature_keys)) if key in ratings]
    solo = {
        v: _percentile75([r["survival_level"][v] for r in rows])
        for v in sorted(CONSTANTS["base_hp"])
    }
    party_rows = {
        key: survival_levels(int(ratings[key]["burst"]), split=CONSTANTS["party_size"])
        for key in creature_keys
        if key in ratings
    }
    party = {
        v: _percentile75([levels[v] for levels in party_rows.values()])
        for v in sorted(CONSTANTS["base_hp"])
    }
    weakest = max(solo.values())
    mean_exp = (
        round(sum(int(row["experience"]) for row in rows) / len(rows)) if rows else 0
    )
    p75_burst = _percentile75([int(row["burst"]) for row in rows])
    calibrated_cap = round(
        (
            CONSTANTS["area_cap_intercept_milli"]
            + CONSTANTS["area_cap_sqrt_exp_milli"] * math.isqrt(mean_exp)
            + CONSTANTS["area_cap_burst_milli"] * p75_burst
        )
        / 1000
    )
    return {
        "id": area_id,
        "name": name,
        "kind": kind,
        "creatures": sorted(set(creature_keys)),
        "min_level": {"solo": solo, "party": party},
        "recommended_range": {"min": weakest, "max": max(weakest, calibrated_cap)},
        "trivial_thresholds": {
            "burst_hp_percent": 10,
            "kill_time_seconds": CONSTANTS["trivial_kill_seconds"],
        },
        "experience_per_creature_mean": mean_exp,
    }


def build_report() -> dict[str, object]:
    profiles, attacks = _profiles()
    ratings = {
        key: rate_creature(key, profile, attacks.get(key, []))
        for key, profile in sorted(profiles.items())
    }
    by_name = {str(row["name"]).casefold(): key for key, row in ratings.items()}
    areas = []
    hunting = _load(
        ROOT / "content/world/areas/hunting-places/hunting-places-00000-00444.json"
    )
    for record in hunting["records"]:  # type: ignore[index]
        declaration = record["declaration"]
        names = declaration.get("source_facts", {}).get("creature_names", [])
        keys = [
            by_name[name.casefold()] for name in names if name.casefold() in by_name
        ]
        areas.append(
            _area(
                declaration["name"],
                declaration["identity"]["key"],
                keys,
                ratings,
                "named_area",
            )
        )
    regions: dict[str, list[str]] = {}
    for path in sorted((ROOT / "content/world/spawns").glob("spawns-*.json")):
        for record in _load(path)["records"]:  # type: ignore[index]
            declaration = record["declaration"]
            centre = declaration["centre"]
            region = f"z{centre['floor']:02d}-x{centre['x'] // 256:03d}-y{centre['y'] // 256:03d}"
            regions.setdefault(region, []).extend(
                point["creature"] for point in declaration.get("points", [])
            )
    areas.extend(
        _area(region, f"spawn-region:{region}", keys, ratings, "spawn_region")
        for region, keys in sorted(regions.items())
    )
    calibration_names = {
        "Bulltaur Lair": 300,
        "Nimmersatt's Breeding Ground": 454,
        "Rotten Wasteland": 790,
    }
    calibration = [
        {
            "area": name,
            "observed_tibia": observed,
            "oteryn": next(
                area["recommended_range"]["max"]
                for area in areas
                if area["name"] == name
            ),
        }
        for name, observed in calibration_names.items()
    ]
    return {
        "schema": "OTERYN_CREATURE_LEVEL_RATING/v1",
        "label": LABEL,
        "constants": CONSTANTS,
        "calibration": calibration,
        "creatures": list(ratings.values()),
        "areas": sorted(areas, key=lambda row: row["id"]),
    }


def render() -> str:
    return (
        json.dumps(build_report(), indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check", action="store_true", help="fail if the committed report is stale"
    )
    args = parser.parse_args()
    expected = render()
    if args.check:
        if not REPORT.exists() or REPORT.read_text(encoding="utf-8") != expected:
            print(f"stale: {REPORT.relative_to(ROOT)}")
            return 1
        print(f"current: {REPORT.relative_to(ROOT)}")
        return 0
    REPORT.write_text(expected, encoding="utf-8")
    print(f"wrote {REPORT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
