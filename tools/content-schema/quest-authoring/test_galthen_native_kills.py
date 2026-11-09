"""M25 selectively lowers the authored Galthen boss counters, not quest entry gates."""
import copy
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import quest_state_lowering as tool
import galthen_native_kills as selected

QUEST = 'oteryn:quest.adventures_of_galthen_quest'
TRACK = 'oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/'
TRANSITION = 'oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/'


def source_definition():
    records = json.loads((ROOT / 'content/quests/definitions/quests-00100-00199.json').read_text())['records']
    rows = [r['definition'] for r in records if r['definition']['identity']['key'] == QUEST]
    if len(rows) != 1:
        raise ValueError('one canonical Galthen definition')
    return rows[0]


def lower(definition):
    return tool.lower_quest(definition, tool.owners([definition]), tool.mission_values([definition]))


class GalthenNativeKillLowering(unittest.TestCase):
    def test_actual_authoring_lowering_preserves_exact_boss_predecessors_and_separate_completion(self):
        definition = source_definition()
        before = copy.deepcopy(definition)
        stages = {s['key']: s for s in definition['oteryn_recipe']['payload']['recipe']['stages']}
        self.assertEqual((stages['s5']['kind'], stages['s5']['count'], stages['s5']['targets']), ('kill', 1, ['Ahau']))
        self.assertEqual((stages['s8']['kind'], stages['s8']['count'], stages['s8']['targets']), ('kill', 1, ['Mitmah Vanguard']))
        self.assertEqual((stages['s9']['kind'], stages['s9']['next']), ('complete', []))
        actual = lower(definition)
        self.assertEqual(definition, before)
        selected = [t for t in actual['transitions'] if t['key'].startswith(TRANSITION)]
        self.assertEqual({t['key'] for t in selected}, {TRANSITION+'s5', TRANSITION+'s8'}, 'real lowering must bind only the two selected boss stages')
        tracks = {t['key']: t for t in actual['tracks']}
        self.assertEqual({key for key in tracks if key.startswith(TRACK)}, {TRACK+s for s in ['s4','s5','s7','s8']})
        for stage, predecessor in [('s5','s4'), ('s8','s7')]:
            transition = next(t for t in selected if t['key'] == TRANSITION+stage)
            self.assertFalse(transition['completes'])
            self.assertIsNone(transition['requested_by'])
            self.assertEqual(transition['effects'], [
                {'track': TRACK+predecessor, 'from': {'op':'EQ','value':1}, 'from_exact':True, 'effect':{'kind':'SET','value':1}},
                {'track': TRACK+stage, 'from': {'op':'LT','value':1}, 'from_exact':True, 'effect':{'kind':'ADD','value':1}},
            ])
            for key in [TRACK+predecessor, TRACK+stage]:
                self.assertEqual((tracks[key]['min'],tracks[key]['initial'],tracks[key]['max']), (0,0,1))
        self.assertNotIn(TRACK+'s9', tracks)
        self.assertEqual(actual['completion'], 'NOT_LOWERED_NO_MISSIONS')
        tool.validate([actual])

    def test_selected_source_guards_fail_closed_on_recipe_drift(self):
        for stage, field, replacement in [('s4','count',2),('s5','targets',['Rat']),('s7','next',['s9']),('s8','kind','talk'),('s9','kind','kill')]:
            with self.subTest(stage=stage, field=field):
                definition = source_definition()
                row = next(s for s in definition['oteryn_recipe']['payload']['recipe']['stages'] if s['key'] == stage)
                row[field] = replacement
                with self.assertRaises(ValueError):
                    lower(definition)

    def test_completion_plan_binds_two_kills_and_does_not_activate_other_stages(self):
        rows = json.loads((ROOT / 'content/quests/missions/completion-binding-plan.json').read_text())['records']
        before = copy.deepcopy(rows)
        selected.bind_selected_plan(rows)
        for original, actual in zip(before, rows):
            if actual['quest'] != QUEST:
                self.assertEqual(actual, original)
                continue
            self.assertFalse(actual['runtime_enabled'])
            for old_stage, stage in zip(original['stages'],actual['stages']):
                if stage['stage_key'] in ['s5','s8']:
                    self.assertFalse(stage['runtime_enabled'])
                    self.assertEqual(stage['native_dispatch_binding']['transition'],TRANSITION+stage['stage_key'])
                else:
                    self.assertEqual(stage,old_stage)


if __name__ == '__main__':
    unittest.main()
