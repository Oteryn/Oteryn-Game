"""Closed-set/provenance regressions for the ordinary native Item registration."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import drome_item_profiles as drome


def known(value):
    return {"state": "KNOWN", "value": value}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


class DromeItemProfilesTests(unittest.TestCase):
    def setUp(self):
        self.world = {"records": []}
        self.bindings = {"bindings": []}
        for item in drome.ITEM_IDS:
            identity = {"family": "Item", "key": f"oteryn:item.tibia.i{item}", "revision": "definition-r1"}
            self.world["records"].append({
                "identity": identity, "materializable": True, "stack_class": "StackCapable",
                "semantics": {
                    "physical": known({"movable": known(True), "pickupable": known(True), "weight": known(180)}),
                    "stack": known({"stackable": known(True), "stack_max": known(100)}),
                    "container": {"state": "NOT_APPLICABLE"},
                    "presentation": known({"name": known(f"fixture item {item}")}),
                },
            })
            self.bindings["bindings"].append({"target": copy.deepcopy(identity), "disposition": "EXACT"})

    def generated(self, records=()):
        return {
            "profiles.json": canonical({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": list(records)}),
            "manifest.json": canonical({"item_profiles": {"path": "profiles.json", "sha256": "old"}}),
        }

    def admit(self, generated):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path, value in ((drome.WORLD_PATH, self.world), (drome.BINDINGS_PATH, self.bindings)):
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(canonical(value))
            drome.admit(root, generated, "profiles.json", "manifest.json",
                        lambda row: int(row["production_definition"]["production_key"].rsplit("i", 1)[1]), canonical)

    def test_all_twenty_rewards_and_the_cube_are_registered_and_pinned(self):
        generated = self.generated()
        self.admit(generated)
        records = json.loads(generated["profiles.json"])["records"]
        self.assertEqual([row["production_definition"]["production_key"] for row in records],
                         [f"oteryn:item.tibia.i{item}" for item in drome.ITEM_IDS])
        self.assertEqual(len(records), 21)
        self.assertEqual(json.loads(generated["manifest.json"])["item_profiles"]["sha256"],
                         hashlib.sha256(generated["profiles.json"]).hexdigest())
        before = copy.deepcopy(generated)
        self.admit(generated)
        self.assertEqual(generated, before)

    def test_missing_or_duplicate_source_and_binding_fail_before_output_changes(self):
        world, bindings = copy.deepcopy(self.world), copy.deepcopy(self.bindings)
        for field in ("identity", "target"):
            for duplicate in (False, True):
                with self.subTest(field=field, duplicate=duplicate):
                    self.world, self.bindings = copy.deepcopy(world), copy.deepcopy(bindings)
                    rows = self.world["records"] if field == "identity" else self.bindings["bindings"]
                    if duplicate:
                        rows.append(copy.deepcopy(rows[0]))
                    else:
                        rows.pop()
                    generated = self.generated()
                    before = copy.deepcopy(generated)
                    with self.assertRaisesRegex(ValueError, "DROME_ITEM_PROFILE_(MISSING|DUPLICATE)"):
                        self.admit(generated)
                    self.assertEqual(generated, before)

    def test_non_exact_binding_or_changed_revision_is_not_promoted(self):
        for field, value in (("disposition", "CONFLICT"), ("target", {
            "family": "Item", "key": "oteryn:item.tibia.i36723", "revision": "other",
        })):
            with self.subTest(field=field):
                bindings = copy.deepcopy(self.bindings)
                bindings["bindings"][0][field] = value
                with self.assertRaisesRegex(ValueError, "IDENTITY_OR_BINDING"):
                    drome.profiles(self.world, bindings)

    def test_unknown_physical_facts_are_not_guessed(self):
        for field in ("movable", "pickupable", "weight"):
            with self.subTest(field=field):
                world = copy.deepcopy(self.world)
                world["records"][0]["semantics"]["physical"]["value"][field] = {"state": "UNKNOWN"}
                with self.assertRaisesRegex(ValueError, "UNKNOWN_"):
                    drome.profiles(world, self.bindings)

    def test_invalid_custody_weight_and_stack_bounds_are_refused(self):
        mutations = (
            ("physical", "weight", known(-1)),
            ("physical", "weight", known(True)),
            ("physical", "weight", known(0x1_0000_0000)),
            ("physical", "movable", known(False)),
            ("physical", "pickupable", known(False)),
            ("stack", "stackable", known(False)),
            ("stack", "stack_max", known(0)),
            ("stack", "stack_max", known(True)),
            ("stack", "stack_max", {"state": "UNKNOWN"}),
        )
        for group, field, value in mutations:
            with self.subTest(group=group, field=field, value=value):
                world = copy.deepcopy(self.world)
                world["records"][0]["semantics"][group]["value"][field] = value
                with self.assertRaises(ValueError):
                    drome.profiles(world, self.bindings)

    def test_non_materializable_or_container_record_is_refused(self):
        for change in ("materializable", "container"):
            world = copy.deepcopy(self.world)
            if change == "materializable":
                world["records"][0][change] = False
            else:
                world["records"][0]["semantics"][change] = known({"capacity": known(20)})
            with self.subTest(change=change), self.assertRaises(ValueError):
                drome.profiles(world, self.bindings)

    def test_actual_non_stackable_source_is_preserved_not_invented_as_stackable(self):
        cube = self.world["records"][-1]
        cube["stack_class"] = "NonStackable"
        cube["semantics"]["stack"] = known({"stackable": known(False), "stack_max": {"state": "NOT_APPLICABLE"}})
        row = drome.profiles(self.world, self.bindings)[-1]
        self.assertEqual(row["admission"]["stack_class"], "NonStackable")
        self.assertEqual(row["semantics"], cube["semantics"])

    def test_agreeing_existing_native_profile_keeps_richer_fields(self):
        existing = drome.profiles(self.world, self.bindings)[0]
        existing["attributes"] = {"fixture_existing_attribute": 7}
        generated = self.generated([existing])
        self.admit(generated)
        rows = json.loads(generated["profiles.json"])["records"]
        self.assertEqual(rows[0], existing)
        self.assertEqual(len(rows), 21)

    def test_conflicting_or_duplicate_native_profile_does_not_overwrite_output(self):
        existing = drome.profiles(self.world, self.bindings)[0]
        changed = copy.deepcopy(existing)
        changed["semantics"]["physical"]["value"]["weight"] = known(181)
        for rows in ([changed], [existing, existing]):
            with self.subTest(duplicates=len(rows) == 2):
                generated = self.generated(rows)
                before = copy.deepcopy(generated)
                with self.assertRaisesRegex(ValueError, "DROME_ITEM_PROFILE_(NATIVE_CONFLICT|DUPLICATE_NATIVE)"):
                    self.admit(generated)
                self.assertEqual(generated, before)

    def test_source_semantics_and_bindings_are_not_mutated_or_aliased(self):
        world, bindings = copy.deepcopy(self.world), copy.deepcopy(self.bindings)
        records = drome.profiles(self.world, self.bindings)
        records[0]["semantics"]["physical"]["value"]["weight"] = known(0)
        records[0]["production_binding"]["target"]["revision"] = "other"
        self.assertEqual(self.world, world)
        self.assertEqual(self.bindings, bindings)

    def test_missing_and_oversize_reward_names_are_not_silent_notices(self):
        for name in ({"state": "UNKNOWN"}, known(""), known("x" * 256)):
            world = copy.deepcopy(self.world)
            world["records"][0]["semantics"]["presentation"]["value"]["name"] = name
            with self.subTest(name=name), self.assertRaises(ValueError):
                drome.profiles(world, self.bindings)


if __name__ == "__main__":
    unittest.main()
