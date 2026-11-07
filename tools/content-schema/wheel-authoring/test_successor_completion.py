import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import gem_revisions
import wheel_authoring as wheel


class CompatibleRowTests(unittest.TestCase):
    def setUp(self):
        self.reference = wheel.read(wheel.ROOT / 'samples/wheel-candidate.json')

    def qualify(self, previous, kind):
        raw = json.dumps(previous, allow_nan=False).encode()
        candidate = copy.deepcopy(self.reference)
        candidate.update(revision='r2', release={'kind': 'wheel_reset',
            'predecessor': previous['revision'],
            'predecessor_sha256': hashlib.sha256(raw).hexdigest()})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'samples/gem-revisions/test.json'
            path.parent.mkdir(parents=True)
            record = {'schema': 'OTERYN_WHEEL_GEM_REVISION_REFERENCE/v1', 'kind': kind,
                'source_revision': previous['revision'], 'destination_revision': 'r2',
                'source_gem_sha256': gem_revisions.contract_digest(gem_revisions.gem_contract(previous)),
                'destination_gem_sha256': gem_revisions.contract_digest(gem_revisions.gem_contract(candidate)),
                'runtime_admitted': False, 'runtime_validation': 'PENDING_GEM_R_ADMISSION'}
            if kind == 'staged_migration':
                record['native_migration_reference'] = 'GEM-R pending migration plan'
            path.write_text(json.dumps(record))
            candidate['release']['gem_revision'] = {'kind': kind,
                'reference': 'samples/gem-revisions/test.json',
                'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
            with patch.object(gem_revisions, 'ROOT', root):
                wheel.validate(candidate, previous, raw)

    def test_compatible_declarations_refuse_changed_activation_and_stored_row_semantics(self):
        for mutation in ('activation', 'non_current_effects', 'grade_iv_points', 'future_field'):
            previous = copy.deepcopy(self.reference)
            atelier = previous['gems']['atelier']
            if mutation == 'activation':
                atelier['resonance_activation_order'] = ['supreme', 'basic_1', 'basic_2']
            elif mutation == 'non_current_effects':
                atelier['operation_policy']['non_current_gem_and_grade_effects_zero'] = False
            elif mutation == 'grade_iv_points':
                atelier['grade_iv_promotion_points_per_mod_type'] += 1
            else:
                atelier['future_stored_row_interpretation'] = 'different'
            with self.subTest(mutation=mutation), self.assertRaisesRegex(ValueError, 'GEM_COMPATIBLE_ROW_CHANGED'):
                self.qualify(previous, 'declared_compatible')

    def test_compatible_declarations_allow_economic_changes(self):
        previous = copy.deepcopy(self.reference)
        atelier = previous['gems']['atelier']
        previous['gems']['grade_costs'][0]['basic']['gold'] += 1
        atelier['fees']['reveal']['lesser'] += 1
        atelier['fragment_yields']['lesser']['revealed'][0] += 1
        atelier['operation_policy']['vendor_reference_prices']['greater_fragment'] += 1
        self.qualify(previous, 'declared_compatible')

    def test_changed_activation_requires_staged_authoring_without_runtime_admission(self):
        previous = copy.deepcopy(self.reference)
        previous['gems']['atelier']['resonance_activation_order'] = ['supreme', 'basic_1', 'basic_2']
        self.qualify(previous, 'staged_migration')
