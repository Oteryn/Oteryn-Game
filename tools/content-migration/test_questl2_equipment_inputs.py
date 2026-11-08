"""Composition and source-pin refusals for the runtime equipment audit."""
import hashlib
import json
import unittest
from questl2_equipment_inputs import keys


class ComposedEquipmentInputs(unittest.TestCase):
    def setUp(self):
        self.files = {}
        self.manifest = {}
        self.base_sha = self.pin('item_profiles', 'base', {
            'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': self.rows(1, 2)})
        self.pin('item_profile_updates', 'updates', {
            'schema': 'OTERYN_NATIVE_ITEM_UPDATES/v1', 'base_sha256': self.base_sha,
            'records': self.rows(2, 3)})
        self.pin('starting_loadout_item_profiles', 'starter', {
            'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': self.rows(4)})
    def rows(self, *ids):
        return [{'production_definition': {'production_key': f'oteryn:item.tibia.i{i}'}} for i in ids]
    def pin(self, name, leaf, document):
        path = 'items/definitions/' + leaf + '.json'
        raw = json.dumps(document).encode()
        self.files['content/' + path] = raw
        digest = hashlib.sha256(raw).hexdigest()
        self.manifest[name] = {'path': path, 'sha256': digest}
        self.files['content/spells.manifest.json'] = json.dumps(self.manifest).encode()
        return digest
    def test_updates_replace_and_add_then_starter_extends(self):
        self.assertEqual(keys(self.files.__getitem__), {f'oteryn:item.tibia.i{i}' for i in (1, 2, 3, 4)})
    def test_tampered_provider_bytes_are_not_counted(self):
        self.files['content/items/definitions/updates.json'] += b' '
        with self.assertRaisesRegex(ValueError, 'PROVIDER_PIN'):
            keys(self.files.__getitem__)
    def test_valid_digest_cannot_bypass_wrong_update_base(self):
        self.pin('item_profile_updates', 'updates', {'schema': 'OTERYN_NATIVE_ITEM_UPDATES/v1',
            'base_sha256': '0' * 64, 'records': self.rows(3)})
        with self.assertRaisesRegex(ValueError, 'UPDATE_BASE_PIN'):
            keys(self.files.__getitem__)
    def test_starter_overlap_is_refused(self):
        self.pin('starting_loadout_item_profiles', 'starter', {
            'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': self.rows(3)})
        with self.assertRaisesRegex(ValueError, 'STARTER_COLLISION'):
            keys(self.files.__getitem__)
    def test_duplicate_update_identity_is_refused(self):
        self.pin('item_profile_updates', 'updates', {'schema': 'OTERYN_NATIVE_ITEM_UPDATES/v1',
            'base_sha256': self.base_sha, 'records': self.rows(3, 3)})
        with self.assertRaisesRegex(ValueError, 'DUPLICATE_KEY'):
            keys(self.files.__getitem__)


if __name__ == '__main__':
    unittest.main()
