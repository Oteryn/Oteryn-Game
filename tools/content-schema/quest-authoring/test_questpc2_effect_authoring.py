"""Lane source and rejection proofs, alongside main-owned engine tests."""
import copy
import unittest
import qengine_access
import questpc2_access
import quest_batch4


class QuestPC2SourceEffects(unittest.TestCase):
    def test_reward_door_matches_pinned_source_and_refuses_shadow_progress(self):
        questpc2_access.verify_reward_door_source()
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

    def test_source_four_stations_and_return_phase_are_pinned(self):
        questpc2_access.verify_desert_source()
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


if __name__ == "__main__":
    unittest.main()
