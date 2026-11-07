"""Closed real sources and adversarial identity/provenance boundaries."""

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import official_corpses as official
import qualified_world as qualified
import world_objects


class QualifiedWorldTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = qualified.source_inputs()
        cls.entries = {row["appearance_id"]: row for row in cls.inputs[0]["records"]}

    def args(self, item_id=1019):
        (
            document,
            members,
            client,
            definitions,
            admitted,
            taxonomy,
            owners,
            bindings,
            reverse,
            old_members,
            snapshot,
        ) = self.inputs
        entry = self.entries[item_id]
        return copy.deepcopy(
            [
                entry,
                client[item_id],
                definitions[entry["target"]["key"]],
                members[item_id],
                admitted,
                taxonomy,
                owners,
                bindings,
                reverse,
                old_members,
                document,
                snapshot,
            ]
        )

    def test_all49_real_sources_and_existing_kinds(self):
        counts = {"Terrain": 0, "WorldObject": 0}
        for item_id in sorted(qualified.IDS):
            record = qualified.expected_record(item_id, self.inputs)
            self.assertEqual(world_objects.validate_record(record), [])
            counts[record["identity"]["family"]] += 1
            source = record["provenance"]["source"]
            self.assertEqual(
                source["revision"], self.inputs[0]["official_source"]["revision"]
            )
            labels = [
                label
                for labels in record["provenance"]["fields"].values()
                for label in labels
            ]
            self.assertFalse(any(label.startswith("items_xml:") for label in labels))
            self.assertTrue(
                any(label.startswith("official_client:") for label in labels)
            )
            self.assertTrue(any(label.startswith("derived:") for label in labels))
            self.assertEqual(
                source["profile"],
                qualified.WIKI_PROFILE
                if item_id in qualified.WIKI_IDS
                else qualified.FIXED_PROFILE,
            )
            self.assertEqual("wiki_source" in source, item_id in qualified.WIKI_IDS)
            self.assertEqual(record["kind"]["state"], "KNOWN")
        self.assertEqual(counts, {"Terrain": 34, "WorldObject": 15})
        self.assertIsNone(self.entries[1127]["official_name"])
        self.assertEqual(
            self.entries[1127]["native_presentation"]["value"]["name"]["value"],
            "mountain",
        )

    def test_identity_native_binding_and_precedence_guards(self):
        key = self.entries[1019]["target"]["key"]
        cases = (
            (1, lambda x: x.update(id=1127)),
            (2, lambda x: x["identity"].update(key="oteryn:item.tibia.i1127")),
            (2, lambda x: x.update(materializable=True)),
            (2, lambda x: x.update(stack_class="NonStackable")),
            (3, lambda x: x.__setitem__(1, "a" * 64)),
            (5, lambda x: x.add(key)),
            (6, lambda x: x.add(key)),
            (7, lambda x: x.pop(key)),
            (8, lambda x: x["1019"].append(copy.deepcopy(x["1019"][0]))),
            (9, lambda x: x[1019].__setitem__(1, "a" * 64)),
            (
                2,
                lambda x: x["semantics"].update(
                    presentation={"state": "NOT_APPLICABLE"}
                ),
            ),
            (
                2,
                lambda x: x["semantics"]["presentation"]["value"].update(
                    name={"state": "CONFLICT"}
                ),
            ),
            (
                2,
                lambda x: x["semantics"]["presentation"]["value"]["name"].update(
                    value="invented"
                ),
            ),
        )
        for index, mutate in cases:
            args = self.args()
            mutate(args[index])
            with self.subTest(index=index), self.assertRaises(ValueError):
                qualified.build_record(*args)
        args = self.args()
        args[1]["flags"]["flags.unmove"] = False
        args[0]["decoded_record_sha256"] = official.digest(args[1])
        with self.assertRaisesRegex(ValueError, "POSITIVE_FIXED_FLAG"):
            qualified.build_record(*args)

    def test_own_wiki_not_sibling_lexical_or_duplicate_identity(self):
        snapshot = self.inputs[-1]
        page = snapshot["pages"][self.entries[1019]["wiki_page_index"]]
        for old, new in (
            ("= 1019", "= 1127"),
            ("= 1019", "= 1019x"),
            ("= no", "= yes"),
            ("= Natural Tiles", "= Weapons"),
            ("= 1019", "= 1019\n|itemid=1019"),
        ):
            changed = copy.deepcopy(page)
            changed["raw_infobox"] = changed["raw_infobox"].replace(old, new)
            changed["raw_infobox_sha256"] = hashlib.sha256(
                changed["raw_infobox"].encode()
            ).hexdigest()
            values = {}
            for part in qualified.split_template_params(changed["raw_infobox"][2:-2])[
                1:
            ]:
                if "=" in part:
                    key, value = part.split("=", 1)
                    values.setdefault(key.strip().lower(), []).append(value.strip())
            changed["parameter_values"] = values
            with self.subTest(change=new), self.assertRaises(ValueError):
                qualified.wiki_primary(changed, 1019, snapshot["qualification_cutoff"])
        changed = copy.deepcopy(page)
        changed["revision_timestamp"] = "2026-09-28T00:00:00Z"
        with self.assertRaises(ValueError):
            qualified.wiki_primary(changed, 1019, snapshot["qualification_cutoff"])

    def test_fixed_native_admission_and_terrain_profile_prohibition(self):
        for mutate in (
            lambda args: args[4].discard(36929),
            lambda args: args[7].update({args[0]["target"]["key"]: [{}]}),
            lambda args: args[1]["flags"].update(**{"market.category": 1}),
            lambda args: args[1]["flags"].update(**{"clothes.slot": 1}),
            lambda args: args[1]["flags"].update(**{"flags.unpass": False}),
        ):
            args = self.args(36929)
            mutate(args)
            args[0]["decoded_record_sha256"] = official.digest(args[1])
            with self.assertRaisesRegex(ValueError, "NATIVE_FIXED_ROUTE"):
                qualified.build_record(*args)
        record = qualified.expected_record(1019, self.inputs)
        record["provenance"]["source"].pop("wiki_source")
        for profile in (qualified.FIXED_PROFILE, official.PROFILE):
            record["provenance"]["source"]["profile"] = profile
            self.assertTrue(world_objects.schema_errors("Terrain", record))

    def test_scope_seal_and_relabelled_source_guards(self):
        for item_id in (470, 16249, 21887, 53197):
            with self.assertRaisesRegex(ValueError, "UNREVIEWED_ID"):
                qualified.expected_record(item_id, self.inputs)
        record = qualified.expected_record(1019, self.inputs)
        for mutate in (
            lambda r: r["provenance"]["source"].update(revision="a" * 64),
            lambda r: r["provenance"]["source"]["wiki_source"].update(page_id=1),
            lambda r: r["provenance"]["source"].update(engine="crystal"),
            lambda r: r["provenance"].update(source_item_id=1127),
            lambda r: r["provenance"]["fields"].update(kind=["items_xml:primarytype"]),
        ):
            changed = copy.deepcopy(record)
            mutate(changed)
            self.assertTrue(world_objects.validate_record(changed))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "changed.json"
            path.write_bytes(qualified.QUALIFICATION.read_bytes() + b" ")
            with (
                patch.object(qualified, "QUALIFICATION", path),
                self.assertRaisesRegex(ValueError, "SEAL"),
            ):
                qualified.qualification()
        with self.assertRaisesRegex(ValueError, "EXISTING_OWNER"):
            qualified.outputs(
                {self.entries[1019]["target"]["key"]},
                {"Terrain": 8578, "WorldObject": 12917},
            )

    def test_wiki_source_is_verified_in_full(self):
        artifact = qualified.ROOT / self.inputs[0]["wiki_source"]["path"]
        original = Path.read_bytes

        def changed(path):
            raw = original(path)
            return raw + b" " if path == artifact else raw

        qualified.source_inputs.cache_clear()
        try:
            with (
                patch.object(Path, "read_bytes", changed),
                self.assertRaisesRegex(ValueError, "WIKI_SOURCE"),
            ):
                qualified.source_inputs()
        finally:
            qualified.source_inputs.cache_clear()

    def test_an_existing_copy_of_a_qualified_record_remains_an_owner_conflict(self):
        record = qualified.expected_record(1019, self.inputs)
        original = Path.glob
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "copied-record.json"
            path.write_text(json.dumps({"records": [record]}))

            def with_copy(folder, pattern):
                paths = list(original(folder, pattern))
                if folder == qualified.ROOT / "content/world/terrain":
                    paths.append(path)
                return iter(paths)

            qualified.source_inputs.cache_clear()
            try:
                with patch.object(Path, "glob", with_copy):
                    inputs = qualified.source_inputs()
                with self.assertRaisesRegex(ValueError, "IDENTITY_OR_PRECEDENCE"):
                    qualified.expected_record(1019, inputs)
            finally:
                qualified.source_inputs.cache_clear()


if __name__ == "__main__":
    unittest.main()
