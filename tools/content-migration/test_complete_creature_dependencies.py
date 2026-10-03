import copy
import json
from pathlib import Path
import tempfile
import unittest

import complete_creature_dependencies as batch


class CompletionMergeTests(unittest.TestCase):
    def setUp(self):
        self.source = batch.MONSTERS / 'samples/canary-47dfd51f/rat'
        self.row = {'monster': 'rat', 'file': 'mammals/rat',
                    'sha256': batch.admission.bundle_digest(self.source)}
        self.index = {'source': {'revision': 'pinned'}, 'monsters': [self.row], 'bundles': 1}

    def test_baseline_is_immutable_and_shared_not_duplicated(self):
        before = copy.deepcopy(self.index)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            result = batch.merge_population(self.index, self.source.parent, output, [])
            self.assertEqual(self.index, before)
            self.assertEqual(result, before)
            self.assertFalse((output / 'bundles/rat').is_symlink())
            for name in batch.FILES:
                self.assertEqual((output / 'bundles/rat' / name).read_bytes(), (self.source / name).read_bytes())
            self.assertEqual(batch.admission.bundle_digest(output / 'bundles/rat'), self.row['sha256'])

    def test_supplement_cannot_hide_a_forged_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'digest mismatch'):
                batch.merge_population(self.index, self.source.parent, Path(directory),
                                       [(self.source, {**self.row, 'sha256': '0' * 64})])

    def test_duplicate_writers_cannot_overwrite_a_valid_bundle(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'duplicate supplemental'):
                batch.merge_population(self.index, self.source.parent, Path(directory),
                                       [(self.source, self.row), (self.source, self.row)])

    def test_replacement_is_a_valid_copy_with_same_identity_and_retained_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            result = batch.merge_population(self.index, self.source.parent, Path(directory),
                                            [(self.source, self.row)])
            self.assertEqual(result, self.index)
            target = Path(directory) / 'bundles/rat'
            self.assertFalse(target.is_symlink())
            for name in batch.FILES:
                self.assertEqual((target / name).read_bytes(), (self.source / name).read_bytes())

    def test_schema_invalid_bundle_is_rejected_even_with_matching_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'invalid'; source.mkdir()
            for name in batch.FILES:
                (source / name).write_text('{}\n')
            with self.assertRaisesRegex(ValueError, 'invalid source completion'):
                batch.install_bundle(source, root, {**self.row, 'sha256': batch.admission.bundle_digest(source)})

    def test_duplicate_baseline_or_path_escape_is_rejected(self):
        for rows in ([self.row, self.row], [{**self.row, 'monster': '../rat'}]):
            with self.subTest(rows=rows), tempfile.TemporaryDirectory() as directory:
                with self.assertRaisesRegex(ValueError, 'duplicate/invalid'):
                    batch.merge_population({'monsters': rows}, self.source.parent, Path(directory), [])


class SeacrestCorrectionTests(unittest.TestCase):
    def documents(self):
        key = 'canary:ability/seacrest_serpent/defense-2'
        return [{'creature': {'identity': {'key': 'canary:creature/seacrest_serpent'},
                              'stats': {'max_health': 20000}},
                 'behavior': {'attacks': [{'ability': 'retained'}],
                              'defenses': [{'ability': {'key': 'heal'}}, {'ability': {'key': key}}]},
                 'loot': {'entries': [{'probability_percent': 2}]}},
                {'abilities': [{'identity': {'key': key}, 'kind': 'melee'}]}, {},
                {'entries': [{'destination': '/monster/behavior/defenses/1', 'status': 'mapped'}]}]

    def test_only_known_action_is_omitted_and_original_provenance_is_retained(self):
        documents = self.documents()
        before = copy.deepcopy(documents)
        output, receipt = batch.correct_seacrest_defense(documents)
        self.assertEqual(documents, before)
        self.assertEqual(output[0]['creature'], before[0]['creature'])
        self.assertEqual(output[0]['loot'], before[0]['loot'])
        self.assertEqual(output[0]['behavior']['attacks'], before[0]['behavior']['attacks'])
        self.assertEqual(output[0]['behavior']['defenses'], before[0]['behavior']['defenses'][:1])
        self.assertEqual(output[1:3], before[1:3])
        self.assertEqual(receipt['original_manifest_entry'], before[3]['entries'][0])
        self.assertEqual(output[3]['entries'][0]['status'], 'approved_omission')
        self.assertNotIn('destination', output[3]['entries'][0])

    def test_wrong_owner_changed_action_or_missing_provenance_cannot_be_omitted(self):
        for mutation in ('owner', 'ability', 'manifest'):
            with self.subTest(mutation=mutation):
                documents = self.documents()
                if mutation == 'owner':
                    documents[0]['creature']['identity']['key'] = 'canary:creature/dragon'
                elif mutation == 'ability':
                    documents[1]['abilities'][0]['kind'] = 'spell'
                else:
                    documents[3]['entries'] = []
                with self.assertRaises(ValueError):
                    batch.correct_seacrest_defense(documents)


class EncounterInputTests(unittest.TestCase):
    def test_default_unresolved_samples_still_block(self):
        _, waiting = batch.admission.load_encounters({})
        self.assertEqual(waiting['ferumbras_mortal_shell'], 'unresolved_semantics')
        self.assertEqual(waiting['soul_war_taint_zones'], 'unresolved_semantics')

    def test_alternate_directory_does_not_silently_clear_unresolved_marker(self):
        sample = batch.admission.ENCOUNTERS / 'ferumbras_mortal_shell'
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / sample.name; out.mkdir()
            for name in ('encounter.json', 'manifest.json'):
                (out / name).write_bytes((sample / name).read_bytes())
            encounters, waiting = batch.admission.load_encounters({}, out.parent)
            self.assertEqual(set(encounters), {sample.name})
            self.assertEqual(waiting[sample.name], 'unresolved_semantics')


if __name__ == '__main__':
    unittest.main()
