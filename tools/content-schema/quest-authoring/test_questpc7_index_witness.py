"""Planned-index witnesses retain exact native facts and reject stale sources."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import questpc7_index_witness as witness


class PlannedIndexWitnessTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.path = self.root / witness.INDEX
        self.path.parent.mkdir(parents=True)
        self.before = json.dumps({'palette': [], 'regions': []}).encode()
        self.path.write_bytes(self.before)
        self.after = json.dumps({'palette': [], 'regions': [], 'authored_props': []}).encode()
        self.prefix = witness.INDEX + ':1:sha256:'
        self.callbacks = [{'sources': [self.prefix + hashlib.sha256(self.before).hexdigest()]}]

    def test_planned_props_rebind_without_mutating_native_input(self):
        witness.rebind(self.root, self.callbacks, {witness.INDEX: self.after})
        self.assertEqual(self.callbacks[0]['sources'],
                         [self.prefix + hashlib.sha256(self.after).hexdigest()])
        self.assertEqual(self.path.read_bytes(), self.before)

    def test_shared_source_list_is_rebound_once(self):
        self.callbacks *= 3
        witness.rebind(self.root, self.callbacks, {witness.INDEX: self.after})
        self.assertTrue(all(c['sources'] == [self.prefix + hashlib.sha256(self.after).hexdigest()]
                            for c in self.callbacks))

    def test_changed_palette_is_rejected(self):
        changed = json.dumps({'palette': [1], 'regions': []}).encode()
        with self.assertRaisesRegex(ValueError, 'native palette'):
            witness.rebind(self.root, self.callbacks, {witness.INDEX: changed})

    def test_stale_witness_is_rejected(self):
        self.callbacks[0]['sources'] = [self.prefix + '0' * 64]
        with self.assertRaisesRegex(ValueError, 'stale native index witness'):
            witness.rebind(self.root, self.callbacks, {witness.INDEX: self.after})


if __name__ == '__main__':
    unittest.main()
