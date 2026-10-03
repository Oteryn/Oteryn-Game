import copy
import json
from pathlib import Path
import tempfile
import unittest

import classify_monster_population as classification


class ClassificationIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        original = classification.population.MONSTERS / 'samples/canary-47dfd51f/rat'
        self.bundle = self.root / 'bundles/rat'
        classification.population.shutil.copytree(original, self.bundle)
        self.index = self.root / 'index.json'
        self.annotations = self.root / 'annotations.json'

    def make(self, mitigation=None):
        path = self.bundle / 'monster.json'
        monster = classification.read(path)
        monster['creature']['stats'].pop('mitigation_percent', None)
        if mitigation is not None:
            monster['creature']['stats']['mitigation_percent'] = mitigation
        path.write_text(json.dumps(monster) + '\n')
        digest = classification.population.admission.bundle_digest(self.bundle)
        self.index.write_text(json.dumps({'monsters': [{'monster': 'rat', 'sha256': digest}]}) + '\n')
        self.annotations.write_text(json.dumps({'population_index_sha256': classification.sha(self.index),
            'annotations': {'rat': {'roles': [{'role': 'unknown', 'confidence': 'unknown', 'evidence': []}],
                                  'contexts': [], 'variant_relations': [], 'encounter_memberships': []}}}) + '\n')
        return classification.build(self.index, self.root / 'bundles', self.annotations)

    def check(self, catalogue):
        classification.validate(catalogue, self.index, self.root / 'bundles')

    def test_zero_is_present_and_cannot_be_hidden_as_unknown(self):
        catalogue = self.make({'numerator': 0, 'denominator': 1})
        self.assertEqual(catalogue['entries'][0]['mitigation']['status'], 'present')
        changed = copy.deepcopy(catalogue)
        changed['entries'][0]['mitigation']['status'] = 'unknown'
        changed['entries'][0]['mitigation'].pop('value')
        changed['counts'] = classification.counts(changed['entries'])
        with self.assertRaisesRegex(ValueError, 'hides an existing'):
            self.check(changed)

    def test_boss_role_does_not_establish_mitigation_non_applicability(self):
        catalogue = self.make()
        record = catalogue['entries'][0]
        record['roles'] = [{'role': 'boss', 'confidence': 'inferred',
                            'evidence': [{'kind': 'source_folder', 'source': 'monster/bosses/rat.lua'}]}]
        record['mitigation']['status'] = 'not_applicable'
        catalogue['counts'] = classification.counts(catalogue['entries'])
        with self.assertRaisesRegex(ValueError, 'positive source evidence'):
            self.check(catalogue)

    def test_source_folder_cannot_confirm_a_role(self):
        catalogue = self.make()
        catalogue['entries'][0]['roles'] = [{'role': 'boss', 'confidence': 'confirmed',
            'evidence': [{'kind': 'source_folder', 'source': 'monster/bosses/rat.lua'}]}]
        catalogue['counts'] = classification.counts(catalogue['entries'])
        with self.assertRaisesRegex(ValueError, 'folder alone'):
            self.check(catalogue)

    def test_catalogue_must_cover_exact_current_bundles(self):
        catalogue = self.make()
        broken = copy.deepcopy(catalogue)
        broken['entries'][0]['bundle_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'bundle SHA is stale'):
            self.check(broken)
        broken = copy.deepcopy(catalogue)
        broken['entries'].append(copy.deepcopy(broken['entries'][0]))
        broken['counts'] = classification.counts(broken['entries'])
        with self.assertRaisesRegex(ValueError, 'incomplete or duplicated'):
            self.check(broken)

    def test_confirmed_bundle_claim_must_match_its_positive_evidence(self):
        catalogue = self.make()
        catalogue['entries'][0]['roles'] = [{'role': 'boss', 'confidence': 'confirmed',
            'evidence': [{'kind': 'bundle_field', 'source': 'bundles/rat/monster.json',
                          'pointer': '/creature/system_eligibility/reward_boss', 'value': True}]}]
        catalogue['counts'] = classification.counts(catalogue['entries'])
        with self.assertRaisesRegex(ValueError, 'evidence value differs'):
            self.check(catalogue)


if __name__ == '__main__':
    unittest.main()
