"""Only the chosen personal Bewitched survey may use the permanent cauldron."""
import copy
import json
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.authored.bewitched'
REFERENCE = 'cauldron-approach:green-claw-swamp'


def definition():
    return next(r['definition']
                for p in (ROOT / 'content/quests/definitions').glob('quests-*.json')
                for r in json.loads(p.read_text())['records']
                if r['definition']['identity']['key'] == QUEST)


class CauldronApproachTests(unittest.TestCase):
    def test_personal_cauldron_survey_is_live(self):
        self.assertEqual(sum(c['quest'] == QUEST for c in explore.callbacks(ROOT)), 1)

    def test_personal_survey_can_reach_the_retained_cauldron(self):
        binding = ((QUEST, 's1', REFERENCE, 'authored/bewitched'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['selector']['native'], dict(
            position=dict(x=32733, y=31935, floor=-12), ordinal=0, appearance=7272))
        self.assertNotIn('world_change', callback)
        self.assertNotIn('admission', callback)
        self.assertEqual(callback['storage'][1]['track'],
                         'oteryn:quest-progress/authored/bewitched/s1')

    def test_global_or_changed_objectives_cannot_borrow_the_personal_survey(self):
        import questpc_cauldron_approaches as cauldrons
        original = definition()
        for field in ('objective', 'adaptations', 'source'):
            changed = copy.deepcopy(original)
            if field == 'objective':
                changed['recipe']['stages'][0]['objective'] = 'Reach an active global cauldron'
            elif field == 'adaptations':
                changed['recipe']['adaptations'] = []
            else:
                changed['recipe']['source_refs'] = []
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'personal cauldron'):
                cauldrons.resolve(ROOT, QUEST, 's1', REFERENCE, definition=changed)
        with self.assertRaisesRegex(ValueError, 'personal cauldron'):
            cauldrons.resolve(ROOT, QUEST, 's4', REFERENCE, definition=original)


if __name__ == '__main__':
    unittest.main()
