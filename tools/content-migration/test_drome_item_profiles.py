"""Bounded source-capture/admission regressions; fixtures are not donor evidence."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import drome_item_profiles as drome


def known(value):
    return {"state": "KNOWN", "value": value}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


class DromeItemProfilesTests(unittest.TestCase):
    def setUp(self):
        self.bindings = {}
        self.records = []
        for item, key in zip(drome.ITEM_IDS, drome.KEYS):
            identity = {"family": "Item", "key": key, "revision": "definition-r1"}
            binding = {"target": identity, "disposition": "EXACT", "external_id": str(item)}
            self.bindings[str(item)] = binding
            self.records.append({
                "production_definition": {"family": "Item", "production_key": key, "revision_ref": "definition-r1"},
                "production_binding": copy.deepcopy(binding),
                "production_binding_qualification": None,
                "admission": {"materializable": True, "stack_class": "StackCapable",
                              "legal_destinations": ["CharacterInventory", "Ground"]},
                "semantics": {
                    "physical": known({"movable": known(True), "pickupable": known(True), "weight": known(180)}),
                    "stack": known({"stackable": known(True), "stack_max": known(100)}),
                    "container": copy.deepcopy(drome.NA), "temporal": copy.deepcopy(drome.NA),
                    "presentation": known({"name": known(f"fixture item {item}")}),
                },
                "attributes": {"speed_bonus": 0},
            })

    def overlay(self):
        return {
            "schema": drome.SCHEMA, "source_revision": drome.SOURCE_PIN,
            "identity_binding_sha256": drome.BINDINGS_SHA,
            "sources": [{"path": path, "git_blob": blob, "sha256": "a" * 64}
                        for path, blob in drome.SOURCE_BLOBS.items()],
            "records": copy.deepcopy(self.records),
            "records_sha256": hashlib.sha256(canonical(self.records)).hexdigest(),
        }

    def generated(self, records=()):
        return {"profiles.json": canonical({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": list(records)}),
                "manifest.json": canonical({"item_profiles": {"path": "profiles.json", "sha256": "old"}})}

    def admit(self, generated):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = canonical({"bindings": list(self.bindings.values())})
            # Only the fixture digest is substituted. Production always requires the
            # repository's pinned binding table; this is not real capture qualification.
            with patch.object(drome, "BINDINGS_SHA", hashlib.sha256(raw).hexdigest()):
                for path, data in ((drome.BINDINGS_PATH, raw), (drome.PATH, canonical(self.overlay()))):
                    destination = root / path
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(data)
                drome.admit(root, generated, "profiles.json", "manifest.json",
                            lambda row: int(row["production_definition"]["production_key"].rsplit("i", 1)[1]), canonical)

    def test_all_twenty_rewards_and_cube_are_registered_pinned_and_idempotent(self):
        generated = self.generated()
        self.admit(generated)
        rows = json.loads(generated["profiles.json"])["records"]
        self.assertEqual(rows, self.records)
        self.assertEqual(len(rows), 21)
        self.assertEqual(json.loads(generated["manifest.json"])["item_profiles"]["sha256"],
                         hashlib.sha256(generated["profiles.json"]).hexdigest())
        before = copy.deepcopy(generated)
        self.admit(generated)
        self.assertEqual(generated, before)

    def test_missing_duplicate_extra_and_reordered_records_are_refused(self):
        for rows in (self.records[:-1], self.records + self.records[:1], self.records[::-1]):
            with self.subTest(size=len(rows)), self.assertRaisesRegex(ValueError, "CLOSED_SET"):
                drome.validate_records(rows, self.bindings)

    def test_non_exact_missing_or_changed_binding_is_not_promoted(self):
        for field, value in (("disposition", "CONFLICT"), ("target", {
            "family": "Item", "key": drome.KEYS[0], "revision": "other",
        })):
            bindings = copy.deepcopy(self.bindings)
            bindings["36723"][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "IDENTITY_OR_BINDING"):
                drome.validate_records(self.records, bindings)
        del self.bindings["36723"]
        with self.assertRaisesRegex(ValueError, "IDENTITY_OR_BINDING"):
            drome.validate_records(self.records, self.bindings)

    def test_unknown_physical_facts_are_not_guessed(self):
        for field in ("movable", "pickupable", "weight"):
            rows = copy.deepcopy(self.records)
            rows[0]["semantics"]["physical"]["value"][field] = {"state": "UNKNOWN"}
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "UNKNOWN_"):
                drome.validate_records(rows, self.bindings)

    def test_invalid_custody_weight_and_stack_bounds_are_refused(self):
        mutations = (("physical", "weight", known(-1)), ("physical", "weight", known(True)),
                     ("physical", "weight", known(0x1_0000_0000)),
                     ("physical", "movable", known(False)), ("physical", "pickupable", known(False)),
                     ("stack", "stackable", known(False)), ("stack", "stack_max", known(0)),
                     ("stack", "stack_max", known(256)), ("stack", "stack_max", known(True)),
                     ("stack", "stack_max", {"state": "UNKNOWN"}))
        for group, field, value in mutations:
            rows = copy.deepcopy(self.records)
            rows[0]["semantics"][group]["value"][field] = value
            with self.subTest(group=group, field=field, value=value), self.assertRaises(ValueError):
                drome.validate_records(rows, self.bindings)

    def test_unqualified_admission_container_or_temporal_record_is_refused(self):
        for field in ("materializable", "container", "temporal"):
            rows = copy.deepcopy(self.records)
            if field == "materializable":
                rows[0]["admission"][field] = False
            else:
                rows[0]["semantics"][field] = {"state": "UNKNOWN"}
            with self.subTest(field=field), self.assertRaises(ValueError):
                drome.validate_records(rows, self.bindings)

    def test_actual_non_stackable_source_is_preserved(self):
        cube = self.records[-1]
        cube["admission"]["stack_class"] = "NonStackable"
        cube["semantics"]["stack"] = known({"stackable": known(False), "stack_max": copy.deepcopy(drome.NA)})
        generated = self.generated()
        self.admit(generated)
        self.assertEqual(json.loads(generated["profiles.json"])["records"][-1], cube)

    def test_agreeing_existing_native_profile_keeps_richer_fields(self):
        existing = copy.deepcopy(self.records[0])
        existing["attributes"]["fixture_existing_attribute"] = 7
        generated = self.generated([existing])
        self.admit(generated)
        rows = json.loads(generated["profiles.json"])["records"]
        self.assertEqual(rows[0], existing)
        self.assertEqual(len(rows), 21)

    def test_conflicting_or_duplicate_native_profile_does_not_change_output(self):
        existing = self.records[0]
        changed = copy.deepcopy(existing)
        changed["semantics"]["physical"]["value"]["weight"] = known(181)
        for rows in ([changed], [existing, existing]):
            generated = self.generated(rows)
            before = copy.deepcopy(generated)
            with self.subTest(size=len(rows)), self.assertRaisesRegex(ValueError, "(NATIVE_CONFLICT|DUPLICATE_NATIVE)"):
                self.admit(generated)
            self.assertEqual(generated, before)

    def test_missing_and_oversize_reward_names_are_not_silent_notices(self):
        for name in ({"state": "UNKNOWN"}, known(""), known("x" * 256)):
            rows = copy.deepcopy(self.records)
            rows[0]["semantics"]["presentation"]["value"]["name"] = name
            with self.subTest(name=name), self.assertRaises(ValueError):
                drome.validate_records(rows, self.bindings)

    def test_overlay_requires_primary_revision_full_source_closure_and_record_digest(self):
        for field in ("schema", "source_revision", "identity_binding_sha256", "records_sha256"):
            overlay = self.overlay()
            overlay[field] = "wrong"
            with self.subTest(field=field), self.assertRaises(ValueError):
                drome.validate_overlay(overlay, self.bindings)
        for mutation in (lambda rows: rows.pop(), lambda rows: rows.reverse(),
                         lambda rows: rows[0].update(git_blob="0" * 40),
                         lambda rows: rows[0].update(sha256="not-a-digest")):
            overlay = self.overlay()
            mutation(overlay["sources"])
            with self.assertRaisesRegex(ValueError, "SOURCE_CLOSURE"):
                drome.validate_overlay(overlay, self.bindings)

    def test_unknown_or_modified_source_bytes_fail_git_blob_proof(self):
        for path in (*drome.SOURCE_BLOBS, "unlisted"):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "SOURCE_BLOB"):
                drome.verify_blob(path, b"substituted source")
        raw = b"fixture source"
        blob = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
        with patch.dict(drome.SOURCE_BLOBS, {"fixture": blob}):
            drome.verify_blob("fixture", raw)
            with self.assertRaisesRegex(ValueError, "SOURCE_BLOB"):
                drome.verify_blob("fixture", raw + b"!")

    def test_pool_capture_requires_exact_ids_counts_and_cube(self):
        pool = "local dromeCube = 36827\nlocal rewards = {\n" + "\n".join(
            f"{{ id = {item}, count = 1 }}," for item in drome.REWARD_IDS) + "\n}"
        drome.verify_pool(pool.encode())
        for changed in (pool.replace("36827", "36828"), pool.replace("count = 1", "count = 2", 1),
                        pool.replace("36723", "36724"), pool.replace("{ id = 36723, count = 1 },", "")):
            with self.assertRaisesRegex(ValueError, "SOURCE_POOL"):
                drome.verify_pool(changed.encode())

    def test_binding_bytes_are_checked_before_any_native_output_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / drome.BINDINGS_PATH
            path.parent.mkdir(parents=True)
            path.write_bytes(b"substituted registry")
            generated = self.generated()
            before = copy.deepcopy(generated)
            with self.assertRaisesRegex(ValueError, "BINDING_DIGEST"):
                drome.admit(root, generated, "profiles.json", "manifest.json", lambda row: 0, canonical)
            self.assertEqual(generated, before)

    def test_validation_does_not_mutate_source_profiles_or_bindings(self):
        records, bindings = copy.deepcopy(self.records), copy.deepcopy(self.bindings)
        overlay = self.overlay()
        original = copy.deepcopy(overlay)
        self.assertEqual(drome.validate_overlay(overlay, self.bindings), records)
        self.assertEqual(overlay, original)
        self.assertEqual(self.records, records)
        self.assertEqual(self.bindings, bindings)


if __name__ == "__main__":
    unittest.main()
