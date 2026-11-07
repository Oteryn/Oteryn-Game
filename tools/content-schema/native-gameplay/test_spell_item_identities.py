"""Identity import regression checks: exact binding, not numeric promotion."""
import copy
import hashlib
import json
import unittest
from unittest.mock import patch

import build_spell_item_identities as identity
from build_spell_item_identities import CRYSTAL_PIN, bind_profiles, binding_index
from canary_generic_item_identity import PACKET


class ItemIdentityTests(unittest.TestCase):
    def setUp(self):
        self.row = {"source_key": "oteryn:source.crystalserver", "source_revision": CRYSTAL_PIN,
            "identity_namespace": "ots/item_server_id", "external_id": "3582",
            "target": {"family": "Item", "key": "oteryn:item.tibia.i3582", "revision": "definition-r1"},
            "disposition": "EXACT"}
        self.document = {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [
            {"authoring": {"item": {"family": "Item", "key": "candidate:item/3582", "revision": "spell-p2-r21"}},
                "admission": {"materializable": True}}]}

    def index(self, rows):
        data = json.dumps({"schema": "OTERYN_SOURCE_IDENTITY_BINDINGS/v1", "family": "Item", "bindings": rows}).encode()
        return binding_index(data, hashlib.sha256(data).hexdigest())

    def test_explicit_target_revision_survives_import(self):
        actual = bind_profiles(self.document, self.index([self.row]), {3582})["records"][0]
        self.assertEqual(actual["production_definition"]["revision_ref"], "definition-r1")
        self.assertEqual(actual["production_binding"], self.row)
        self.assertEqual(actual["authoring"]["item"]["revision"], "spell-p2-r21")
        self.assertNotIn("production_binding", self.document["records"][0])

    def test_missing_binding_cannot_be_derived_from_numeric_id(self):
        with self.assertRaises(ValueError):
            bind_profiles(self.document, {}, {3582})

    def test_membership_alone_does_not_supply_source_policy(self):
        with self.assertRaises(ValueError):
            bind_profiles(self.document, self.index([self.row]), set())

    def test_changed_identity_cannot_overwrite_existing_binding(self):
        candidate = copy.deepcopy(self.document)
        candidate["records"][0]["production_definition"] = {"family": "Item", "production_key": "oteryn:item.tibia.i3582", "revision_ref": "unqualified"}
        with self.assertRaises(ValueError):
            bind_profiles(candidate, self.index([self.row]), {3582})

    def test_ambiguous_or_foreign_binding_is_rejected(self):
        with self.assertRaises(ValueError):
            self.index([self.row, self.row])
        changed = copy.deepcopy(self.row)
        changed["target"]["key"] = "oteryn:item.tibia.i3583"
        with self.assertRaises(ValueError):
            self.index([changed])

    def test_binding_digest_is_verified_before_consumption(self):
        with self.assertRaises(ValueError):
            binding_index(b"{}", "0"*64)

    def test_unqualified_revision_does_not_inherit_old_policy(self):
        changed = copy.deepcopy(self.row)
        changed["source_revision"] = "0"*40
        with self.assertRaises(ValueError):
            bind_profiles(self.document, self.index([changed]), {3582})

    def test_independent_source_requires_its_exact_committed_xml_bytes(self):
        changed = copy.deepcopy(self.row)
        changed["source_revision"] = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
        with self.assertRaisesRegex(ValueError, "Unqualified independent Crystal XML pin"):
            bind_profiles(self.document, self.index([changed]), {3582},
                independent_crystal_source=(identity.INDEPENDENT_CRYSTAL_PIN, b"<items/>"))

    def test_independent_membership_does_not_rewrite_an_admitted_identity(self):
        bindings = binding_index(identity.BINDINGS.read_bytes())
        historical = bindings["43963"]
        self.assertEqual(historical["source_revision"], "00ce02a57ca5a12e48f32a3476e37471167e4c3f")
        document = copy.deepcopy(self.document)
        document["records"][0]["authoring"]["item"]["key"] = "candidate:item/43963"
        # Isolate the identity join. The actual capture/check gate separately
        # exercises the exact SHA-pinned source parser with the real XML bytes.
        with patch.object(identity, "independent_crystal_membership", return_value={43963}):
            bound = bind_profiles(document, bindings, {43963},
                independent_crystal_source=(identity.INDEPENDENT_CRYSTAL_PIN, b"fixture"))
        actual = bound["records"][0]
        self.assertEqual(actual["production_binding"], historical)
        self.assertEqual(actual["production_definition"], {"family": "Item",
            "production_key": historical["target"]["key"], "revision_ref": "definition-r1"})
        self.assertIsNone(actual["production_binding_qualification"])
        self.assertNotIn("production_binding", document["records"][0])

    def test_independent_membership_cannot_replace_or_invent_a_binding_revision(self):
        changed = copy.deepcopy(self.row)
        changed["source_revision"] = identity.INDEPENDENT_CRYSTAL_PIN
        with (
            patch.object(identity, "independent_crystal_membership", return_value={3582}),
            self.assertRaisesRegex(ValueError, "independently qualified source revision"),
        ):
            bind_profiles(self.document, self.index([changed]), {3582},
                independent_crystal_source=(identity.INDEPENDENT_CRYSTAL_PIN, b"fixture"))

    def generic(self):
        known = lambda value: {"state": "KNOWN", "value": value}
        return {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [{
            "authoring": {"item": {"family": "Item", "key": "candidate:item/40450", "revision": "spell-p2-r21"}},
            "admission": {"materializable": True, "stack_class": "NonStackable", "legal_destinations": ["Ground"]},
            "semantics": {"physical": known({"weight": known(0), "movable": known(False), "pickupable": known(False)}),
                "stack": known({"stackable": known(False), "stack_max": {"state": "NOT_APPLICABLE"}})},
            "attributes": {"speed_bonus": 0, "field_condition": None, "blocks_movement": False,
                "blocks_projectile": False, "immovable_block_solid": False}}]}

    def test_generic_factory_identity_requires_complete_qualification(self):
        packet = json.loads(PACKET.read_bytes())
        record = bind_profiles(self.generic(), {}, set(), packet)["records"][0]
        self.assertEqual(record["production_binding_qualification"], packet)
        self.assertEqual(record["production_binding"]["source_key"], "oteryn:source.canary")
        self.assertEqual(record["production_definition"]["production_key"], "oteryn:item.tibia.i40450")
        with self.assertRaises(ValueError):
            bind_profiles(self.generic(), {}, {40450})

    def test_generic_flag_substitution_and_inventory_grant_are_rejected(self):
        packet = json.loads(PACKET.read_bytes())
        for field in ("blocks_movement", "blocks_projectile", "immovable_block_solid"):
            with self.subTest(field=field):
                record = self.generic()
                record["records"][0]["attributes"][field] = True
                with self.assertRaises(ValueError):
                    bind_profiles(record, {}, set(), packet)
        record = self.generic()
        record["records"][0]["admission"]["legal_destinations"].append("CharacterInventory")
        with self.assertRaises(ValueError):
            bind_profiles(record, {}, set(), packet)

    def test_generic_provenance_substitution_is_rejected(self):
        packet = json.loads(PACKET.read_bytes())
        packet["qualification"]["sources"][0]["sha256"] = "0"*64
        with self.assertRaises(ValueError):
            bind_profiles(self.generic(), {}, set(), packet)


if __name__ == "__main__":
    unittest.main()
