"""Source-capture guards for the four isolated dialogue generators."""
import json
from pathlib import Path
import unittest
import grave_danger_content as grave
import liquid_black_content as liquid
import primal_ordeal_dialogue as primal

ROOT = Path(__file__).resolve().parents[3]


class SourceDialogueGuards(unittest.TestCase):
    def test_literal_extractors_refuse_computed_or_executable_text(self):
        self.assertEqual(grave.literal('{"first", "second"}', 1), ['first', 'second'])
        for expression in ('"first" .. missing', 'player:getStorageValue(1)', '{"first", loadstring("x")}'):
            with self.assertRaises(ValueError):
                grave.literal(expression, 1)
        for line in ('npcHandler:say("first" .. missing, npc, creature)',
                     'npcHandler:say(callback(), npc, creature)'):
            with self.assertRaises(ValueError):
                liquid.literal_speech([line], 1, 'fixture.lua')
        self.assertEqual(liquid.literal_speech(['npcHandler:say({"first", "second"}, npc, creature)'], 1, 'fixture.lua'), ['first', 'second'])

    def test_primal_templates_allow_only_the_three_exact_numeric_source_locals(self):
        self.assertEqual(primal.speech('"Current " .. current .. ", maximum {" .. maximum .. "}."'),
                         ('Current |HAZARD_CURRENT|, maximum {|HAZARD_MAXIMUM|}.', ['current', 'maximum']))
        self.assertEqual(primal.speech('"Selected " .. desiredLevel'), ('Selected |HAZARD_DESIRED|', ['desiredLevel']))
        for expression in ('"Current " .. unrelated', '"Current " .. player:getLevel()', '"A" + "B"'):
            with self.assertRaises(ValueError):
                primal.speech(expression)

    def test_coordinator_holds_are_excluded_from_every_active_state_transition(self):
        liquid_state = json.loads((ROOT / 'content/quests/liquid-black-npc-state.json').read_text())
        liquid_keys = {t['key'] for q in liquid_state['quests'] for t in q['transitions']}
        self.assertEqual(liquid_keys, {liquid.START, liquid.ACCESS})
        grave_state = json.loads((ROOT / 'content/quests/grave-danger-npc-state.json').read_text())
        grave_keys = {t['key'] for q in grave_state['quests'] for t in q['transitions']}
        self.assertEqual(grave_keys, {grave.ACCESS})
        held_start = json.loads((ROOT / 'content/quests/grave-danger-start-source.json').read_text())
        held_final = json.loads((ROOT / 'content/quests/grave-danger-final-source.json').read_text())
        self.assertEqual(len(held_start['transition']['effects']), 13)
        self.assertEqual({t['key'] for t in held_final['transitions']}, {grave.FINISH, grave.FINISH_HELD})
        self.assertEqual(held_start['runtime_status'], 'LEFT_OUT_COORDINATOR_OWNER')
        self.assertEqual(held_final['runtime_status'], 'LEFT_OUT_APPEARANCE_MAPPING')

    def test_all_held_primal_source_speeches_remain_present_without_native_admission(self):
        document = json.loads((ROOT / 'content/dialogues/primal-ordeal.json').read_text())
        declaration = document['records'][0]['declaration']
        self.assertEqual({row['key'] for row in declaration['keywords']},
                         {'hazard', 'invalid_level', 'level_selected', 'prize', 'above_maximum'})
        source = next(field['value']['value'] for field in declaration['fields']
                      if field['field_path'] == 'oteryn:source.quest.primal_ordeal.dialogue')
        self.assertEqual(json.loads(source)['revision'], primal.PIN)
        self.assertTrue(any(field['field_path'] == 'oteryn:runtime.quest.primal_ordeal.hold'
                            for field in declaration['fields']))

    def test_additional_liquid_static_speech_is_source_only_and_never_computed(self):
        document = json.loads((ROOT / 'content/dialogues/quests-2/liquid-black-source-keywords.json').read_text())
        self.assertEqual(document['native_binding'], 'SOURCE_ONLY')
        self.assertEqual(len(document['records']), 8)
        self.assertEqual([row['source']['line'] for row in document['records']],
                         [83, 463, 467, 471, 475, 479, 483, 487])
        for row in document['records']:
            self.assertEqual(row['native_binding'], 'SOURCE_ONLY')
            self.assertEqual(row['source']['revision'], liquid.PIN)
            self.assertEqual(row['literal_messages'], [json.loads(row['source_expression'])])
        source = {'repository': 'zimbadev/crystalserver', 'revision': liquid.PIN,
                  'path': 'fixture.lua', 'sha256': '0' * 64}
        with self.assertRaises(ValueError):
            liquid.source_keyword(
                ['keywordHandler:addKeyword({ "name" }, StdModule.say, { npcHandler = npcHandler, text = "prefix" .. missing, reset = true })'],
                1, 1, 1, source)


if __name__ == '__main__':
    unittest.main()
