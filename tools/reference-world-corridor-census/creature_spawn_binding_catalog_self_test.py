#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
GAME_ROOT = HERE.parent.parent
MODULE_PATH = HERE / "creature_spawn_binding_catalog.py"

spec = importlib.util.spec_from_file_location("cw2_b2_catalog", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def test_unresolved_does_not_mint_native_identity() -> None:
    result = catalog.resolve_native_mapping("definition:monster:0001", ())
    assert result["disposition"] == "UNRESOLVED"
    assert result["content_key"] is None
    assert result.get("candidate_content_keys", []) == []


def test_asserted_binding_can_resolve_only_explicit_native_key() -> None:
    result = catalog.resolve_native_mapping(
        "definition:monster:0001",
        (
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.example",
                "evidence_ref": "protected-binding-a",
                "state": "ASSERTED",
            },
        ),
    )
    assert result["disposition"] == "RESOLVED"
    assert result["content_key"] == "oteryn:creature.example"


def test_candidate_ambiguity_and_asserted_conflict_are_distinct() -> None:
    ambiguous = catalog.resolve_native_mapping(
        "definition:monster:0001",
        (
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.a",
                "evidence_ref": "candidate-a",
                "state": "CANDIDATE",
            },
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.b",
                "evidence_ref": "candidate-b",
                "state": "CANDIDATE",
            },
        ),
    )
    assert ambiguous["disposition"] == "AMBIGUOUS"

    conflict = catalog.resolve_native_mapping(
        "definition:monster:0001",
        (
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.a",
                "evidence_ref": "asserted-a",
                "state": "ASSERTED",
            },
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.b",
                "evidence_ref": "asserted-b",
                "state": "ASSERTED",
            },
        ),
    )
    assert conflict["disposition"] == "CONFLICT"


def test_candidate_only_single_target_remains_unresolved() -> None:
    result = catalog.resolve_native_mapping(
        "definition:monster:0001",
        (
            {
                "source_identity": "definition:monster:0001",
                "content_key": "oteryn:creature.a",
                "evidence_ref": "candidate-a",
                "state": "CANDIDATE",
            },
        ),
    )
    assert result["disposition"] == "UNRESOLVED"
    assert result["content_key"] is None


def test_fake_or_fixture_native_key_promotion_fails_closed() -> None:
    for target, expected in (
        ("monster-entity:80295e51265b3662bfbea2ea01ee3ccb", "BINDING_TARGET_INVALID"),
        ("oteryn:vsl.creature.rat", "SYNTHETIC_VSL_TARGET_FORBIDDEN"),
    ):
        try:
            catalog.resolve_native_mapping(
                "definition:monster:0001",
                (
                    {
                        "source_identity": "definition:monster:0001",
                        "content_key": target,
                        "evidence_ref": "bad-binding",
                        "state": "ASSERTED",
                    },
                ),
            )
        except catalog.CatalogError as exc:
            assert expected in str(exc)
        else:
            raise AssertionError(f"forbidden target accepted: {target}")


def test_duplicate_normalized_names_are_collision_evidence_only() -> None:
    records = [
        {
            "source_file_ref": "monster:0001",
            "normalized_name": "rat",
            "atlas_entity_id": "monster-entity:one",
        },
        {
            "source_file_ref": "monster:0002",
            "normalized_name": "rat",
            "atlas_entity_id": "monster-entity:one",
        },
    ]
    collisions = catalog.definition_name_collisions(records)
    assert len(collisions) == 1
    assert collisions[0]["classification"] == (
        "SOURCE_NAME_COLLISION_NOT_NATIVE_AUTHORITY"
    )
    assert collisions[0]["source_file_refs"] == ["monster:0001", "monster:0002"]
    for source_ref in ("monster:0001", "monster:0002"):
        mapping = catalog.resolve_native_mapping(f"definition:{source_ref}", ())
        assert mapping["disposition"] == "UNRESOLVED"
        assert mapping["content_key"] is None


