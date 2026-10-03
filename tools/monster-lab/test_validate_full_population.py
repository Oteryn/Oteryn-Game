import copy
import unittest

from lab import LabError
from validate_full_population import validate_stage


class PromotionPacketTests(unittest.TestCase):
    def fixture(self):
        actor = {'family': 'Creature', 'key': 'oteryn:creature.rat', 'revision': 'definition-r1'}
        stage = {'source': {'census_index_sha256': 'index'},
                 'records': [{'identity': actor, 'kind': 'Creature'}],
                 'declarations': [], 'authoring_profiles': [{'target': actor,
                    'data': {'kind': 'Creature', 'profile': {'health': 20, 'speed': 10,
                        'armor': 2, 'experience': 5, 'details': {'defense': 2},
                        'mitigation': {'numerator': 0, 'denominator': 1}}}}],
                 'counts': {'records': 1, 'profiles': 1, 'creatures': 1, 'encounters': 0},
                 'completion_flags': {actor['key']: ['OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE']}}
        index = {'monsters': [{'monster': 'rat', 'completion_flags': ['OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE']}]}
        items = {'schema': 'OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1', 'records': []}
        return stage, index, items

    def validate(self, stage, index, items):
        return validate_stage(stage, index, 'index', items)

    def test_known_zero_mitigation_is_valid(self):
        self.assertEqual(self.validate(*self.fixture())['creatures'], 1)

    def test_stale_index_hash_cannot_qualify(self):
        stage, index, items = self.fixture()
        stage['source']['census_index_sha256'] = 'old'
        with self.assertRaisesRegex(LabError, 'exact population index'):
            self.validate(stage, index, items)

    def test_dropped_acceptance_flags_cannot_qualify(self):
        stage, index, items = self.fixture()
        stage['completion_flags'] = {}
        with self.assertRaisesRegex(LabError, 'quality flags'):
            self.validate(stage, index, items)

    def test_unregistered_loot_reference_cannot_qualify(self):
        stage, index, items = self.fixture()
        stage['records'][0]['loot'] = {'family': 'Item', 'key': 'oteryn:item.missing', 'revision': 'definition-r1'}
        with self.assertRaisesRegex(LabError, 'Unresolved native references'):
            self.validate(stage, index, items)

    def test_verified_external_item_reference_is_resolved(self):
        stage, index, items = self.fixture()
        stage['records'][0]['loot'] = {'family': 'Item', 'key': 'oteryn:item.verified', 'revision': 'definition-r1'}
        items['records'] = [{'native_key': 'oteryn:item.verified'}]
        self.validate(stage, index, items)

    def test_duplicate_profile_cannot_hide_in_counts(self):
        stage, index, items = self.fixture()
        stage['authoring_profiles'].append(copy.deepcopy(stage['authoring_profiles'][0]))
        stage['counts']['profiles'] = 2
        with self.assertRaisesRegex(LabError, 'duplicate or unresolved'):
            self.validate(stage, index, items)

    def test_mitigation_unknown_is_not_silently_zero(self):
        stage, index, items = self.fixture()
        del stage['authoring_profiles'][0]['data']['profile']['mitigation']
        with self.assertRaisesRegex(LabError, 'mitigation'):
            self.validate(stage, index, items)

    def test_false_counts_cannot_qualify(self):
        stage, index, items = self.fixture()
        stage['counts']['creatures'] = 100
        with self.assertRaisesRegex(LabError, 'counts disagree'):
            self.validate(stage, index, items)

    def test_encounter_profiles_resolve_against_declarations(self):
        stage, index, items = self.fixture()
        declared = {'key': 'oteryn:encounter.rat_room', 'revision': 'definition-r1'}
        stage['declarations'] = [{'kind': 'Encounter', 'identity': declared}]
        stage['authoring_profiles'].append({'target': dict(declared, family='Encounter'), 'data': {'kind': 'Encounter'}})
        stage['counts'].update(profiles=2, encounters=1)
        self.assertEqual(self.validate(stage, index, items)['encounters'], 1)


if __name__ == '__main__':
    unittest.main()
