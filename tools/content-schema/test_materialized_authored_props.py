"""Pinned authored prop documents remain part of the exact world family inventory."""
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import validate_materialized_game_tree as tree


class AuthoredPropInventory(unittest.TestCase):
    def test_static_source_registry_is_a_successor_not_a_legacy_locator(self):
        registry = 'source-static-definitions.json'
        self.assertNotIn(registry, tree.legacy_locators())
        self.assertIn(registry, tree.world_successor_files(tree.directory_nodes()))

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.family = 'content/world/placements/'
        self.path = self.family + 'authored-fixture.json'
        target = self.root / self.path
        target.parent.mkdir(parents=True)
        target.write_bytes(b'{"props":[]}\n')
        self.payload = {'population_state': 'POPULATED', 'shards': [],
                        'authored_props': [{'path': self.path,
                         'sha256': hashlib.sha256(target.read_bytes()).hexdigest()}]}
        self.local = {'index.json', 'authored-fixture.json'}

    def validate(self):
        with patch.object(tree, 'ROOT', self.root):
            tree.check_family_index(self.family, self.payload, self.local, set())

    def test_exact_pinned_document_is_in_inventory(self):
        self.validate()

    def test_unpinned_document_still_fails(self):
        self.payload['authored_props'] = []
        with self.assertRaisesRegex(tree.ValidationError, 'WORLD_STRAY_FILE'):
            self.validate()

    def test_wrong_hash_fails(self):
        self.payload['authored_props'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(tree.ValidationError, 'WORLD_AUTHORED_PROP_HASH'):
            self.validate()

    def test_path_escape_fails(self):
        self.payload['authored_props'][0]['path'] = self.family + '../outside.json'
        with self.assertRaisesRegex(tree.ValidationError, 'WORLD_AUTHORED_PROP_PATH'):
            self.validate()

    def test_symlink_fails(self):
        target = self.root / self.path
        target.unlink()
        outside = self.root / 'outside.json'
        outside.write_bytes(b'{"props":[]}\n')
        target.symlink_to(outside)
        with self.assertRaisesRegex(tree.ValidationError, 'WORLD_AUTHORED_PROP_FILE'):
            self.validate()


if __name__ == '__main__':
    unittest.main()
