import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import house_command_references as exporter

STAMP='2026-10-04T01:19:02Z'


class HouseCommandReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.rows=exporter.build_records(STAMP)

    def test_eight_donors_four_exact_commands_and_kick_word(self):
        self.assertEqual(len(self.rows),8)
        self.assertEqual({r['source']['source'] for r in self.rows},{'canary','crystal'})
        self.assertEqual({r['words'] for r in self.rows},{'aleta sio','aleta som','aleta grav','alana sio'})
        for row in self.rows:
            if row['operation']=='kick_character':
                self.assertEqual(row['name'],'House Kick')
                self.assertEqual(row['words'],'alana sio')
                self.assertTrue(row['typed_note_facts']['self_kick_available'])

    def test_quotes_are_exact_secondary_cache_notes_not_new_wiki_or_manual_text(self):
        notes=exporter.NOTES.read_text()
        for row in self.rows:
            self.assertIn(row['external_reference']['quote'],notes)
            self.assertEqual(row['external_reference']['cache_sha256'],exporter.sha(exporter.NOTES.read_bytes()))
            self.assertFalse(row['external_reference']['original_manual_text_verified'])
            self.assertFalse(row['fresh_wiki_verified'])
            self.assertFalse(row['runtime_activation'])
        guest=next(r for r in self.rows if r['operation']=='guest_access_list')
        self.assertTrue(guest['typed_note_facts']['premium_subowner_mentioned'])
        self.assertEqual(guest['upstream_comparison']['permission_alignment'],'basic_roles_only_premium_not_verified')

    def test_changed_cached_command_word_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            notes=Path(directory)/'houses.md'
            notes.write_text(exporter.NOTES.read_text().replace('`Aleta Sio`','`Aleta Something`'))
            with patch.object(exporter,'NOTES',notes),self.assertRaises(StopIteration):
                exporter.build_records(STAMP)

    def test_strict_schema_and_deterministic_export(self):
        from jsonschema import Draft202012Validator
        schema=Path(__file__).parent/'house-command-reference.schema.json'
        validator=Draft202012Validator(json.loads(schema.read_text()))
        for row in self.rows:validator.validate(row)
        invalid=json.loads(json.dumps(self.rows[0]));invalid['words']='aleta kick'
        self.assertTrue(list(validator.iter_errors(invalid)))
        with tempfile.TemporaryDirectory() as directory:
            first=exporter.generate(Path(directory)/'first',STAMP)
            second=exporter.generate(Path(directory)/'second',STAMP)
            self.assertEqual(first['gzip_sha256'],second['gzip_sha256'])
            self.assertEqual(first['record_count'],8)
            self.assertEqual(first['command_count'],4)
            self.assertTrue(any(a['result']=='ALL_DEVICES_OFFLINE_NO_BROWSER_ACTIONS' for a in first['online_attempts']))


if __name__=='__main__':unittest.main()
