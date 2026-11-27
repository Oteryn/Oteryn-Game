"""Exercise the shared Item writer without native, Prey, or World generation."""

import importlib.util
import json
import shutil
import sys
import tempfile
import unittest
from collections import Counter
from contextlib import ExitStack, contextmanager, redirect_stderr
from io import StringIO
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import boss_conditional_item_reference as conditional
import item_tree_index
import ranged_item_profiles as ranged
from dispenser_item_profiles import APPEARANCES

TOOL = Path(__file__).with_name("world_project_v2_to_tree.py")
spec = importlib.util.spec_from_file_location("item_bootstrap_converter", TOOL)
converter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(converter)


def write(root, relative, value):
    target = root / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(converter.canonical_bytes(value))


def files(root):
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


def identity(number, family="Item"):
    return {
        "family": family,
        "key": f"oteryn:item.tibia.i{number}",
        "revision": "definition-r1",
    }


def fixture(root):
    # The shared writer now verifies Global weights in both entry points.
    # Supply its actual sealed capture without mocking that admission owner.
    capture = Path("docs/reference/content-prep/item-cyclopedia-capture-1")
    (root / capture).mkdir(parents=True)
    for name in ("items.jsonl", "manifest.json", "weight-corrections.json"):
        shutil.copyfile(TOOL.parents[2] / capture / name, root / capture / name)
    project = TOOL.parents[2]
    with patch.object(
        sys, "path", [str(project / "tools/content-schema/native-gameplay"), *sys.path]
    ):
        import crystal_equipment_profiles as crystal
        from lower_elemental_magic_modifier_packet import ENUM

    for relative in (crystal.FACTS, crystal.BINDINGS, ENUM):
        (root / relative).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(project / relative, root / relative)
    source_rows = json.loads((root / crystal.FACTS).read_bytes())["records"]
    bindings = {
        row["external_id"]: row["target"]
        for row in json.loads((root / crystal.BINDINGS).read_bytes())["bindings"]
    }
    identities = [identity(number) for number in (1, 2, 3)]
    identities.extend(
        bindings[str(row["id"])] for row in source_rows if "held" not in row
    )
    for relative in (APPEARANCES, "imports/tibiawiki/facts/items-stats.json"):
        (root / relative).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(TOOL.parents[2] / relative, root / relative)
    records = [
        {"kind": "Item", "identity": target, "materializable": False}
        for target in identities
    ]
    reference = {
        "schema": "fixture-reference",
        "world_id": "fixture-world",
        "coordinate_frame": "fixture-frame",
        "records": records,
    }
    authoring = {
        "item": identity(1),
        "taxonomy": {"owner": "taxonomy"},
        "weapon_attack_modifier_points": 17,
        "sealed_owner": {"value": 9},
    }
    declarations = {"item_authoring": [authoring], "records": []}
    editor = {
        "entries": [
            {"target": identity(1), "label": "authored Item"},
            {"target": identity(9, "Creature"), "label": "other family"},
        ]
    }
    sources = {
        "source_identity_bindings": [
            {"target": identity(1), "source": "z"},
            {"target": identity(1), "source": "a"},
            {"target": identity(9, "Creature"), "source": "creature"},
        ]
    }
    for name, document in (
        ("definitions/reference.json", reference),
        ("definitions/declarations.json", declarations),
        ("editor/author.json", editor),
        ("provenance/sources.json", sources),
        ("provenance/imports.json", {"batches": []}),
    ):
        write(root, "content/world/" + name, document)
    write(
        root,
        "content/items/index.json",
        {
            "shards": ["content/items/definitions/items-00002-00005.json"],
            "record_count": 6,
        },
    )
    write(
        root,
        "content/items/definitions/items-00002-00004.json",
        {
            "records": [
                {"definition": records[2]},
                {"definition": {"identity": identity(999)}},
            ],
        },
    )
    write(root, "content/items/definitions/items-note.json", {"records": []})
    for relative in (
        "content/manifest.json",
        "content/content.lock.json",
        "content/world/pins/oteryn.json",
        "imports/tibiawiki/sources.json",
        "tools/content-schema/native-gameplay/grizzly-shop-profiles.json",
        "tools/content-schema/quest-authoring/samples/donor-source/kept.json",
        "content/items/definitions/spell-native-profiles.json",
        "content/creatures/definitions/creatures-00000-00001.json",
    ):
        write(root, relative, {"preserved": relative})
    write(root, "imports/crystalserver/bindings/ordinary-doors.json", {"bindings": []})
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
        ("nrwicked_profiles", "extend_records"),
        ("encounter_loot_item_admissions", "apply_admissions"),
        ("itemhold_q", "apply_admissions"),
        ("tainted_talisman", "apply_admissions"),
        ("engclaim_item_profiles", "apply_admissions"),
    ):
        modules[name] = SimpleNamespace(**{function: admission(name)})
    if real_conditional:
        modules["boss_conditional_item_reference"] = conditional
    modules["primal_reference_overlay"] = SimpleNamespace(
        apply_reference=lambda *_: None
    )
    with ExitStack() as stack:
        native_tools = TOOL.parents[2] / "tools/content-schema/native-gameplay"
        stack.enter_context(patch.object(sys, "path", [str(native_tools), *sys.path]))
        stack.enter_context(patch.dict(sys.modules, modules))
        stack.enter_context(
            patch.multiple(
                converter,
                ROOT=root,
                LEGACY=root / "content/world",
                ITEM_SHARD_SIZE=2,
                FIELD_CENSUS=root / "fixture/census.json",
                WAVE1_STAGED=root / "fixture/wave.json",
            )
        )
        stack.enter_context(
            patch.object(item_tree_index, "ITEM_SHARD_SIZE", converter.ITEM_SHARD_SIZE)
        )
        for name in (
            "apply_summer_item_admissions",
            "apply_primal_item_admissions",
            "apply_encounter_corpse_admissions",
        ):
            stack.enter_context(
                patch.object(converter, name, side_effect=admission(name))
            )
        stack.enter_context(
            patch.object(converter, "item_alias_targets", return_value={})
        )
        stack.enter_context(patch.object(converter, "taxonomy_inputs", return_value=()))
        stack.enter_context(patch.object(converter, "build_taxonomy", return_value=[]))
        stack.enter_context(
            patch.object(
                ranged,
                "source_profiles",
                return_value=(
                    {
                        1: {"attack": 10, "hit_chance": 90},
                        2: {"attack": 6},
                        3: {"attack": 8},
                        999: {"attack": 5},
                    },
                    [],
                    {},
                ),
            )
        )
        ranged_call = stack.enter_context(
            patch.object(ranged, "outputs", wraps=ranged.outputs)
        )
        stack.enter_context(
            patch.object(
                converter.subprocess,
                "run",
                side_effect=AssertionError("unexpected later producer"),
            )
        )
        yield calls, ranged_call


