"""Regression checks for fail-closed reward Item data admission."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from quest_reward_item_semantics import canonical, CLIENT_SHA, PACKET, EXTRA_PACKET, SNAPSHOT, EXTRA_SNAPSHOT, load_packet, enrich, item_facts, known, UNKNOWN, wiki_object_fields, verify_wiki


class RewardItemAdmissionTests(unittest.TestCase):
    def fixture(self, flags=None, attrs=None):
        flags = {"flags.take": True, **(flags or {})}
        entry = {"item_id": 1, "client_name": "fixture", "client_flags": flags,
            "owning_donors": ["canary", "crystal", "crystal-summer"],
            "xml": {source: {"item_id": 1, "name": "fixture", "attrs": attrs or {}}
                for source in ("canary", "crystal", "crystal-summer")}}
        appearance = {"name": "fixture", "flags": flags}
        definition = {"identity": {"family": "Item", "key": "oteryn:item.tibia.i1", "revision": "definition-r1"},
            "kind": "Item", "materializable": False, "stack_class": "Unknown"}
        return entry, appearance, definition

    def test_starter_admission_namespace_is_not_a_quest_core_packet(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for packet_path, snapshot_path in ((PACKET, SNAPSHOT), (EXTRA_PACKET, EXTRA_SNAPSHOT)):
                path = root / packet_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(json.dumps({"schema": "OTERYN_ITEM_ADMISSION/v1", "client_sha256": CLIENT_SHA}))
                with self.assertRaisesRegex(ValueError, "ADMISSION_PIN"):
                    load_packet(root, packet_path, snapshot_path)

    def test_default_false_is_only_the_pinned_proto_getter_derivation(self):
        entry, appearance, definition = self.fixture()
        facts = item_facts(entry, appearance)
        enrich(definition, facts, entry["xml"])
        self.assertEqual(definition["stack_class"], "NonStackable")
        self.assertEqual(definition["semantics"]["stack"]["value"]["stack_max"], UNKNOWN)
        self.assertTrue(definition["materializable"])

    def test_b3_stack_maximum_is_100(self):
        entry, appearance, definition = self.fixture({"flags.cumulative": True})
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        self.assertEqual(definition["stack_class"], "StackCapable")
        self.assertEqual(definition["semantics"]["stack"]["value"]["stack_max"], known(100))

    def test_missing_take_or_immovable_rejects_admission(self):
        for flags in ({"flags.take": False}, {"flags.unmove": True}):
            entry, appearance, _ = self.fixture(flags)
            with self.assertRaisesRegex(ValueError, "NON_PORTABLE"):
                item_facts(entry, appearance)

    def test_counterfeit_client_flags_fail(self):
        entry, appearance, _ = self.fixture()
        entry["client_flags"] = {"flags.take": True, "flags.cumulative": True}
        with self.assertRaisesRegex(ValueError, "CLIENT_FLAGS_MISMATCH"):
            item_facts(entry, appearance)

    def test_source_itemid_mismatch_fails(self):
        entry, appearance, _ = self.fixture()
        entry["xml"]["crystal"]["item_id"] = 2
        with self.assertRaisesRegex(ValueError, "XML_ID_MISMATCH"):
            item_facts(entry, appearance)

    def test_absent_canary_is_unknown_when_crystal_proves_the_exact_item(self):
        entry, appearance, definition = self.fixture({"flags.container": True}, {"containersize": "32"})
        del entry["xml"]["canary"]
        entry["owning_donors"] = ["crystal", "crystal-summer"]
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        self.assertTrue(definition["materializable"])
        self.assertEqual(definition["semantics"]["container"]["value"]["capacity"], known(32))

    def test_zero_or_counterfeit_owning_donors_cannot_prove_an_item(self):
        entry, appearance, _ = self.fixture()
        entry["owning_donors"] = ["crystal"]
        with self.assertRaisesRegex(ValueError, "XML_OWNING_DONORS"):
            item_facts(entry, appearance)
        entry["xml"] = {}
        with self.assertRaisesRegex(ValueError, "XML_SOURCE_SET"):
            item_facts(entry, appearance)

    def test_fresh_exact_wiki_volume_conflict_cannot_be_admitted(self):
        entry, appearance, definition = self.fixture({"flags.container": True}, {"containersize": "20"})
        entry["wiki"] = [{"fields": {"volume": "22"}}]
        facts = item_facts(entry, appearance)
        enrich(definition, facts, entry["xml"])
        self.assertFalse(definition["materializable"])
        self.assertIsNone(facts["capacity"])
        self.assertIn("WIKI_XML_CONTAINER_CAPACITY_CONFLICT", facts["holds"])

    def test_nested_wiki_notes_cannot_supply_the_outer_item_id(self):
        text = "{{Infobox Object\n|name=Outer\n|notes={{Infobox Object\n|itemid=53074\n|volume=22\n}}\n}}"
        fields = wiki_object_fields(text)
        self.assertNotIn("itemid", fields)
        self.assertNotIn("volume", fields)

    def test_wiki_revision_and_exact_item_identity_are_mandatory(self):
        import hashlib
        text = "{{Infobox Object\n|itemid=2\n|volume=22\n}}"
        page = {"pageid": 1, "revisions": [{"revid": 5, "timestamp": "2026-10-01", "slots": {"main": {"*": text}}}]}
        observation = {"page_id": 1, "revision_id": 5, "revision_timestamp": "2026-10-01",
            "content_sha256": hashlib.sha256(text.encode()).hexdigest(), "fields": wiki_object_fields(text)}
        with self.assertRaisesRegex(ValueError, "WIKI_FACT_BINDING"):
            verify_wiki([observation], {(1, 5): page}, 1)
        with self.assertRaisesRegex(ValueError, "WIKI_REVISION_MISSING"):
            verify_wiki([observation], {}, 2)

    def test_charged_nonstack_is_described_without_new_admission(self):
        entry, appearance, definition = self.fixture(attrs={"charges": "5"})
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        self.assertFalse(definition["materializable"])
        self.assertEqual(definition["semantics"]["charges"]["value"]["count"], known(5))

    def test_existing_charge_admission_is_retained_without_clearing_hold(self):
        entry, appearance, definition = self.fixture(attrs={"charges": "5"})
        definition["materializable"] = True
        facts = item_facts(entry, appearance)
        enrich(definition, facts, entry["xml"])
        self.assertTrue(definition["materializable"])
        self.assertIn("INSTANCE_CHARGE_ATTRIBUTE_LOWERING_MISSING", facts["holds"])

    def test_fluid_subtype_is_not_fabricated(self):
        entry, appearance, definition = self.fixture({"flags.liquidcontainer": True})
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        self.assertFalse(definition["materializable"])
        self.assertNotIn("fluid", definition["semantics"])

    def test_container_requires_client_kind_and_agreed_capacity(self):
        for flags, attrs in (({}, {"containersize": "8"}), ({"flags.container": True}, {})):
            entry, appearance, definition = self.fixture(flags, attrs)
            enrich(definition, item_facts(entry, appearance), entry["xml"])
            self.assertFalse(definition["materializable"])
            self.assertNotIn("container", definition["semantics"])
        entry, appearance, definition = self.fixture({"flags.container": True}, {"containersize": "8"})
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        self.assertEqual(definition["semantics"]["container"]["value"]["capacity"], known(8))

    def test_known_facts_cannot_be_silently_overwritten(self):
        entry, appearance, definition = self.fixture()
        definition["semantics"] = {"physical": known({"pickupable": known(False)})}
        with self.assertRaisesRegex(ValueError, "EXISTING_FACT_CONFLICT"):
            enrich(definition, item_facts(entry, appearance), entry["xml"])

    def test_protected_native_mismatch_preserves_the_whole_accepted_definition(self):
        entry, appearance, definition = self.fixture({"flags.cumulative": True})
        definition["stack_class"] = "NonStackable"
        definition["materializable"] = True
        expected = copy.deepcopy(definition)
        entry["native_core_hold"] = {"accepted_stack_class": "NonStackable",
            "source_stack_class": "StackCapable",
            "accepted_definition_sha256": hashlib.sha256(canonical(definition)).hexdigest()}
        facts = item_facts(entry, appearance)
        self.assertEqual(enrich(definition, facts, entry["xml"]), expected)
        self.assertIn("ACCEPTED_NATIVE_SOURCE_STACK_CLASS_CONFLICT", facts["holds"])
        definition["materializable"] = False
        with self.assertRaisesRegex(ValueError, "PROTECTED_NATIVE_CORE_HOLD_CHANGED"):
            enrich(definition, facts, entry["xml"])

    def test_written_carrier_preserves_missing_bounds_and_transform(self):
        entry, appearance, definition = self.fixture(attrs={"writeable": "1", "writeonceitemid": "2"})
        enrich(definition, item_facts(entry, appearance), entry["xml"])
        facts = definition["semantics"]["readable_writeable"]["value"]
        self.assertEqual(facts["readable"], known(True))
        self.assertEqual(facts["writeable"], known(True))
        self.assertEqual(facts["max_text_length"], UNKNOWN)
        self.assertEqual(facts["write_once_target"], UNKNOWN)
        self.assertNotIn("equipment", definition["semantics"])


if __name__ == "__main__":
    unittest.main()
