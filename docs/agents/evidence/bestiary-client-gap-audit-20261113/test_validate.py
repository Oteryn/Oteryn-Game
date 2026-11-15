"""Guard the evidence boundary, not the game implementation."""
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('bestiary_evidence_validate', HERE / 'validate.py')
VALIDATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATE)


class EvidenceBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.data = VALIDATE.load(HERE)

    def test_retained_packet(self):
        VALIDATE.validate(self.data)

    def test_missing_client_row_is_not_complete(self):
        self.data['client1533-creatures.json']['records'].pop()
        with self.assertRaisesRegex(ValueError, 'CLIENT_POPULATION'):
            VALIDATE.validate(self.data)

    def test_duplicate_id_is_not_a_second_creature(self):
        rows = self.data['client1533-creatures.json']['records']
        rows[-1]['source_id'] = rows[0]['source_id']
        with self.assertRaisesRegex(ValueError, 'CLIENT_POPULATION'):
            VALIDATE.validate(self.data)

    def test_candidate_cannot_be_promoted_to_identity_authority(self):
        self.data['provenance.json']['server_identity_mapping_verified'] = True
        with self.assertRaisesRegex(ValueError, 'AUTHORITY_BOUNDARY'):
            VALIDATE.validate(self.data)

    def test_partial_ui_sample_cannot_be_called_all_cards_audited(self):
        self.data['provenance.json']['all_845_cards_ui_audited'] = True
        with self.assertRaisesRegex(ValueError, 'AUTHORITY_BOUNDARY'):
            VALIDATE.validate(self.data)

    def test_new_record_cannot_also_be_an_unchanged_old_record(self):
        self.data['staticdata-delta.json']['added'][0]['source_id'] = 21
        with self.assertRaisesRegex(ValueError, 'STATICDATA_DELTA'):
            VALIDATE.validate(self.data)

    def test_duplicate_class_does_not_prove_21_classes(self):
        rows = self.data['ui-class-counts.json']
        rows[-1]['class'] = rows[0]['class']
        with self.assertRaisesRegex(ValueError, 'UI_CLASSES'):
            VALIDATE.validate(self.data)

    def test_missing_proof_receipt_is_not_ui_evidence(self):
        self.data['private-image-receipts.json']['images'] = []
        with self.assertRaisesRegex(ValueError, 'PRIVATE_PROOF'):
            VALIDATE.validate(self.data)

    def test_animated_grid_change_does_not_qualify_a_search(self):
        row = next(row for row in self.data['qualified-reader-checkpoint.json']['searches']
                   if row['query'] == 'nightslayer')
        row['visually_confirmed_result'] = True
        with self.assertRaisesRegex(ValueError, 'UNQUALIFIED_SEARCH_RESULT'):
            VALIDATE.validate(self.data)

    def test_blank_loot_panel_cannot_be_promoted_to_no_loot(self):
        self.data['qualified-reader-checkpoint.json']['wafer_paper_butterfly']['loot'] = 'NO_LOOT'
        with self.assertRaisesRegex(ValueError, 'WAFER_EVIDENCE_BOUNDARY'):
            VALIDATE.validate(self.data)


if __name__ == '__main__':
    unittest.main()
