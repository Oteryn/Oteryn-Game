#!/usr/bin/env python3
"""No-network tests for condition_authoring.py. Run with `python test_condition_authoring.py`."""

from __future__ import annotations

import copy
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

import condition_authoring as ca


def committed() -> dict:
    return json.loads(ca.CATALOGUE.read_text(encoding="utf-8"))


def by_key(catalogue: dict, suffix: str) -> dict:
    key = f"oteryn:condition.{suffix}"
    return next(c for c in catalogue["conditions"] if c["identity"]["key"] == key)


def expect_invalid(catalogue: dict, fragment: str) -> None:
    errors = ca.validate(catalogue)
    assert any(fragment in e for e in errors), f"{fragment!r} not in {errors}"


def test_field_schedule_follows_canary_order() -> None:
    fixed = ca.field_schedule([["ticks", 10000], ["count", 7], ["damage", 20]])
    assert fixed == {
        "tick_profile": "Fixed",
        "first_tick": "Immediate",
        "ticks": [{"count": 7, "interval_ms": 10000, "amount": 20}],
    }
    decreasing = ca.field_schedule([["ticks", 5000], ["start", 5], ["damage", 100]])
    assert decreasing["tick_profile"] == "Decreasing"
    assert decreasing["initial_tick_amount"] == 5 and decreasing["total_maximum"] == 100
    # A `start` after `damage` is ignored, as ItemParse::parseFieldCombatDamage reads in order.
    late = ca.field_schedule([["ticks", 5000], ["damage", 100], ["start", 5]])
    assert late["ticks"] == [{"count": 1, "interval_ms": 5000, "amount": 100}]
    assert "no damage" in ca.field_schedule([["damage", 0]])
    assert "0 damage entries" in ca.field_schedule([])
    assert "unknown field attribute" in ca.field_schedule([["radius", 1]])


def test_committed_build_is_current_and_valid() -> None:
    catalogue = committed()
    assert ca.dumps(ca.build()) == ca.CATALOGUE.read_text(encoding="utf-8")
    assert ca.validate(catalogue) == []


def test_authored_values() -> None:
    catalogue = committed()
    haste = by_key(catalogue, "spell.haste")["speed"]
    assert (haste["a_min"], haste["b_min"], haste["duration_ms"]) == (1300, 40, 30000)
    cripple = by_key(catalogue, "charm.cripple")
    assert cripple["negative"] and cripple["speed"]["kind"] == "paralysis"
    shield = by_key(catalogue, "spell.magic_shield")["mana_shield"]["capacity_milli"]
    assert shield == {"constant": 300000, "per_level": 7600, "per_magic_level": 7000}
    assert (
        by_key(catalogue, "food.regeneration")["food_regeneration"]["max_duration_ms"]
        == 1_200_000
    )
    holy = by_key(catalogue, "spell.holy_flash")
    assert holy["status"] == "blocked" and "damage_over_time" not in holy


def test_monster_conditions_mirror_content() -> None:
    effects, _ = ca.content_effects()
    catalogue = committed()
    keys = {c["identity"]["key"] for c in catalogue["conditions"]}
    for _rel, effect_key, op in effects:
        ctype = op["condition"]["condition_type"]
        key = "oteryn:condition.monster." + effect_key.removeprefix("oteryn:effect.")
        assert (key in keys) == (ctype in ca.ELEMENTS or ctype in ca.SPEED_KINDS), key
        if ctype in ca.ELEMENTS:
            dot = by_key(catalogue, key.removeprefix("oteryn:condition."))[
                "damage_over_time"
            ]
            assert dot["schedule"] == op["condition"]["damage_over_time"]


def test_blocked_speed_coefficients() -> None:
    blocked = by_key(committed(), "monster.creature.bazir.defense-3")
    assert blocked["status"] == "blocked" and "thousandths" in blocked["blocked_reason"]
    assert ca.thousandths({"numerator": 3, "denominator": 20}) == 150
    assert ca.thousandths({"numerator": 2901, "denominator": 2000}) is None


def test_content_files_exclude_blocked() -> None:
    catalogue = committed()
    files = ca.content_files(catalogue)
    index = json.loads(files[ca.INDEX_PATH])
    admitted = [c for c in catalogue["conditions"] if c["status"] == "admitted"]
    assert index["record_count"] == len(admitted)
    shard_keys = [
        r["definition"]["identity"]["key"]
        for path in index["shards"]
        for r in json.loads(files[path])["records"]
    ]
    assert shard_keys == [c["identity"]["key"] for c in admitted]
    assert "oteryn:condition.spell.holy_flash" not in shard_keys


