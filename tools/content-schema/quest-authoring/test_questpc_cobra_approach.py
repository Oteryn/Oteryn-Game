"""The Cobra investigation remains behind every preceding Grave Danger stage."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
import test_questpc_explore_bindings as explore_tests

ROOT = Path(__file__).resolve().parents[3]


class CobraApproachTests(unittest.TestCase):
    def test_cobra_plan_preserves_all_predecessor_guards(self):
        binding = (('oteryn:quest.grave_danger_quest', 's4',
                    'area-approach:oteryn:area.hunting_place.cobra_bastion',
                    'chosen-source/grave_danger_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            explore_tests.ExploreBindingsTests(
                'test_declared_area_point_really_has_the_retained_native_ground'
            ).test_declared_area_point_really_has_the_retained_native_ground()
        self.assertEqual(callback['minimum_level'], 250)
        self.assertEqual(callback['admission'], {'premium': True})
        self.assertEqual(callback['selector']['native'], dict(
            position=dict(x=33301,y=32648,floor=-7), ordinal=0, appearance=231))
        guards = [s for s in callback['storage'] if s.get('preserve')]
        self.assertEqual([(s['track'].rsplit('/',1)[-1],s['minimum'],s['maximum'])
                          for s in guards], [('s1',12,12),('s2',5,5),('s3',1,1)])
        self.assertEqual(len([s for s in callback['storage'] if not s.get('preserve')]),2)
        self.assertTrue(any(c['quest']=='oteryn:quest.grave_danger_quest'
                             and '/s4/' in c['key'] for c in explore.callbacks(ROOT)))
