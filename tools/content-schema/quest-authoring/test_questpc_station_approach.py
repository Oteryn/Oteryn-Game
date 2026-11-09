"""A permanent pasture survey does not qualify the escape/taming World change."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
import questpc_world_requirements as owners
import questpc_station_approaches as stations
import copy
import test_questpc_explore_bindings as explore_tests

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.horse_station_world_change'
REFERENCE = 'station-approach:palomino-pasture'


class StationApproachTests(unittest.TestCase):
    def test_first_pasture_visit_is_independent_of_the_held_escape_cycle(self):
        binding = ((QUEST,'s1',REFERENCE,'chosen-source/horse_station_world_change'),)
        with mock.patch.object(explore,'BINDINGS',binding):
            callback, = explore.callbacks(ROOT)
            explore_tests.ExploreBindingsTests(
                'test_declared_area_point_really_has_the_retained_native_ground'
            ).test_declared_area_point_really_has_the_retained_native_ground()
        self.assertNotIn('world_change',callback)
        self.assertNotIn('admission',callback)
        self.assertNotIn('minimum_level',callback)
        self.assertTrue(callback['requires_world_admission'])
        self.assertEqual(callback['selector']['native'],dict(
            position=dict(x=32442,y=32229,floor=-7),ordinal=0,appearance=4521))
        self.assertEqual(len(callback['storage']),2)
        self.assertTrue(any(c['quest']==QUEST for c in explore.callbacks(ROOT)))

    def test_whole_world_change_stays_held(self):
        catalogue = owners.catalogue(ROOT)
        self.assertEqual(catalogue[QUEST][0]['status'],'HELD')
        with self.assertRaisesRegex(ValueError,'admitted World owner'):
            owners.requirement(QUEST,catalogue)

    def test_other_stages_and_changed_objectives_cannot_use_the_exception(self):
        import json
        definitions = [r['definition'] for p in (ROOT / 'content/quests/definitions').glob('quests-*.json')
                       for r in json.loads(p.read_text())['records']
                       if r['definition']['identity']['key'] == QUEST]
        definition, = definitions
        for quest, stage in [(QUEST,'s2'),('oteryn:quest.authored.kingsday_mini_world_change','s1')]:
            with self.assertRaisesRegex(ValueError,'permanent station survey'):
                stations.resolve(quest,stage,REFERENCE,definition=definition)
        changed = copy.deepcopy(definition)
        recipe = changed.get('recipe') or changed['oteryn_recipe']['payload']['recipe']
        recipe['stages'][0]['objective'] = 'Find active escaped horses'
        with self.assertRaisesRegex(ValueError,'survey contract'):
            stations.resolve(QUEST,'s1',REFERENCE,definition=changed)
