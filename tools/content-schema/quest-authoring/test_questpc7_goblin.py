"""Source-bound Goblin introduction; no engine or quest-state fixture shortcuts."""
import copy
import unittest

import qengine_rewards as sources
import questpc7_goblin_programme as goblin


class GoblinProgramme(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = sources.donor_inputs()

    def build(self, inputs=None):
        rows = []
        goblin.append(sources.ROOT, rows, self.inputs if inputs is None else inputs, sources.PIN)
        return rows

    def test_start_initializes_all_six_jobs_atomically_without_free_stock(self):
        intro, _ = self.build()
        self.assertEqual(len(intro['storage']), 7)
        self.assertEqual(len({s['track'] for s in intro['storage']}), 7)
        self.assertTrue(all((s['minimum'], s['maximum'], s['set']) == (-1, -1, 0)
                            for s in intro['storage']))
        self.assertIsNone(intro['give'])
        self.assertNotIn('completes', intro)

    def test_bundle_preserves_source_prices_and_stack_class(self):
        _, kit = self.build()
        self.assertEqual(kit['fee'], {'gold': 1125, 'include_bank': True})
        self.assertEqual((kit['give']['item'], kit['give']['count']),
                         ('oteryn:item.tibia.i17668', 3))
        self.assertEqual({r['item'] for r in kit['give']['backpack_outputs']},
                         {'oteryn:item.tibia.i17669', 'oteryn:item.tibia.i17670',
                          'oteryn:item.tibia.i17671', 'oteryn:item.tibia.i17676'})
        self.assertTrue(all(r['count'] == 1 for r in kit['give']['backpack_outputs']))
        self.assertTrue(any(s['track'].endswith('/kit_bought') and s['minimum'] == -1
                            and s['maximum'] == -1 and s['set'] == 1 for s in kit['storage']))

    def test_changed_donor_price_refuses_the_authored_bundle(self):
        inputs = copy.copy(self.inputs)
        path = 'data-global/npc/rafzan.lua'
        original = b'clientId = 17668, buy = 75'
        self.assertEqual(inputs[path].count(original), 1)
        inputs[path] = inputs[path].replace(original, b'clientId = 17668, buy = 74')
        with self.assertRaisesRegex(ValueError, 'GOBLIN_SOURCE_TOOL_PRICE_DRIFT'):
            self.build(inputs)


if __name__ == '__main__':
    unittest.main()
