"""Real pinned source data and fail-closed boundaries of the six-row A12 navigation."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import item_official_navigation as official
import item_taxonomy as taxonomy
from engine_items import load_appearance_objects


class OfficialNavigationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document, cls.members = official.qualification_inputs()
        raw = (official.ROOT / cls.document["client_artifact"]["path"]).read_bytes()
        cls.client = load_appearance_objects(raw)
        cls.entries = {row["appearance_id"]: row for row in cls.document["records"]}
        cls.definitions = {}
        for path in (official.ROOT / "content/items/definitions").glob("items-*.json"):
            for row in json.loads(path.read_bytes())["records"]:
                definition = row["definition"]
                cls.definitions[definition["identity"]["key"]] = definition
        snapshot = json.loads((official.ROOT / taxonomy.SNAPSHOT).read_bytes())
        cls.wiki = {
            row["item_id"]: row["observations"] for row in snapshot["records"].values()
        }

    def args(self, item_id=53197):
        entry = self.entries[item_id]
        return [
            copy.deepcopy(entry),
            copy.deepcopy(self.client[item_id]),
            copy.deepcopy(self.definitions[entry["target"]["key"]]),
            copy.deepcopy(self.wiki[item_id]),
            set(),
            set(),
            copy.deepcopy(self.members[item_id]),
            taxonomy.MARKET_PROFILES,
            taxonomy.MARKET_CLOTHES_SLOT,
        ]

    def test_qualification_digest_and_current_generation_are_sealed(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "qualification.json"
            path.write_bytes(official.QUALIFICATION.read_bytes() + b" ")
            with (
                patch.object(official, "QUALIFICATION", path),
                self.assertRaisesRegex(ValueError, "QUALIFICATION_DIGEST"),
            ):
                official.qualification_inputs()
        index, manifests = official.load_admitted()
        index = copy.deepcopy(index)
        index["files"][-1]["manifest_sha256"] = "a" * 64
        with (
            patch.object(official, "load_admitted", return_value=(index, manifests)),
            self.assertRaisesRegex(ValueError, "CURRENT_GENERATION"),
        ):
            official.qualification_inputs()

    def test_explicit_native_admission_is_required(self):
        admission = self.document["admission_source"]
        source = (official.ROOT / admission["path"]).read_text()
        ids = official.admission_ids(source, admission)
        self.assertEqual(len(ids), 60)
        self.assertTrue(official.REVIEWED_IDS.issubset(ids))
        for changed in (source.replace("53197, 53199", "53196, 53199"), ""):
            with self.assertRaisesRegex(ValueError, "NATIVE_ADMISSION"):
                official.admission_ids(changed, admission)

    def test_real_six_records_are_navigation_only(self):
        for item_id in sorted(official.REVIEWED_IDS):
            args = self.args(item_id)
            before = copy.deepcopy(args[2])
            row = official.derive_row(*args)
            self.assertIsNotNone(row, item_id)
            self.assertEqual(row["target"], self.entries[item_id]["target"])
            self.assertEqual(row["source_evidence"]["scope"], "NAVIGATION_ONLY")
            self.assertEqual(args[2], before)
            self.assertNotIn("source_key", row["source_evidence"]["identity_bridge"])
            self.assertEqual(
                row["family_profile"], self.entries[item_id]["family_profile"]
            )

    def test_identity_admission_owner_and_name_guards(self):
        cases = (
            ("wrong object", 1, lambda value: value.update(id=53199)),
            (
                "mutated official observation",
                1,
                lambda value: value.update(name="wrong name"),
            ),
            (
                "wrong canonical identity",
                2,
                lambda value: value["identity"].update(key="oteryn:item.tibia.i53199"),
            ),
            ("materializable Item", 2, lambda value: value.update(materializable=True)),
            (
                "known stack class",
                2,
                lambda value: value.update(stack_class="NonStackable"),
            ),
            (
                "existing world owner or taxonomy",
                4,
                lambda value: value.add("oteryn:item.tibia.i53197"),
            ),
            (
                "Crystal-bound path retains precedence",
                5,
                lambda value: value.add("oteryn:item.tibia.i53197"),
            ),
            (
                "presentation group not applicable",
                2,
                lambda value: value.update(
                    semantics={"presentation": {"state": "NOT_APPLICABLE"}}
                ),
            ),
            (
                "name leaf not applicable",
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
                "canonical name conflict",
                2,
                lambda value: value.update(
                    semantics={
                        "presentation": {
                            "state": "KNOWN",
                            "value": {
                                "name": {"state": "KNOWN", "value": "different item"}
                            },
                        }
                    }
                ),
            ),
            (
                "explicit unknown primary",
                3,
                lambda value: value[0]["fields"].update(primarytype="Others"),
            ),
            (
                "explicit conflicting primary",
                3,
                lambda value: value[0]["fields"].update(primarytype="Food"),
            ),
            (
                "Wiki name disagreement",
                3,
                lambda value: value[0].update(wiki_title="Wrong Item"),
            ),
        )
        for label, index, mutate in cases:
            with self.subTest(label=label):
                args = self.args()
                mutate(args[index])
                self.assertIsNone(official.derive_row(*args))
        args = self.args()
        args[6] = None
        self.assertIsNone(official.derive_row(*args))
        args = self.args()
        args[6][1] = "a" * 64
        self.assertIsNone(official.derive_row(*args))

    def test_missing_slot_observation_or_mapping_cannot_qualify(self):
        for missing_observation, missing_mapping in (
            (True, False),
            (False, True),
            (True, True),
        ):
            with self.subTest(observation=missing_observation, mapping=missing_mapping):
                args = self.args()
                args[8] = dict(args[8])
                if missing_observation:
                    args[1]["flags"].pop("clothes.slot")
                    # Isolate slot inference from the separate sealed-record guards.
                    args[0]["official_flags"].pop("clothes.slot")
                    args[0]["decoded_record_sha256"] = official.decoded_digest(args[1])
                if missing_mapping:
                    args[8].pop(2)
                self.assertIsNone(official.derive_row(*args))

    def test_missing_or_ambiguous_positive_facts_do_not_default(self):
        for item_id, mutation in (
            (53197, lambda flags: flags.pop("flags.take")),
            (53197, lambda flags: flags.update({"clothes.slot": 5})),
            (53197, lambda flags: flags.update({"flags.corpse": True})),
            (53197, lambda flags: flags.update({"flags.unmove": True})),
            (51276, lambda flags: flags.pop("flags.cumulative")),
        ):
            with self.subTest(item_id=item_id, mutation=mutation):
                args = self.args(item_id)
                mutation(args[1]["flags"])
                # Bypass only the fixture's observation hash to exercise the positive-fact guard.
                # Production entries stay digest-bound by qualification_inputs().
                args[0]["decoded_record_sha256"] = official.decoded_digest(args[1])
                self.assertIsNone(official.derive_row(*args))


if __name__ == "__main__":
    unittest.main()
