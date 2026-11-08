"""Merged tree registries must count the exact admitted family indexes."""
import json
import tempfile
import unittest
from pathlib import Path
import questpc5_tree_registry as author


class TreeRegistryCounts(unittest.TestCase):
    def test_current_cook_and_source_leaf_counts_are_derived_from_indexes(self):
        output = author.outputs()
        manifest = json.loads(output['content/manifest.json'])
        lock = json.loads(output['content/content.lock.json'])
        self.assertEqual(manifest['families']['Item']['records'], 34114)
        self.assertEqual(manifest['families']['RewardClaim']['records'], 337)
        self.assertEqual(lock['family_counts']['Item'], 34114)
        self.assertEqual(lock['family_counts']['RewardClaim'], 337)
        self.assertEqual(lock['source_binding_counts']['Item'], 479)
        original = json.loads((author.ROOT / 'content/content.lock.json').read_bytes())
        for family in set(original['family_counts']) - set(author.FAMILIES):
            self.assertEqual(lock['family_counts'][family], original['family_counts'][family])
        old_manifest = json.loads((author.ROOT / 'content/manifest.json').read_bytes())
        self.assertEqual(manifest['managed_files'], old_manifest['managed_files'])

    def test_pending_native_registration_is_not_replaced_by_old_disk_bytes(self):
        manifest = json.loads((author.ROOT / 'content/manifest.json').read_bytes())
        manifest['managed_files'].append({'path': 'content/test-pending-provider.json'})
        item_index = json.loads((author.ROOT / author.FAMILIES['Item']).read_bytes())
        item_index['record_count'] += 1
        item_index['attached_source_bindings'] += 1
        pending = {'content/manifest.json': json.dumps(manifest).encode(),
                   author.FAMILIES['Item']: json.dumps(item_index).encode()}
        output = author.outputs(generated=pending)
        actual = json.loads(output['content/manifest.json'])
        self.assertEqual(actual['managed_files'], manifest['managed_files'])
        self.assertEqual(actual['families']['Item']['records'], item_index['record_count'])
        lock = json.loads(output['content/content.lock.json'])
        self.assertEqual(lock['source_binding_counts']['Item'], item_index['attached_source_bindings'])

    def test_wrong_index_family_or_non_integer_count_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ('content/manifest.json', 'content/content.lock.json', *author.FAMILIES.values()):
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((author.ROOT / name).read_bytes())
            path = root / author.FAMILIES['Item']
            original = json.loads(path.read_bytes())
            for field, value in [('family', 'Mount'), ('record_count', True), ('attached_source_bindings', -1)]:
                with self.subTest(field=field):
                    altered = dict(original)
                    altered[field] = value
                    path.write_text(json.dumps(altered))
                    with self.assertRaisesRegex(ValueError, 'TREE_REGISTRY_INDEX'):
                        author.outputs(root)


if __name__ == '__main__':
    unittest.main()
