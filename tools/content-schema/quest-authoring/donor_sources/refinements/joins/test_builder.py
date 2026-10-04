import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

import builder

ROOT = Path(os.environ.get('QUEST_COMPONENT_REPO_ROOT', str(Path(__file__).resolve().parents[6])))
ASSIGNMENT = Path(os.environ.get('QUEST_COMPONENT_ASSIGNMENT', str(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/components248/assignments/all.json')))


class RegistrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
        import lua_writers
        cls.mask = staticmethod(lua_writers.mask_code)

    def test_unique_registrar(self):
        text = 'local event=CreatureEvent("BrokulThink")\nevent:register()'
        self.assertEqual(builder.event_registrations(text, self.mask)[0][0], 'BrokulThink')

    def test_comments_and_string_body_do_not_register(self):
        text = '-- local ev=CreatureEvent("False");ev:register()\nlocal msg=\'local ev=CreatureEvent("False");ev:register()\''
        self.assertEqual(builder.event_registrations(text, self.mask), [])

    def test_double_register_is_unresolved(self):
        text = 'local ev=CreatureEvent("Twice"); ev:register();ev:register()'
        self.assertEqual(builder.event_registrations(text, self.mask), [])

    def test_chained_call_and_quoted_fake_caller(self):
        text = 'Game.createMonster("Pillar",pos):registerEvent("HealthPillar")\nlocal s=\':registerEvent("Fake")\'\n--p:registerEvent("Comment")'
        self.assertEqual([m.group(2) for m in builder.literal_event_calls(text, self.mask)], ['HealthPillar'])

    def test_unregistered_constructor_is_unresolved(self):
        self.assertEqual(builder.event_registrations('local ev=CreatureEvent("Never")', self.mask), [])


class PacketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import tarfile
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        directory = Path(cls.temp.name)
        with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as stream:
            stream.extractall(directory, filter='data')
        cls.manifest = directory / 'corpus-manifest.json'
        cls.packet = builder.build(ROOT, ASSIGNMENT, cls.manifest)
        cls.corpus = builder.Corpus(cls.manifest)

    def test_full_assignment_and_new_associations(self):
        self.assertEqual(len(self.packet['records']), 248)
        self.assertEqual({r['source_component_id'] for r in self.packet['records']}, {r['source_component_id'] for r in builder.read(ASSIGNMENT)['records']})
        self.assertEqual(self.packet['summary']['joined_components'], 17)
        self.assertEqual(self.packet['summary']['joined_quests'], 7)

    def test_brokul_both_variants(self):
        rows = [r for r in self.packet['records'] if 'actions_brokulLever.lua' in r['source_component_id']]
        self.assertEqual(len(rows), 2)
        self.assertTrue(all(r['quest_keys'] == ['oteryn:quest.the_secret_library_quest'] for r in rows))
        self.assertTrue(all(len(p['source_witnesses']) == 2 for r in rows for p in r['proofs']))

    def test_pin_and_pack_fences_and_actual_source_spans(self):
        for row in self.packet['records']:
            for proof in row['proofs']:
                for witness in proof['source_witnesses']:
                    key = (witness['source'], witness['revision'], witness['path'])
                    file = self.corpus.files[key]
                    text = self.corpus.text(file)
                    self.assertEqual(witness['source'], row['provenance']['source'])
                    self.assertEqual(witness['revision'], row['provenance']['revision'])
                    self.assertEqual(witness['path'].split('/')[0], row['provenance']['path'].split('/')[0])
                    self.assertLess(witness['char_start'], witness['char_end'])
                    self.assertEqual(builder.sha(text[witness['char_start']:witness['char_end']].encode()), witness['token_sha256'])
                    self.assertEqual(builder.sha(text.splitlines()[witness['line'] - 1].encode()), witness['line_sha256'])

    def test_never_promotes_runtime_or_domain(self):
        self.assertFalse(self.packet['native_admission'])
        self.assertEqual(self.packet['quest_semantic_completeness'], 'NOT_ESTABLISHED')
        for row in self.packet['records']:
            self.assertFalse(row['native_admission'])
            self.assertIn('COMPONENT_SEMANTIC_COVERAGE_NOT_ESTABLISHED', row['remaining_holds'])
            for proof in row['proofs']:
                self.assertEqual(proof['dispatch_binding'], 'NOT_PROVEN')
                self.assertEqual(proof['storage_domain_equivalence'], 'NOT_ASSESSED')

    def test_schema_rejects_extra_and_native_promotion(self):
        import jsonschema
        schema = builder.read(Path(__file__).with_name('schema.json'))
        jsonschema.Draft202012Validator(schema).validate(self.packet)
        for altered in [dict(self.packet, added_field=1), dict(self.packet, native_admission=True)]:
            with self.assertRaises(jsonschema.ValidationError):
                jsonschema.Draft202012Validator(schema).validate(altered)

    def test_duplicate_assignment_rejected(self):
        assignment = builder.read(ASSIGNMENT)
        assignment['records'].append(assignment['records'][0])
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / 'assignment.json'
            path.write_text(json.dumps(assignment))
            with self.assertRaisesRegex(ValueError, 'duplicate component'):
                builder.build(ROOT, path, self.manifest)

    def test_bad_component_digest_rejected(self):
        assignment = builder.read(ASSIGNMENT)
        assignment['records'][0]['provenance']['sha256'] = '0' * 64
        with tempfile.TemporaryDirectory() as scratch:
            path = Path(scratch) / 'assignment.json'
            path.write_text(json.dumps(assignment))
            with self.assertRaisesRegex(ValueError, 'component identity mismatch'):
                builder.build(ROOT, path, self.manifest)

    def test_new_supplement_refs_do_not_affect_source_digest(self):
        with tempfile.TemporaryDirectory(prefix='join-cycle-') as temporary:
            mirror = Path(temporary)
            (mirror / 'tools').symlink_to(ROOT / 'tools', target_is_directory=True)
            destination = mirror / 'content/quests/definitions'
            destination.mkdir(parents=True)
            for source in (ROOT / 'content/quests/definitions').glob('quests-*.json'):
                data = builder.read(source)
                for record in data['records']:
                    record['definition'].setdefault('oteryn_recipe', {})['donor_source_data'] = {'new_packet': 'new-supplement.json'}
                (destination / source.name).write_text(json.dumps(data))
            replay = builder.build(mirror, ASSIGNMENT, self.manifest)
            self.assertEqual(replay['input_sha256'], self.packet['input_sha256'])
            self.assertEqual(replay['records'], self.packet['records'])



if __name__ == '__main__':
    unittest.main()
