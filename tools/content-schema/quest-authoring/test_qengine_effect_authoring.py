import copy
import json
import unittest

import qengine_access
import quest_batch4


class SharedEffectAuthoring(unittest.TestCase):
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

    def test_scripted_landing_requires_explicit_native_player_destination(self):
        document = json.loads((qengine_access.ROOT / 'content/quests/authored-bindings/m03-25-years-time-machine.json').read_text())
        source = document['quests'][0]['stages'][0]['trigger']['callback']
        source['effects'] = {'scripted_teleport': True}
        quest_batch4.validate_effects(source, 'scripted source')
        for change in ('not_boolean', 'no_destination', 'no_native', 'monster'):
            row = copy.deepcopy(source)
            if change == 'not_boolean':
                row['effects']['scripted_teleport'] = 1
            elif change == 'no_destination':
                row['teleport'] = None
            elif change == 'no_native':
                row['selector']['native'] = None
            else:
                row['actor_kind'] = 'monster'
            with self.assertRaises(ValueError, msg=change):
                quest_batch4.validate_effects(row, 'scripted source')

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
