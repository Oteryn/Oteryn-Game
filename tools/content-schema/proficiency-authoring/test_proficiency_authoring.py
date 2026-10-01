#!/usr/bin/env python3
"""No-network tests for proficiency_authoring.py. Run with `python test_proficiency_authoring.py`."""

from __future__ import annotations

import copy
import json

import proficiency_authoring as pa


def expect_error(fn, *args, fragment: str) -> None:
    try:
        fn(*args)
    except ValueError as error:
        assert fragment in str(error), f"{fragment!r} not in {error}"
        return
    raise AssertionError(f"expected ValueError containing {fragment!r}")


def committed() -> dict:
    return json.loads(pa.CATALOGUE.read_text(encoding="utf-8"))


def expect_invalid(catalogue: dict, fragment: str) -> None:
    errors = pa.validate(catalogue, pa.staged_records()[1])
    assert any(fragment in e for e in errors), f"{fragment!r} not in {errors}"


def test_encode_perk() -> None:
    assert pa.encode_perk({"SkillId": 8, "Type": 3, "Value": 1}) == {
        "kind": "skill_bonus",
        "skill": "sword",
        "value": 1,
    }
    assert (
        pa.encode_perk({"ElementId": 8, "Type": 13, "Value": 0.1})["element"] == "fire"
    )
    missile = {
        "ElementId": 32,
        "MissileId": 71,
        "Multiplier": 2.0,
        "Probability": 0.01,
        "Type": 32,
    }
    assert pa.encode_perk(missile) == {
        "kind": "homing_missile",
        "element": "energy",
        "missile_client_id": 71,
        "probability": 0.01,
        "multiplier": 2.0,
    }
    for raw in (
        {"SkillId": 8, "Type": 3, "Value": 1},
        {"AugmentType": 6, "SpellId": 105, "Type": 5, "Value": -2.0},
        {"BestiaryId": 21, "BestiaryName": "Inkborn", "Type": 6, "Value": 0.03},
        {"Range": 3, "Type": 22, "Value": 10},
        missile,
    ):
        assert pa.decode_perk(pa.encode_perk(raw)) == raw
    expect_error(
        pa.encode_perk, {"Type": 33, "Value": 1}, fragment="unmapped perk Type 33"
    )
    expect_error(
        pa.encode_perk, {"Type": -1, "Value": 1}, fragment="unmapped perk Type -1"
    )
    expect_error(
        pa.encode_perk,
        {"SkillId": 2, "Type": 3, "Value": 1},
        fragment="unmapped SkillId 2",
    )
    expect_error(
        pa.encode_perk,
        {"ElementId": 2, "Type": 9, "Value": 0.1},
        fragment="unmapped ElementId",
    )
    expect_error(
        pa.encode_perk, {"SkillId": 8, "Type": 0, "Value": 1}, fragment="has keys"
    )
    expect_error(pa.encode_perk, {"Type": 3, "Value": 1}, fragment="has keys")


def test_every_type_is_mapped_once() -> None:
    assert sorted(pa.PERK_TYPES) == list(range(33))
    assert len(pa.KIND_TYPES) == 33


def test_committed_build() -> None:
    catalogue = committed()
    assert pa.dumps(pa.build()) == pa.CATALOGUE.read_text(encoding="utf-8")
    assert pa.validate(catalogue, pa.staged_records()[1]) == []
    assert len(catalogue["proficiencies"]) == pa.EXPECTED_COUNT
    first = catalogue["proficiencies"][0]
    assert first["identity"] == {
        "family": "Proficiency",
        "key": "oteryn:proficiency.tibia.p6",
        "revision": "definition-r1",
    }
    assert first["name"] == "Sanguine 1H Sword" and len(first["levels"]) == 7


