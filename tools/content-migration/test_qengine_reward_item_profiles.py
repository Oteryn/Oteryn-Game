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



import copy
import unittest

import qengine_reward_item_profiles as q


class NativeFactJoin(unittest.TestCase):
    def record(self):
        return dict(production_definition={'production_key': 'item'},
                    production_binding={'exact': True}, admission={'materializable': True},
                    authoring={'item': 'established'}, attributes={'blocks': None},
                    semantics={'charges': {'state': 'UNKNOWN'}})

    def test_source_can_fill_unknown_without_changing_owner(self):
        old = self.record()
        new = copy.deepcopy(old)
        new['semantics']['charges'] = {'state': 'NOT_APPLICABLE'}
        new['attributes']['blocks'] = False
        new['authoring'] = {'item': 'candidate'}
        joined = q.reconcile(old, new)
        self.assertEqual(joined['authoring'], old['authoring'])
        self.assertEqual(joined['semantics'], new['semantics'])
        self.assertEqual(joined['attributes'], new['attributes'])
        self.assertEqual(old, self.record())

    def test_known_identity_custody_and_physical_conflicts_fail(self):
        old = self.record()
        old['semantics']['charges'] = {'state': 'NOT_APPLICABLE'}
        old['attributes']['blocks'] = False
        for field in ('production_definition', 'production_binding', 'admission', 'semantics', 'attributes'):
            new = copy.deepcopy(old)
            if field == 'semantics':
                new[field]['charges'] = {'state': 'KNOWN', 'value': 1}
            elif field == 'attributes':
                new[field]['blocks'] = True
            else:
                new[field]['different'] = True
            with self.assertRaises(ValueError, msg=field):
                q.reconcile(old, new)


if __name__ == '__main__':
    unittest.main()
