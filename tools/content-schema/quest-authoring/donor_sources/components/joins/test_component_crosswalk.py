import json
import os
from pathlib import Path
import unittest
import component_crosswalk as subject

ROOT = next(p for p in Path(__file__).resolve().parents if (p / 'tools/content-schema/quest-authoring').is_dir())
ASSIGNMENT = ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/components248/assignments/all.json'

class LexicalEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import sys
        sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
        import lua_writers
        cls.mask = staticmethod(lua_writers.mask_code)

    def test_comments_and_strings_do_not_join(self):
        text = '-- x:getStorageValue(Storage.Foo)\nlocal a="x:getStorageValue(Storage.Foo)"\n--[[x:getStorageValue(123)]]'
        self.assertEqual(subject.storage_mentions(text, self.mask), [])

    def test_accessor_only(self):
        text = 'local x=Storage.Foo\np:getStorageValue(Storage.Foo.Bar)\np:setStorageValue(123, 1)'
        self.assertEqual([x[0] for x in subject.storage_mentions(text, self.mask)], ['Storage.Foo.Bar', '123'])

    def test_shadowed_accessor_remains_lexical_only(self):
        text = 'local custom={getStorageValue=function(x) return x end};custom:getStorageValue(Storage.Foo)'
        self.assertEqual([x[0] for x in subject.storage_mentions(text, self.mask)], ['Storage.Foo'])
        # A token witness is not receiver dispatch resolution. Packet enforces this qualification.

    def test_offsets(self):
        text = '-- łódź\np:getStorageValue(Storage.Foo)'
        token, start, end = subject.storage_mentions(text, self.mask)[0]
        self.assertEqual(text[start:end], token)

class RealPacketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import tempfile, tarfile
        temporary = tempfile.TemporaryDirectory()
        cls.addClassCleanup(temporary.cleanup)
        extracted = Path(temporary.name)
        with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz', 'r:gz') as archive:
            archive.extractall(extracted, filter='data')
        cls.packet = subject.build(ROOT, ASSIGNMENT, extracted / 'corpus-manifest.json')

    def test_all_components_and_quests(self):
        self.assertEqual(self.packet['summary']['components'], 248)
        self.assertEqual(self.packet['summary']['quests'], 352)
        self.assertEqual({r['source_component_id'] for r in self.packet['components']}, {r['source_component_id'] for r in subject.read_json(ASSIGNMENT)['records']})

    def test_directory_does_not_promote(self):
        for r in self.packet['components']:
            self.assertEqual(bool(r['accepted_quest_keys']), bool(r['proofs']))
            self.assertFalse(set(r['accepted_quest_keys']) & set(r['candidate_quest_keys']))

    def test_no_completeness_promotion(self):
        self.assertEqual(self.packet['semantic_quest_completeness'], 'NOT_ESTABLISHED')
        self.assertTrue(all(q['quest_completeness'] == 'NOT_ASSESSED' for q in self.packet['quests']))
        self.assertTrue(all('WHOLE_FILE_SEMANTIC_COVERAGE_NOT_ESTABLISHED' in c['holds'] for c in self.packet['components']))

    def test_pinned_proof_span(self):
        for r in self.packet['components']:
            for p in r['proofs']:
                if 'component_evidence' in p:
                    e = p['component_evidence']
                    self.assertEqual(p['accessor_dispatch_binding'], 'NOT_PROVEN')
                    self.assertEqual(e['token_sha256'], subject.digest(e['token'].encode()))
                    self.assertEqual(p['existing_occurrence']['revision'], r['provenance']['revision'])

    def test_schema_rejects_extra_and_promoted_quest(self):
        import jsonschema
        schema = subject.read_json(Path(__file__).with_name('component-crosswalk.schema.json'))
        jsonschema.validate(self.packet, schema)
        altered = dict(self.packet, invented_field=True)
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate(altered, schema)
        altered = json.loads(json.dumps(self.packet))
        altered['quests'][0]['quest_completeness'] = 'COMPLETE'
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate(altered, schema)

if __name__ == '__main__':
    unittest.main()
