"""Primal explicit-empty source admission stays narrow and source-fenced."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import primal_reference_overlay as overlay


class PrimalReferenceTests(unittest.TestCase):
    def records(self):
        source = json.loads((overlay.ROOT / "content/world/definitions/reference.json").read_bytes())["records"]
        return copy.deepcopy([row for row in source if row["identity"]["key"] in {overlay.CREATURE, overlay.BODY}])

    def test_only_exact_primal_gets_explicit_empty_table_and_replay_is_idempotent(self):
        records = self.records()
        other = {"kind": "Creature", "identity": overlay.reference("Creature", "oteryn:creature.other"), "loot": None}
        records.append(other)
        original_body = copy.deepcopy(next(r for r in records if r["identity"]["key"] == overlay.BODY))
        overlay.apply_reference(records)
        expected = copy.deepcopy(records)
        overlay.apply_reference(records)
        self.assertEqual(records, expected)
        self.assertIsNone(other["loot"])
        self.assertEqual(next(r for r in records if r["identity"]["key"] == overlay.BODY), original_body)
        loot = next(r for r in records if r["identity"]["key"] == overlay.LOOT)
        self.assertEqual(loot["entries"], [])
        self.assertEqual(loot["algorithm"], "IndependentBernoulliPpm")
        self.assertEqual(next(r for r in records if r["identity"]["key"] == overlay.CREATURE)["loot"], loot["identity"])

    def test_changed_creature_body_binding_or_nonempty_existing_table_refuses(self):
        for target in [overlay.CREATURE, overlay.BODY]:
            with self.subTest(target=target):
                records = self.records()
                next(r for r in records if r["identity"]["key"] == target)["identity"]["revision"] = "unqualified"
                with self.assertRaisesRegex(ValueError, "identity changed"):
                    overlay.apply_reference(records)
        records = self.records()
        overlay.apply_reference(records)
        next(r for r in records if r["identity"]["key"] == overlay.LOOT)["entries"] = [{"item": overlay.reference("Item", overlay.BODY)}]
        with self.assertRaisesRegex(ValueError, "explicit-empty loot changed"):
            overlay.apply_reference(records)
        records = self.records()
        next(r for r in records if r["identity"]["key"] == overlay.CREATURE)["loot"] = overlay.reference("Loot", "oteryn:loot.other")
        with self.assertRaisesRegex(ValueError, "binding changed"):
            overlay.apply_reference(records)

    def test_mutated_witness_fails_before_any_reference_write(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            path = root / overlay.CAPTURE
            path.parent.mkdir(parents=True)
            witness = json.loads((overlay.ROOT / overlay.CAPTURE).read_bytes())
            witness["loot"]["entries"] = [{"invented": True}]
            path.write_bytes(overlay.canonical(witness))
            records = self.records()
            original = copy.deepcopy(records)
            with self.assertRaisesRegex(ValueError, "witness digest changed"):
                overlay.apply_reference(records, root)
            self.assertEqual(records, original)


    def native(self):
        document = json.loads((overlay.ROOT / overlay.NATIVE_LOOT).read_bytes())
        # Exercise migration from the frozen provider as well as idempotent re-entry.
        next(row for row in document["creature_loot"] if row["creature"]["key"] == overlay.CREATURE)["loot"] = None
        document["tables"] = [row for row in document["tables"] if row["identity"]["key"] != overlay.LOOT]
        return document

    def generated_native(self, document):
        return {overlay.NATIVE_LOOT: overlay.canonical(document),
                "manifest": overlay.canonical({"loot_tables": {"path": "retained", "sha256": "old"}, "unrelated": 19}),
                "unrelated": b"unchanged-provider-bytes"}

    def test_native_projection_preserves_every_unrelated_table_binding_and_item(self):
        original = self.native()
        generated = self.generated_native(original)
        overlay.apply_native(overlay.ROOT, generated, "manifest", overlay.canonical)
        after = json.loads(generated[overlay.NATIVE_LOOT])
        self.assertEqual(after["items"], original["items"])
        self.assertEqual([overlay.canonical(row) for row in after["tables"] if row["identity"]["key"] != overlay.LOOT],
                         [overlay.canonical(row) for row in original["tables"]])
        self.assertEqual([overlay.canonical(row) for row in after["creature_loot"] if row["creature"]["key"] != overlay.CREATURE],
                         [overlay.canonical(row) for row in original["creature_loot"] if row["creature"]["key"] != overlay.CREATURE])
        table, = [row for row in after["tables"] if row["identity"]["key"] == overlay.LOOT]
        self.assertEqual(table, {"identity": overlay.reference("Loot", overlay.LOOT), "algorithm": "IndependentBernoulliPpm", "entries": []})
        target, = [row for row in after["creature_loot"] if row["creature"]["key"] == overlay.CREATURE]
        self.assertEqual(target["loot"], table["identity"])
        manifest = json.loads(generated["manifest"])
        self.assertEqual(manifest["loot_tables"], {"path": "retained", "sha256": overlay.hashlib.sha256(generated[overlay.NATIVE_LOOT]).hexdigest()})
        self.assertEqual(manifest["unrelated"], 19)
        self.assertEqual(generated["unrelated"], b"unchanged-provider-bytes")
        first = copy.deepcopy(generated)
        overlay.apply_native(overlay.ROOT, generated, "manifest", overlay.canonical)
        self.assertEqual(generated, first)

    def test_native_projection_rejects_conflicting_reference_table_and_corpse_atomically(self):
        for mutate, error in [
            (lambda doc: next(row for row in doc["creature_loot"] if row["creature"]["key"] == overlay.CREATURE).update(loot=overlay.reference("Loot", "oteryn:loot.foreign")), "binding changed"),
            (lambda doc: doc["tables"].append({"identity": overlay.reference("Loot", overlay.LOOT), "algorithm": "IndependentBernoulliPpm", "entries": [{"invented": True}]}), "explicit-empty loot changed"),
            (lambda doc: next(row for row in doc["items"] if row["item"]["key"] == overlay.BODY).update(materializable=False), "corpse closure changed"),
            (lambda doc: doc["creature_loot"].append(copy.deepcopy(next(row for row in doc["creature_loot"] if row["creature"]["key"] == overlay.CREATURE))), "binding changed"),
        ]:
            document = self.native()
            mutate(document)
            generated = self.generated_native(document)
            original = copy.deepcopy(generated)
            with self.assertRaisesRegex(ValueError, error):
                overlay.apply_native(overlay.ROOT, generated, "manifest", overlay.canonical)
            self.assertEqual(generated, original)

    def test_native_projection_admits_provider_before_repository_input_pinning(self):
        generated = self.generated_native(self.native())
        generated["manifest"] = overlay.canonical({"unrelated": 19})
        overlay.apply_native(overlay.ROOT, generated, "manifest", overlay.canonical)
        manifest = json.loads(generated["manifest"])
        self.assertEqual(manifest["unrelated"], 19)
        self.assertEqual(manifest["loot_tables"], {
            "path": Path(overlay.NATIVE_LOOT).name,
            "sha256": overlay.hashlib.sha256(generated[overlay.NATIVE_LOOT]).hexdigest(),
        })

    def test_native_projection_requires_pinned_witness_before_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / overlay.CAPTURE
            path.parent.mkdir(parents=True)
            document = json.loads((overlay.ROOT / overlay.CAPTURE).read_bytes())
            document["loot"]["entries"] = [{"invented": True}]
            path.write_bytes(overlay.canonical(document))
            generated = self.generated_native(self.native())
            original = copy.deepcopy(generated)
            with self.assertRaisesRegex(ValueError, "witness digest changed"):
                overlay.apply_native(root, generated, "manifest", overlay.canonical)
            self.assertEqual(generated, original)


if __name__ == "__main__":
    unittest.main()
