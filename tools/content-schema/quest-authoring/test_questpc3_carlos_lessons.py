import json
import unittest

import questpc3_carlos_lessons as authoring


class CarlosLessons(unittest.TestCase):
    def test_five_source_milestones_are_once_only_and_grant_no_items(self):
        callbacks = authoring.documents()['content/quests/shared-reward-callbacks.json']['callbacks']
        rows = [row for row in callbacks if row['key'].startswith(authoring.PREFIX)
                and row['key'] != authoring.PREFIX + 'food-evidence']
        self.assertEqual(len(rows), 5)
        expected = {'greet': (-1, 0, 1, 0, 1), 'outfit-lesson': (1, 1, 2, 1, 0),
                    'favor': (2, 2, 3, 2, 3), 'hunt': (3, 3, 4, 3, 4),
                    'await-food': (4, 4, 5, 4, 0)}
        for row in rows:
            name = row['key'].removeprefix(authoring.PREFIX)
            low, high, value, topic, next_topic = expected[name]
            self.assertEqual((row['topic'], row['next_topic']), (topic, next_topic))
            self.assertEqual(row['storage'], [authoring.storage(track, low, high, value) for track in [authoring.GREET, authoring.LOG]])
            self.assertEqual(row['take'], [])
            for field in ['give', 'outfit', 'achievement', 'experience', 'health_loss']:
                self.assertIsNone(row.get(field))
            self.assertFalse(row.get('completes', False))
            self.assertEqual(row['npc'], authoring.NPC)
            self.assertTrue(all(source.startswith(authoring.SOURCE + '#L') for source in row['sources']))

    def test_return_greetings_only_check_committed_progress(self):
        offers = authoring.documents()['content/quests/shared-reward-offers.json']['offers']
        rows = [row for row in offers if row['npc'] == authoring.NPC]
        self.assertEqual(len(rows), 6)
        self.assertEqual([row['next_topic'] for row in rows], [1, 2, 3, 4, 5, 6])
        for stage, row in enumerate(rows, 1):
            self.assertEqual(row['storage'], [authoring.storage(authoring.GREET, stage, stage, stage)])
            self.assertEqual(row['topic'], 0)

    def test_food_evidence_accepts_meat_or_ham_without_a_hand_in(self):
        callbacks = authoring.documents()['content/quests/shared-reward-callbacks.json']['callbacks']
        owned = [row for row in callbacks if row['key'].startswith(authoring.PREFIX)]
        self.assertEqual(len(owned), 6)
        row = next(row for row in owned if row['key'] == authoring.PREFIX + 'food-evidence')
        self.assertEqual((row['topic'], row['next_topic']), (5, 6))
        self.assertEqual(row['present_any'], [{'item': 'oteryn:item.tibia.i3577', 'count': 1},
                                             {'item': 'oteryn:item.tibia.i3582', 'count': 1}])
        self.assertEqual(row['storage'], [authoring.storage(authoring.GREET, 5, 5, 6),
                                         authoring.storage(authoring.LOG, 5, 5, 6),
                                         authoring.storage(authoring.BASE + 'carlos_npc_trade_storage', -1, 0, 1)])
        self.assertEqual(row['take'], [])
        self.assertIsNone(row['give'])
        self.assertIsNone(row['outfit'])
        self.assertFalse(row.get('completes', False))

    def test_sale_progress_requires_an_actual_eligible_source_trade(self):
        rows = authoring.documents()['content/quests/shared-trade-callbacks.json']['callbacks']
        row = next(row for row in rows if row['key'] == 'oteryn:quest-trade/questpc3/beginning/carlos-food-sale')
        self.assertEqual((row['npc'], row['side'], row['unit_price'], row['offer_count'], row['minimum_units']), (authoring.NPC, 'sell', 2, 1, 1))
        self.assertEqual(row['items'], ['oteryn:item.tibia.i3577', 'oteryn:item.tibia.i3582'])
        self.assertEqual(row['storage'][0], authoring.storage(authoring.GREET, 6, 6, 7))
        self.assertTrue(all(condition['preserve'] for condition in row['storage'][1:]))

    def test_other_lane_and_npc_records_are_preserved(self):
        for relative, document in authoring.documents().items():
            old = json.loads((authoring.ROOT / relative).read_text())
            field = 'callbacks' if 'callbacks' in document else 'offers' if 'offers' in document else 'npcs'
            foreign = lambda row: (row['key'] != 'oteryn:quest-trade/questpc3/beginning/carlos-food-sale' if relative.endswith('shared-trade-callbacks.json') else not row['key'].startswith(authoring.PREFIX)) if field == 'callbacks' else row['npc'] != authoring.NPC
            self.assertEqual([row for row in old[field] if foreign(row)], [row for row in document[field] if foreign(row)])


if __name__ == '__main__':
    unittest.main()
