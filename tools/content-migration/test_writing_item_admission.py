"""Source provenance and preservation of already accepted text/item facts."""
import copy
import json
import unittest
from pathlib import Path
import writing_item_admission as writing
import dispenser_item_profiles as donor

class WritingAdmission(unittest.TestCase):
    def test_captured_cohort_matches_qualified_text_owner(self):
        packet=json.loads((donor.ROOT/writing.PATH).read_bytes())
        self.assertEqual(tuple(row['id'] for row in packet['source_rows']),writing.ids())
        self.assertEqual(len(packet['records']),38)
        self.assertEqual(packet['source_revision'],donor.REVISION)
        self.assertEqual(packet['source_xml_sha256'],donor.XML_SHA)
        self.assertEqual(packet['records'],donor.records(packet['source_rows'],ids=writing.ids()))
        for row in packet['records']:
            self.assertTrue(row['admission']['materializable'])
    def test_physical_admission_does_not_replace_accepted_text_rules(self):
        old={'state':'KNOWN','value':{'writable':False,'limit':1024}}
        self.assertEqual(writing.merge_known(old,{'state':'UNKNOWN'}),old)
        with self.assertRaisesRegex(ValueError,'KNOWN_FACT_CONFLICT'):
            writing.merge_known(old,{'state':'KNOWN','value':{'writable':True,'limit':1024}})
        self.assertEqual(writing.merge_known(200,150,('physical','value','weight','value')),200)
    def test_current_xml_digest_is_required(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'items.xml';path.write_text('<items/>')
            with self.assertRaisesRegex(ValueError,'CURRENT_XML_DIGEST'):
                donor.capture(path,writing.ids())

if __name__=='__main__': unittest.main()
