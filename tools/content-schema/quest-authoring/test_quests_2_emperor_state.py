import copy
import json
import os
import unittest
from pathlib import Path

import quests_2_emperor_state as authoring


class EmperorTracks(unittest.TestCase):
    def setUp(self):
        self.root = Path(os.environ.get('OTERYN_EMPEROR_TEST_ROOT', authoring.ROOT))
        self.quests = json.loads((self.root / 'content/quests/missions/quest-state.json').read_bytes())['quests']
        # The standalone writer may already have been run on this checkout.
        for quest in self.quests:
            quest['tracks'] = [t for t in quest['tracks'] if t['key'] not in authoring.KEYS]

    def test_every_callback_source_flag_has_the_existing_quest_owner(self):
        authoring.append_tracks(self.quests)
        found = {t['key']: t for q in self.quests for t in q['tracks'] if t['key'] in authoring.KEYS}
        self.assertEqual(set(found), set(authoring.KEYS))
        for key, track in found.items():
            self.assertEqual((track['initial'], track['min'], track['max']), (-1, -1, 1))
            self.assertEqual(track['source_key'], 'crystalserver:' + key.removeprefix('oteryn:'))
            self.assertIn(track['quest'], {q['quest'] for q in self.quests})

    def test_existing_source_or_initial_state_is_never_overwritten(self):
        authoring.append_tracks(self.quests)
        changed = copy.deepcopy(self.quests)
        target = next(t for q in changed for t in q['tracks'] if t['key'] in authoring.KEYS)
        target['initial'] = 0
        before = copy.deepcopy(changed)
        with self.assertRaisesRegex(ValueError, 'TRACK_CONFLICT'):
            authoring.append_tracks(changed)
        self.assertEqual(changed, before)

    def test_missing_quest_is_an_error_without_partial_mutation(self):
        changed = [q for q in self.quests if q['quest'] != 'oteryn:quest.the_new_frontier_quest']
        before = copy.deepcopy(changed)
        with self.assertRaisesRegex(ValueError, 'QUEST_OWNER'):
            authoring.append_tracks(changed)
        self.assertEqual(changed, before)


if __name__ == '__main__':
    unittest.main()
