"""Source-only pattern packets cannot silently choose parameters or admission."""
import json
import copy
import hashlib
import os
import tempfile
import unittest
from pathlib import Path

import prepare_custom_patterns as prep
from jsonschema import Draft202012Validator


class SourceFacts(unittest.TestCase):
    @unittest.skipUnless(os.environ.get('OTERYN_CANARY'), 'set pinned source path')
    def test_prepared_population_has_95_pinned_packets_and_no_executable_flags(self):
        sample = json.loads((prep.ROOT / 'samples/custom-pattern-preparation-canary-47dfd51f.json').read_text())
        validator = Draft202012Validator(json.loads(prep.SCHEMA.read_text()))
        self.assertEqual([], list(validator.iter_errors(sample)))
        self.assertEqual(95, len(sample['spells']))
        self.assertEqual(40, sample['counts']['current_open_registered_spells'])
        for row in sample['spells']:
            content, blob = prep.pinned(Path(os.environ['OTERYN_CANARY']), row['source']['file'])
            self.assertEqual(blob, row['source']['blob_sha1'])
            self.assertEqual(hashlib.sha256(content).hexdigest(), row['source']['sha256'])
            self.assertTrue(all(0 < f['source_line'] <= len(content.splitlines()) for f in row['source_facts']))
        for field in ('admission_authorized', 'runtime_qualified'):
            mutated = dict(sample, **{field: True})
            self.assertTrue(list(validator.iter_errors(mutated)))
            mutated = dict(sample, spells=[dict(sample['spells'][0], **{field: True})] + sample['spells'][1:])
            self.assertTrue(list(validator.iter_errors(mutated)))

    def test_only_lexical_literals_are_known_and_arithmetic_is_unknown(self):
        facts = prep.source_facts('local count = 3\nlocal delay = Quest.seconds * 1000\n'
                                  'condition:setParameter(CONDITION_PARAM_TICKS, delay)\n')
        count = next(f for f in facts if f.get('name') == 'count')
        delay = next(f for f in facts if f.get('name') == 'delay')
        self.assertEqual('SOURCE_LITERAL', count['value_state'])
        self.assertEqual('3', count['source_expression'])
        self.assertEqual('UNKNOWN_NOT_EVALUATED', delay['value_state'])
        self.assertEqual('Quest.seconds * 1000', delay['source_expression'])
        self.assertFalse(any('value' in f for f in facts))

    def test_crystal_or_wiki_manifests_do_not_bind_canary_scripts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, source in [('canary', {'repository': 'opentibiabr/canary', 'revision': prep.cb.REVISION}),
                                 ('crystal', {'repository': 'zimbadev/crystalserver'}), ('wiki', {'kind': 'mediawiki'})]:
                path = root / name / 'manifest.json';path.parent.mkdir()
                path.write_text(json.dumps({'sources': [source], 'entries': [{'source_index': 0, 'source_field': 'attacks[1]',
                    'source_file': 'source.lua', 'source_line': 2, 'status': 'unresolved_semantics',
                    'resolution': 'registered instant spell "example" (example.lua)'}]}))
            refs, hashes = prep.dispositions([root])
            self.assertEqual(['example'], list(refs))
            self.assertEqual('canary', refs['example'][0]['monster'])
            self.assertEqual(1, len(hashes))

    def test_missing_or_wrong_canary_epoch_and_cross_source_rows_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'monster' / 'manifest.json'
            path.parent.mkdir()
            source = {'repository': prep.cb.REPOSITORY, 'revision': prep.cb.REVISION}
            row = {'source_index': 0, 'source_field': 'attacks[1]', 'source_file': 'source.lua',
                   'source_line': 2, 'status': 'unresolved_semantics',
                   'resolution': 'registered instant spell "example" (example.lua)'}
            for revision in (None, '0' * 40):
                with self.subTest(revision=revision):
                    changed = dict(source)
                    if revision is None:
                        changed.pop('revision')
                    else:
                        changed['revision'] = revision
                    path.write_text(json.dumps({'sources': [changed], 'entries': [row]}))
                    with self.assertRaisesRegex(ValueError, 'revision'):
                        prep.dispositions([root])
            for index in (None, 1):
                with self.subTest(source_index=index):
                    changed = dict(row)
                    if index is None:
                        changed.pop('source_index')
                    else:
                        changed['source_index'] = index
                    path.write_text(json.dumps({'sources': [source, {'kind': 'mediawiki'}], 'entries': [changed]}))
                    with self.assertRaisesRegex(ValueError, 'not bound'):
                        prep.dispositions([root])

    def test_current_spell_cannot_rebind_a_historical_name_to_another_script(self):
        record = json.loads(prep.GROUPING.read_text())['spells'][0]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'monster' / 'manifest.json'
            path.parent.mkdir()
            path.write_text(json.dumps({'sources': [{'repository': prep.cb.REPOSITORY, 'revision': prep.cb.REVISION}],
                'entries': [{'source_index': 0, 'source_field': 'attacks[1]', 'source_file': 'source.lua',
                    'source_line': 2, 'status': 'unresolved_semantics',
                    'resolution': 'registered instant spell "' + record['spell'] + '" (different-source.lua)'}]}))
            with self.assertRaisesRegex(ValueError, 'binding mismatch'):
                prep.prepare(Path('/unused-source'), [root])

    @unittest.skipUnless(os.environ.get('OTERYN_CANARY'), 'set pinned source path')
    def test_all_93_historical_scripts_and_19_groups_have_exact_source_evidence(self):
        report = prep.prepare(Path(os.environ['OTERYN_CANARY']), [])
        self.assertEqual(93, len(report['spells']))
        self.assertEqual(19, len(report['patterns']))
        self.assertTrue(all(r['source']['blob_sha1'] and r['source']['sha256'] for r in report['spells']))
        self.assertTrue(all(not r['admission_authorized'] and not r['runtime_qualified'] for r in report['spells']))
        self.assertTrue(all(not r['parameter_values_from_model_assisted_grouping_adopted'] for r in report['spells']))
        original = json.loads(prep.GROUPING.read_text())
        self.assertEqual({r['spell'] for r in original['spells']}, {r['spell'] for r in report['spells']})
        self.assertFalse(any('parameters' in r for r in report['spells']))
        validator = Draft202012Validator(json.loads(prep.SCHEMA.read_text()))
        self.assertEqual([], list(validator.iter_errors(report)))
        for path in ('admission_authorized', 'runtime_qualified'):
            mutated = copy.deepcopy(report)
            mutated[path] = True
            self.assertTrue(list(validator.iter_errors(mutated)))
            mutated = copy.deepcopy(report)
            mutated['spells'][0][path] = True
            self.assertTrue(list(validator.iter_errors(mutated)))
        mutated = copy.deepcopy(report)
        mutated['spells'][0]['source_facts'] = [{'kind': 'assignment', 'name': 'delay', 'source_line': 1,
            'source_expression': 'Quest.seconds * 1000', 'value_state': 'SOURCE_LITERAL'}]
        self.assertTrue(list(validator.iter_errors(mutated)))


if __name__ == '__main__':
    unittest.main()
