"""The fixed expedition's chamber visit must follow the real first survey and kills."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
from test_questpc_iceberg_approach import definition, QUEST

ROOT = Path(__file__).resolve().parents[3]
REFERENCE = 'iceberg-chamber:northern-alcove'


class IcebergChamberTests(unittest.TestCase):
    def test_chamber_has_exact_native_room_and_both_predecessors(self):
        with mock.patch.object(explore, 'BINDINGS', (
                (QUEST,'s3',REFERENCE,'authored/chakoya_iceberg_mini_world_change'),)):
            callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['selector']['native'],dict(
            position=dict(x=32621,y=32555,floor=-6),ordinal=0,appearance=6585))
        self.assertEqual(callback['admission'],{'premium':True})
        self.assertNotIn('world_change',callback)
        guards = {s['track'].rsplit('/',1)[-1]:s for s in callback['storage']
                  if s.get('preserve')}
        self.assertEqual({k:(s['minimum'],s['maximum']) for k,s in guards.items()},
                         {'s1':(1,1),'s2':(3,3)})
        self.assertEqual(callback['offerings'],[])
        self.assertIsNone(callback['teleport'])

    def test_changed_chamber_intent_and_other_stage_cannot_borrow_room(self):
        import questpc_iceberg_chamber as chamber
        current = definition()
        with self.assertRaisesRegex(ValueError,'chamber survey'):
            chamber.resolve(ROOT,QUEST,'s1',REFERENCE,definition=current)
        current['recipe']['stages'][2]['objective'] = 'Inspect any visible snow bank'
        with self.assertRaisesRegex(ValueError,'chamber survey'):
            chamber.resolve(ROOT,QUEST,'s3',REFERENCE,definition=current)

    def test_chamber_is_live(self):
        key = 'oteryn:quest-callback/m01b/chakoya-iceberg-mini-world-change/s3/entry'
        self.assertEqual(sum(c['key']==key for c in explore.callbacks(ROOT)),1)


if __name__ == '__main__':
    unittest.main()