def test_duplicate_spawn_source_identity_fails_closed() -> None:
    try:
        catalog.ensure_unique_source_identities(
            ["spawn:world:0000:a", "spawn:world:0000:a"],
            "SPAWN",
        )
    except catalog.CatalogError as exc:
        assert "DUPLICATE_SPAWN_SOURCE_IDENTITY" in str(exc)
    else:
        raise AssertionError("duplicate spawn source identity was accepted")


def test_input_enumeration_order_is_semantically_identical() -> None:
    records = [
        {"source_identity": "b", "value": 2},
        {"source_identity": "a", "value": 1},
    ]
    forward = catalog.canonical_record_list(records)
    reverse = catalog.canonical_record_list(reversed(records))
    assert catalog.canonical_bytes(forward) == catalog.canonical_bytes(reverse)


MONSTER_LUA = """
local mType = Game.createMonsterType("Rat")
monster.outfit = { lookType = 21 }
monster.health = 30
monster.experience = 5
monster.speed = 100
monster.defenses = { defense = 4, armor = 2 }
monster.elements = {}
monster.immunities = {}
monster.loot = {
    { name = "gold coin", chance = 50000, maxCount = 2 }
}
"""

SPAWN_XML = """
<monsters>
  <monster centerx="300" centery="400" centerz="8" radius="3">
    <monster name="Rat" x="-2" y="2" z="8" spawntime="30"/>
  </monster>
</monsters>
"""


def test_pinned_game_exporters_feed_candidate_only_catalogue() -> None:
    producers = catalog.verify_game_producers(GAME_ROOT)
    assert producers["admission_main"] == catalog.ADMISSION_MAIN
    identity, creature, gameplay = catalog._load_game_modules(GAME_ROOT)

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        monster_root = root / "monster"
        monster_root.mkdir()
        monster_path = monster_root / "rat.lua"
        monster_path.write_text(MONSTER_LUA, encoding="utf-8")

        definition_snapshot = {
            "files": [
                {
                    "source_file_ref": "monster:0000",
                    "path": f"{catalog.MONSTER_ROOT}/rat.lua",
                    "role": "MONSTER_DEFINITION_LUA",
                }
            ]
        }
        definitions = catalog.build_definition_catalog(
            definition_snapshot,
            monster_root,
            identity,
            creature,
            gameplay,
        )
        assert len(definitions["records"]) == 1
        record = definitions["records"][0]
        assert record["native_mapping"]["disposition"] == "UNRESOLVED"
        assert record["atlas_entity_id"].startswith("monster-entity:")
        profile = definitions["candidate_profiles"][0]
        assert profile["classification"] == catalog.EVIDENCE_STATUS
        assert profile["presentation"]["state"] == "CANDIDATE"
        assert profile["stats"]["health"] == 30
        assert definitions["deferred_b3_loot"]["observed_rows"] == 1

        spawn_path = f"{catalog.WORLD_ROOT}/world-monster.xml"
        spawn_payload = SPAWN_XML.encode("utf-8")
        spawn_snapshot = {
            "files": [
                {
                    "source_file_ref": "world:0000",
                    "path": spawn_path,
                    "role": "MONSTER_SPAWN_XML",
                }
            ]
        }
        spawns = catalog.build_spawn_catalog(
            spawn_snapshot,
            {spawn_path: spawn_payload},
            root / "scratch",
            identity,
            creature,
        )
        assert len(spawns["records"]) == 1
        spawn = spawns["records"][0]
        assert spawn["native_disposition"] == "UNRESOLVED"
        assert spawn["position"] == {"x": 298, "y": 402, "floor": -8}
        assert spawn["spawn_time_seconds"] == 30


def main() -> int:
    test_unresolved_does_not_mint_native_identity()
    test_asserted_binding_can_resolve_only_explicit_native_key()
    test_candidate_ambiguity_and_asserted_conflict_are_distinct()
    test_candidate_only_single_target_remains_unresolved()
    test_fake_or_fixture_native_key_promotion_fails_closed()
    test_duplicate_normalized_names_are_collision_evidence_only()
    test_duplicate_spawn_source_identity_fails_closed()
    test_input_enumeration_order_is_semantically_identical()
    test_pinned_game_exporters_feed_candidate_only_catalogue()
    print("creature-spawn-binding-catalog self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
