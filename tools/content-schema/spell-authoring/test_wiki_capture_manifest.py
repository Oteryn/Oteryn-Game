"""A supplementary web source must retain its real provenance and capture pin."""
import copy
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator


class CommunityCaptureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        schema = Path(__file__).resolve().parent.parent / 'monster-authoring' / 'monster-import-readiness.schema.json'
        cls.validator = Draft202012Validator(json.loads(schema.read_text()))

    def manifest(self):
        return {'sources': [{'kind': 'community_capture',
                             'url': 'https://tibiopedia.pl/spells/Sudden_Death_Rune',
                             'title': 'Sudden Death Rune', 'captured': '2026-09-28',
                             'content_sha256': 'e214da90f5ccf7fed7f27393413b5d9ddbc548c72c9d2dfefe49f3075a3e1da9'}],
                'entries': [{'source_index': 0, 'source_file': 'Sudden Death Rune',
                             'source_line': 1, 'source_field': 'spellrange', 'kind': 'field',
                             'status': 'mapped', 'destination': '/spell/spell/targeting/range_tiles',
                             'resolution': 'Supplementary range evidence where primary sources are silent.'}]}

    def test_dated_hashed_community_page_does_not_require_fake_revision(self):
        self.assertEqual(list(self.validator.iter_errors(self.manifest())), [])

    def test_unpinned_or_unattributed_capture_is_rejected(self):
        original = self.manifest()
        for field in ('url', 'title', 'captured', 'content_sha256'):
            with self.subTest(field=field):
                manifest = copy.deepcopy(original)
                del manifest['sources'][0][field]
                self.assertTrue(list(self.validator.iter_errors(manifest)))
        for field, value in (('url', 'http://example.com/'), ('content_sha256', 'unverified'),
                             ('captured', 'today'), ('title', '')):
            with self.subTest(field=field, value=value):
                manifest = copy.deepcopy(original)
                manifest['sources'][0][field] = value
                self.assertTrue(list(self.validator.iter_errors(manifest)))

    def test_community_capture_cannot_smuggle_mediawiki_revision_fields(self):
        manifest = self.manifest()
        manifest['sources'][0].update(page_id=1, revision_id=1)
        self.assertTrue(list(self.validator.iter_errors(manifest)))


if __name__ == '__main__':
    unittest.main()
