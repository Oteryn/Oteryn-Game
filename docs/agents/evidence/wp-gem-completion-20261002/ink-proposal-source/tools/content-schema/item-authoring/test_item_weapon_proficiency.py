"""Tests for item_weapon_proficiency (ITEM-PROF-1, ITEM-PROF-1b). Run with `python test_item_weapon_proficiency.py`."""

import copy
import os
import tempfile
from pathlib import Path
from unittest.mock import patch

import json
import unittest

import item_weapon_proficiency as iwp


class ThresholdClassTests(unittest.TestCase):
    def test_bolt_ammunition_is_crossbow_even_in_a_shared_profile(self):
        shared = "Generic 2H Distance"  # crossbow 3349, bow 3350 share this profile
        self.assertEqual(iwp.threshold_class(shared, "bolt"), "crossbow")
        self.assertEqual(iwp.threshold_class(shared, "arrow"), "standard")
        # D198: profile names without a Crossbow word are still crossbows on bolt evidence
        for name in (
            "Distance 2H Arbalest",
            "Distance 2H Chain Bolter",
            "Distance 2H The Devileye",
        ):
            self.assertEqual(iwp.threshold_class(name, "bolt"), "crossbow")

    def test_ranged_without_ammunition_evidence_is_unknown(self):
        self.assertEqual(
            iwp.threshold_class("Replica Mayhem Distance", None), "unknown"
        )
        self.assertEqual(iwp.threshold_class("Amber 2H Crossbow", None), "unknown")

    def test_ranged_without_ammunition_uses_wiki_secondarytype(self):
        replica = "Replica Mayhem Distance"
        self.assertEqual(
            iwp.threshold_class(replica, None, None, "Crossbows"), "crossbow"
        )
        self.assertEqual(iwp.threshold_class(replica, None, None, "Bows"), "standard")
        # ammunition evidence decides first
        self.assertEqual(
            iwp.threshold_class("Generic 2H Distance", "bolt", None, None), "crossbow"
        )

    def test_knight_only_for_knight_restricted_melee(self):
        sword = "Sword 1H Crimson Sword"
        self.assertEqual(iwp.threshold_class(sword, None, "Knights"), "knight")
        self.assertEqual(
            iwp.threshold_class(sword, None, "players without vocation"), "standard"
        )
        self.assertEqual(iwp.threshold_class(sword, None, "Monks"), "standard")
        self.assertEqual(iwp.threshold_class(sword, None, None), "unknown")
        self.assertEqual(
            iwp.threshold_class("Grand Sanguine 2H Axe", None, "Knights"), "knight"
        )

    def test_unrestricted_melee_is_standard(self):
        self.assertEqual(
            iwp.threshold_class("Generic 1H Sword Class 1", None, "unrestricted"),
            "standard",
        )
        self.assertEqual(
            iwp.threshold_class("Sword 1H Bright Sword", None, "None"), "standard"
        )
        self.assertEqual(
            iwp.threshold_class("Axe 1H X", None, "Knight;true, Elite Knight"),
            "knight",
        )

    def test_vocation_evidence_source_order(self):
        weapon = {"name": "x", "weapon": True, "vocation": []}
        knight = {"name": "x", "weapon": True, "vocation": ["Knight;true"]}
        proven = iwp.vocation_evidence({"vocrequired": ["knights"]}, weapon)
        self.assertEqual((proven["basis"], proven["value"]), ("PROVEN", "knights"))
        derived = iwp.vocation_evidence({}, knight)
        self.assertEqual(
            (derived["basis"], derived["value"]), ("DERIVED", "Knight;true")
        )
        free = iwp.vocation_evidence({}, weapon)
        self.assertEqual((free["basis"], free["value"]), ("DERIVED", "unrestricted"))
        for wiki, crystal in (
            ({}, None),
            ({}, {"name": "x", "weapon": False, "vocation": []}),
            ({"vocrequired": ["knights", "paladins"]}, knight),
        ):
            evidence = iwp.vocation_evidence(wiki, crystal)
            self.assertEqual(evidence["basis"], "UNKNOWN")
            self.assertNotIn("value", evidence)
            self.assertTrue(evidence["reason"])

    def test_other_weapons_are_standard(self):
        self.assertEqual(iwp.threshold_class("Wand 1H Wand of Decay"), "standard")
        self.assertEqual(iwp.threshold_class("Throw - Small Stone"), "standard")
        self.assertEqual(
            iwp.threshold_class("Fist 1H Light Jo Staff", None, "Monks"), "standard"
        )

    def test_threshold_tables_have_nine_increasing_levels(self):
        self.assertEqual(set(iwp.THRESHOLDS), {"standard", "knight", "crossbow"})
        self.assertEqual(set(iwp.CLASSES), set(iwp.THRESHOLDS) | {"unknown"})
        for table in iwp.THRESHOLDS.values():
            self.assertEqual(len(table), 9)
            self.assertEqual(table, sorted(table))


class ArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = iwp.build()

    def test_counts_and_keys(self):
        document = self.document
        self.assertEqual(len(document["profiles"]), iwp.EXPECTED_PROFILES)
        self.assertEqual(len(document["bindings"]), iwp.EXPECTED_BINDINGS)
        keys = [b["item_key"] for b in document["bindings"]]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertTrue(all(k.startswith("oteryn:item.tibia.i") for k in keys))
        ids = {p["proficiency_id"] for p in document["profiles"]}
        self.assertTrue(all(b["proficiency_id"] in ids for b in document["bindings"]))
        self.assertEqual(document["counts"]["referenced_profiles"], 430)

    def test_bindings_are_classified_per_item(self):
        by_id = {b["client_object_id"]: b for b in self.document["bindings"]}
        self.assertEqual(by_id[3349]["threshold_class"], "crossbow")  # shared profile
        self.assertEqual(
            by_id[3350]["threshold_class"], "standard"
        )  # bow, same profile
        self.assertEqual(by_id[5803]["threshold_class"], "crossbow")  # arbalest
        # replica crossbow / bow: TibiaWiki secondarytype
        self.assertEqual(by_id[26004]["threshold_class"], "crossbow")
        self.assertEqual(by_id[26067]["threshold_class"], "crossbow")
        self.assertEqual(by_id[26001]["threshold_class"], "standard")
        self.assertEqual(by_id[53227]["threshold_class"], "crossbow")  # moonsilver
        self.assertEqual(by_id[3265]["threshold_class"], "knight")  # two handed sword
        # short sword: no TibiaWiki vocrequired, unrestricted Crystal weapon (DERIVED)
        self.assertEqual(by_id[3294]["threshold_class"], "standard")
        self.assertEqual(
            by_id[3294]["threshold_evidence"]["vocation"]["basis"], "DERIVED"
        )
        # ink sword: no evidence in either source stays unknown with a reason
        ink = by_id[51666]
        self.assertEqual(ink["threshold_class"], "unknown")
        self.assertEqual(ink["threshold_evidence"]["vocation"]["basis"], "UNKNOWN")
        counts = self.document["counts"]["bindings_by_threshold_class"]
        self.assertEqual(sum(counts.values()), iwp.EXPECTED_BINDINGS)
        self.assertEqual(
            counts, {"crossbow": 37, "knight": 158, "standard": 470, "unknown": 1}
        )
        for binding in self.document["bindings"]:
            vocation = binding["threshold_evidence"]["vocation"]
            self.assertIn(vocation["basis"], {"PROVEN", "DERIVED", "UNKNOWN"})
            if vocation["basis"] == "UNKNOWN":
                self.assertNotIn("value", vocation)

    def test_perk_mapping_covers_every_raw_value(self):
        mapping = self.document["perk_mapping"]
        for name, key in (
            ("Type", "Type"),
            ("SkillId", "SkillId"),
            ("AugmentType", "AugmentType"),
        ):
            mapped = {row["value"] for row in mapping[key]}
            raw = {row["value"] for row in self.document["perk_raw_enums"][name]}
            self.assertLessEqual(raw, mapped, name)
        combat = {row["value"] for row in mapping["ElementId_DamageType"]}
        for name in ("ElementId", "DamageType"):
            raw = {row["value"] for row in self.document["perk_raw_enums"][name]}
            self.assertLessEqual(raw, combat, name)
        self.assertNotIn(-1, {row["value"] for row in mapping["Type"]})
        by_type = {row["value"]: row for row in mapping["Type"]}
        self.assertEqual(by_type[13 + 6]["meaning"], "hit points on hit")
        self.assertEqual(by_type[22]["unit"], "flat_tiles")

    def test_mastery_is_top_level_plus_two(self):
        for profile in self.document["profiles"]:
            self.assertEqual(profile["top_level"], len(profile["levels"]))
            self.assertEqual(profile["mastery_level"], profile["top_level"] + 2)
            self.assertLessEqual(profile["mastery_level"], 9)

    def test_committed_file_is_current_and_valid_json(self):
        text = iwp.render(self.document)
        self.assertEqual(json.loads(text)["schema"], iwp.SCHEMA)
        self.assertEqual(iwp.DEFAULT_OUTPUT.read_text(encoding="utf-8"), text)


