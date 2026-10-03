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

    def container_override_fixture(self, item_id):
        from quest_reward_item_semantics import CONTAINER_SUPERSESSIONS
        page, revision, digest, name, previous, stackable, capacity = CONTAINER_SUPERSESSIONS[item_id]
        flags = {"flags.cumulative": True} if stackable else {}
        if capacity is not None:
            flags["flags.container"] = True
        entry, appearance, definition = self.fixture(flags, {"containersize": str(previous)})
        entry.update(item_id=item_id, item_key=f"oteryn:item.tibia.i{item_id}", client_name=name)
        appearance["name"] = name
        for row in entry["xml"].values():
            row.update(item_id=item_id, name=name)
        definition["identity"]["key"] = entry["item_key"]
        definition["semantics"] = {
            "container": known({"capacity": known(previous)}),
            "physical": known({"weight": known(40000), "movable": known(True), "pickupable": known(True)}),
            "presentation": known({"name": known(name), "description": known("Keep this exact fact")}),
        }
        fields = {"itemid": str(item_id), "actualname": name, "pickupable": "yes", "immobile": "no"}
        if capacity is None:
            fields["stackable"] = "yes" if stackable else "no"
        else:
            fields["volume"] = str(capacity)
        entry["wiki"] = [{"page_id": page, "revision_id": revision,
                          "content_sha256": digest, "fields": fields}]
        return entry, appearance, definition

    def test_exact_client_and_wiki_remove_two_wrong_container_facts(self):
        for item_id in (235, 25302):
            with self.subTest(item_id=item_id):
                entry, appearance, definition = self.container_override_fixture(item_id)
                facts = item_facts(entry, appearance)
                enrich(definition, facts, entry["xml"])
                self.assertEqual(definition["semantics"]["container"], {"state": "NOT_APPLICABLE"})
                self.assertTrue(definition["materializable"])
                self.assertIsNone(facts["capacity"])
                self.assertEqual(facts["holds"], [])
                self.assertEqual(set(facts["container_supersession"]["xml_capacity_conflict_witness"].values()),
                                 {"8" if item_id == 235 else "5"})

    def test_exact_wiki_capacity22_supersedes_donor20_without_inventing_runtime_support(self):
        entry, appearance, definition = self.container_override_fixture(53074)
        del entry["xml"]["canary"]
        entry["owning_donors"] = ["crystal", "crystal-summer"]
        facts = item_facts(entry, appearance)
        enrich(definition, facts, entry["xml"])
        self.assertEqual(definition["semantics"]["container"], known({"capacity": known(22)}))
        self.assertEqual(facts["capacity"], 22)
        self.assertTrue(definition["materializable"])
        self.assertNotIn("runtime_ready", facts)

    def test_supersession_is_idempotent_and_preserves_identity_and_other_known_facts(self):
        for item_id in (235, 25302, 53074):
            entry, appearance, definition = self.container_override_fixture(item_id)
            identity = copy.deepcopy(definition["identity"])
            physical = copy.deepcopy(definition["semantics"]["physical"])
            presentation = copy.deepcopy(definition["semantics"]["presentation"])
            facts = item_facts(entry, appearance)
            enrich(definition, facts, entry["xml"])
            frozen = canonical(definition)
            enrich(definition, facts, entry["xml"])
            self.assertEqual(canonical(definition), frozen)
            self.assertEqual(definition["identity"], identity)
            self.assertEqual(definition["semantics"]["physical"], physical)
            self.assertEqual(definition["semantics"]["presentation"], presentation)

    def test_nonmatching_wiki_cut_retains_original_hold(self):
        for field, replacement in (("revision_id", 1), ("content_sha256", "0" * 64)):
            entry, appearance, _ = self.container_override_fixture(235)
            entry["wiki"][0][field] = replacement
            facts = item_facts(entry, appearance)
            self.assertNotIn("container_supersession", facts)
            self.assertIn("CONTAINER_KIND_CLIENT_XML_CONFLICT", facts["holds"])

    def test_duplicate_exact_wiki_cut_is_rejected(self):
        entry, appearance, _ = self.container_override_fixture(235)
        entry["wiki"].append(copy.deepcopy(entry["wiki"][0]))
        with self.assertRaisesRegex(ValueError, "CONTAINER_SUPERSESSION_DUPLICATE_WIKI"):
            item_facts(entry, appearance)

    def test_supersession_rejects_changed_source_identity_applicability_or_volume(self):
        for change in ("wrong_item", "wrong_name", "client_container", "client_stack", "volume_zero", "xml"):
            entry, appearance, _ = self.container_override_fixture(235)
            if change == "wrong_item":
                entry["wiki"][0]["fields"]["itemid"] = "2853"
            elif change == "wrong_name":
                entry["wiki"][0]["fields"]["actualname"] = "bag variant"
            elif change == "client_container":
                appearance["flags"]["flags.container"] = True
            elif change == "client_stack":
                appearance["flags"]["flags.cumulative"] = True
            elif change == "volume_zero":
                entry["wiki"][0]["fields"]["volume"] = "0"
            else:
                entry["xml"]["canary"]["attrs"]["containersize"] = "9"
            with self.assertRaisesRegex(ValueError, "CONTAINER_SUPERSESSION_(SOURCE|XML)_CHANGED"):
                item_facts(entry, appearance)

    def test_supersession_rejects_unknown_changed_or_extra_known_base_container_fields(self):
        for current in (UNKNOWN, known({"capacity": known(9)}),
                        known({"capacity": known(8), "unexpected": known(1)})):
            entry, appearance, definition = self.container_override_fixture(235)
            definition["semantics"]["container"] = copy.deepcopy(current)
            with self.assertRaisesRegex(ValueError, "CONTAINER_SUPERSESSION_BASE_CHANGED"):
                enrich(definition, item_facts(entry, appearance), entry["xml"])

    def test_supersession_cannot_be_replayed_against_another_item_or_definition_revision(self):
        entry, appearance, definition = self.container_override_fixture(235)
        facts = item_facts(entry, appearance)
        for field, replacement in (("key", "oteryn:item.tibia.i2853"), ("family", "Creature"),
                                   ("revision", "definition-r2")):
            candidate = copy.deepcopy(definition)
            candidate["identity"][field] = replacement
            with self.assertRaisesRegex(ValueError, "(CONTAINER_SUPERSESSION_IDENTITY|ITEM_IDENTITY_SCOPE)"):
                enrich(candidate, facts, entry["xml"])

    def test_supersession_proof_cannot_change_its_target_or_known_base(self):
        entry, appearance, definition = self.container_override_fixture(235)
        facts = item_facts(entry, appearance)
        for field, replacement in (("item_id", 1), ("previous_capacity", 9),
                                   ("replacement", known({"capacity": known(0)}))):
            candidate = copy.deepcopy(facts)
            candidate["container_supersession"][field] = replacement
            with self.assertRaisesRegex(ValueError, "CONTAINER_SUPERSESSION_PROOF"):
                enrich(copy.deepcopy(definition), candidate, entry["xml"])

    def test_committed_i901_whole_definition_is_preserved_even_with_a_foreign_supersession(self):
        root = Path(__file__).resolve().parents[2]
        packet = json.loads((root / EXTRA_PACKET).read_bytes())
        entry = next(e for e in packet["admissions"] if e["item_id"] == 901)
        shard = json.loads((root / "content/items/definitions/items-32500-32999.json").read_bytes())
        definition = next(r["definition"] for r in shard["records"]
                          if r["definition"]["identity"]["key"] == entry["item_key"])
        frozen = canonical(definition)
        facts = item_facts(entry, {"name": entry["client_name"], "flags": entry["client_flags"]})
        foreign, appearance, _ = self.container_override_fixture(235)
        facts["container_supersession"] = item_facts(foreign, appearance)["container_supersession"]
        enrich(definition, facts, entry["xml"])
        self.assertEqual(canonical(definition), frozen)

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
