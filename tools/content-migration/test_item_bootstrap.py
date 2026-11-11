#!/usr/bin/env python3
"""Exercise the shared Item writer without native, Prey, or World generation."""
import importlib.util
import json
from contextlib import ExitStack, contextmanager
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import boss_conditional_item_reference as conditional
import item_tree_index
import ranged_item_profiles as ranged

TOOL = Path(__file__).with_name("world_project_v2_to_tree.py")
spec = importlib.util.spec_from_file_location("item_bootstrap_converter", TOOL)
converter = importlib.util.module_from_spec(spec)
# Taxonomy belongs to the later, non-Item portion of the full converter.
with patch.dict(sys.modules, {"item_taxonomy": SimpleNamespace(
        build_taxonomy=None, taxonomy_inputs=None)}):
    spec.loader.exec_module(converter)


def write(root, relative, value):
    target = root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(converter.canonical_bytes(value))


def files(root):
    return {path.relative_to(root).as_posix(): path.read_bytes()
            for path in root.rglob("*") if path.is_file()}


def identity(number, family="Item"):
    return {"family": family, "key": f"oteryn:item.tibia.i{number}",
            "revision": "definition-r1"}


def fixture(root):
    records = [{"kind": "Item", "identity": identity(number), "materializable": False}
               for number in (1, 2, 3)]
    reference = {"schema": "fixture-reference", "world_id": "fixture-world",
                 "coordinate_frame": "fixture-frame", "records": records}
    authoring = {"item": identity(1), "taxonomy": {"owner": "taxonomy"},
                 "weapon_attack_modifier_points": 17, "sealed_owner": {"value": 9}}
    declarations = {"item_authoring": [authoring], "records": []}
    editor = {"entries": [{"target": identity(1), "label": "authored Item"},
                          {"target": identity(9, "Creature"), "label": "other family"}]}
    sources = {"source_identity_bindings": [
        {"target": identity(1), "source": "z"},
        {"target": identity(1), "source": "a"},
        {"target": identity(9, "Creature"), "source": "creature"},
    ]}
    for name, document in (("definitions/reference.json", reference),
                           ("definitions/declarations.json", declarations),
                           ("editor/author.json", editor),
                           ("provenance/sources.json", sources),
                           ("provenance/imports.json", {"batches": []})):
        write(root, "content/world/" + name, document)
    write(root, "content/items/index.json", {
        "shards": ["content/items/definitions/items-00002-00005.json"],
        "record_count": 6,
    })
    write(root, "content/items/definitions/items-00002-00004.json", {
        "records": [{"definition": records[2]}, {"definition": {"identity": identity(999)}}],
    })
    write(root, "content/items/definitions/items-note.json", {"records": []})
    for relative in ("content/manifest.json", "content/content.lock.json",
                     "content/world/pins/oteryn.json", "imports/tibiawiki/sources.json",
                     "tools/content-schema/native-gameplay/grizzly-shop-profiles.json",
                     "tools/content-schema/quest-authoring/samples/donor-source/kept.json",
                     "content/items/definitions/spell-native-profiles.json",
                     "content/creatures/definitions/creatures-00000-00001.json"):
        write(root, relative, {"preserved": relative})
    write(root, "fixture/census.json", {"family_assignments": {}})
    write(root, "fixture/wave.json", {})
    return reference, declarations, editor, sources


@contextmanager
def input_helpers(root, *, real_conditional=False):
    calls = []

    def admission(name):
        def apply(definitions, *args):
            calls.append(name)
            if name == "quest_reward_item_semantics":
                definitions[0]["materializable"] = True
        return apply

    modules = {}
    for name, function in (
        ("quest_reward_item_semantics", "apply_admissions"),
        ("amber_item_semantics", "apply_container_fact"),
        ("enchanting_item_profiles", "apply_admissions"),
        ("boss_conditional_item_reference", "extend_records"),
        ("boss_movement_item_profiles", "extend_records"),
        ("boss_corpse_terminal_reference", "extend_records"),
        ("qready1_cradle_item_profiles", "extend_records"),
        ("cake_bridge_reference", "extend_records"),
        ("encounter_loot_item_admissions", "apply_admissions"),
        ("itemhold_q", "apply_admissions"),
        ("tainted_talisman", "apply_admissions"),
        ("engclaim_item_profiles", "apply_admissions"),
    ):
        modules[name] = SimpleNamespace(**{function: admission(name)})
    if real_conditional:
        modules["boss_conditional_item_reference"] = conditional
    modules["primal_reference_overlay"] = SimpleNamespace(apply_reference=lambda *_: None)
    with ExitStack() as stack:
        stack.enter_context(patch.dict(sys.modules, modules))
        stack.enter_context(patch.multiple(
            converter, ROOT=root, LEGACY=root / "content/world", ITEM_SHARD_SIZE=2,
            FIELD_CENSUS=root / "fixture/census.json", WAVE1_STAGED=root / "fixture/wave.json"))
        stack.enter_context(patch.object(item_tree_index, "ITEM_SHARD_SIZE", converter.ITEM_SHARD_SIZE))
        for name in ("apply_summer_item_admissions", "apply_primal_item_admissions",
                     "apply_encounter_corpse_admissions"):
            stack.enter_context(patch.object(converter, name, side_effect=admission(name)))
        stack.enter_context(patch.object(converter, "item_alias_targets", return_value={}))
        stack.enter_context(patch.object(ranged, "source_profiles", return_value=(
            {1: {"attack": 10, "hit_chance": 90}, 2: {"attack": 6},
             3: {"attack": 8}, 999: {"attack": 5}}, [], {})))
        ranged_call = stack.enter_context(patch.object(ranged, "outputs", wraps=ranged.outputs))
        stack.enter_context(patch.object(converter.subprocess, "run",
                                        side_effect=AssertionError("unexpected later producer")))
        yield calls, ranged_call


