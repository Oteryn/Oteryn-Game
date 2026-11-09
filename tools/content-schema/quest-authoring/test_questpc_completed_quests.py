"""A named prerequisite must resolve to one different canonical quest."""
import json
from pathlib import Path
import unittest

from questpc_completed_quests import resolve

ROOT = Path(__file__).resolve().parents[3]


class CompletedQuestRequirementsTests(unittest.TestCase):
    def test_lion_trophy_uses_the_canonical_ice_islands_completion(self):
        definitions = [row['definition']
                       for path in (ROOT/'content/quests/definitions').glob('quests-*.json')
                       for row in json.loads(path.read_text())['records']]
        self.assertEqual(resolve('oteryn:quest.lion_trophy_quest',
                                 ['The Ice Islands Quest'], definitions),
                         ['oteryn:quest.the_ice_islands_quest'])

    def test_unknown_and_ambiguous_titles_are_not_dropped(self):
        definition = {'display_name':'First Quest',
                      'identity':{'key':'oteryn:quest.first'}}
        for definitions in ([], [definition, definition]):
            with self.subTest(definitions=definitions):
                with self.assertRaisesRegex(ValueError,'one canonical prerequisite'):
                    resolve('oteryn:quest.second',['First Quest'],definitions)

    def test_self_dependency_is_rejected(self):
        with self.assertRaisesRegex(ValueError,'different canonical quest'):
            resolve('oteryn:quest.first',['First Quest'],[
                {'display_name':'First Quest','identity':{'key':'oteryn:quest.first'}}])

    def test_duplicate_or_unbounded_lists_are_rejected(self):
        for names in (['First Quest','First Quest'], [str(i) for i in range(17)]):
            with self.assertRaisesRegex(ValueError,'bounded distinct prerequisite'):
                resolve('oteryn:quest.second',names,[])
