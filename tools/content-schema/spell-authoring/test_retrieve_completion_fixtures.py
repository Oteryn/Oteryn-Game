"""External completion inputs retain byte/hash/pin/path checks before staging."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import retrieve_completion_fixtures as retrieval

class RetrievalTests(unittest.TestCase):
    def entry(self, raw=b'source\n'):
        return {'source_path':'data/scripts/spells/example.lua', 'bytes':len(raw),
                'sha256':hashlib.sha256(raw).hexdigest(),
                'git_blob':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()}
    def test_exact_bytes_are_returned_and_each_hash_component_is_required(self):
        raw=b'source\n'; entry=self.entry(raw)
        self.assertEqual(retrieval.verified_payload(entry,raw),raw)
        for changed in [{**entry,'bytes':len(raw)+1}, {**entry,'sha256':'0'*64}, {**entry,'git_blob':'0'*40}]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):retrieval.verified_payload(changed,raw)
        with self.assertRaises(ValueError):retrieval.verified_payload(entry,b'source!')
    def test_wrong_checkout_pin_refuses_before_creating_fixtures(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); manifest=root/'manifest.json'
            manifest.write_text(json.dumps({'schema':'OTERYN_SPELL_TEST_EXTERNAL_FIXTURES/v1','entries':[{}]*6}))
            with patch.object(retrieval.subprocess,'check_output',return_value='0'*40), self.assertRaises(ValueError):retrieval.stage(manifest,root,root/'sources')
            self.assertFalse((root/retrieval.PREFIX).exists())
    def test_traversal_refuses_even_with_matching_declared_pins(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); manifest=root/'manifest.json';pin=retrieval.SOURCES['canary']
            entry={**self.entry(),'source_repository':pin['repository'],'source_revision':pin['revision'],'source_path':'../private.lua','local_path':str(retrieval.PREFIX/'canary/../private.lua')}
            manifest.write_text(json.dumps({'schema':'OTERYN_SPELL_TEST_EXTERNAL_FIXTURES/v1','entries':[entry]*6}))
            revisions=[p['revision'] for p in retrieval.SOURCES.values()]
            with patch.object(retrieval.subprocess,'check_output',side_effect=revisions), self.assertRaises(ValueError):retrieval.stage(manifest,root,root/'sources')
            self.assertFalse((root/retrieval.PREFIX).exists())
if __name__=='__main__':unittest.main()
