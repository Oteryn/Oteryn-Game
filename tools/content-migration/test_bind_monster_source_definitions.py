"""Source membership comes from an independently pinned definition closure."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from bind_monster_source_definitions import bind_attack_definitions, bind_definitions, definition_digest, canonical, CREATURES, MANIFEST

class DefinitionBindingTests(unittest.TestCase):
    def fixture(self, root):
        values = {
            "definitions/reference.json": {"records": [{"identity": "rat", "health": 20}]},
            "definitions/declarations.json": {"authoring_profiles": [{"attack": 10}], "records": [{"encounter": "rat"}]},
            "provenance/sources.json": {"sources": [{"revision": "pinned"}], "source_identity_bindings": [{"target": "rat"}]},
        }
        for name, value in values.items():
            path = root / "content/world" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(canonical(value))
        return values

    def test_binds_exact_definitions_and_updates_independent_native_input_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            original = {CREATURES: canonical({"records": [{"profile": "original"}]}), MANIFEST: canonical({"creature_profiles": {"path": "original", "sha256": "old"}})}
            before = copy.deepcopy(original)
            bound = bind_definitions(root, original)
            self.assertEqual(original, before)
            self.assertEqual(json.loads(bound[CREATURES])["records"], json.loads(original[CREATURES])["records"])
            self.assertEqual(json.loads(bound[CREATURES])["source_definitions_sha256"], definition_digest(root))
            self.assertEqual(json.loads(bound[MANIFEST])["creature_profiles"]["sha256"], hashlib.sha256(bound[CREATURES]).hexdigest())

    def test_every_native_creature_and_attack_closure_emits_reference_level(self):
        with tempfile.TemporaryDirectory(dir=Path.cwd()) as directory:
            root = Path(directory)
            self.fixture(root)
            creature = {"kind": "Creature", "profile": {"details": {"display_name": "rat"}}}
            original = {CREATURES: canonical({"records": [{"profile": {"data": creature},
                "attack_definitions": {"profiles": [{"data": creature}]}}]}),
                MANIFEST: canonical({"creature_profiles": {"sha256": "old"}})}
            records = json.loads(bind_definitions(root, original)[CREATURES])["records"]
            self.assertIsNone(records[0]["profile"]["data"]["profile"]["details"]["reference_level"])
            self.assertIsNone(records[0]["attack_definitions"]["profiles"][0]["data"]["profile"]["details"]["reference_level"])

    def test_actual_admitted_monsters_carry_source_attack_closures(self):
        # W01c: the serving provider must carry executable bodies, not only the
        # existing three Summer examples or unattached Ability references.
        root = Path(__file__).resolve().parents[2]
        original = {CREATURES: (root / CREATURES).read_bytes(),
                    MANIFEST: (root / MANIFEST).read_bytes()}
        bound = bind_definitions(root, original)
        before = json.loads(original[CREATURES])["records"]
        after = json.loads(bound[CREATURES])["records"]
        canonical_records = json.loads(
            (root / "content/world/definitions/reference.json").read_bytes())["records"]
        canonical_keys = {row["identity"]["key"] for row in canonical_records
                          if row["identity"]["family"] == "Creature"}
        by_key = {row["profile"]["target"]["key"]: row for row in after}
        for row in before:
            key = row["profile"]["target"]["key"]
            current = by_key[key]
            self.assertEqual(current["profile"], row["profile"])
            self.assertEqual(current.get("behavior"), row.get("behavior"))
            if key not in canonical_keys or not row.get("behavior"):
                continue
            with self.subTest(creature=key):
                closure = current.get("attack_definitions")
                self.assertIsNotNone(closure, "canonical attack bodies must reach the native provider")
                self.assertIn(current["profile"], closure["profiles"])
                self.assertIn(current["behavior"], closure["profiles"])
                self.assertEqual(len(closure["records"]), len(closure["profiles"]))
                self.assertLessEqual(len(closure["records"]), 128)
                refs = {tuple(p["target"][k] for k in ("family", "key", "revision"))
                        for p in closure["profiles"]}
                for attack in row["behavior"]["data"]["profile"].get("attacks", []):
                    self.assertIn(tuple(attack["ability"][k]
                                        for k in ("family", "key", "revision")), refs)
        rat = by_key["oteryn:creature.rat"]["attack_definitions"]
        rat_effect = next(p for p in rat["profiles"]
                          if p["target"]["key"] == "oteryn:effect.creature.rat.attack-1")
        self.assertEqual(rat_effect["target"]["family"], "Effect")

    def typed_attack_fixture(self, root):
        def ref(family, key):
            return {"family": family, "key": key, "revision": "source-r1"}
        creature, behavior, ability, effect = (
            ref("Creature", "creature"), ref("Behavior", "behavior"),
            ref("Ability", "ability"), ref("Effect", "effect"),
        )
        profiles = [
            {"target": creature, "data": {"kind": "Creature",
                "profile": {"details": {}, "abilities": [ability]}}},
            {"target": behavior, "data": {"kind": "Behavior",
                "profile": {"attacks": [{"ability": ability}]}}},
            {"target": ability, "data": {"effect": effect}},
            {"target": effect, "data": {"damage": 8}},
        ]
        records = [{"identity": value} for value in (creature, behavior, ability, effect)]
        for name, value in (
            ("reference.json", {"records": records}),
            ("declarations.json", {"authoring_profiles": profiles}),
        ):
            path = root / "content/world/definitions" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(canonical(value))
        return {"records": [{"profile": profiles[0], "behavior": profiles[1]}]}

    def test_missing_source_body_refuses_instead_of_admitting_a_partial_attack(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            native = self.typed_attack_fixture(root)
            path = root / "content/world/definitions/declarations.json"
            declarations = json.loads(path.read_bytes())
            declarations["authoring_profiles"].pop()
            path.write_bytes(canonical(declarations))
            before = copy.deepcopy(native)
            with self.assertRaisesRegex(ValueError, "missing canonical monster attack body"):
                bind_attack_definitions(root, native)
            self.assertEqual(native, before)

    def test_duplicate_and_incomplete_canonical_roots_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            native = self.typed_attack_fixture(root)
            path = root / "content/world/definitions/declarations.json"
            original = json.loads(path.read_bytes())
            altered = copy.deepcopy(original)
            altered["authoring_profiles"].append(altered["authoring_profiles"][0])
            path.write_bytes(canonical(altered))
            with self.assertRaisesRegex(ValueError, "duplicate canonical"):
                bind_attack_definitions(root, native)
            altered = copy.deepcopy(original)
            altered["authoring_profiles"].pop(0)
            path.write_bytes(canonical(altered))
            with self.assertRaisesRegex(ValueError, "incomplete canonical"):
                bind_attack_definitions(root, native)

    def test_each_definition_or_provenance_substitution_changes_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = self.fixture(root)
            expected = definition_digest(root)
            for name, original in values.items():
                for field in original:
                    with self.subTest(document=name, field=field):
                        altered = copy.deepcopy(original)
                        altered[field].append({"substitution": "detached"})
                        path = root / "content/world" / name
                        path.write_bytes(canonical(altered))
                        self.assertNotEqual(definition_digest(root), expected)
                        path.write_bytes(canonical(original))
            self.assertEqual(definition_digest(root), expected)

if __name__ == "__main__":
    unittest.main()
