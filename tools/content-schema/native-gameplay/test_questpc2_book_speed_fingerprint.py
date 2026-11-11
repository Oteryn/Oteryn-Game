import contextlib
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import questpc2_book_speed_fingerprint as tool


class BookFingerprintBounds(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.key = 'oteryn:item.tibia.i2824'
        self.path = 'content/items/definitions/spell-native-profiles.json'
        self.old = {self.key: {'production_definition': {'production_key': self.key}, 'attributes': {}},
                    'other': {'production_definition': {'production_key': 'other'}, 'attributes': {'speed_bonus': 5}}}
        self.current = copy.deepcopy(self.old)
        self.current[self.key]['attributes']['speed_bonus'] = 0
        self.hashes = {'CANONICAL_OUTSIDE': 'a' * 64, 'NATIVE_OUTSIDE': 'b' * 64}
        self.next_hashes = {**self.hashes, 'NATIVE_OUTSIDE': 'c' * 64}
        self.text = '\n'.join(f"{k} = '{v}'" for k, v in self.hashes.items()) + '\n'
        self.head = 'base'
        self.target = self.root / tool.proof.TEST
        self.target.parent.mkdir(parents=True)
        self.target.write_text(self.text)
        (self.root / self.path).parent.mkdir(parents=True)
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(patch.object(tool.proof, 'ROOT', self.root))
        self.stack.enter_context(patch.object(tool.proof, 'git', side_effect=self.git))
        self.stack.enter_context(patch.object(tool.proof, 'fingerprints', side_effect=
            lambda revision=None: self.hashes if revision else self.next_hashes))
        self.source = self.stack.enter_context(patch.object(tool.source, 'profiles', return_value=
            {self.key: {'attributes': {'speed_bonus': 0}}}))

    def git(self, *args):
        if args[0] == 'rev-parse':
            return (self.head if args[1] == 'HEAD' else 'base').encode()
        if args == ('show', 'base:' + tool.proof.TEST):
            return self.text.encode()
        if args == ('show', 'base:' + self.path):
            return json.dumps({'records': list(self.old.values())}).encode()
        raise AssertionError(args)

    def refresh(self):
        (self.root / self.path).write_text(json.dumps({'records': list(self.current.values())}))
        tool.refresh('base')

    def refuses(self, error):
        before = self.target.read_bytes()
        with self.assertRaisesRegex(ValueError, error):
            self.refresh()
        self.assertEqual(self.target.read_bytes(), before)

    def test_exact_leaf_refresh_keeps_canonical_proof(self):
        self.refresh()
        self.assertEqual(self.target.read_text(), self.text.replace('b' * 64, 'c' * 64))

    def test_another_record_cannot_be_hidden(self):
        self.current['other']['attributes']['speed_bonus'] = 4
        self.refuses('ONLY_ONE_EXISTING_RECORD')

    def test_other_book_fields_cannot_be_hidden(self):
        self.current[self.key]['semantics'] = {}
        self.refuses('EXACT_SOURCE_LEAF')

    def test_canonical_change_cannot_be_hidden(self):
        self.next_hashes['CANONICAL_OUTSIDE'] = 'd' * 64
        self.refuses('CANONICAL_DEFINITIONS_MUST_STAY_IDENTICAL')

    def test_known_stat_cannot_be_retuned(self):
        self.old[self.key]['attributes']['speed_bonus'] = 4
        self.refuses('ACCEPTED_STAT_MUST_NOT_BE_RETUNED')

    def test_unqualified_source_cannot_authorize_zero(self):
        self.source.return_value[self.key]['attributes']['speed_bonus'] = 7
        self.refuses('EXACT_SOURCE_LEAF')

    def test_old_proof_must_reproduce(self):
        self.text = self.text.replace('b' * 64, 'e' * 64)
        self.refuses('PREVIOUS_PROOF_DOES_NOT_REPRODUCE')

    def test_wrong_base_cannot_refresh(self):
        self.head = 'other'
        self.refuses('BASE_MUST_BE_HEAD')


if __name__ == '__main__':
    unittest.main()
