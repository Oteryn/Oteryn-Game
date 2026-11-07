import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import current_source_inventory as subject


class InventoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.capture = Path(self.temp.name) / 'capture.jsonl'
        self.path = 'data/scripts/spells/attack/example.lua'
        self.excluded = 'data/scripts/spells/#example.lua'
        self.data = b'not executed: os.remove("anything")'
        self.blob = 'a' * 40
        self.row = dict(snapshot=subject.CRYSTAL_MAIN.snapshot,
                        source_revision=subject.CRYSTAL_MAIN.revision,
                        source_file=self.path, source_sha256=hashlib.sha256(self.data).hexdigest(),
                        git_blob=self.blob,
                        registration_key=subject.CRYSTAL_MAIN.snapshot + '/' + self.path + '#1',
                        logical_key=['instant', 'Example'], registrar={'name': 'Example'},
                        engine_enabled_path=True)
        self.write([self.row])

    def write(self, rows):
        self.capture.write_text('\n'.join(json.dumps(r) for r in rows))
        self.digest = hashlib.sha256(self.capture.read_bytes()).hexdigest()

    def git(self, repo, *args):
        if args[0] == 'ls-tree':
            self.assertEqual(args[3], subject.CRYSTAL_MAIN.revision)
            return b''.join(('100644 blob ' + self.blob + '\t' + p + '\0').encode()
                            for p in (self.path, self.excluded))
        self.assertEqual(args, ('cat-file', 'blob', self.blob))
        return self.data

    def load(self, **kwargs):
        with patch.object(subject, '_git', side_effect=self.git):
            return subject.inventory('/unused', subject.CRYSTAL_MAIN, self.capture, self.digest, **kwargs)

    def test_separate_main_provenance_and_explicit_exclusion(self):
        rows = self.load(exclusions={self.excluded: 'documentation example'})
        self.assertEqual(len(rows), 2)
        registered = next(r for r in rows if r['registration_key'])
        self.assertEqual(registered['source_revision'], subject.CRYSTAL_MAIN.revision)
        self.assertNotEqual(registered['source_revision'], subject.CRYSTAL_SUMMER.revision)
        self.assertEqual(registered['carrier'], 'instant')
        self.assertEqual(registered['registrar'], {'name': 'Example'})
        self.assertIsNone(rows[0]['carrier'])

    def test_reference_and_monster_exclusions_from_capture(self):
        self.row['excluded_reason'] = 'monster_words'
        reference = {**self.row, 'source_file': self.excluded,
                     'registration_key': self.excluded + '#0', 'logical_key': None,
                     'registrar': None, 'engine_enabled_path': False,
                     'excluded_reason': 'documentation example'}
        self.write([self.row, reference])
        rows = self.load()
        self.assertEqual({r['excluded_reason'] for r in rows},
                         {'monster_words', 'documentation example'})
        self.assertTrue(all(r['registration_key'].startswith(subject.CRYSTAL_MAIN.snapshot + '/')
                            for r in rows))

    def test_missing_file_cannot_claim_complete_inventory(self):
        with self.assertRaisesRegex(subject.InventoryError, 'Unexplained'):
            self.load()

    def test_source_tamper_is_detected(self):
        self.row['source_sha256'] = '0' * 64
        self.write([self.row])
        with self.assertRaisesRegex(subject.InventoryError, 'Source checksum'):
            self.load()

    def test_capture_tamper_does_not_touch_source(self):
        self.capture.write_text('{}')
        with patch.object(subject, '_git') as git:
            with self.assertRaisesRegex(subject.InventoryError, 'Capture checksum'):
                subject.inventory('/unused', subject.CRYSTAL_MAIN, self.capture, self.digest)
            git.assert_not_called()

    def test_wrong_branch_pin_and_duplicate_registration_rejected(self):
        self.row['source_revision'] = subject.CRYSTAL_SUMMER.revision
        self.write([self.row])
        with self.assertRaisesRegex(subject.InventoryError, 'donor/path'):
            self.load()
        self.row['source_revision'] = subject.CRYSTAL_MAIN.revision
        self.write([self.row, self.row])
        with self.assertRaisesRegex(subject.InventoryError, 'repeated'):
            self.load()


if __name__ == '__main__':
    unittest.main()
