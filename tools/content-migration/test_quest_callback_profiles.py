"""Rebuild callback Item admissions against retained, pinned source inputs."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import quest_callback_profiles as profiles

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/items/native-manifest.json'
KEY = 'oteryn:item.tibia.i7934'


class QuestCallbackProfiles(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='.qcb3-profile-', dir=ROOT)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'content/quests').mkdir(parents=True)
        (self.root / 'content/interactions').symlink_to(ROOT / 'content/interactions', target_is_directory=True)
        (self.root / 'content/items').symlink_to(ROOT / 'content/items', target_is_directory=True)
        (self.root / 'imports').symlink_to(ROOT / 'imports', target_is_directory=True)
        self.callbacks = json.loads((ROOT / 'content/quests/world-callbacks.json').read_bytes())
        self.document = json.loads((ROOT / PROFILE).read_bytes())
        self.document['records'] = [r for r in self.document['records']
                                    if r['production_definition']['production_key'] not in
                                    (KEY, 'oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019')]
        self.generated = {PROFILE: json.dumps(self.document).encode(),
                          MANIFEST: b'{"item_profiles":{"sha256":"pending"}}'}

    def admit(self):
        (self.root / 'content/quests/world-callbacks.json').write_text(json.dumps(self.callbacks))
        profiles.admit(self.root, self.generated, PROFILE, MANIFEST,
                       lambda r: r['production_definition']['production_key'],
                       lambda v: json.dumps(v, sort_keys=True).encode())

    def test_fish_key_is_rebuilt_with_portable_admission_and_manifest_digest(self):
        self.admit()
        rows = [r for r in json.loads(self.generated[PROFILE])['records']
                if r['production_definition']['production_key'] == KEY]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['admission'], {'materializable': True,
                         'stack_class': 'NonStackable',
                         'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(json.loads(self.generated[MANIFEST])['item_profiles']['sha256'],
                         hashlib.sha256(self.generated[PROFILE]).hexdigest())

    def test_quest_chest_rewards_preserve_canonical_item_admission(self):
        self.admit()
        rows = {r['production_definition']['production_key']: r
                for r in json.loads(self.generated[PROFILE])['records']}
        for key in ('oteryn:item.tibia.i6120', 'oteryn:item.tibia.i9019'):
            row = rows[key]
            self.assertEqual(row['admission'], {'materializable': True,
                             'stack_class': 'NonStackable',
                             'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['production_binding']['target']['key'], key)
            self.assertEqual(row['semantics']['physical']['value']['pickupable'],
                             {'state': 'KNOWN', 'value': True})

    def test_unadmitted_replacement_refuses_instead_of_manufacturing_a_profile(self):
        callback = next(c for c in self.callbacks['callbacks'] if c.get('item_on_target'))
        callback['item_on_target']['replacement'] = 'oteryn:item.unadmitted'
        with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_ITEM_MISSING'):
            self.admit()

    def test_changed_reference_digest_refuses_admission(self):
        with patch.object(profiles, 'SOURCE_SHA256', '0' * 64):
            with self.assertRaisesRegex(ValueError, 'QUEST_CALLBACK_SOURCE_DIGEST'):
                self.admit()


if __name__ == '__main__':
    unittest.main()
