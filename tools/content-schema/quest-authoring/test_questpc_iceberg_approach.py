"""The chosen personal expedition uses the actual retained Port Hope iceberg."""
import copy
import json
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore
import questpc_world_requirements as owners

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.authored.chakoya_iceberg_mini_world_change'
REFERENCE = 'iceberg-approach:port-hope'


def definition():
    return next(r['definition']
                for p in (ROOT / 'content/quests/definitions').glob('quests-*.json')
                for r in json.loads(p.read_text())['records']
                if r['definition']['identity']['key'] == QUEST)


class IcebergApproachTests(unittest.TestCase):
    def test_native_landmark_keeps_its_complete_source_stack(self):
        from questpc_native_anchors import NativeAnchors
        native = NativeAnchors(ROOT)
        at = dict(x=32623, y=32584, floor=-7)
        native.require(at, [799, 7152])
        with self.assertRaisesRegex(ValueError, 'native EXPLORE ground'):
            native.require(at, [799, 7153])

    def test_fixed_personal_expedition_keeps_premium_without_world_activation(self):
        binding = ((QUEST, 's1', REFERENCE, 'authored/chakoya_iceberg_mini_world_change'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['selector']['native'], dict(
            position=dict(x=32622, y=32585, floor=-7), ordinal=0, appearance=6585))
        self.assertEqual(callback['admission'], {'premium': True})
        self.assertNotIn('world_change', callback)
        self.assertTrue(callback['requires_world_admission'])
        with self.assertRaisesRegex(ValueError, 'admitted World owner'):
            owners.requirement(QUEST, owners.catalogue(ROOT))

    def test_dynamic_iceberg_or_other_stages_cannot_borrow_the_fixed_route(self):
        import questpc_iceberg_approaches as iceberg
        original = definition()
        for field in ('objective', 'adaptations', 'source', 'repeat'):
            changed = copy.deepcopy(original)
            if field == 'objective':
                changed['recipe']['stages'][0]['objective'] = 'Find the currently active iceberg'
            elif field == 'adaptations':
                changed['recipe']['adaptations'] = []
            elif field == 'source':
                changed['recipe']['source_refs'] = []
            else:
                changed['recipe']['repeat']['kind'] = 'daily'
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'personal iceberg'):
                iceberg.resolve(ROOT, QUEST, 's1', REFERENCE, definition=changed)
        with self.assertRaisesRegex(ValueError, 'personal iceberg'):
            iceberg.resolve(ROOT, QUEST, 's3', REFERENCE, definition=original)


if __name__ == '__main__':
    unittest.main()
