"""Closed scope, source identity and existing family precedence for the 265-row batch."""

import copy
import json
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest.mock import patch

import item_engine_navigation as nav


class EngineNavigationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = nav.source_inputs()
        cls.entries = cls.inputs[0]["records"]
        pin = cls.inputs[0]["source_pins"]["official"]["appearances_sha256"]
        cls.client = nav.engine.load_appearance_objects(
            (nav.ROOT / f"content/assets/files/appearances-{pin}.dat").read_bytes()
        )
        cls.definitions = {}
        for path in (nav.ROOT / "content/items/definitions").glob("items-*.json"):
            for row in json.loads(path.read_bytes())["records"]:
                definition = row["definition"]
                cls.definitions[definition["identity"]["key"]] = definition

    def args(self, entry=None):
        entry = entry or self.entries[0]
        return [
            copy.deepcopy(entry),
            copy.deepcopy(self.client[entry["appearance_id"]]),
            copy.deepcopy(self.definitions[entry["target"]["key"]]),
            copy.deepcopy(self.inputs),
            set(),
        ]

    def test_real_closed_batch_and_existing_rows_take_precedence(self):
        before = copy.deepcopy(self.definitions)
        rows = nav.build_navigation(self.definitions, self.client, set())
        self.assertEqual(len(rows), 265)
        self.assertEqual(
            Counter(r["family_profile"] for r in rows),
            {
                "material_valuable": 36,
                "document": 5,
                "container": 28,
                "tool": 5,
                "decoration": 131,
                "light_source": 28,
                "quest_item": 15,
                "equipment_offhand": 1,
                "plant": 15,
                "equipment_armor": 1,
            },
        )
        self.assertEqual(self.definitions, before)
        self.assertTrue(
            all(r["source_evidence"]["scope"] == "NAVIGATION_ONLY" for r in rows)
        )
        self.assertEqual(
            nav.build_navigation(
                self.definitions,
                self.client,
                {e["target"]["key"] for e in self.entries},
            ),
            [],
        )
        self.assertEqual(
            sum(
                e["binding"]["source_revision"] == nav.donor_census.DONOR_COMMIT
                for e in self.entries
            ),
            5,
        )

    def test_foreign_primary_absence_is_not_a_derived_family_label(self):
        entries = {e["target"]["key"]: e for e in self.entries}
        absent = 0
        for row in nav.build_navigation(self.definitions, self.client, set()):
            own_primary = entries[row["target"]["key"]]["xml_record"]["attrs"].get(
                "primarytype"
            )
            if isinstance(own_primary, str) and own_primary.strip():
                self.assertEqual(row["source_taxonomy"]["primary"], own_primary)
                self.assertEqual(
                    row["source_evidence"]["source_primarytype"],
                    {"state": "KNOWN", "value": own_primary},
                )
            else:
                absent += 1
                self.assertEqual(
                    row["source_taxonomy"]["primary"], "Source primarytype absent"
                )
                self.assertNotEqual(
                    row["source_taxonomy"]["primary"], row["family_profile"]
                )
                self.assertEqual(
                    row["source_evidence"]["source_primarytype"],
                    {"state": "UNKNOWN", "reason": "ABSENT_OR_EMPTY"},
                )
            self.assertEqual(row["source_evidence"]["classification"], "DERIVED")
            self.assertEqual(
                row["source_evidence"]["family_basis"],
                entries[row["target"]["key"]]["basis"],
            )
        self.assertEqual(absent, 123)

    def test_scope_seal_and_current_artifact_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "qualification.json"
            path.write_bytes(nav.QUALIFICATION.read_bytes() + b" ")
            with (
                patch.object(nav, "QUALIFICATION", path),
                self.assertRaisesRegex(ValueError, "SEAL"),
            ):
                nav.qualification()
        index, manifests = nav.appearance_membership.load_admitted()
        index = copy.deepcopy(index)
        index["newest"] = "crystal-ff7ede5"
        nav.source_inputs.cache_clear()
        try:
            with (
                patch.object(
                    nav.appearance_membership,
                    "load_admitted",
                    return_value=(index, manifests),
                ),
                self.assertRaisesRegex(ValueError, "CURRENT_MEMBERSHIP"),
            ):
                nav.source_inputs()
        finally:
            nav.source_inputs.cache_clear()

    def test_owner_current_identity_reverse_binding_and_native_guards(self):
        mutations = [
            lambda a: a[4].add(a[0]["target"]["key"]),
            lambda a: a[0].update(profile="food"),
            lambda a: a[1].update(id=1),
            lambda a: a[2].update(materializable=True),
            lambda a: a[2].update(stack_class="Cumulative"),
            lambda a: a[2]["identity"].update(key="oteryn:item.tibia.i1"),
            lambda a: a[3][1]["client-15.30"][a[0]["appearance_id"]].__setitem__(
                1, "0" * 64
            ),
            lambda a: a[3][3][str(a[0]["appearance_id"])].append(a[0]["binding"]),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                args = self.args()
                mutate(args)
                with self.assertRaises(ValueError):
                    nav.derive_row(*args)
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            for leaf in (False, True):
                args = self.args()
                presentation = args[2]["semantics"]["presentation"]
                (presentation["value"]["name"] if leaf else presentation)["state"] = (
                    state
                )
                # Keep the sealed projection equal to exercise the state predicate itself.
                args[0]["native_presentation"] = copy.deepcopy(presentation)
                args[3][0]["records"] = [args[0]]
                with self.assertRaisesRegex(ValueError, "NATIVE_PRESENTATION"):
                    nav.derive_row(*args)

    def test_single_hop_requires_actual_target_membership_and_native_identity(self):
        entry = next(e for e in self.entries if "wrap_target_id" in e["family_proof"])
        for part in ("reverse", "membership", "native"):
            args = self.args(entry)
            target_id = entry["family_proof"]["wrap_target_id"]
            if part == "reverse":
                args[3][3][str(target_id)][0]["identity_namespace"] = "wrong"
            elif part == "membership":
                args[3][1]["client-15.30"].pop(target_id)
            else:
                args[3][5].pop(nav.tibia_key(target_id))
            with self.assertRaisesRegex(ValueError, "WRAP_TARGET_IDENTITY"):
                nav.derive_row(*args)

    def test_no_clothing_only_or_unlisted_dead_name_family(self):
        with self.assertRaisesRegex(ValueError, "CLOTHING_ONLY"):
            nav.family(
                {
                    "appearance_id": 1,
                    "xml_record": {"attrs": {}, "name": "old tibia item"},
                    "xml_record_sha256": nav.digest(
                        {"attrs": {}, "name": "old tibia item"}
                    ),
                },
                {"clothes.slot": 5},
            )
        entry = copy.deepcopy(
            next(
                e
                for e in self.entries
                if e["basis"] == "EXISTING_EXACT_REVIEWED_TAKEABLE_DEAD_NAME"
            )
        )
        entry["xml_record"]["name"] = "dead invented creature"
        entry["xml_record_sha256"] = nav.digest(entry["xml_record"])
        with self.assertRaisesRegex(ValueError, "EXACT_DEAD_RULE"):
            nav.family(entry, self.client[entry["appearance_id"]]["flags"])

    def test_own_wiki_numeric_priority_duplicates_and_disagreement(self):
        original = next(e for e in self.entries if e["wiki_own_sources"])
        for field, value in (
            ("primarytype", "Others"),
            ("primarytype", "Food"),
            ("status", "Event"),
            ("itemid", "1"),
            ("itemid", f"{original['appearance_id']} malformed"),
        ):
            entry = copy.deepcopy(original)
            source = entry["wiki_own_sources"][0]
            source["parameter_values"][field] = [value]
            source["parameter_excerpts"] = [
                x
                for x in source["parameter_excerpts"]
                if x.split("=", 1)[0].strip().lower() != field
            ] + [f"{field}={value}"]
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                nav.wiki_priority(entry, [])
        entry = copy.deepcopy(original)
        source = entry["wiki_own_sources"][0]
        source["parameter_excerpts"].append("itemid=" + str(entry["appearance_id"]))
        source["parameter_values"]["itemid"].append(str(entry["appearance_id"]))
        with self.assertRaisesRegex(ValueError, "DUPLICATE"):
            nav.wiki_priority(entry, [])
        with self.assertRaisesRegex(ValueError, "RETAINED_PRIMARY"):
            nav.wiki_priority(original, [{"fields": {"primarytype": "Others"}}])


if __name__ == "__main__":
    unittest.main()
