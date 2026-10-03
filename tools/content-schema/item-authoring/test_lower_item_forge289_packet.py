"""Closed289 source identity, literal pairs, atomic-owner and historical165 guards."""

import copy
import hashlib
import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

import check_tibiawiki165_historical_context as context
import lower_item_forge289_packet as forge
import sys

# Main's migration validator imports its sibling modules, as its own scripts do.
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))


class Forge289(unittest.TestCase):
    def arguments(self):
        proof = json.loads((forge.ROOT / forge.PROOF).read_bytes())
        source = copy.deepcopy(proof["records"][0])
        q = source["qualification"]
        page = next(
            p
            for p in json.loads((forge.ROOT / forge.SNAPSHOT).read_bytes())["pages"]
            if p["page_id"] == q["BR_source_witness"]["BR_page_id"]
        )
        obj = {
            "id": q["source_item_id"],
            "name": q["BR_source_witness"]["official_object_name"],
            "flags": {
                "flags.take": True,
                "upgradeclassification.upgrade_classification": q["forge"][
                    "classification"
                ],
            },
        }
        return [
            source,
            copy.deepcopy(page),
            copy.deepcopy(q["native_definition"]),
            obj,
            [],
            set(),
            proof["qualification_cutoff"],
            proof["source_type_corroboration"],
        ]

    def test_whole289_pairs_mapper_and_empty_native_import_candidates(self):
        packet = forge.build()
        self.assertEqual(packet, json.loads(forge.OUTPUT.read_bytes()))
        self.assertEqual(
            packet["counts"],
            {
                "items": 289,
                "fields": 578,
                "bindings": 289,
                "literal_distribution": {"1/1": 135, "2/2": 55, "3/3": 18, "4/10": 81},
            },
        )
        self.assertEqual(packet["import_batch"]["candidates"], [])
        self.assertEqual(packet["import_batch"]["reimport_states"], [])
        actual = hashlib.sha256((forge.ROOT / forge.COMPILER).read_bytes()).hexdigest()
        self.assertEqual(
            packet["import_batch"]["mapper_revision"],
            "forge289-source-mapper-v1:" + actual,
        )
        self.assertEqual(packet["compiler"]["sha256"], actual)
        self.assertEqual(packet["import_batch"]["mapper_sha256"], actual)
        self.assertEqual(
            {b["target"]["key"] for b in packet["bindings"]},
            {r["item"]["key"] for r in packet["promotions"]},
        )

    def test_explicit_pair_own_id_nonname_stat_and_name_conflicts_fail(self):
        def missing(a):
            a[1]["raw_full_article"] = a[1]["raw_full_article"].replace(
                "| max_tier       = 2", ""
            )
            text = a[1]["raw_full_article"]
            a[1]["raw_content_sha256"] = forge.old.sha(text.encode())
            a[1]["raw_content_bytes"] = len(text.encode())
            a[0]["qualification"]["BR_source_witness"]["BR_raw_content_sha256"] = a[1][
                "raw_content_sha256"
            ]

        for change in [
            missing,
            lambda a: a[0]["own_fandom"].update(
                raw=a[0]["own_fandom"]["raw"].replace("3208", "3209")
            ),
            lambda a: a[2]["semantics"]["weapon"]["value"]["attack"].update(value=True),
            lambda a: a[2]["semantics"]["presentation"]["value"]["name"].update(
                state="UNKNOWN"
            ),
            lambda a: a[3]["flags"].update({"flags.take": False}),
            lambda a: a[3]["flags"].update(
                {"upgradeclassification.upgrade_classification": True}
            ),
            lambda a: a[5].add(a[2]["identity"]["key"]),
            lambda a: a[0]["qualification"].update(
                forge={"classification": 2, "max_tier": 3}
            ),
        ]:
            args = self.arguments()
            change(args)
            before = copy.deepcopy(args)
            with self.assertRaises(ValueError):
                forge.qualify(*args)
            self.assertEqual(args, before)

    def test_scoped_sibling_tolerance_owner_conflict_and_idempotence(self):
        args = self.arguments()
        row, _ = forge.qualify(*args)
        args[2]["semantics"]["stack"] = {"state": "UNKNOWN"}
        args[2]["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": "independent successor source",
        }
        args[4] = [row | {"required_magic_level": 7}]
        self.assertEqual(forge.qualify(*args)[0], row)
        args[4][0]["forge"] = {"classification": 2, "max_tier": 3}
        with self.assertRaisesRegex(ValueError, "owner conflict"):
            forge.qualify(*args)

    def test_new454_does_not_replace_historical165_or_change3332(self):
        self.assertEqual(context.check()["status"], "PASS")
        original = json.loads(context.read_context()["old165_bindings"]["raw_utf8"])
        packet = json.loads(forge.OUTPUT.read_bytes())
        with tempfile.TemporaryDirectory(
            prefix="forge289-aggregate-", dir=forge.ROOT.parent
        ) as directory:
            root = Path(directory)
            for name in ("tools", "docs", "content"):
                (root / name).symlink_to(forge.ROOT / name, target_is_directory=True)
            context.hardlink_tree(
                forge.ROOT / "imports",
                root / "imports",
                {"tibiawiki/bindings/items.json"},
            )
            p = root / context.BINDINGS
            p.write_bytes(
                context.canonical(
                    original | {"bindings": original["bindings"] + packet["bindings"]}
                )
            )
            self.assertEqual(context.check(root)["status"], "PASS")
            broken = copy.deepcopy(original)
            broken["bindings"][0]["target"]["revision"] = "wrong"
            p.write_bytes(
                context.canonical(
                    broken | {"bindings": broken["bindings"] + packet["bindings"]}
                )
            )
            with self.assertRaises(ValueError):
                context.read_context(root)

    def test_formal_replay_never_writes_through_current_generated_outputs(self):
        # Simulate newer aggregate bindings and legitimate current outputs in an
        # entirely disposable source tree. A historical rebuild must not replace
        # any source-tree output, even when its bytes differ from old165 output.
        with tempfile.TemporaryDirectory(
            prefix="forge289-formal-isolation-", dir=forge.ROOT.parent
        ) as directory:
            root = Path(directory)
            shutil.copytree(
                forge.ROOT / "tools",
                root / "tools",
                ignore=shutil.ignore_patterns("__pycache__"),
            )
            for name in ("docs", "content"):
                (root / name).symlink_to(forge.ROOT / name, target_is_directory=True)
            context.hardlink_tree(
                forge.ROOT / "imports",
                root / "imports",
                {"tibiawiki/bindings/items.json"},
            )
            original = json.loads(context.read_context()["old165_bindings"]["raw_utf8"])
            packet = json.loads(forge.OUTPUT.read_bytes())
            (root / context.BINDINGS).write_bytes(
                context.canonical(
                    original | {"bindings": original["bindings"] + packet["bindings"]}
                )
            )
            subtree = root / "tools/content-schema/item-authoring"
            (subtree / "item.schema.json").write_text(
                '{"current_output_sentinel":true}\n'
            )
            (subtree / "formal-schema-validation-report.json").write_text(
                '{"current_report_sentinel":true}\n'
            )

            def bytes_inventory():
                return {
                    str(p.relative_to(subtree)): p.read_bytes()
                    for p in subtree.rglob("*")
                    if p.is_file() and "__pycache__" not in p.parts
                }

            before = bytes_inventory()
            self.assertEqual(context.check(root, cohort="formal")["status"], "PASS")
            self.assertEqual(bytes_inventory(), before)

    def test_weapon_replay_exact700_closure_rejects_unsealed_owners_atomically(self):
        packet = json.loads(forge.OUTPUT.read_bytes())
        proof = json.loads((forge.ROOT / forge.PROOF).read_bytes())
        with context.historical_context(forge.ROOT, weapon=True) as root:
            path = root / "content/world/definitions/declarations.json"
            original = json.loads(path.read_bytes())
            current = copy.deepcopy(original)
            current["item_authoring"] += packet["promotions"]
            path.write_bytes(context.canonical(current))
            before = path.read_bytes()
            self.assertEqual(
                context.current_weapon103(root)["current_owner_count"], 700
            )
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(
                current["item_authoring"][:411], proof["current_parent_authoring"]
            )
            for change in (
                lambda d: d["item_authoring"].append(d["item_authoring"][0]),
                lambda d: d["item_authoring"].pop(0),
                lambda d: d["item_authoring"].append(
                    {
                        "item": {
                            "family": "Item",
                            "key": "unsealed",
                            "revision": "definition-r1",
                        }
                    }
                ),
                lambda d: d["item_authoring"][699]["forge"].update(max_tier=99),
                lambda d: d["item_authoring"][699]["forge"].update(classification=True),
            ):
                bad = copy.deepcopy(current)
                change(bad)
                path.write_bytes(context.canonical(bad))
                sealed = path.read_bytes()
                with self.assertRaisesRegex(ValueError, "sealed411/700 closure"):
                    context.current_weapon103(root)
                self.assertEqual(path.read_bytes(), sealed)

    def test_closed700_owner_and_exact289_forge138_native_slot_relations(self):
        spec = importlib.util.spec_from_file_location(
            "forge289_migration",
            forge.ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py",
        )
        migration = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(migration)
        rows, parent, relations, sources = migration.closed_forge289()
        original = copy.deepcopy(parent)
        migration.extend_forge289_owners(parent)
        self.assertEqual(len(parent), 700)
        self.assertEqual({k: parent[k] for k in original}, original)
        aliases = migration.item_alias_targets()
        staged = {
            migration.staged_item_target(r["target"], aliases): r
            for r in json.loads(
                (
                    forge.ROOT
                    / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json"
                ).read_bytes()
            )["items"]
        }
        migration.validate_item_authoring_targets(parent, staged)
        key = next(iter(rows))
        for value in (True, 0, 256):
            bad = copy.deepcopy(parent)
            bad[key]["forge"]["max_tier"] = value
            with self.assertRaises(migration.ValidationError):
                migration.validate_item_authoring_targets(bad, staged)
        key = next(k for k, row in relations.items() if len(row["relations"]) == 2)
        definition = sources[key]["native_definition"]
        migration.validate_forge289_relation(
            relations[key], definition, relations[key], sources[key]
        )
        bad = copy.deepcopy(relations[key])
        bad["relations"][1]["basis"] = "classification"
        with self.assertRaises(migration.ValidationError):
            migration.validate_forge289_relation(
                bad, definition, relations[key], sources[key]
            )
        bad = copy.deepcopy(definition)
        bad["semantics"]["imbuement"]["value"]["slot_count"]["value"] = True
        with self.assertRaises(migration.ValidationError):
            migration.validate_forge289_relation(
                relations[key], bad, relations[key], sources[key]
            )

    def test_pre_overlay_header_drift_still_fails_closed(self):
        # D316: the guard reads the pre-overlay stage; a real header drift there must fail.
        original = forge.pre_overlay_definitions
        key = json.loads(forge.OUTPUT.read_text())["promotions"][0]["item"]["key"]

        def drifted(root):
            definitions = original(root)
            row = definitions[key]
            definitions[key] = row | {"materializable": not row["materializable"]}
            return definitions

        forge.pre_overlay_definitions = drifted
        try:
            with self.assertRaisesRegex(ValueError, "headers drift"):
                forge.build()
        finally:
            forge.pre_overlay_definitions = original


if __name__ == "__main__":
    unittest.main()
