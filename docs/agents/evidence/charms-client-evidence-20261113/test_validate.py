"""Negative controls for partial UI observations and source/proof bindings."""
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('charms_evidence_validate', HERE / 'validate.py')
V = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V)


class BoundaryTests(unittest.TestCase):
    def setUp(self):
        self.data = V.load(HERE)

    def row(self, name):
        return next(row for row in self.data['observations-25.json']['rows'] if row['name'] == name)

    def test_retained_packet(self):
        V.validate(self.data)

    def test_missing_initial_vampiric_capture_is_incomplete(self):
        rows = self.data['observations-25.json']['rows']
        rows.remove(self.row('Vampiric Embrace'))
        with self.assertRaisesRegex(ValueError, 'CARD_POPULATION'):
            V.validate(self.data)

    def test_duplicate_keepalive_capture_is_not_a_26th_charm(self):
        self.data['observations-25.json']['rows'].append(self.row('Scavenge'))
        with self.assertRaisesRegex(ValueError, 'CARD_POPULATION'):
            V.validate(self.data)

    def test_locked_preview_is_not_an_owned_stage(self):
        self.row('Cleanse')['current_owned_stage_candidate'] = 1
        with self.assertRaisesRegex(ValueError, 'LOCKED_PREVIEW_NOT_OWNED'):
            V.validate(self.data)

    def test_one_visible_stage_does_not_qualify_all_three(self):
        self.data['provenance.json']['all_three_stages_ui_verified'] = True
        with self.assertRaisesRegex(ValueError, 'EVIDENCE_BOUNDARY'):
            V.validate(self.data)

    def test_reset_gold_cannot_replace_unlock_cost(self):
        self.row('Cleanse')['visible_unlock_or_upgrade_cost'] = 5495250
        with self.assertRaisesRegex(ValueError, 'VISIBLE_VALUE_OR_COST'):
            V.validate(self.data)

    def test_minor_echoes_are_not_major_points(self):
        self.row('Cleanse')['currency'] = 'charm_points'
        with self.assertRaisesRegex(ValueError, 'CURRENCY_BOUNDARY'):
            V.validate(self.data)

    def test_description_does_not_prove_runtime_defect(self):
        self.data['findings.json']['findings'][0]['runtime_defect_asserted'] = True
        with self.assertRaisesRegex(ValueError, 'FINDING_BOUNDARY'):
            V.validate(self.data)

    def test_swapped_source_image_hash_is_rejected(self):
        self.row('Cleanse')['proof']['sha256'] = self.row('Scavenge')['proof']['sha256']
        with self.assertRaisesRegex(ValueError, 'CARD_PROOF'):
            V.validate(self.data)


if __name__ == '__main__':
    unittest.main()
