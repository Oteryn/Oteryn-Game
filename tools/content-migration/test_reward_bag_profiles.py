"""Admission boundaries for the three pinned reward-bag lists."""
import copy
import hashlib
import json
import unittest
from pathlib import Path
from unittest.mock import patch

import reward_bag_profiles as bags


class RewardBagProfilesTests(unittest.TestCase):
    def capture(self, blocked=None):
        ids = (34109, 39546, 43895)
        rewards = (34082, 39150, 43864)
        constants = '\n'.join(f'{name} = {i}' for name, i in zip(bags.BAGS, ids)).encode()

        def read(path):
            if path.name == 'constants.lua':
                return constants
            if path.name == 'reward_bags.lua':
                weight = ', weight = 1' if path.parts[0] == 'canary' else ''
                return '\n'.join(
                    f'[{name}] = {{\n\t{{ id = {i}, name = "reward"{weight} }}\n\t}},'
                    for name, i in zip(bags.BAGS, rewards)).encode()
            return b''

        def profile(i, *_):
            return {
                'production_definition': {'production_key': bags.key(i)},
                'admission': {
                    'stack_class': 'StackCapable' if blocked == i else 'NonStackable',
                    'legal_destinations': ['CharacterInventory'],
                },
                'semantics': {
                    'container': bags.source.K({'capacity': bags.source.K(4)})
                    if i == 39150 else copy.deepcopy(bags.source.NA),
                },
            }

        all_ids = ids + rewards
        with (patch.object(Path, 'read_bytes', read),
              patch.object(bags.source, 'xml_index', return_value=dict.fromkeys(all_ids)),
              patch.object(bags.source.engine_items, 'load_appearance_objects',
                           return_value=dict.fromkeys(all_ids)),
              patch.object(bags.source, 'policies', return_value=[
                  {'server_item_id': i} for i in all_ids]),
              patch.object(bags.source, 'profile', side_effect=profile),
              patch.object(bags.source, 'attrs', return_value={}),
              patch.object(bags, 'binding_index', return_value={}),
              patch.object(bags, 'bind_profiles', side_effect=lambda doc, *_: doc)):
            return bags.capture(Path('canary'), Path('crystal'))

    def test_desire_and_empty_quiver_are_admitted(self):
        overlay = self.capture()
        self.assertTrue(all(b['playable'] for b in overlay['bags']))
        self.assertEqual(len(overlay['records']), 6)
        quiver = next(r for r in overlay['records']
                      if r['production_definition']['production_key'] == bags.key(39150))
        self.assertEqual(quiver['semantics']['container'],
                         bags.source.K({'capacity': bags.source.K(4)}))

    def test_quiver_exception_does_not_admit_stackable_rewards(self):
        overlay = self.capture(blocked=39150)
        primal = overlay['bags'][1]
        self.assertFalse(primal['playable'])
        self.assertEqual(primal['excluded_reason'], 'NON_LEAF_REWARD:39150')
        self.assertNotIn(bags.key(39150), {
            r['production_definition']['production_key'] for r in overlay['records']})

    def test_existing_quiver_profile_is_preserved_and_conflicts_fail_closed(self):
        overlay = json.loads((bags.ROOT / bags.PATH).read_bytes())
        native = json.loads((bags.ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        quiver = next(r for r in native['records']
                      if r['production_definition']['production_key'] == bags.key(39150))
        profile_path, manifest_path = 'profiles.json', 'manifest.json'

        def admit(records):
            generated = {
                profile_path: json.dumps({'records': records}).encode(),
                manifest_path: b'{"item_profiles": {}}',
            }
            bags.admit(bags.ROOT, generated, profile_path, manifest_path,
                       lambda r: r['production_definition']['production_key'],
                       lambda doc: json.dumps(doc).encode())
            return generated

        generated = admit([quiver])
        records = json.loads(generated[profile_path])['records']
        self.assertEqual(next(r for r in records
                              if r['production_definition']['production_key'] == bags.key(39150)),
                         quiver)
        self.assertEqual(len(records), len(overlay['records']))
        self.assertEqual(json.loads(generated[manifest_path])['item_profiles']['sha256'],
                         hashlib.sha256(generated[profile_path]).hexdigest())
        for field in ('physical', 'stack', 'container'):
            conflict = copy.deepcopy(quiver)
            conflict['semantics'][field] = bags.source.U
            with self.assertRaisesRegex(ValueError, 'REWARD_BAG_PROFILE_DUPLICATE'):
                admit([conflict])
        conflict = copy.deepcopy(quiver)
        conflict['admission']['materializable'] = False
        with self.assertRaisesRegex(ValueError, 'REWARD_BAG_PROFILE_DUPLICATE'):
            admit([conflict])
        with self.assertRaisesRegex(ValueError, 'REWARD_BAG_PROFILE_DUPLICATE'):
            admit([overlay['records'][0]])

    def test_registration_delegates_shared_quiver_to_existing_owner(self):
        import quick_loot_item_profiles as quick
        import register_spell_families as registration

        profile_path, manifest_path = 'profiles.json', 'manifest.json'
        generated = {profile_path: b'{"records": []}',
                     manifest_path: b'{"item_profiles": {}}'}
        bags.admit(bags.ROOT, generated, profile_path, manifest_path,
                   registration.provider_order, registration.spell_import.canonical_bytes)
        records = json.loads(generated[profile_path])['records']
        self.assertNotIn(bags.key(39150), {r['production_definition']['production_key']
                                         for r in records})
        quick.admit(bags.ROOT, generated, profile_path, manifest_path,
                    registration.provider_order, registration.spell_import.canonical_bytes)
        records = json.loads(generated[profile_path])['records']
        quivers = [r for r in records
                   if r['production_definition']['production_key'] == bags.key(39150)]
        self.assertEqual(len(quivers), 1)
        self.assertEqual(quivers[0]['semantics']['container'],
                         bags.source.K({'capacity': bags.source.K(12)}))
        self.assertEqual(quivers[0]['semantics']['equipment']['state'], 'KNOWN')

    def test_bundled_parity_and_profile_set(self):
        overlay = json.loads((bags.ROOT / bags.PATH).read_bytes())
        self.assertEqual([(b['constant'], len(b['rewards']), b['playable'])
                          for b in overlay['bags']], [
                              ('BAG_YOU_DESIRE', 21, True),
                              ('PRIMAL_BAG', 14, True),
                              ('BAG_YOU_COVET', 26, True)])
        expected = {b['bag'] for b in overlay['bags']} | {
            r['item'] for b in overlay['bags'] for r in b['rewards']}
        self.assertEqual(expected, {r['production_definition']['production_key']
                                    for r in overlay['records']})


if __name__ == '__main__':
    unittest.main()
