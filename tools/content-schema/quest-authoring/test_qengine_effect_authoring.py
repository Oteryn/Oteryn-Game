import copy
import unittest

import qengine_access
import quest_batch4


class SharedEffectAuthoring(unittest.TestCase):
    def test_existing_callback_requires_equivalent_behavior_and_source_pin(self):
        incoming = qengine_access.document()
        row = next(row for row in incoming['callbacks'] if row['key'].endswith('waterfall-cave-entry'))
        existing = copy.deepcopy(row)
        existing['facing'] = 2
        existing['effects']['facing'] = None
        self.assertNotIn(row['key'], [r['key'] for r in qengine_access.compose(incoming, {'callbacks': [existing]})['callbacks']])
        for mutation in ('teleport', 'source', 'facing'):
            bad = copy.deepcopy(existing)
            if mutation == 'teleport':
                bad['teleport']['x'] += 1
            elif mutation == 'source':
                bad['sources'] = [bad['sources'][0].replace('201189f', '0000000')]
            else:
                bad['facing'] = 1
            with self.assertRaises(ValueError, msg=mutation):
                qengine_access.compose(incoming, {'callbacks': [bad]})

    def test_selector_alias_requires_identical_asymmetric_cycle_and_source(self):
        candidate = next(row for row in qengine_access.document()['callbacks']
                         if row['key'].endswith('triangle-tower-lever'))
        established = copy.deepcopy(candidate)
        established['key'] = 'oteryn:quest-callback/triangle-tower-wall'
        cycle = established['effects'].pop('item_cycle')
        established['passages'] = [dict(position=cycle['position'], appearance=cycle['remove'],
                                         replacement=None, create_on_close=cycle['create'])]
        existing = {'callbacks': [established]}
        incoming = {'schema': 1, 'callbacks': [candidate]}
        self.assertEqual(qengine_access.compose(incoming, existing)['callbacks'], [])
        for mutation in ('create', 'source', 'owner'):
            bad = copy.deepcopy(candidate)
            if mutation == 'create':
                bad['effects']['item_cycle']['create'] = 1270
            elif mutation == 'source':
                bad['sources'] = [bad['sources'][0].replace('201189f', '0000000')]
            else:
                bad['quest'] = 'oteryn:quest.another_quest'
            with self.assertRaises(ValueError, msg=mutation):
                qengine_access.compose({'schema': 1, 'callbacks': [bad]}, existing)

    def test_source_plate_stepout_and_paired_portals_are_authorable(self):
        for row in qengine_access.document()['callbacks']:
            if row['quest'] == 'oteryn:quest.thais_lighthouse_quest':
                quest_batch4.validate_callback(row, 'thais_lighthouse')

    def test_invalid_shared_effects_are_refused(self):
        rows = qengine_access.document()['callbacks']
        portal = next(row for row in rows if row['key'].endswith('thais-lighthouse-portals'))
        for change in ('duplicate_endpoint', 'zero_lifetime', 'unknown_effect', 'facing_without_teleport'):
            row = copy.deepcopy(portal)
            if change == 'duplicate_endpoint':
                row['effects']['portals']['portals'][1]['position'] = row['effects']['portals']['portals'][0]['position']
            elif change == 'zero_lifetime':
                row['effects']['portals']['lifetime_us'] = 0
            elif change == 'unknown_effect':
                row['effects']['unknown'] = True
            else:
                row['effects']['facing'] = 'east'
            with self.assertRaises(ValueError, msg=change):
                quest_batch4.validate_callback(row, 'thais_lighthouse')

    def test_all_source_effect_shapes_validate(self):
        for row in qengine_access.document()['callbacks']:
            quest_batch4.validate_effects(row, row['key'])

    def test_encounter_bounds_and_atomic_effect_composition(self):
        source = next(row for row in qengine_access.document()['callbacks']
                      if row['key'].endswith('annihilator-entry'))
        for change in ('empty_party', 'duplicate_landing', 'overlap_spawn', 'bad_level',
                       'outside_room', 'multiple_operations', 'unknown_member', 'return_stepout'):
            row = copy.deepcopy(source)
            encounter = row['effects']['encounter']
            if change == 'empty_party':
                encounter['players'] = []
            elif change == 'duplicate_landing':
                encounter['players'][1]['to'] = encounter['players'][0]['to']
            elif change == 'overlap_spawn':
                encounter['monsters'][0]['position'] = encounter['players'][0]['to']
            elif change == 'bad_level':
                encounter['minimum_level'] = True
            elif change == 'outside_room':
                encounter['players'][0]['to']['floor'] = -1
            elif change == 'multiple_operations':
                row['effects']['item_cycle'] = dict(position=encounter['room_min'], remove=1270, create=1025)
            elif change == 'unknown_member':
                encounter['players'][0]['unknown'] = True
            else:
                row['trigger'], row['return_on_failure'] = 'step_out', True
            with self.assertRaises(ValueError, msg=change):
                quest_batch4.validate_effects(row, row['key'])


if __name__ == '__main__':
    unittest.main()