class SupplementTests(unittest.TestCase):
    # Fixture invocation is prospective only; it does not establish owner admission.
    SAMPLES = Path(__file__).resolve().parent / "samples"
    MANIFEST = SAMPLES / "weapon-threshold-tibiopedia-ink-manifest.json"

    @classmethod
    def setUpClass(cls):
        cls.baseline = iwp.build()
        cls.manifest = json.loads(cls.MANIFEST.read_text())
        cls.observations = json.loads((cls.SAMPLES / cls.manifest["observations"]["path"]).read_text())
        cls.source_directory = tempfile.TemporaryDirectory()
        cls.source_html = Path(cls.source_directory.name) / "source.html"
        supplied = os.environ.get("OTERYN_THRESHOLD_SUPPLEMENT_SOURCE_HTML")
        cls.source_html.write_bytes(Path(supplied).read_bytes() if supplied
                                    else b"synthetic unit-test source bytes")
        if not supplied:  # Synthetic pin for portable no-network unit fixtures only.
            digest = iwp.sha256_bytes(cls.source_html.read_bytes())
            cls.manifest["source"]["html_sha256"] = digest
            cls.observations["source"]["html_sha256"] = digest

    @classmethod
    def tearDownClass(cls):
        cls.source_directory.cleanup()

    def candidate(self, manifest=None, observations=None, html=None):
        # Re-pin mutated fixtures to exercise semantic rejection beyond digest checking.
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            package = copy.deepcopy(self.manifest if manifest is None else manifest)
            facts = self.observations if observations is None else observations
            raw = (json.dumps(facts, ensure_ascii=False) + "\n").encode()
            (base / package["observations"]["path"]).write_bytes(raw)
            package["observations"]["sha256"] = iwp.sha256_bytes(raw)
            path = base / "manifest.json"
            path.write_text(json.dumps(package))
            return iwp.build(path, iwp.sha256_bytes(path.read_bytes()), html or self.source_html)

    def test_prospective_fixture_changes_only_ink_binding_and_derived_counts(self):
        actual = self.candidate()
        self.assertEqual(actual["profiles"], self.baseline["profiles"])  # all 443, raw perks
        for key in ("perk_mapping", "perk_raw_enums", "thresholds", "threshold_source"):
            self.assertEqual(actual[key], self.baseline[key])
        changed = [(a, b) for a, b in zip(actual["bindings"], self.baseline["bindings"]) if a != b]
        self.assertEqual(len(changed), 1)
        ink = changed[0][0]
        self.assertEqual(ink["client_object_id"], 51666)
        self.assertEqual(ink["threshold_class"], "standard")
        self.assertEqual(ink["threshold_evidence"]["vocation"]["value"], "unrestricted")
        self.assertEqual(actual["counts"]["bindings_by_threshold_class"],
                         {"crossbow": 37, "knight": 158, "standard": 471})
        profile = next(p for p in actual["profiles"] if p["proficiency_id"] == 371)
        self.assertTrue(any(perk.get("Type") == 20 and perk.get("Value") == 8
                            for level in profile["levels"] for perk in level["Perks"]))
        for key in set(actual) - {"bindings", "counts"}:
            self.assertEqual(actual[key], self.baseline[key])

    def test_missing_external_pin_and_wrong_bytes_rejected(self):
        args = (self.MANIFEST, iwp.sha256_bytes(self.MANIFEST.read_bytes()), self.source_html)
        for index in range(3):
            bad = list(args)
            bad[index] = None
            with self.subTest(index=index), self.assertRaises(SystemExit):
                iwp.build(*bad)
        with self.assertRaises(SystemExit):
            iwp.build(self.MANIFEST, "0" * 64, self.source_html)
        with tempfile.TemporaryDirectory() as directory:
            html = Path(directory) / "source.html"
            html.write_bytes(b"wrong source bytes")
            with self.assertRaises(SystemExit):
                self.candidate(html=html)
            manifest = Path(directory) / "manifest.json"
            manifest.write_bytes(self.MANIFEST.read_bytes())
            (Path(directory) / self.manifest["observations"]["path"]).write_bytes(b"{}")
            with self.assertRaises(SystemExit):
                iwp.build(manifest, args[1], self.source_html)

    def test_manifest_source_and_identity_mismatches_rejected(self):
        for key, value in (("schema", "wrong"), ("scope", "perks"),
                           ("threshold_source", {}), ("client_inputs", {})):
            manifest = copy.deepcopy(self.manifest)
            manifest[key] = value
            with self.subTest(key=key), self.assertRaises(SystemExit):
                self.candidate(manifest=manifest)
        for key, value in (("url", "https://tibiopedia.pl/items/Other"),
                           ("provider", "unadmitted"), ("locale", "en"),
                           ("captured_at", "2026-10-01"), ("html_sha256", "0" * 64)):
            manifest, facts = copy.deepcopy(self.manifest), copy.deepcopy(self.observations)
            manifest["source"][key] = facts["source"][key] = value
            with self.subTest(source=key), self.assertRaises(SystemExit):
                self.candidate(manifest, facts)
        facts = copy.deepcopy(self.observations)
        facts["source"]["url"] += "?wrong"
        with self.assertRaises(SystemExit):
            self.candidate(observations=facts)
        for key, value in (("client_object_id", 3294), ("item_key", "wrong"),
                           ("proficiency_id", 238), ("profile_name", "wrong"),
                           ("client_name", "wrong")):
            facts = copy.deepcopy(self.observations)
            facts["item"][key] = value
            with self.subTest(item=key), self.assertRaises(SystemExit):
                self.candidate(observations=facts)

    def test_vocation_levels_and_mastery_must_corroborate_table(self):
        for key, value in (("vocation_literal", None), ("vocation_literal", ""),
                           ("vocation_literal", "absent restriction"),
                           ("vocation_literal", "Knight"), ("displayed_levels", [1, 2, 3]),
                           ("displayed_levels", [1, 2, 4, 3]), ("mastery_points", 6000000),
                           ("mastery_points", 2500000), ("mastery_points", "8000000")):
            facts = copy.deepcopy(self.observations)
            facts["observations"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(SystemExit):
                self.candidate(observations=facts)
        tables = copy.deepcopy(iwp.THRESHOLDS)
        tables["knight"][5] = tables["standard"][5]
        with patch.object(iwp, "THRESHOLDS", tables), self.assertRaises(SystemExit):
            self.candidate()

    def test_existing_evidence_and_wiki_disagreement_keep_precedence(self):
        for wiki, crystal in (({"vocrequired": ["Knights"]}, None),
                              ({"vocrequired": ["Monks"]}, None),
                              ({}, {"weapon": True, "vocation": []}),
                              ({}, {"weapon": True, "vocation": ["Knight;true"]}),
                              ({"vocrequired": ["Knights", "Monks"]}, None)):
            meta, records = iwp.wiki_stats()
            records[51666] = wiki
            crystal_records = iwp.crystal_vocations()
            crystal_records[51666] = crystal
            with patch.object(iwp, "wiki_stats", return_value=(meta, records)), \
                 patch.object(iwp, "crystal_vocations", return_value=crystal_records):
                candidate = self.candidate()
            ink = next(b for b in candidate["bindings"] if b["client_object_id"] == 51666)
            self.assertEqual(ink["threshold_evidence"]["vocation"], iwp.vocation_evidence(wiki, crystal))

    def test_normal_cli_uses_supplement_and_check(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "candidate.json"
            manifest = Path(directory) / "manifest.json"
            facts = json.dumps(self.observations, ensure_ascii=False).encode()
            (Path(directory) / self.manifest["observations"]["path"]).write_bytes(facts)
            package = copy.deepcopy(self.manifest)
            package["observations"]["sha256"] = iwp.sha256_bytes(facts)
            manifest.write_text(json.dumps(package))
            args = ["--output", str(output), "--supplement-manifest", str(manifest),
                    "--supplement-manifest-sha256", iwp.sha256_bytes(manifest.read_bytes()),
                    "--supplement-source-html", str(self.source_html)]
            self.assertEqual(iwp.main(args), 0)
            self.assertEqual(iwp.main(args + ["--check"]), 0)
            ink = next(b for b in json.loads(output.read_text())["bindings"]
                       if b["client_object_id"] == 51666)
            self.assertEqual(ink["threshold_class"], "standard")


if __name__ == "__main__":
    unittest.main()