class ItemBootstrapTests(unittest.TestCase):
    def test_dangling_index_repaired_with_exact_attachments_and_no_foreign_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference, declarations, editor, sources = fixture(root)
            before = files(root)
            with input_helpers(root) as (_calls, ranged_call):
                self.assertEqual(converter.cli(["--items-only"]), 0)
                ranged_call.assert_called_once_with(root)
            index = json.loads((root / "content/items/index.json").read_bytes())
            expected_identities = [row["identity"] for row in reference["records"]] + [identity(8748)]
            count = len(expected_identities)
            expected_shards = [
                f"content/items/definitions/items-{start:05d}-{min(start + 1, count - 1):05d}.json"
                for start in range(0, count, 2)
            ]
            self.assertEqual(index["shards"], expected_shards)
            self.assertEqual(index["record_count"], count)
            self.assertEqual(
                index["legacy_source"]["git_blob_sha"],
                converter.git_blob_sha(
                    root / "content/world/definitions/reference.json"
                ),
            )
            self.assertEqual(
                (
                    index["attached_editor_entries"],
                    index["attached_source_bindings"],
                    index["attached_authoring_entries"],
                ),
                (1, 2, 133),
            )
            rows = [
                row
                for path in expected_shards
                for row in json.loads((root / path).read_bytes())["records"]
            ]
            self.assertEqual(
                [row["definition"]["identity"] for row in rows], expected_identities
            )
            self.assertTrue(rows[0]["definition"]["materializable"])
            self.assertTrue(
                all(row["definition"]["materializable"] for row in rows[3:])
            )
            self.assertEqual(rows[0]["editor"], editor["entries"][0])
            self.assertEqual(
                rows[0]["source_bindings"],
                sorted(
                    sources["source_identity_bindings"][:2],
                    key=converter.canonical_bytes,
                ),
            )
            self.assertEqual(
                rows[0]["authoring"],
                {
                    key: value
                    for key, value in declarations["item_authoring"][0].items()
                    if key not in {"item", "taxonomy"}
                },
            )
            self.assertNotIn("authoring", rows[1])
            profiles = json.loads((root / ranged.RANGED_PROFILES).read_bytes())[
                "records"
            ]
            self.assertEqual(
                [row["item"] for row in profiles],
                [identity(1), identity(2), identity(3)],
            )
            self.assertEqual(profiles[0]["profile"], {"hit_chance": 90})
            after = files(root)
            removed = "content/items/definitions/items-00002-00004.json"
            self.assertEqual(set(before) - set(after), {removed})
            taxonomy = "content/items/taxonomy/items.json"
            relations = "content/items/relations/items.json"
            lock = "content/content.lock.json"
            added = {taxonomy, relations, ranged.RANGED_PROFILES, *expected_shards}
            allowed = added | {"content/items/index.json", lock}
            self.assertEqual(set(after) - set(before), added)
            self.assertEqual(json.loads(after[taxonomy])["records"], [])
            relation_rows = json.loads(after[relations])["records"]
            self.assertEqual(len(relation_rows), 144)
            self.assertEqual(
                Counter(
                    entry["basis"]
                    for row in relation_rows
                    for entry in row["relations"]
                ),
                {"forge": 132, "imbuement.slot_count>=1": 93},
            )
            self.assertEqual(
                json.loads(after[lock]),
                {
                    **json.loads(before[lock]),
                    "item_authoring_counts": {
                        "authoring": 133,
                        "taxonomy": 0,
                        "relation_sources": 144,
                    },
                },
            )
            for path in before.keys() & after.keys() - allowed:
                self.assertEqual(after[path], before[path], path)
            with input_helpers(root):
                self.assertEqual(converter.cli(["--items-only"]), 0)
            self.assertEqual(files(root), after)

    def test_normal_main_and_item_only_use_the_same_writer_and_emit_identical_bytes(
        self,
    ):
        class ItemPhaseComplete(Exception):
            pass

        with (
            tempfile.TemporaryDirectory() as first,
            tempfile.TemporaryDirectory() as second,
        ):
            normal, bootstrap = Path(first), Path(second)
            fixture(normal)
            fixture(bootstrap)
            shared = converter.materialize_item_tree

            def stop_after_item_phase(*args, **kwargs):
                result = shared(*args, **kwargs)
                self.assertIs(result[0][0], args[0]["records"][0])
                self.assertTrue(args[0]["records"][0]["materializable"])
                raise ItemPhaseComplete

            with (
                input_helpers(normal),
                patch.object(
                    converter,
                    "materialize_item_tree",
                    side_effect=stop_after_item_phase,
                ) as owner,
            ):
                with self.assertRaises(ItemPhaseComplete):
                    converter.main()
                owner.assert_called_once()
            with (
                input_helpers(bootstrap),
                patch.object(converter, "materialize_item_tree", wraps=shared) as owner,
            ):
                self.assertEqual(converter.cli(["--items-only"]), 0)
                owner.assert_called_once()
            self.assertEqual(files(normal), files(bootstrap))

    def test_real_capture_pin_refusal_precedes_any_output_or_later_producer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture(root)
            write(root, conditional.PATH, {"unapproved": True})
            write(
                root,
                conditional.PIN_PATH,
                {"path": conditional.PATH, "sha256": "0" * 64},
            )
            before = files(root)
            with input_helpers(root, real_conditional=True) as (calls, ranged_call):
                with self.assertRaisesRegex(
                    ValueError, "CONDITIONAL_REFERENCE_CAPTURE_PIN"
                ):
                    converter.cli(["--items-only"])
                ranged_call.assert_not_called()
                self.assertNotIn("boss_movement_item_profiles", calls)
            self.assertEqual(files(root), before)

    def test_item_only_cannot_be_combined_with_other_modes(self):
        with (
            patch.object(converter, "main") as full,
            patch.object(converter, "refresh_bestiary_inventory") as bestiary,
        ):
            diagnostic = StringIO()
            with redirect_stderr(diagnostic), self.assertRaises(SystemExit) as error:
                converter.cli(["--items-only", "--refresh-bestiary-inventory"])
            self.assertEqual(error.exception.code, 2)
            self.assertIn(
                "not allowed with argument --items-only", diagnostic.getvalue()
            )
            full.assert_not_called()
            bestiary.assert_not_called()


if __name__ == "__main__":
    unittest.main()
