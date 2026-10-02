"""Explicit pair source/cut and owner guards, independent of native document facts."""

import copy
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch

import lower_item_forge3332_packet as forge


class ForgeSources(unittest.TestCase):
    def migration(self):
        spec = importlib.util.spec_from_file_location(
            "forge_migration",
            forge.ROOT / "tools/content-migration/validate_world_project_v2_to_tree.py",
        )
        migration = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(migration)
        return migration

    def test_current_receipt_sealed(self):
        with self.assertRaisesRegex(ValueError, "source digest drift"):
            forge.checked(forge.ROOT, forge.RECEIPT, "0" * 64)

    def test_migration_accepts_only_closed_forge_owner_and_preserves_prior_cohorts(
        self,
    ):
        migration = self.migration()
        aliases = migration.item_alias_targets()
        staged = {
            migration.staged_item_target(r["target"], aliases): r
            for r in json.loads(
                (
                    forge.ROOT
                    / "docs/agents/evidence/OTV2-20260925-item-enrichment-wave1-staged.json"
                ).read_text()
            )["items"]
        }
        owners = {
            migration.target_id(r["item"]): r
            for r in json.loads(
                (forge.ROOT / "content/world/definitions/declarations.json").read_text()
            )["item_authoring"]
        }
        owner = json.loads(forge.OUTPUT.read_text())["promotion"]
        key = migration.target_id(owner["item"])
        owners[key] = owner
        migration.validate_item_authoring_targets(owners, staged)
        for path, value in [
            ("classification", 3),
            ("max_tier", 3),
            ("classification", True),
        ]:
            bad = copy.deepcopy(owners)
            bad[key]["forge"][path] = value
            with self.assertRaises(migration.ValidationError):
                migration.validate_item_authoring_targets(bad, staged)
        for bad in [
            owners | {("Item", "unqualified", "definition-r1"): {}},
            {k: v for k, v in owners.items() if k != key},
            owners | {key: owner | {"use_ability": "invented"}},
        ]:
            with self.assertRaises(migration.ValidationError):
                migration.validate_item_authoring_targets(bad, staged)
        read_bytes = Path.read_bytes
        with (
            patch(
                "pathlib.Path.read_bytes",
                lambda p: b"forged" if p.name == forge.OUTPUT.name else read_bytes(p),
            ),
            self.assertRaisesRegex(migration.ValidationError, "FORGE_PACKET_DIGEST"),
        ):
            migration.validate_item_authoring_targets(owners, staged)

    def test_migration_requires_exact_forge_and_existing_native_slot_relations(self):
        migration = self.migration()
        receipt = json.loads((forge.ROOT / forge.RECEIPT).read_text())
        row, definition = receipt["expected_relation"], receipt["current_definition"]
        migration.validate_forge_relation(row, definition)
        for change in [
            lambda r: r["relations"].pop(),
            lambda r: r["relations"][0].update(basis="classification"),
            lambda r: r.update(extra="unqualified"),
            lambda r: r["relations"][0].update(ruleset="rulesets/items/enchanting/"),
            lambda r: r["relations"].append(r["relations"][0]),
            lambda r: r["source"].update(revision="wrong"),
        ]:
            bad = copy.deepcopy(row)
            change(bad)
            with self.assertRaises(migration.ValidationError):
                migration.validate_forge_relation(bad, definition)
        bad = copy.deepcopy(definition)
        bad["semantics"]["imbuement"] = {"state": "UNKNOWN"}
        with self.assertRaises(migration.ValidationError):
            migration.validate_forge_relation(row, bad)
        for value in (True, 0):
            bad = copy.deepcopy(definition)
            bad["semantics"]["imbuement"]["value"]["slot_count"]["value"] = value
            with self.assertRaises(migration.ValidationError):
                migration.validate_forge_relation(row, bad)

    def args(self):
        proof = json.loads((forge.ROOT / forge.PROOF).read_text())
        record = proof["current_native_identity"]["retained_current_record"]
        definition = {"identity": record["identity"], "semantics": record["semantics"]}
        return [
            proof,
            copy.deepcopy(proof["source_pair"]["retained_source_row"]),
            definition,
            {"id": 3332, "name": "hammer of wrath", "flags": {"flags.take": True}},
            [],
            set(),
        ]

    def test_separate_current_cut_and_idempotent_existing_owner(self):
        args = self.args()
        row = forge.qualify(*args)
        self.assertEqual(row["forge"], {"classification": 2, "max_tier": 2})
        args[4] = [row | {"required_magic_level": 7}]
        self.assertEqual(forge.qualify(*args), row)
        args[0]["current_selector"]["qualification_cutoff"] = "2026-07-28T23:59:59Z"
        with self.assertRaisesRegex(ValueError, "cutoff"):
            forge.qualify(*args)

    def test_source_and_current_target_owner_fail_closed(self):
        def source_pair(a):
            a[1]["fields"]["max_tier"] = {"state": "ABSENT"}

        def native_name(a):
            a[2]["semantics"]["presentation"]["value"]["name"] = {"state": "UNKNOWN"}

        def conflict(a):
            a[4] = [
                {"item": a[0]["target"], "forge": {"classification": 2, "max_tier": 3}}
            ]

        for change in [
            source_pair,
            native_name,
            conflict,
            lambda a: a[1].update(revision_id=1),
            lambda a: a[2]["identity"].update(revision="other"),
            lambda a: a[3]["flags"].update({"flags.take": False}),
            lambda a: a[5].add(a[0]["target"]["key"]),
        ]:
            args = self.args()
            change(args)
            before = copy.deepcopy(args)
            with self.assertRaises(ValueError):
                forge.qualify(*args)
            self.assertEqual(args, before)

    def test_proof_and_input_digest_drift(self):
        with self.assertRaisesRegex(ValueError, "source digest drift"):
            forge.checked(forge.ROOT, forge.PROOF, "0" * 64)
        proof = json.loads((forge.ROOT / forge.PROOF).read_text())
        with self.assertRaisesRegex(ValueError, "source digest drift"):
            forge.checked(forge.ROOT, proof["source_pair"]["artifact_path"], "0" * 64)

    def test_forward_reverse_binding_uniqueness(self):
        proof = self.args()[0]
        row = proof["identity_bridge"]["br_to_full_native"]["row"]
        target = proof["target"]
        self.assertEqual(forge.exact_binding([row], row, target), row)
        other = copy.deepcopy(row)
        other["target"]["key"] = "oteryn:item.tibia.i3333"
        for bindings in ([row, other], [row, row], [other]):
            with self.assertRaisesRegex(ValueError, "binding drift"):
                forge.exact_binding(bindings, row, target)
        alias = row | {"disposition": "ACCEPTED_ALIAS"}
        with self.assertRaisesRegex(ValueError, "binding drift"):
            forge.exact_binding([alias], alias, target)


if __name__ == "__main__":
    unittest.main()
