"""Production coverage for the next selected TALK routes, not whole quests."""
import json
from pathlib import Path
import unittest
from native_talk_binding_evidence import canonical_definitions, verify_talk_binding
from quest_native_binding_preflight import verify_talk_state_bounds

ROOT = Path(__file__).resolve().parents[3]
DOCUMENT = ROOT / 'content/quests/authored-bindings/m01a-starter-talk-routes.json'

class StarterTalkRoutes(unittest.TestCase):
    def test_every_selected_route_has_a_real_nonterminal_callback(self):
        rows = {row['quest']: row for row in json.loads(DOCUMENT.read_text())['quests']}
        for slug, npc, minimum in [('goblin_merchant_quest', 'rafzan', None),
                                   ('machinery_of_war_quest', 'baxter', 50),
                                   ('authored.annual_autumn_vintage', 'emilie', None)]:
            with self.subTest(quest=slug):
                row = rows['oteryn:quest.' + slug]
                stage = row['stages'][0]
                self.assertEqual(stage['key'], 's1')
                self.assertFalse(stage['completes'])
                callback = stage['trigger']['callback']
                self.assertEqual(callback['npc'], 'oteryn:npc.' + npc)
                self.assertEqual(callback.get('minimum_level'), minimum)
                self.assertEqual([(s['minimum'], s['maximum'], s['set']) for s in callback['storage']], [(0, 0, 1)])
                self.assertTrue(callback['keywords'])
                self.assertEqual(callback['quest'], row['quest'])
                callbacks = verify_talk_binding(ROOT, canonical_definitions(ROOT)[row['quest']], 's1',
                    {'kind': 'npc_dialogue', 'document': DOCUMENT.relative_to(ROOT).as_posix(),
                     'stage': 's1', 'callback': callback['key']})
                verify_talk_state_bounds(row, callbacks)
                definition = canonical_definitions(ROOT)[row['quest']]
                recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
                self.assertEqual(callback['reply'], ['Next: ' + recipe['stages'][1]['objective']])

if __name__ == '__main__':
    unittest.main()
