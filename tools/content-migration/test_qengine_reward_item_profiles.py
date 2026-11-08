"""Existing native identities retain accepted facts during shared reward admission."""
import copy
import json
import unittest

import qengine_reward_item_profiles as q
import register_spell_families as native


class SharedRewardProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profile = json.loads((q.ROOT / q.PROFILE).read_bytes())

    def admit(self, profile):
        generated = {q.PROFILE: json.dumps(profile).encode(),
                     'content/spells.manifest.json': (q.ROOT / 'content/spells.manifest.json').read_bytes()}
        q.admit(q.ROOT, generated, q.PROFILE, 'content/spells.manifest.json',
                native.provider_order, native.spell_import.canonical_bytes)
        return json.loads(generated[q.PROFILE])

    def test_oramond_exact_source_root_is_portable_stack100_at_weight200(self):
        profile = next(r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21291')
        self.assertEqual(profile['admission'], {
            'materializable': True, 'stack_class': 'StackCapable',
            'legal_destinations': ['CharacterInventory', 'Ground']})
        self.assertEqual(profile['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 200})
        self.assertEqual(profile['semantics']['stack']['value']['stack_max'],
                         {'state': 'KNOWN', 'value': 100})
        for group in ('charges', 'temporal', 'container'):
            self.assertEqual(profile['semantics'][group], {'state': 'NOT_APPLICABLE'})

    def test_known_physical_conflict_is_refused(self):
        profile = copy.deepcopy(self.profile)
        row = next(r for r in profile['records']
                   if r['production_definition']['production_key'] == 'oteryn:item.tibia.i139')
        row['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 1}
        with self.assertRaisesRegex(ValueError, 'NATIVE_CONFLICT'):
            self.admit(profile)

    def test_existing_identity_and_charge_facts_survive_idempotent_admission(self):
        output = self.admit(copy.deepcopy(self.profile))
        before = {r['production_definition']['production_key']: r for r in self.profile['records']}
        for row in output['records']:
            key = row['production_definition']['production_key']
            if key in before:
                self.assertEqual(row['production_definition'], before[key]['production_definition'])
                self.assertEqual(row['semantics'].get('charges'), before[key]['semantics'].get('charges'))
        self.assertEqual(self.admit(output), output)


if __name__ == '__main__':
    unittest.main()
