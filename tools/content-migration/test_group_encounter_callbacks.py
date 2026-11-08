"""Pinned group source selection, exact native layout and unrelated callback preservation."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import group_encounter_callbacks as group


class GroupEncounterCallbackTests(unittest.TestCase):
    def test_retained_source_and_exact_four_player_six_demon_layout(self):
        group.verify_source()
        callback = group.annihilator()
        self.assertTrue(callback['key'].startswith('oteryn:quest-callback/'))
        encounter = callback['effects']['encounter']
        self.assertEqual(callback['selector']['unique'], 30025)
        self.assertEqual(callback['lever'], [2772, 2773])
        self.assertFalse(callback['reversible'])
        self.assertEqual(encounter['minimum_level'], 100)
        self.assertEqual([m['from']['x'] for m in encounter['players']],
                         [33225, 33224, 33223, 33222])
        self.assertEqual([m['to']['x'] for m in encounter['players']],
                         [33222, 33221, 33220, 33219])
        self.assertEqual({(c['position']['x'], c['position']['y']) for c in encounter['monsters']},
                         {(33219, 31657), (33221, 31657), (33223, 31659),
                          (33224, 31659), (33220, 31661), (33222, 31661)})
        self.assertTrue(all(c['creature'] == 'oteryn:creature.angry_demon'
                            for c in encounter['monsters']))
        self.assertNotIn('cooldown', encounter)
        self.assertEqual(callback['storage'], [])

    def test_overlay_preserves_other_quests_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / group.CALLBACKS
            path.parent.mkdir(parents=True)
            unrelated = {'key': 'other/quest', 'retained': {'exact': [1, 2, 3]}}
            before = {'schema': 1, 'callbacks': [copy.deepcopy(unrelated), group.annihilator()]}
            path.write_text(json.dumps(before))
            with mock.patch.object(group, 'verify_source'):
                first = group.outputs(root)[group.CALLBACKS]
                path.write_bytes(first)
                self.assertEqual(group.outputs(root)[group.CALLBACKS], first)
            self.assertEqual(first, json.dumps(before).encode())
            changed = copy.deepcopy(before)
            changed['callbacks'][1]['effects']['encounter']['minimum_level'] = 99
            path.write_text(json.dumps(changed))
            with mock.patch.object(group, 'verify_source'):
                with self.assertRaisesRegex(ValueError, 'SHARED_OWNER_DRIFT'):
                    group.outputs(root)
            self.assertEqual(json.loads(path.read_bytes()), changed)
            self.assertEqual(next(c for c in json.loads(first)['callbacks']
                                  if c['key'] == unrelated['key']), unrelated)

    def test_tampered_source_and_duplicate_callback_refuse_without_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / group.ARCHIVE
            source.parent.mkdir(parents=True)
            source.write_bytes(b'tampered archive')
            path = root / group.CALLBACKS
            path.parent.mkdir(parents=True)
            path.write_text('{"schema":1,"callbacks":[]}')
            before = path.read_bytes()
            with self.assertRaisesRegex(ValueError, 'SOURCE_ARCHIVE_PIN'):
                group.outputs(root)
            self.assertEqual(path.read_bytes(), before)
            path.write_text(json.dumps({'schema': 1, 'callbacks': [{'key': 'x'}, {'key': 'x'}]}))
            before = path.read_bytes()
            with mock.patch.object(group, 'verify_source'):
                with self.assertRaisesRegex(ValueError, 'DUPLICATE_CALLBACK'):
                    group.outputs(root)
            self.assertEqual(path.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
