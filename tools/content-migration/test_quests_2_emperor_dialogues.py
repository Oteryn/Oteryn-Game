"""Closed-source capture keeps every NPC speech form without executing Lua."""
import unittest
from pathlib import Path
from unittest.mock import patch

import quests_2_emperor_dialogues as authoring


class EmperorSpeechCapture(unittest.TestCase):
    def test_capture_retains_static_child_greeting_voices_and_dynamic_fragments(self):
        source = b'''local state = Storage.Quest.U8_6.WrathOfTheEmperor.Questline
npcHandler:say("A conditional reply.", npc, creature)
keywordHandler:addKeyword({"job"}, StdModule.say, {text = "A static reply."})
node:addChildKeyword({"no"}, StdModule.say, {text = {"First child.", "Second child."}})
keywordHandler:addGreetKeyword({"hail", "king"}, {npcHandler = npcHandler, text = "A source greeting."})
keywordHandler:addAliasKeyword({"work"})
keywordHandler:addKeyword({"cost"}, StdModule.say, {text = price .. " coins"})
npcConfig.voices = {interval = 15000, chance = 50, {text = "A source voice."}}
'''
        root = Path(__file__).resolve().parents[2]

        def source_git(command, **options):
            if 'ls-tree' in command:
                return 'data-global/npc/example.lua\n'
            self.assertIn(authoring.PIN + ':data-global/npc/example.lua', command)
            return source

        with patch.object(authoring.subprocess, 'check_output', side_effect=source_git):
            captured = authoring.capture(Path('/read-only-crystal'), root)['npcs'][0]
        self.assertIn('static_keywords', captured)
        rows = captured['static_keywords']
        self.assertEqual(len(rows), 5)
        self.assertEqual(rows[0]['literal_messages'], ['A static reply.'])
        self.assertEqual(rows[1]['literal_messages'], ['First child.', 'Second child.'])
        self.assertEqual(rows[2]['literal_messages'], ['A source greeting.'])
        self.assertEqual(rows[3]['kind'], 'keyword_alias')
        self.assertIsNone(rows[4]['literal_messages'])
        self.assertIn('price', rows[4]['expression'])
        self.assertTrue(all(row['source_only'] for row in rows))
        self.assertEqual(captured['voices']['parameters'][1]['text'], 'A source voice.')
        fragments = {(part['line'], part['text']) for part in captured['source_string_fragments']}
        self.assertIn((7, ' coins'), fragments)
        self.assertIn((8, 'A source voice.'), fragments)
        self.assertEqual(captured['replies'][0]['literal_messages'], ['A conditional reply.'])


if __name__ == '__main__':
    unittest.main()
