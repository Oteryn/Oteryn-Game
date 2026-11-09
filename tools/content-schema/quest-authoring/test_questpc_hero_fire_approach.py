"""The chosen fire-route survey uses its pinned room and prior chosen stages."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
import questpc_source_approaches as sources
import test_questpc_explore_bindings as explore_tests

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.hero_of_rathleton_quest'
REFERENCE = 'source-approach:hero-into-the-fire'


class HeroFireApproachTests(unittest.TestCase):
    def test_inactive_fire_route_preserves_evidence_and_first_survey_guards(self):
        binding = ((QUEST,'s3',REFERENCE,'chosen-source/hero_of_rathleton_quest'),)
        with mock.patch.object(explore,'BINDINGS',binding):
            callback, = explore.callbacks(ROOT)
            explore_tests.ExploreBindingsTests(
                'test_declared_area_point_really_has_the_retained_native_ground'
            ).test_declared_area_point_really_has_the_retained_native_ground()
        self.assertEqual(callback['admission'],{'premium':True})
        self.assertNotIn('minimum_level',callback)
        self.assertEqual(callback['selector']['native'],dict(
            position=dict(x=33553,y=32049,floor=-15),ordinal=0,appearance=20537))
        self.assertEqual([(s['track'].rsplit('/',1)[-1],s['minimum'],s['maximum'])
                          for s in callback['storage'] if s.get('preserve')],
                         [('s1',1,1),('s2',1,1)])
        self.assertIn('https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/'
                      'data-otservbr-global/scripts/quests/hero_of_rathleton/movements_lava.lua',
                      callback['sources'])
        self.assertFalse(any(c['quest']==QUEST and '/s3/' in c['key']
                             for c in explore.callbacks(ROOT)))

    def test_other_quest_cannot_borrow_the_fire_room(self):
        with self.assertRaisesRegex(ValueError,'exact quest'):
            sources.resolve(ROOT,'oteryn:quest.grave_danger_quest',REFERENCE)
