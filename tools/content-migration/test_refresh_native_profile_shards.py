"""A shard refresh cannot approve a changed native Item or source profile."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import refresh_native_profile_shards as tool


def encoded(value):
    return (json.dumps(value, indent=2) + '\n').encode()


class NativeProfileShardTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.shard = 'content/items/definitions/items-00000-00001.json'
        self.profile = 'tools/content-schema/native-gameplay/fixture.json'
        self.definition = {'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i1', 'revision': 'r1'},
                           'materializable': False}
        self.document = {'records': [{'definition': self.definition}, {'definition': {
            'identity': {'family': 'Item', 'key': 'oteryn:item.tibia.i2', 'revision': 'r1'},
            'materializable': False}}]}
        raw = encoded(self.document)
        self.packet = {'records': [{'production_definition': {
            'family': 'Item', 'production_key': 'oteryn:item.tibia.i1', 'revision_ref': 'r1'},
            'semantics': {'source_term': 20}}], 'source_revision': 'fixed',
            'canonical_inputs': [{'path': self.shard, 'sha256': hashlib.sha256(raw).hexdigest()}]}
        self.base = {self.shard: raw, self.profile: encoded(self.packet)}
        self.write(self.shard, raw)
        self.write(self.profile, self.base[self.profile])

    def write(self, relative, raw):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)

    def refresh(self):
        return tool.refreshed(self.root, self.profile, self.base.__getitem__)

    def test_unrelated_row_changes_rebind_only_the_shard_digest(self):
        changed = copy.deepcopy(self.document)
        changed['records'][1]['definition']['materializable'] = True
        self.write(self.shard, encoded(changed))
        result = json.loads(self.refresh())
        expected = copy.deepcopy(self.packet)
        expected['canonical_inputs'][0]['sha256'] = hashlib.sha256(encoded(changed)).hexdigest()
        self.assertEqual(result, expected)
        self.write(self.profile, encoded(result))
        self.assertEqual(self.refresh(), encoded(result), 'refresh remains idempotent')

    def test_selected_item_change_cannot_be_rebound(self):
        self.document['records'][0]['definition']['materializable'] = True
        self.write(self.shard, encoded(self.document))
        with self.assertRaisesRegex(ValueError, 'CANONICAL_ITEM_CHANGED'):
            self.refresh()

    def test_source_term_change_cannot_be_rebound(self):
        changed = copy.deepcopy(self.packet)
        changed['records'][0]['semantics']['source_term'] = 21
        self.write(self.profile, encoded(changed))
        with self.assertRaisesRegex(ValueError, 'TERMS_CHANGED'):
            self.refresh()

    def test_old_shard_must_match_its_committed_witness(self):
        self.base[self.shard] += b' '
        with self.assertRaisesRegex(ValueError, 'BASE_SHARD_PIN'):
            self.refresh()

    def test_json_value_types_are_part_of_the_selected_witness(self):
        self.document['records'][0]['definition']['materializable'] = 0
        self.write(self.shard, encoded(self.document))
        with self.assertRaisesRegex(ValueError, 'CANONICAL_ITEM_CHANGED'):
            self.refresh()

    def test_rejected_items_are_selected_witnesses_too(self):
        self.packet['rejected'] = [{'key': 'oteryn:item.tibia.i2', 'reason': 'fixed'}]
        self.base[self.profile] = encoded(self.packet)
        self.write(self.profile, self.base[self.profile])
        self.document['records'][1]['definition']['materializable'] = True
        self.write(self.shard, encoded(self.document))
        with self.assertRaisesRegex(ValueError, 'CANONICAL_ITEM_CHANGED'):
            self.refresh()


if __name__ == '__main__':
    unittest.main()
