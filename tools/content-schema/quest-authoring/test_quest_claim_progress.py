"""A source reward cannot teach access through a held or mismatched projection."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import quest_claim_progress as tool

ROOT = Path(__file__).resolve().parents[3]


class NativeClaimProgressTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.row = json.loads((ROOT / tool.PATH).read_text())['bindings'][0]
        self.path = self.root / tool.PATH
        self.path.parent.mkdir(parents=True)
        self.save([self.row])
        source = self.row['source']
        witness = {'repository': source['repository'], 'revision': source['revision'],
                   'source_path': source['path'], 'source_blob_sha1': source['blob_sha1']}
        raw = json.dumps(witness).encode()
        target = self.root / tool.SOURCE
        target.parent.mkdir(parents=True)
        target.write_bytes(raw)
        self.definition = {
            'identity': {'key': self.row['claim'], 'revision': self.row['claim_revision']},
            'definition_profile': 'authored_variant_v1', 'data_holds': [],
            'placements': [{}], 'native_contents': {
                'revision': source['revision'], 'source_sha256': hashlib.sha256(raw).hexdigest(),
                'placements': [{'unique_id': self.row['unique_id'], 'line': source['line'],
                    'root': {'family': 'Item', 'key': self.row['item'], 'revision': 'definition-r1'},
                    'completion': {'kind': 'storage', 'key': self.row['source_completion']}}]}}

    def save(self, rows):
        self.path.write_text(json.dumps({'schema': 1, 'bindings': rows}))

    def test_exact_admitted_source_keeps_reward_and_adds_only_the_obligation(self):
        before = copy.deepcopy(self.definition)
        tool.apply_claims(self.root, [{'definition': self.definition}])
        self.assertEqual(self.definition['placements'][0]['quest_transition'], self.row['transition'])
        before['placements'][0]['quest_transition'] = self.row['transition']
        self.assertEqual(self.definition, before)

    def test_held_missing_or_different_source_cannot_grant_access(self):
        for change in ('held', 'projection', 'item', 'uid', 'source', 'obligation'):
            definition = copy.deepcopy(self.definition)
            if change == 'held':
                definition['data_holds'] = [{'reason': 'test hold'}]
            elif change == 'projection':
                del definition['native_contents']
            elif change == 'item':
                definition['native_contents']['placements'][0]['root']['key'] = 'oteryn:item.tibia.i2821'
            elif change == 'uid':
                definition['native_contents']['placements'][0]['unique_id'] += 1
            elif change == 'source':
                definition['native_contents']['source_sha256'] = '0' * 64
            else:
                definition['placements'][0]['quest_transition'] = 'oteryn:quest-transition/another'
            with self.subTest(change=change), self.assertRaises(ValueError):
                tool.apply_claims(self.root, [{'definition': definition}])

    def test_duplicate_and_out_of_bounds_bindings_fail_closed(self):
        self.save([self.row, self.row])
        with self.assertRaises(ValueError):
            tool.load(self.root)
        invalid = copy.deepcopy(self.row)
        invalid['maximum'] += 1
        self.save([invalid])
        with self.assertRaises(ValueError):
            tool.load(self.root)
        invalid = copy.deepcopy(self.row)
        invalid['value'] = invalid['maximum'] + 1
        self.save([invalid])
        with self.assertRaises(ValueError):
            tool.load(self.root)

    def test_transition_and_track_have_one_owner(self):
        definitions = [{'identity': {'key': self.row['quest']},
                        'source_refs': {'quest': {'key': 'canary:quest/fixture'}}}]
        quests = []
        tool.extend_state(self.root, quests, definitions)
        self.assertEqual(quests[0]['transitions'][0]['effects'][0]['effect'],
                         {'kind': 'SET', 'value': 1})
        self.assertFalse(quests[0]['transitions'][0]['completes'])
        self.assertEqual(quests[0]['transitions'][0]['effects'][0]['from'], {'op': 'ANY'})
        with self.assertRaises(ValueError):
            tool.extend_state(self.root, quests, definitions)


if __name__ == '__main__':
    unittest.main()
