"""Real official corpus and source/admission/domain failure boundaries."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import official_corpses as official
import world_objects


class OfficialCorpseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = official.source_inputs()
        cls.document, cls.members, cls.client, cls.definitions, cls.admitted, *_ = (
            cls.inputs
        )
        cls.entries = {row["appearance_id"]: row for row in cls.document["records"]}
        cls.item_id = min(official.IDS)

    def args(self):
        entry = self.entries[self.item_id]
        return [
            copy.deepcopy(entry),
            copy.deepcopy(self.client[self.item_id]),
            copy.deepcopy(self.definitions[entry["target"]["key"]]),
            copy.deepcopy(self.members[self.item_id]),
            set(self.admitted),
            set(),
            set(),
            set(),
            copy.deepcopy(self.document["source"]),
        ]

    def test_all_forty_real_records_and_source_only_unknowns(self):
        for item_id in sorted(official.IDS):
            record = official.expected_record(item_id, self.inputs)
            self.assertEqual(world_objects.validate_record(record), [])
            self.assertEqual(record["kind"], {"state": "KNOWN", "value": "corpse"})
            self.assertEqual(
                record["collision"]["blocks_projectile"], {"state": "UNKNOWN"}
            )
            self.assertEqual(record["placement"]["rotate_to"], {"state": "UNKNOWN"})
            self.assertNotIn("duration", record["corpse"])
            self.assertNotIn("capacity", record["corpse"])
            self.assertEqual(
                record["provenance"]["source"]["revision"],
                self.document["source"]["revision"],
            )

    def test_identity_owner_native_and_presentation_guards(self):
        key = self.entries[self.item_id]["target"]["key"]
        cases = (
            ("wrong object", 1, lambda value: value.update(id=35384)),
            (
                "wrong key",
                2,
                lambda value: value["identity"].update(key="oteryn:item.tibia.i35384"),
            ),
            ("materializable", 2, lambda value: value.update(materializable=True)),
            (
                "known stack class",
                2,
                lambda value: value.update(stack_class="NonStackable"),
            ),
            ("wrong current member", 3, lambda value: value.__setitem__(1, "a" * 64)),
            ("no explicit admission", 4, lambda value: value.discard(self.item_id)),
            ("taxonomy owner", 5, lambda value: value.add(key)),
            ("world owner", 6, lambda value: value.add(key)),
            ("Crystal binding", 7, lambda value: value.add(key)),
            (
                "group N/A",
                2,
                lambda value: value.update(
                    semantics={"presentation": {"state": "NOT_APPLICABLE"}}
                ),
            ),
            (
                "group conflict",
                2,
                lambda value: value.update(
                    semantics={"presentation": {"state": "CONFLICT"}}
                ),
            ),
            (
                "leaf N/A",
                2,
                lambda value: value.update(
                    semantics={
                        "presentation": {
                            "state": "KNOWN",
                            "value": {"name": {"state": "NOT_APPLICABLE"}},
                        }
                    }
                ),
            ),
            (
                "invented name",
                2,
                lambda value: value.update(
                    semantics={
                        "presentation": {
                            "state": "KNOWN",
                            "value": {
                                "name": {"state": "KNOWN", "value": "invented corpse"}
                            },
                        }
                    }
                ),
            ),
        )
        for label, index, mutate in cases:
            with self.subTest(label=label):
                args = self.args()
                mutate(args[index])
                with self.assertRaises(ValueError):
                    official.build_record(*args)
        args = self.args()
        args[1]["flags"].pop("flags.corpse")
        # Isolate affirmative corpse proof from the independently sealed observation.
        args[0]["decoded_record_sha256"] = official.digest(args[1])
        with self.assertRaisesRegex(ValueError, "POSITIVE_ROUTE"):
            official.build_record(*args)

    def test_fake_source_scope_and_flags_do_not_validate(self):
        record = official.expected_record(self.item_id, self.inputs)
        mutations = (
            lambda value: value["provenance"]["source"].update(revision="a" * 64),
            lambda value: value["provenance"]["source"].update(revision="a" * 40),
            lambda value: value["provenance"]["source"].update(engine="crystal"),
            lambda value: value["provenance"]["source"].pop(
                "membership_manifest_sha256"
            ),
            lambda value: value["corpse"]["corpse"].update(value=False),
            lambda value: value["kind"].update(value="object"),
            lambda value: value["provenance"].update(source_item_id=53197),
        )
        for mutate in mutations:
            changed = copy.deepcopy(record)
            mutate(changed)
            self.assertTrue(world_objects.validate_record(changed))
        changed = copy.deepcopy(record)
        changed["identity"]["family"] = "Terrain"
        changed["identity"]["key"] = "oteryn:terrain.tibia.i21887"
        changed["kind"]["value"] = "ground"
        changed.pop("corpse")
        self.assertTrue(world_objects.validate_record(changed))
        # Even a fully relabelled old-shaped source cannot usurp an admitted official-only key.
        changed = copy.deepcopy(record)
        changed["provenance"]["source"] = {
            "engine": "crystal",
            "repository": "fake",
            "revision": "a" * 40,
            "profile": "fake",
        }
        changed["provenance"]["source_item_id"] = 123
        self.assertTrue(world_objects.validate_record(changed))

    def test_seal_and_current_manifest_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "qualification.json"
            path.write_bytes(official.QUALIFICATION.read_bytes() + b" ")
            with (
                patch.object(official, "QUALIFICATION", path),
                self.assertRaisesRegex(ValueError, "QUALIFICATION_DIGEST"),
            ):
                official.qualification()
        index, manifests = official.appearance_membership.load_admitted()
        index = copy.deepcopy(index)
        index["files"][-1]["manifest_sha256"] = "a" * 64
        official.source_inputs.cache_clear()
        try:
            with (
                patch.object(
                    official.appearance_membership,
                    "load_admitted",
                    return_value=(index, manifests),
                ),
                self.assertRaisesRegex(ValueError, "CURRENT_MEMBERSHIP"),
            ):
                official.source_inputs()
        finally:
            official.source_inputs.cache_clear()

    def test_actual_artifact_digest_is_verified_beyond_manifest_metadata(self):
        artifact = official.ROOT / self.document["source"]["artifact"]
        original = Path.read_bytes

        def changed_bytes(path):
            raw = original(path)
            return raw + b" " if path == artifact else raw

        official.source_inputs.cache_clear()
        try:
            with (
                patch.object(Path, "read_bytes", changed_bytes),
                self.assertRaisesRegex(ValueError, "SOURCE_ARTIFACT"),
            ):
                official.source_inputs()
        finally:
            official.source_inputs.cache_clear()

    def test_outputs_closed_to_forty_and_retain_existing_owner(self):
        files = official.outputs(set(), 12877)
        shard = json.loads(files["content/world/objects/objects-12877-12916.json"])
        self.assertEqual(len(shard["records"]), 40)
        with self.assertRaisesRegex(ValueError, "EXISTING_WORLD_OWNER"):
            official.outputs({self.entries[self.item_id]["target"]["key"]}, 12877)


if __name__ == "__main__":
    unittest.main()
