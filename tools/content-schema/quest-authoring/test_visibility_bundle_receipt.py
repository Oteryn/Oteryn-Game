"""The G-2 receipt refresh cannot approve unrelated SOURCE or schema changes."""
import hashlib
import tempfile
import unittest
from pathlib import Path

from amend_monk_source_receipt import BUNDLE, HERE, verify_visibility_bundle
from bundle_authoring import encoded


class VisibilityBundleReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.schema_path = HERE + 'quest_content.schema.json'
        self.schema = '{"$defs":{"quest":{"properties": {"identity": {"type": "string"}}}}}\n'
        self.bundle = {'quests': [{'name': 'unchanged source'}], 'schema_provenance': [
            {'path': self.schema_path, 'sha256': hashlib.sha256(self.schema.encode()).hexdigest()},
            {'path': 'unrelated.schema.json', 'sha256': 'a' * 64}]}
        self.descriptor = {'path': BUNDLE, 'sha256': hashlib.sha256(encoded(self.bundle).encode()).hexdigest()}
        self.schema = self.schema.replace('"properties": {',
            '"properties": {"visibility":{"type":"string","enum":["public","hidden"],"default":"public"},')
        self.bundle['schema_provenance'][0]['sha256'] = hashlib.sha256(self.schema.encode()).hexdigest()

    def write(self):
        for path, text in ((self.schema_path, self.schema), (BUNDLE, encoded(self.bundle))):
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)

    def test_exact_visibility_addition_and_idempotent_refresh(self):
        self.write()
        verify_visibility_bundle(self.root, self.descriptor)
        updated = dict(self.descriptor, sha256=hashlib.sha256((self.root / BUNDLE).read_bytes()).hexdigest())
        verify_visibility_bundle(self.root, updated)

    def test_source_changes_are_rejected(self):
        self.bundle['quests'][0]['name'] = 'substituted source'
        self.write()
        with self.assertRaisesRegex(ValueError, 'beyond optional G-2'):
            verify_visibility_bundle(self.root, self.descriptor)

    def test_other_schema_changes_are_rejected(self):
        self.schema = self.schema.replace('"identity": {"type": "string"}',
                                          '"identity": {"type": "number"}')
        self.bundle['schema_provenance'][0]['sha256'] = hashlib.sha256(self.schema.encode()).hexdigest()
        self.write()
        with self.assertRaisesRegex(ValueError, 'beyond optional G-2'):
            verify_visibility_bundle(self.root, self.descriptor)

    def test_other_provenance_changes_are_rejected(self):
        self.bundle['schema_provenance'][1]['sha256'] = 'b' * 64
        self.write()
        with self.assertRaisesRegex(ValueError, 'beyond optional G-2'):
            verify_visibility_bundle(self.root, self.descriptor)

    def test_visibility_policy_changes_are_rejected(self):
        self.schema = self.schema.replace('"default":"public"', '"default":"hidden"')
        self.write()
        with self.assertRaisesRegex(ValueError, 'exact optional G-2'):
            verify_visibility_bundle(self.root, self.descriptor)

    def test_visibility_on_another_definition_is_rejected(self):
        self.schema = self.schema.replace('"quest"', '"another"')
        self.write()
        with self.assertRaisesRegex(ValueError, 'must belong to the Quest'):
            verify_visibility_bundle(self.root, self.descriptor)


if __name__ == '__main__':
    unittest.main()