def test_validator_rules() -> None:
    base = committed()

    bad = copy.deepcopy(base)
    bad["proficiencies"][0]["levels"][0]["perks"][0]["kind"] = "not_a_kind"
    expect_invalid(bad, "is not valid under any of the given schemas")

    bad = copy.deepcopy(base)
    bad["proficiencies"][0]["levels"][0]["perks"] *= 4
    expect_invalid(bad, "is too long")

    bad = copy.deepcopy(base)
    bad["proficiencies"][1]["identity"]["key"] = bad["proficiencies"][0]["identity"][
        "key"
    ]
    expect_invalid(bad, "duplicate key")

    bad = copy.deepcopy(base)
    bad["proficiencies"][0]["source"]["proficiency_id"] += 100000
    expect_invalid(bad, "key does not follow")

    bad = copy.deepcopy(base)
    levels = next(p for p in bad["proficiencies"] if len(p["levels"]) > 1)["levels"]
    levels[0]["level"], levels[1]["level"] = 2, 1
    expect_invalid(bad, "levels must be 1..n")

    bad = copy.deepcopy(base)
    perk = next(
        pk
        for p in bad["proficiencies"]
        for lv in p["levels"]
        for pk in lv["perks"]
        if pk["kind"] == "spell_augment" and pk["augment"] == "cooldown"
    )
    perk["value"] = 1
    expect_invalid(bad, "cooldown augment must be negative")

    bad = copy.deepcopy(base)
    perk = next(
        pk
        for p in bad["proficiencies"]
        for lv in p["levels"]
        for pk in lv["perks"]
        if pk["kind"] == "attack_damage"
    )
    perk["value"] = 0
    expect_invalid(bad, "value must be positive")

    bad = copy.deepcopy(base)
    perk = next(
        pk
        for p in bad["proficiencies"]
        for lv in p["levels"]
        for pk in lv["perks"]
        if pk["kind"] == "attack_damage"
    )
    perk["value"] += 1
    expect_invalid(bad, "do not round-trip")

    bad = copy.deepcopy(base)
    bad["threshold_tables"]["standard"][3] = 1
    expect_invalid(bad, "must strictly increase")

    bad = copy.deepcopy(base)
    bad["proficiencies"].pop()
    expect_invalid(bad, "differ from the staged definitions")


def test_content_tree_is_current_and_registered() -> None:
    files = pa.content_files(committed())
    for rel, text in files.items():
        assert (pa.ROOT / rel).read_text(encoding="utf-8") == text, rel
    index = json.loads(files[pa.INDEX_PATH])
    assert index["record_count"] == pa.EXPECTED_COUNT and len(index["shards"]) == 3
    keys = []
    for path in index["shards"]:
        shard = json.loads(files[path])
        assert shard["family"] == "Proficiency"
        assert shard["shard"]["count"] == len(shard["records"])
        for row in shard["records"]:
            assert set(row["definition"]["identity"]) == {"key", "revision"}
            keys.append(row["definition"]["identity"]["key"])
    assert len(keys) == len(set(keys)) == pa.EXPECTED_COUNT
    assert "threshold_tables" not in files[pa.INDEX_PATH]
    assert pa.content_command(check=True) == 0


def test_registration_is_idempotent() -> None:
    docs = [
        json.loads((pa.ROOT / f"content/{n}.json").read_text(encoding="utf-8"))
        for n in ("project", "manifest", "content.lock")
    ]
    paths = sorted(pa.content_files(committed()))
    project, manifest, lock = pa.registered(*docs, pa.EXPECTED_COUNT, paths)
    assert pa.registered(project, manifest, lock, pa.EXPECTED_COUNT, paths) == (
        project,
        manifest,
        lock,
    )
    assert "Proficiency" in project["migrated_families"]
    assert "Proficiency" not in project["next_population_families"]
    assert manifest["families"]["Proficiency"] == {
        "records": pa.EXPECTED_COUNT,
        "index": "content/proficiencies/index.json",
    }
    assert lock["family_counts"]["Proficiency"] == pa.EXPECTED_COUNT


def test_item_bindings() -> None:
    bindings = pa.item_bindings(committed())
    rows = bindings["records"]
    # ITEM-ADD-1: the donor epoch-2 Items define 22 more bound weapons.
    assert bindings["record_count"] == len(rows) == 664
    assert bindings["excluded"] == {
        "item_not_defined": 1,
        "unknown_threshold_class": 1,
    }
    items = pa.content_item_revisions()
    definitions = {d["identity"]["key"] for d in committed()["proficiencies"]}
    keys = [row["item"]["key"] for row in rows]
    assert keys == sorted(set(keys))
    for row in rows:
        assert items[row["item"]["key"]] == row["item"]["revision"]
        assert row["profile_binding"]["key"] in definitions
        assert row["threshold_class"] in pa.THRESHOLD_CLASSES
    source = json.loads((pa.ROOT / pa.BINDING_SOURCE_REL).read_text(encoding="utf-8"))
    by_item = {row["item"]["key"]: row for row in rows}
    for row in source["bindings"]:
        bound = by_item.get(row["item_key"])
        if bound is None:
            continue
        assert bound["threshold_class"] == row["threshold_class"]
        assert bound["profile_binding"]["key"] == (
            f"oteryn:proficiency.tibia.p{row['proficiency_id']}"
        )


if __name__ == "__main__":
    for name, fn in list(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
            print(f"ok {name}")
