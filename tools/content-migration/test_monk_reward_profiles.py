"""Seven finite native admissions; no canonical identity or hold is rewritten."""
import hashlib
import json
from pathlib import Path
import unittest

import monk_reward_profiles as tool


class MonkRewardProfiles(unittest.TestCase):
    def test_original_rewards_are_portable_leaves_with_known_equipment_facts(self):
        packet = json.loads((tool.ROOT / tool.PATH).read_bytes())
        rows = packet['records']
        self.assertEqual([int(r['production_definition']['production_key'].rsplit('i',1)[1]) for r in rows], list(tool.IDS))
        for row in rows:
            self.assertEqual(row['production_binding']['target']['key'], row['production_definition']['production_key'])
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['admission'], {'materializable':True,'stack_class':'NonStackable',
                                             'legal_destinations':['CharacterInventory','Ground']})
            self.assertEqual(row['semantics']['container']['state'], 'NOT_APPLICABLE')
            self.assertEqual(row['semantics']['physical']['state'], 'KNOWN')
            self.assertEqual(row['semantics']['equipment']['state'], 'KNOWN')
        for descriptor in packet['canonical_inputs']:
            self.assertEqual(hashlib.sha256((tool.ROOT / descriptor['path']).read_bytes()).hexdigest(), descriptor['sha256'])

    def test_registration_preserves_every_profile_and_updates_the_manifest_pin(self):
        generated = {'profiles':b'{"records":[]}', 'manifest':b'{"item_profiles":{}}'}
        canonical = lambda v: (json.dumps(v, sort_keys=True)+'\n').encode()
        tool.admit(tool.ROOT, generated, 'profiles','manifest',lambda r:r['production_definition']['production_key'],canonical)
        self.assertEqual(len(json.loads(generated['profiles'])['records']),7)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'], hashlib.sha256(generated['profiles']).hexdigest())
        with self.assertRaisesRegex(ValueError,'MONK_REWARD_PROFILE_DUPLICATE'):
            tool.admit(tool.ROOT, generated, 'profiles','manifest',lambda r:r['production_definition']['production_key'],canonical)


if __name__ == '__main__': unittest.main()