def test_rules() -> None:
    base = committed()

    def mutated(suffix: str, change) -> dict:
        catalogue = copy.deepcopy(base)
        change(by_key(catalogue, suffix))
        return catalogue

    duplicate = copy.deepcopy(base)
    duplicate["conditions"].append(copy.deepcopy(duplicate["conditions"][0]))
    expect_invalid(duplicate, "duplicate key")
    expect_invalid(
        mutated("spell.haste", lambda c: c.update(conflict_key="poison")),
        "does not belong to SPEED",
    )
    expect_invalid(
        mutated("spell.haste", lambda c: c.update(negative=True)), "negative must mark"
    )
    expect_invalid(
        mutated(
            "spell.haste", lambda c: c.update(status="blocked", blocked_reason="x")
        ),
        "exactly its family block",
    )
    expect_invalid(
        mutated("spell.haste", lambda c: c["speed"].update(a_min=2000)),
        "a_min exceeds a_max",
    )
    expect_invalid(
        mutated(
            "spell.envenom",
            lambda c: c["damage_over_time"]["schedule"]["ticks"][0].update(
                interval_ms=500
            ),
        ),
        "below COND0-RL-02",
    )
    expect_invalid(
        mutated("spell.envenom", lambda c: c["damage_over_time"].update(field=True)),
        "only a field",
    )
    expect_invalid(
        mutated(
            "field.i2121",
            lambda c: c["damage_over_time"]["schedule"].update(
                first_tick="AfterInterval"
            ),
        ),
        "only a field",
    )
    expect_invalid(
        mutated(
            "spell.recovery", lambda c: c["recovery"].update(health_interval_ms=500)
        ),
        "regeneration interval",
    )
    expect_invalid(
        mutated(
            "food.regeneration",
            lambda c: c["food_regeneration"].update(max_duration_ms=1),
        ),
        "capped at",
    )
    expect_invalid(
        mutated("spell.light", lambda c: c["light"].update(level=0)), "light/level"
    )
    expect_invalid(
        mutated("spell.holy_flash", lambda c: c.pop("blocked_reason")), "blocked_reason"
    )


def test_schedule_errors_mirror_project_v2() -> None:
    geometric = {
        "tick_profile": "Geometric",
        "first_tick": "AfterInterval",
        "base_minimum": 5,
        "base_maximum": 4,
        "factor": {"numerator": 2, "denominator": 4},
        "tick_counts": [3, 3],
        "tick_interval_ms": 2000,
    }
    assert "base_minimum exceeds" in ca.schedule_errors(geometric)[0]
    geometric["base_maximum"] = 6
    assert "strictly increase" in ca.schedule_errors(geometric)[0]
    geometric["tick_counts"] = [3, 4]
    assert "lowest terms" in ca.schedule_errors(geometric)[0]
    geometric["factor"] = {"numerator": 1, "denominator": 2}
    assert ca.schedule_errors(geometric) == []
    decreasing = {
        "tick_profile": "Decreasing",
        "first_tick": "AfterInterval",
        "total_minimum": 0,
        "total_maximum": 0,
        "tick_interval_ms": 2000,
    }
    assert ca.schedule_errors(decreasing)


def test_capture_rejects_unpinned_checkout() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        git = ["git", "-C", tmp, "-c", "user.name=t", "-c", "user.email=t@t"]
        subprocess.run([*git, "init", "-q"], check=True)
        subprocess.run([*git, "commit", "-q", "--allow-empty", "-m", "x"], check=True)
        try:
            ca.capture(Path(tmp))
        except ValueError as error:
            assert "not the pinned" in str(error)
        else:
            raise AssertionError("capture accepted an unpinned checkout")


def test_content_removes_obsolete_shards() -> None:
    root = ca.ROOT
    with tempfile.TemporaryDirectory() as tmp:
        for name in ("project", "manifest", "content.lock"):
            (Path(tmp) / "content").mkdir(exist_ok=True)
            shutil.copy(
                root / f"content/{name}.json", Path(tmp) / f"content/{name}.json"
            )
        ca.ROOT = Path(tmp)
        try:
            assert ca.content_command(check=False) == 0
            obsolete = Path(tmp) / ca.CONTENT_DIR / "conditions-99999-99999.json"
            obsolete.write_text("{}\n", encoding="utf-8")
            assert ca.content_command(check=True) == 1
            assert ca.content_command(check=False) == 0
            assert not obsolete.exists() and ca.content_command(check=True) == 0
        finally:
            ca.ROOT = root


def main() -> None:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for test in tests:
        test()
        print(f"ok {test.__name__}")


if __name__ == "__main__":
    main()
