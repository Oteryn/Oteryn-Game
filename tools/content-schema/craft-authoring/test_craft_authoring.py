"""Reference parity and reproducibility; Rust integration tests own compiler admission."""
import copy
from fractions import Fraction

import jsonschema
import pytest

import craft_authoring as craft


def test_regeneration_is_deterministic_and_committed_content_is_current():
    first = craft.outputs()
    assert craft.canonical_bytes(first) == craft.canonical_bytes(craft.outputs())
    assert craft.content_command(check=True) == 0
    assert len(first[craft.CATALOGUE]["records"]) == 125


def test_source_snapshot_substitution_is_rejected(monkeypatch):
    monkeypatch.setattr(craft, "SNAPSHOT_SHA256", "0" * 64)
    with pytest.raises(ValueError, match="source snapshot digest"):
        craft.build()


def test_mechanical_corpse_keeps_all_nineteen_exact_probabilities():
    rows = {r["identity"]["key"].split(".")[-1]: r["recipe"] for r in craft.build()}
    alternatives = rows["corpse-mechanical"]["outputs"]["alternatives"]
    observed = {int(a["entries"][0]["item_id"]["key"].split("i")[-1]): a["weight"] for a in alternatives}
    assert observed == {281: 10, 282: 10, 9303: 10, 3026: 12, 3029: 12, 3032: 12, 7158: 12, 7159: 12,
                        3035: 35, 3051: 35, 3052: 35, 3580: 35, 236: 35, 237: 35,
                        3119: 540, 3123: 540, 3264: 540, 3409: 540, 3578: 540}
    assert sum(observed.values()) == 3000
    assert rows["corpse-mechanical"]["target_effect"] == {"effect": "RETIRE"}
    ordinary = rows["corpse-ordinary"]["outputs"]["alternatives"]
    assert sum(a["weight"] for a in ordinary) == 10000
    assert next(a["weight"] for a in ordinary if not a["entries"]) == 8885


def test_dirty_water_nested_tables_are_exact_and_not_fishing():
    row = next(r["recipe"] for r in craft.build() if r["identity"]["key"].endswith(".dirty-water"))
    alternatives = row["outputs"]["alternatives"]
    assert len(alternatives) == 10
    assert sum(a["weight"] for a in alternatives) == 600
    assert sorted(a["weight"] for a in alternatives) == [3, 6, 14, 14, 23, 108, 108, 108, 108, 108]
    assert len(row["operands"]) == 1
    assert "skill" not in row and "tile_cooldown_ms" not in row


def test_all_cooking_branches_and_fluid_pairs_are_present():
    rows = craft.build()
    by_key = {r["identity"]["key"].split(".")[-1]: r["recipe"] for r in rows}
    vessels = craft.source_snapshot()["facts"]["baking"]["tables"]["liquidContainers"]
    assert sum(k.startswith("flour-") for k in by_key) == len(vessels) * 2
    for vessel in vessels:
        for fluid in (1, 9):
            target = by_key[f"flour-{vessel}-{fluid}"]["operands"][1]
            assert (target["role"], target["sub_type"], target["output_sub_type"], target["lifecycle"]) == ("TARGET", fluid, 0, "PRESERVE_INSTANCE")
    assert {"chocolate-dough", "garlic-dough", "garlic-tray", "sugar-oat"} <= set(by_key)
    assert sum(k.startswith("bake-") for k in by_key) == 16
    assert sum(k.startswith("mill-") for k in by_key) == 8
    assert sum(k.startswith("cookies-") for k in by_key) == 4
    assert all(r["outputs"]["entries"][0]["count"] == 12 for k, r in by_key.items() if k.startswith("cookies-"))


def test_fishing_curve_bait_timers_and_complete_adopted_water_sets():
    rows = [r["recipe"] for r in craft.build() if "skill" in r["recipe"]]
    assert len(rows) == 46
    by_rod = {}
    for row in rows:
        rod = row["input_item"]["key"]
        by_rod.setdefault(rod, set()).add(row["target"]["definition"]["key"])
        assert row["chance_curve"] == craft.CURVE
        assert row["operands"][0]["effect"] == "KEEP"
        ingredient = row["operands"][1]
        assert ingredient["role"] == "INGREDIENT" and ingredient["effect"] == "CONSUME"
        assert ingredient["consume_on"] == "SUCCESS" and ingredient["count"] == 1
        assert ingredient["item_id"] == craft.item(3492 if rod == craft.item(3483)["key"] else 953)
        water = row["target"]["definition"]["key"]
        assert row["tile_cooldown_ms"] == (900000 if water == craft.item(7236)["key"] else 3600000 if water == craft.item(13988)["key"] else 1200000)
        if water == craft.item(13988)["key"]:
            assert any("PARITY_PENDING" in d for d in row["source"]["differences"])
    assert by_rod[craft.item(3483)["key"]] == by_rod[craft.item(9306)["key"]]
    assert craft.item(622)["key"] not in by_rod[craft.item(3483)["key"]]
    assert craft.item(13989)["key"] not in by_rod[craft.item(3483)["key"]]


@pytest.mark.parametrize("size,accepted", [(0, False), (1, False), (2, True), (19, True), (24, True), (25, False)])
def test_schema_enforces_amended_weighted_ceiling(size, accepted):
    rows = craft.build()
    row = copy.deepcopy(next(r for r in rows if r["identity"]["key"].endswith(".corpse-mechanical")))
    row["recipe"]["outputs"]["alternatives"] = [row["recipe"]["outputs"]["alternatives"][0]] * size
    validator = jsonschema.Draft202012Validator(craft.load(craft.Path(__file__).with_name("schema.json")))
    assert validator.is_valid([row]) == accepted


def test_weighted_lowering_combines_duplicate_identities_without_rounding():
    result = craft.weighted([(1, Fraction(1, 3)), (1, Fraction(1, 6)), (2, Fraction(1, 2))])
    assert result["alternatives"] == [{"weight": 1, "entries": [craft.output(1)]}, {"weight": 1, "entries": [craft.output(2)]}]
