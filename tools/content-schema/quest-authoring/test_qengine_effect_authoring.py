import copy
import unittest

import qengine_access
import quest_batch4


class SharedEffectAuthoring(unittest.TestCase):
    def test_source_plate_stepout_and_paired_portals_are_authorable(self):
        for row in qengine_access.document()['callbacks']:
            if row['quest'] == 'oteryn:quest.thais_lighthouse_quest':
                quest_batch4.validate_callback(row, 'thais_lighthouse')

    def test_reward_door_matches_pinned_source_and_refuses_shadow_progress(self):
        qengine_access.verify_reward_door_source()
        source = next(row for row in qengine_access.document()['callbacks']
                      if row['key'].endswith('annihilator-reward-door'))
        self.assertEqual(source['lever'], [5113, 5114])
        self.assertEqual(source['selector']['action'], 10102)
        for change in ('foreign_claim', 'unknown_field', 'one_way', 'shadow_track', 'unbound_door'):
            row = copy.deepcopy(source)
            if change == 'foreign_claim':
                row['effects']['reward_door']['claim'] = 'oteryn:quest.shadow'
            elif change == 'unknown_field':
                row['effects']['reward_door']['write'] = True
            elif change == 'one_way':
                row['reversible'] = False
            elif change == 'shadow_track':
                row['storage'] = [dict(track='oteryn:quest.shadow', minimum=0, maximum=1, set=0)]
            else:
                row['selector']['action'] = None
            with self.assertRaises(ValueError, msg=change):
                quest_batch4.validate_effects(row, row['key'])

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

class DesertPartySource(unittest.TestCase):
    def test_source_four_stations_and_return_phase_are_pinned(self):
        qengine_access.verify_desert_source()
        row = next(row for row in qengine_access.document()['callbacks']
                   if row['key'].endswith('desert-dungeon-party-rite'))
        party = row['effects']['party_transfer']
        self.assertEqual(party['activation_appearance'], 2772)
        self.assertEqual([s['vocation'] for s in party['stations']],
                         ['sorcerer', 'druid', 'paladin', 'knight'])
        self.assertEqual([o['item'] for o in row['offerings']],
                         ['oteryn:item.tibia.i3059', 'oteryn:item.tibia.i3585',
                          'oteryn:item.tibia.i3349', 'oteryn:item.tibia.i3264'])
        self.assertTrue(row['reversible'])
        self.assertEqual(row['selector']['unique'], 30030)

    def test_party_rite_refuses_partial_and_unbounded_definitions(self):
        source = next(row for row in qengine_access.document()['callbacks']
                      if row['key'].endswith('desert-dungeon-party-rite'))
        for change in ('empty', 'duplicate_destination', 'foreign_vocation', 'unknown_station',
                       'missing_offering', 'missing_receipt', 'foreign_phase', 'second_operation'):
            row = copy.deepcopy(source)
            party = row['effects']['party_transfer']
            if change == 'empty':
                party['stations'] = []
            elif change == 'duplicate_destination':
                party['stations'][1]['to'] = party['stations'][0]['to']
            elif change == 'foreign_vocation':
                party['stations'][0]['vocation'] = 'none'
            elif change == 'unknown_station':
                party['stations'][0]['client_eligible'] = True
            elif change == 'missing_offering':
                row['offerings'].pop()
            elif change == 'missing_receipt':
                row['storage'] = []
            elif change == 'foreign_phase':
                party['activation_appearance'] = 2774
            else:
                row['effects']['reward_door'] = {'claim': 'oteryn:reward-claim.some_reward'}
            with self.assertRaises(ValueError, msg=change):
                quest_batch4.validate_effects(row, row['key'])
