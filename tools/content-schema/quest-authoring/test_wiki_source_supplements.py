import copy
import json
import tempfile
import unittest
from pathlib import Path

from wiki_source_supplements import digest, generate, read, reconstruct

ROOT = Path(__file__).parent
SAMPLES = ROOT / 'samples/wiki-source-specs'


class OfflineTests(unittest.TestCase):
    def setUp(self):
        self.baseline = read(SAMPLES / 'source-specs-105.json')
        self.packages = [read(SAMPLES / f'supplement-{n}.json') for n in range(1, 6)]
        self.receipt = read(SAMPLES / 'acquisition-receipt.json')
        self.selection = read(SAMPLES / 'selection-receipt.json')

    def reconstruct(self):
        return reconstruct(self.baseline, self.packages, self.receipt, self.selection)

    def test_full_portable_regeneration_needs_no_raw_body_directory(self):
        result = generate(SAMPLES, ROOT / 'wiki_source_specs.schema.json')
        self.assertEqual(result, read(SAMPLES / 'source-specs-105.enriched.json'))
        self.assertFalse((SAMPLES / 'raw-captures').exists())
        self.assertEqual(result['supplement_summary']['titles'], len(self.baseline['entries']))

    def test_offline_verification_does_not_assert_public_body_or_span_recheck(self):
        result = self.reconstruct()
        self.assertFalse(result['offline_verification']['raw_body_rechecked'])
        self.assertFalse(result['offline_verification']['line_spans_rechecked'])
        sources = [source for entry in result['entries'] for source in entry['source_fact_supplements']]
        self.assertTrue(all(source['source_revision_verified'] is False for source in sources))
        self.assertTrue(all(source['historical_acquisition_verified'] is True for source in sources))

    def test_changed_authored_quantity_is_not_a_verified_source_fact(self):
        self.packages[0]['entries'][0]['sources'][0]['semantic_enrichment_delta']['reward_declared_none'] = True
        with self.assertRaisesRegex(ValueError, 'recorded acquisition digest'): self.reconstruct()

    def test_forged_line_span_rejected_against_recorded_fact_digest(self):
        self.packages[0]['entries'][0]['sources'][0]['source_spans'][0]['span_sha256'] = 'f' * 64
        with self.assertRaisesRegex(ValueError, 'recorded acquisition digest'): self.reconstruct()

    def test_changed_revision_rejected(self):
        self.packages[0]['entries'][0]['sources'][0]['revid'] += 1
        with self.assertRaises(ValueError): self.reconstruct()

    def test_missing_title_rejected(self):
        self.packages[0]['entries'].pop()
        with self.assertRaisesRegex(ValueError, 'Incomplete'): self.reconstruct()

    def test_duplicate_source_rejected(self):
        entry = self.packages[0]['entries'][0]
        entry['sources'].append(copy.deepcopy(entry['sources'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate'): self.reconstruct()

    def test_unknown_provenance_qualification_rejected(self):
        self.receipt['qualification'] = 'BODY_PROOF_UNKNOWN'
        with self.assertRaisesRegex(ValueError, 'qualification'): self.reconstruct()

    def test_baseline_item_identity_change_rejected_by_acquisition_anchor(self):
        self.baseline['entries'][0]['source_item_identity_bindings'][0]['item_id'] += 1
        with self.assertRaisesRegex(ValueError, 'acquisition anchor'): self.reconstruct()

    def test_selection_change_rejected_by_acquisition_anchor(self):
        self.selection['revision'] = 'f' * 40
        with self.assertRaisesRegex(ValueError, 'acquisition anchor'): self.reconstruct()

    def test_native_ready_not_inferred_from_complete_inventory(self):
        result = self.reconstruct()
        self.assertTrue(all(entry['definition_complete'] is False and entry['runtime_readiness'] == 'UNKNOWN'
                            for entry in result['entries']))
        self.assertFalse(result['source_definition_complete'])

    def test_original_scalar_values_item_refs_and_unparsed_holds_survive(self):
        result = self.reconstruct()
        for old, entry in zip(self.baseline['entries'], result['entries']):
            self.assertEqual(entry['baseline_entry_sha256'], digest(old))
            self.assertEqual(entry.get('source_item_identity_bindings'), old.get('source_item_identity_bindings'))
            self.assertTrue(all(hold in entry['unresolved'] for hold in old['unresolved']))
            for original_source, augmented in zip(old['source_specification'], entry['source_specification']):
                self.assertEqual(original_source['semantic_enrichment']['level'], augmented['semantic_enrichment']['level'])

    def test_schema_rejects_attempt_to_claim_fresh_recheck(self):
        result = generate(SAMPLES, ROOT / 'wiki_source_specs.schema.json')
        result['offline_verification']['raw_body_rechecked'] = True
        from jsonschema import Draft202012Validator
        self.assertTrue(list(Draft202012Validator(read(ROOT / 'wiki_source_specs.schema.json')).iter_errors(result)))

    def test_snapshot_inventory_does_not_depend_on_current_catalogue_bindings(self):
        self.assertEqual(set(self.selection['selected_titles']), {entry['wiki_title'] for entry in self.reconstruct()['entries']})
        self.assertTrue(self.selection['later_partial_bindings_do_not_remove_snapshot_specifications'])


if __name__ == '__main__':
    unittest.main()
