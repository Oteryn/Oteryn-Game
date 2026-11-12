"""An exchange may reuse compatible custody without replacing another provider."""
import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import qengine_reward_item_profiles as rewards


class ExistingNativeProvider(unittest.TestCase):
    def setUp(self):
        self.profile = json.loads((ROOT / rewards.PROFILE).read_bytes())
        self.generated = {
            rewards.PROFILE: (ROOT / rewards.PROFILE).read_bytes(),
            'content/spells.manifest.json': (ROOT / 'content/spells.manifest.json').read_bytes(),
        }

    def admit(self):
        rewards.admit(ROOT, self.generated, rewards.PROFILE, 'content/spells.manifest.json',
                      lambda row: row['production_definition']['production_key'],
                      lambda doc: (json.dumps(doc) + '\n').encode())

    def test_compatible_provider_record_is_preserved_exactly(self):
        before = {r['production_definition']['production_key']: r for r in self.profile['records']}
        self.admit()
        after = {r['production_definition']['production_key']: r
                 for r in json.loads(self.generated[rewards.PROFILE])['records']}
        for key, record in before.items():
            self.assertEqual(after[key], record, key)

    def test_conflicting_custody_still_refuses(self):
        conflicting = copy.deepcopy(self.profile)
        record = next(r for r in conflicting['records']
                      if r['production_definition']['production_key'] == 'oteryn:item.tibia.i139')
        record['admission']['legal_destinations'] = []
        self.generated[rewards.PROFILE] = json.dumps(conflicting).encode()
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            self.admit()


if __name__ == '__main__':
    unittest.main()
