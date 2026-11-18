"""Middle Spike nest count in the authored source and its published projection."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.spike_task'


def read(path):
    return json.loads((ROOT / path).read_bytes())


class SpikeNestRecipeTest(unittest.TestCase):
    def test_middle_spike_requires_eight_nests(self):
        # Crystal 201189f8, movements_geomantic_charges.lua:14.
        source = read('tools/content-schema/quest-authoring/samples/completion242/recipes.json')
        recipe, = (row['recipe'] for row in source['records'] if row['identity']['key'] == QUEST)
        stage, = (stage for stage in recipe['stages'] if stage['key'] == 's7')
        self.assertEqual(stage['count'], 8, 'Middle Spike nest objective')

        definitions = read('content/quests/definitions/quests-00200-00299.json')
        projected, = (row['definition']['oteryn_recipe']['payload']['recipe']
                      for row in definitions['records']
                      if row['definition']['identity']['key'] == QUEST)
        published, = (stage for stage in projected['stages'] if stage['key'] == 's7')
        self.assertEqual(published['count'], 8, 'Published Middle Spike nest objective')
