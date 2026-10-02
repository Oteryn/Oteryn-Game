"""Check proposed Items against the owning contract and admission boundary."""
from copy import deepcopy
import json
import unittest
from unittest.mock import patch

import missing_item_proposals as proposals


class MissingItemProposalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = proposals.build()

    def test_both_proposals_pass_owning_validator_with_no_warnings(self):
        self.assertEqual(len(self.packet["proposals"]), 2)
        for row in self.packet["proposals"]:
            self.assertEqual(row["identity_state"], "PROPOSED_NOT_REGISTERED")
            self.assertEqual(row["validation"]["errors"], [])
            self.assertEqual(row["validation"]["warnings"], [])

    def test_both_items_are_included_in_current_october_target(self):
        rows = {row["source_client_id"]: row for row in self.packet["proposals"]}
        self.assertEqual(self.packet["target"], "global-tibia-current-2026-10-01")
        self.assertEqual(self.packet["target_date"], "2026-10-01")
        for row in rows.values():
            self.assertEqual(row["target_time_status"], "PREEXISTING_TARGET_SOURCE_VERSION")
            self.assertTrue(row["current_target_included"])
        self.assertEqual(rows[53192]["source_facts"]["facts"]["introduced_on"], "2026-08-04")
        self.assertEqual(rows[49160]["authoring_definition"]["container"]["capacity"], 1)
        self.assertEqual(rows[49160]["source_facts"]["facts"]["introduced_on"], "2024-08-06")

    def test_future_introduction_cannot_be_qualified_for_current_target(self):
        self.assertEqual(proposals.qualify_current_target("2026-10-01", "2026-10-01"),
                         "PREEXISTING_TARGET_SOURCE_VERSION")
        for introduced in ("2026-10-02", "2027-01-01"):
            with self.subTest(introduced=introduced):
                with self.assertRaisesRegex(ValueError, "after the current target"):
                    proposals.qualify_current_target(introduced, "2026-10-01")
        for introduced in ("2026-02-30", "2026-8-4", "unknown"):
            with self.subTest(introduced=introduced):
                with self.assertRaises(ValueError):
                    proposals.qualify_current_target(introduced, "2026-10-01")

    def test_revision_links_match_named_wiki_identity(self):
        for source in json.loads(proposals.FACTS.read_bytes())["records"]:
            proposals.validate_source_provenance(source)
            wiki = source["sources"]["wiki_br"]
            self.assertNotIn("](", wiki["revision_url"])
            self.assertTrue(wiki["revision_url"].endswith("oldid=" + str(wiki["revision_id"])))

    def test_rejects_malformed_or_misbound_provenance_links(self):
        original = json.loads(proposals.FACTS.read_bytes())["records"][0]
        mutations = [
            ("wiki_br", "revision_url", original["sources"]["wiki_br"]["revision_url"] + "](https://example.com)"),
            ("wiki_br", "revision_url", "https://www.tibiawiki.com.br/index.php?title=Bursa_Obscura&oldid=442016"),
            ("wiki_br", "revision_url", "https://www.tibiawiki.com.br/index.php?title=Sailor%27s_Backpack&oldid=427070"),
            ("wiki_br", "revision_url", "https://example.com/index.php?title=Bursa_Obscura&oldid=427070"),
            ("wiki_br", "revision_url", "https://www.tibiawiki.com.br/index.php?title=Bursa_Obscura&oldid=427070&oldid=427070"),
            ("wiki_br", "url", "https://www.tibiawiki.com.br/wiki/Another_Item"),
            ("tibiopedia", "url", "https://tibiopedia.pl/items/Bursa_Obscura#fake"),
            ("tibiopedia", "sha256", "missing"),
            ("tibiopedia_update", "url", "https://tibiopedia.pl/updates/13.41.a953fd"),
            ("tibiopedia_update", "observed_item_url", "https://tibiopedia.pl/items/Another_Item"),
            ("tibiopedia_update", "published_on", "2024-08-07"),
            ("tibiopedia_update", "observed_publication_date", "07.08.2024"),
        ]
        for provider, field, value in mutations:
            with self.subTest(provider=provider, field=field, value=value):
                source = deepcopy(original)
                source["sources"][provider][field] = value
                with self.assertRaises(ValueError):
                    proposals.validate_source_provenance(source)

    def test_raw_client_slot_count_corroborates_both_authored_items(self):
        for row in self.packet["proposals"]:
            self.assertEqual(row["primary_client"]["imbuement_slots"], 1)
            self.assertEqual(row["primary_client"]["imbuement_slots"],
                             row["authoring_definition"]["imbuement"]["slot_count"])

    def test_owning_contract_rejects_impossible_container_capacity(self):
        import validate_item
        row = deepcopy(self.packet["proposals"][0])
        row["authoring_definition"]["container"]["capacity"] = 0
        errors, _ = validate_item.validate(row["authoring_definition"], row["dependencies"])
        self.assertTrue(errors)

    def test_proposal_cannot_silently_overwrite_admitted_identity(self):
        with patch.object(proposals, "canonical_items", return_value={"oteryn:item.tibia.i49160": {}}):
            with self.assertRaisesRegex(ValueError, "already admitted"):
                proposals.build()

    def test_test_labels_do_not_assert_obtainability(self):
        self.assertEqual(len(self.packet["test_item_evidence"]), 4)
        for row in self.packet["test_item_evidence"]:
            self.assertIn("AVAILABILITY_UNPROVEN", row["classification"])
            self.assertTrue(row["wiki_observations"])
            self.assertTrue(row["primary_client"]["flags"]["flags.take"])

    def test_every_authored_and_acquisition_fact_has_qualified_excerpt(self):
        for row in self.packet["proposals"]:
            source = row["source_facts"]
            expected = {f"{section}.{field}" for section in
                        ("facts", "acquisition_facts", "lifecycle_facts")
                        for field in source.get(section, {})}
            self.assertEqual(set(source["field_evidence"]), expected)
            self.assertEqual(proposals.digest(source["field_evidence"]),
                             source["field_evidence_sha256"])

    def test_missing_misbound_and_changed_fact_evidence_is_rejected(self):
        original = json.loads(proposals.FACTS.read_bytes())["records"][0]
        for mutation in ("missing", "wrong_value", "wrong_provider", "empty_quote", "not_observed"):
            with self.subTest(mutation=mutation):
                source = deepcopy(original)
                claim = source["field_evidence"]["facts.container_capacity"]
                if mutation == "missing":
                    del source["field_evidence"]["facts.container_capacity"]
                elif mutation == "wrong_value":
                    claim["observed_value"] = 24
                elif mutation == "wrong_provider":
                    claim["source"] = "invented_provider"
                elif mutation == "empty_quote":
                    claim["literal_excerpt"] = " "
                else:
                    source["sources"][claim["source"]]["observed_fields"].remove("facts.container_capacity")
                source["field_evidence_sha256"] = proposals.digest(source["field_evidence"])
                with self.assertRaises(ValueError):
                    proposals.validate_source_provenance(source)

    def test_selected_evidence_digest_detects_unrecorded_changes(self):
        source = deepcopy(json.loads(proposals.FACTS.read_bytes())["records"][0])
        source["field_evidence"]["facts.container_capacity"]["literal_excerpt"] += " changed"
        with self.assertRaisesRegex(ValueError, "evidence digest"):
            proposals.validate_source_provenance(source)

    def test_primary_slot_and_trade_flags_reject_source_conflicts(self):
        original = json.loads(proposals.FACTS.read_bytes())
        for field, value, expected_error in (
            ("imbuement_slots", 2, "primary field 60"),
            ("marketable", False, "primary market flag"),
            ("stackable", True, "primary cumulative flag"),
        ):
            with self.subTest(field=field):
                facts = deepcopy(original)
                source = facts["records"][0]
                source["facts"][field] = value
                source["field_evidence"]["facts." + field]["observed_value"] = value
                source["field_evidence_sha256"] = proposals.digest(source["field_evidence"])
                from pathlib import Path
                import tempfile
                with tempfile.TemporaryDirectory() as folder:
                    path = Path(folder) / "facts.json"
                    path.write_text(json.dumps(facts))
                    with patch.object(proposals, "FACTS", path):
                        with self.assertRaisesRegex(ValueError, expected_error):
                            proposals.build()


if __name__ == "__main__":
    unittest.main()