class ItemBootstrapTests(unittest.TestCase):
    def test_dangling_index_repaired_with_exact_attachments_and_no_foreign_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference, declarations, editor, sources = fixture(root)
            before = files(root)
            with input_helpers(root) as (calls, ranged_call):
                self.assertEqual(converter.cli(["--items-only"]), 0)
                ranged_call.assert_called_once_with(root)
            index = json.loads((root / "content/items/index.json").read_bytes())
            expected_shards = ["content/items/definitions/items-00000-00001.json",
                               "content/items/definitions/items-00002-00002.json"]
            self.assertEqual(index["shards"], expected_shards)
            self.assertEqual(index["record_count"], 3)
            self.assertEqual(index["legacy_source"]["git_blob_sha"],
                             converter.git_blob_sha(root / "content/world/definitions/reference.json"))
            self.assertEqual((index["attached_editor_entries"], index["attached_source_bindings"],
                              index["attached_authoring_entries"]), (1, 2, 1))
            rows = [row for path in expected_shards
                    for row in json.loads((root / path).read_bytes())["records"]]
            self.assertEqual([row["definition"]["identity"] for row in rows],
                             [identity(1), identity(2), identity(3)])
            self.assertTrue(rows[0]["definition"]["materializable"])
            self.assertEqual(rows[0]["editor"], editor["entries"][0])
            self.assertEqual(rows[0]["source_bindings"], sorted(
                sources["source_identity_bindings"][:2], key=converter.canonical_bytes))
            self.assertEqual(rows[0]["authoring"], {
                key: value for key, value in declarations["item_authoring"][0].items()
                if key not in {"item", "taxonomy"}})
            self.assertNotIn("authoring", rows[1])
            profiles = json.loads((root / ranged.RANGED_PROFILES).read_bytes())["records"]
            self.assertEqual([row["item"] for row in profiles], [identity(1), identity(2), identity(3)])
            self.assertEqual(profiles[0]["profile"], {"hit_chance": 90})
            after = files(root)
            removed = "content/items/definitions/items-00002-00004.json"
            self.assertEqual(set(before) - set(after), {removed})
            allowed = {"content/items/index.json", ranged.RANGED_PROFILES, *expected_shards}
            self.assertEqual(set(after) - set(before), allowed - {"content/items/index.json"})
            for path in before.keys() & after.keys() - allowed:
                self.assertEqual(after[path], before[path], path)
            with input_helpers(root):
                self.assertEqual(converter.cli(["--items-only"]), 0)
            self.assertEqual(files(root), after)

    def test_normal_main_and_item_only_use_the_same_writer_and_emit_identical_bytes(self):
        class ItemPhaseComplete(Exception):
            pass

        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            normal, bootstrap = Path(first), Path(second)
            fixture(normal)
            fixture(bootstrap)
            shared = converter.materialize_item_tree

            def stop_after_item_phase(*args, **kwargs):
                result = shared(*args, **kwargs)
                self.assertIs(result[0][0], args[0]["records"][0])
                self.assertTrue(args[0]["records"][0]["materializable"])
                raise ItemPhaseComplete

            with input_helpers(normal), patch.object(
                    converter, "materialize_item_tree", side_effect=stop_after_item_phase) as owner:
                with self.assertRaises(ItemPhaseComplete):
                    converter.main()
                owner.assert_called_once()
            with input_helpers(bootstrap), patch.object(
                    converter, "materialize_item_tree", wraps=shared) as owner:
                self.assertEqual(converter.cli(["--items-only"]), 0)
                owner.assert_called_once()
            self.assertEqual(files(normal), files(bootstrap))

    def test_real_capture_pin_refusal_precedes_any_output_or_later_producer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture(root)
            write(root, conditional.PATH, {"unapproved": True})
            write(root, conditional.PIN_PATH, {"path": conditional.PATH, "sha256": "0" * 64})
            before = files(root)
            with input_helpers(root, real_conditional=True) as (calls, ranged_call):
                with self.assertRaisesRegex(ValueError, "CONDITIONAL_REFERENCE_CAPTURE_PIN"):
                    converter.cli(["--items-only"])
                ranged_call.assert_not_called()
                self.assertNotIn("boss_movement_item_profiles", calls)
            self.assertEqual(files(root), before)

    def test_item_only_cannot_be_combined_with_other_modes(self):
        with patch.object(converter, "main") as full, \
                patch.object(converter, "refresh_bestiary_inventory") as bestiary:
            with self.assertRaises(SystemExit) as error:
                converter.cli(["--items-only", "--refresh-bestiary-inventory"])
            self.assertEqual(error.exception.code, 2)
            full.assert_not_called()
            bestiary.assert_not_called()


if __name__ == "__main__":
    unittest.main()
