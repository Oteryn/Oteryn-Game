"""Adversarial checks for identity-only projection and provenance isolation."""
import copy
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
import unittest

from current_spell_adapter import attach_source, canonical_bytes, project_candidate, project_local_candidate, project_supported_candidate
from validate_spell import structural


class CurrentSpellAdapterTests(unittest.TestCase):
    def setUp(self):
        catalog = json.loads((Path(__file__).parent / "samples/executable-spell-catalog.json").read_text())
        self.entry = next(row for row in catalog["bundles"] if row["bundle"]["spell"]["name"] == "Cancel Magic Shield")
        self.target = copy.deepcopy(self.entry["bundle"]["spell"]["identity"])
        self.bundle = copy.deepcopy(self.entry["bundle"])
        self.bundle["spell"]["identity"] = {"key": "candidate:spell/source/test", "revision": "source-r58"}
        self.deps = copy.deepcopy(self.entry["dependencies"])
        self.proof = {"source_id": "canary-main-current", "repository": "opentibiabr/canary", "branch": "main",
                      "revision": "04b83b512114bfd888000d6e1433ed8ecaec7c5b",
                      "path": "data/scripts/spells/support/cancel_magic_shield.lua", "sha256": "a" * 64}

    def test_identity_only_projection_does_not_mutate_inputs(self):
        original = copy.deepcopy(self.bundle)
        self.assertTrue(project_candidate(self.bundle, self.deps, self.entry, self.target))
        self.assertEqual(self.bundle, original)

    def test_mechanic_changes_are_rejected(self):
        for key, value in (("costs", {"mana": 999, "soul": 0}), ("targeting", {}),
                           ("execution", {"native_behavior": {"key": "stance_toggle", "parameters": {}}})):
            with self.subTest(key=key):
                altered = copy.deepcopy(self.bundle)
                altered["spell"][key] = value
                with self.assertRaisesRegex(ValueError, "FULL_HEADER_MISMATCH"):
                    project_candidate(altered, self.deps, self.entry, self.target)
        altered = copy.deepcopy(self.deps)
        altered["formulas"].append({"kind": "player_expression", "maximum": {"const": "999"}})
        with self.assertRaisesRegex(ValueError, "DEPENDENCIES_MISMATCH"):
            project_candidate(self.bundle, altered, self.entry, self.target)

    def test_explicit_identity_and_exact_source_header_are_required(self):
        for target in (None, {}, {**self.target, "key": "another"}):
            with self.subTest(target=target), self.assertRaises(ValueError):
                project_candidate(self.bundle, self.deps, self.entry, target)
        for field, value in (("name", "cancel magic shield"), ("carrier", "rune"), ("reference_spell_id", -1)):
            altered = copy.deepcopy(self.bundle)
            altered["spell"][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "SOURCE_HEADER_MISMATCH"):
                project_candidate(altered, self.deps, self.entry, self.target)
        altered = copy.deepcopy(self.entry)
        altered["source_identities"][0]["identity"]["key"] = "another"
        with self.assertRaisesRegex(ValueError, "AMBIGUOUS_CURRENT_IDENTITY"):
            project_candidate(self.bundle, self.deps, altered, self.target)

    def test_equal_python_numbers_cannot_change_json_types(self):
        for value in (False, 0.0):
            altered = copy.deepcopy(self.bundle)
            altered["spell"]["costs"]["soul"] = value
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, "FULL_HEADER_MISMATCH"):
                project_candidate(altered, self.deps, self.entry, self.target)

    def test_local_identity_mapping_keeps_the_full_graph(self):
        bundle, deps = copy.deepcopy(self.bundle), copy.deepcopy(self.deps)
        replacements = {}
        for section in deps.values():
            for index, record in enumerate(section):
                old = record["identity"]
                replacements[old["key"]] = {"key": "local/" + old["key"], "revision": "source-r55"}
        def local(value):
            if isinstance(value, list):
                return [local(child) for child in value]
            if not isinstance(value, dict):
                return value
            result = {key: local(child) for key, child in value.items()}
            if set(value) in ({"key", "revision"}, {"family", "key", "revision"}) and value["key"] in replacements:
                result.update(replacements[value["key"]])
            return result
        bundle, deps = local(bundle), local(deps)
        bundle["spell"].pop("library_text", None)
        self.assertTrue(project_local_candidate(bundle, deps, self.entry, self.target))
        bundle["spell"]["library_text"] = "source disagrees"
        with self.assertRaisesRegex(ValueError, "FULL_HEADER_MISMATCH"):
            project_local_candidate(bundle, deps, self.entry, self.target)
        bundle["spell"].pop("library_text")
        deps["effects"].reverse()
        with self.assertRaisesRegex(ValueError, "DEPENDENCIES_MISMATCH|FULL_HEADER_MISMATCH"):
            project_local_candidate(bundle, deps, self.entry, self.target)
        deps["effects"][1]["identity"] = copy.deepcopy(deps["effects"][0]["identity"])
        with self.assertRaisesRegex(ValueError, "AMBIGUOUS_DEPENDENCY_IDENTITY"):
            project_local_candidate(bundle, deps, self.entry, self.target)

    def test_local_projection_refuses_native_replacement_and_extra_operations(self):
        entry = copy.deepcopy(self.entry)
        entry["bundle"]["spell"]["execution"] = {"native_behavior": {"key": "wheel_combat", "parameters": {}}}
        with self.assertRaisesRegex(ValueError, "NATIVE_PROFILE_REPLACEMENT_HELD"):
            project_local_candidate(self.bundle, self.deps, entry, self.target)
        deps = copy.deepcopy(self.deps)
        deps["effects"].append(copy.deepcopy(deps["effects"][0]))
        with self.assertRaisesRegex(ValueError, "DEPENDENCY_COUNT_MISMATCH"):
            project_local_candidate(self.bundle, deps, self.entry, self.target)
        deps = copy.deepcopy(self.deps)
        deps["abilities"][0]["range_tiles"] = 0.0
        with self.assertRaisesRegex(ValueError, "DEPENDENCIES_MISMATCH"):
            project_local_candidate(self.bundle, deps, self.entry, self.target)
        entry = copy.deepcopy(self.entry)
        entry["bundle"]["spell"]["base_power"] = 50
        self.assertTrue(project_local_candidate(self.bundle, self.deps, entry, self.target))
        bundle = copy.deepcopy(self.bundle)
        bundle["spell"]["base_power"] = 51
        with self.assertRaisesRegex(ValueError, "FULL_HEADER_MISMATCH"):
            project_local_candidate(bundle, self.deps, entry, self.target)

    def test_supported_projection_accepts_changed_payload_without_mutation(self):
        bundle = copy.deepcopy(self.bundle)
        bundle["spell"]["costs"]["mana"] = 75
        bundle["spell"].pop("library_text")
        entry = copy.deepcopy(self.entry)
        entry["bundle"]["spell"]["base_power"] = 50
        source_deps = copy.deepcopy(self.deps)
        source_deps["effects"][0]["presentation"]["impact_asset_binding"] = "canary.appearance:effect/magic_red"
        original = copy.deepcopy(bundle)
        original_deps = copy.deepcopy(source_deps)
        projected, deps = project_supported_candidate(bundle, source_deps, entry, self.target)
        self.assertEqual(projected["spell"]["costs"]["mana"], 75)
        self.assertEqual(projected["spell"]["identity"], self.target)
        self.assertEqual(projected["spell"]["base_power"], 50)
        self.assertEqual(projected["spell"]["library_text"], self.entry["bundle"]["spell"]["library_text"])
        self.assertEqual(bundle, original)
        self.assertEqual(deps, source_deps)
        self.assertEqual(source_deps, original_deps)
        self.assertNotEqual(deps, self.deps)

    def test_supported_projection_rewrites_local_but_preserves_external_refs(self):
        bundle, deps, entry = copy.deepcopy(self.bundle), copy.deepcopy(self.deps), copy.deepcopy(self.entry)
        old = deps["abilities"][0]["identity"]
        local = {"key": "source:ability/local", "revision": "source-1"}
        deps["abilities"][0]["identity"] = local
        bundle["spell"]["execution"]["ability"] = {"family": "Ability", **local}
        external = {"family": "Effect", "key": "external:effect/presentation", "revision": "external-1"}
        deps["abilities"][0]["effects"].append(external)
        entry["catalog"]["definitions"].append(external)
        projected, result = project_supported_candidate(bundle, deps, entry, self.target)
        self.assertEqual(result["abilities"][0]["identity"], old)
        self.assertEqual(result["abilities"][0]["effects"][-1], external)
        self.assertEqual(projected["spell"]["execution"]["ability"], {"family": "Ability", **old})
        entry["catalog"]["definitions"].clear()
        with self.assertRaisesRegex(ValueError, "unresolved exact definition"):
            project_supported_candidate(bundle, deps, entry, self.target)

    def test_supported_projection_preserves_canonical_effect_roles_in_source_order(self):
        bundle, deps = copy.deepcopy(self.bundle), copy.deepcopy(self.deps)
        self.assertGreaterEqual(len(deps["effects"]), 2)
        deps["effects"].reverse()
        deps["abilities"][0]["effects"].reverse()
        projected, result = project_supported_candidate(bundle, deps, self.entry, self.target)
        self.assertEqual(result, deps)
        self.assertEqual(projected["spell"]["execution"], bundle["spell"]["execution"])
        for effect in result["effects"]:
            baseline = next(row for row in self.deps["effects"] if row["identity"] == effect["identity"])
            self.assertEqual(effect, baseline)
        result["effects"][1]["identity"] = copy.deepcopy(result["effects"][0]["identity"])
        with self.assertRaisesRegex(ValueError, "AMBIGUOUS_DEPENDENCY_IDENTITY"):
            project_supported_candidate(bundle, result, self.entry, self.target)

    def test_supported_projection_fails_closed_for_unknown_and_incomplete_fields(self):
        bundle = copy.deepcopy(self.bundle)
        bundle["spell"]["unknown_source_metadata"] = True
        with self.assertRaisesRegex(ValueError, "PROJECTED_V1_INVALID"):
            project_supported_candidate(bundle, self.deps, self.entry, self.target)
        self.assertIn("unknown_source_metadata", bundle["spell"])
        bundle = copy.deepcopy(self.bundle)
        del bundle["spell"]["targeting"]
        with self.assertRaisesRegex(ValueError, "PROJECTED_V1_INVALID"):
            project_supported_candidate(bundle, self.deps, self.entry, self.target)
        deps = copy.deepcopy(self.deps)
        deps["effects"][0]["source_callback"] = "unconsumed"
        with self.assertRaisesRegex(ValueError, "PROJECTED_V1_INVALID"):
            project_supported_candidate(self.bundle, deps, self.entry, self.target)

    def test_supported_projection_holds_native_wheel_and_source_header_changes(self):
        for key in ("wheel_combat", "pvp_safe_item"):
            for side in ("candidate", "current"):
                bundle, entry = copy.deepcopy(self.bundle), copy.deepcopy(self.entry)
                spell = bundle["spell"] if side == "candidate" else entry["bundle"]["spell"]
                spell["execution"] = {"native_behavior": {"key": key, "parameters": {}}}
                with self.subTest(key=key, side=side), self.assertRaisesRegex(ValueError, "NATIVE_PROFILE"):
                    project_supported_candidate(bundle, self.deps, entry, self.target)
        bundle = copy.deepcopy(self.bundle)
        bundle["spell"]["requirements"]["wheel_unlock"] = True
        with self.assertRaisesRegex(ValueError, "WHEEL_UNLOCK"):
            project_supported_candidate(bundle, self.deps, self.entry, self.target)
        bundle = copy.deepcopy(self.bundle)
        bundle["spell"]["reference_spell_id"] += 1
        with self.assertRaisesRegex(ValueError, "SOURCE_HEADER_MISMATCH"):
            project_supported_candidate(bundle, self.deps, self.entry, self.target)

    def test_pvp_safe_item_on_either_dependency_side_is_held(self):
        for side in ("candidate", "current"):
            deps, entry = copy.deepcopy(self.deps), copy.deepcopy(self.entry)
            effects = deps["effects"] if side == "candidate" else entry["dependencies"]["effects"]
            effects[0]["pvp_safe_item"] = {"source": "native"}
            with self.subTest(side=side), self.assertRaisesRegex(ValueError, "NATIVE_PVP_SAFE_ITEM"):
                project_supported_candidate(self.bundle, deps, entry, self.target)
        deps = copy.deepcopy(self.deps)
        deps["effects"][0]["pvp_safe_item"] = None
        with self.assertRaisesRegex(ValueError, "PROJECTED_V1_INVALID"):
            project_supported_candidate(self.bundle, deps, self.entry, self.target)

    def test_external_file_import_does_not_require_tool_directory_on_sys_path(self):
        adapter_path = Path(__file__).with_name("current_spell_adapter.py").resolve()
        program = """import importlib.util,json,sys
original=list(sys.path)
spec=importlib.util.spec_from_file_location('external_adapter',sys.argv[1])
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
bundle,deps,entry,target=json.load(sys.stdin)
projected,_=module.project_supported_candidate(bundle,deps,entry,target)
assert projected['spell']['identity']==target
assert sys.path==original
"""
        result = subprocess.run([sys.executable, "-c", program, str(adapter_path)], cwd=tempfile.gettempdir(),
                                input=json.dumps([self.bundle, self.deps, self.entry, self.target]),
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_source_attachment_preserves_payload_and_binds_manifest(self):
        original = copy.deepcopy(self.entry)
        result = attach_source(self.entry, self.proof)
        self.assertEqual(self.entry, original)
        self.assertEqual(result["bundle"], self.entry["bundle"])
        self.assertEqual(result["dependencies"], self.entry["dependencies"])
        self.assertEqual(structural("monster-import-readiness.schema.json", result["manifest"]), [])
        for row in result["source_identities"]:
            self.assertEqual(row["sources"], result["manifest"]["sources"])
            self.assertEqual(row["manifest_sha256"], hashlib.sha256(canonical_bytes(result["manifest"])).hexdigest())
        self.assertEqual(attach_source(result, self.proof), result)

    def test_untrusted_source_proofs_are_rejected(self):
        for field, value in (("source_id", "unknown"), ("repository", "https://user:secret@github.com/opentibiabr/canary"),
                             ("branch", "other"), ("revision", "0" * 40), ("sha256", "bad"),
                             ("path", "data/scripts/../private.lua"), ("path", "https://host/token.lua")):
            altered = {**self.proof, field: value}
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                attach_source(self.entry, altered)
        with self.assertRaisesRegex(ValueError, "SOURCE_PROOF_SHAPE"):
            attach_source(self.entry, {**self.proof, "url": "https://host/?token=secret"})


if __name__ == "__main__":
    unittest.main()
