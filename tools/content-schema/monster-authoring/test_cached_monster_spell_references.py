import copy
import json
import unittest
from pathlib import Path
import jsonschema
from export_cached_monster_spell_references import bind, slot_key


class CachedReferences(unittest.TestCase):
    def setUp(self):
        identity = {'revision': 'a' * 40, 'path': 'monster/frog.lua', 'sha256': 'b' * 64, 'git_blob': 'c' * 40}
        self.profile = {'candidate_id': 'canary/frog', 'source': 'canary', 'name': 'Frog', 'provenance': identity}
        self.slot = {'candidate_id': 'canary/frog', 'source': 'canary', 'monster': 'Frog', 'monster_source': identity, 'source_parameters': {'interval': 2000, 'target': False, 'minDamage': 0, 'effect': None}, 'group': 'attacks', 'source_slot_index': 1, 'conversion_status': 'unresolved_semantics'}
        self.fact = {'monster_name': 'Frog', 'url': 'https://tibiopedia.pl/monsters/Frog', 'read_method': 'normal_http', 'observed_at': '2026-10-02', 'status': 'attack_facts_read', 'http_status': 200, 'final_url': 'https://tibiopedia.pl/monsters/Frog', 'page_sha256': 'd' * 64, 'title': 'Monsters: Frog', 'attacks': [{'label': 'Basic attack (0-20, effect: )', 'damage_ranges': [{'element': 'physical', 'text': '0-20', 'numeric_range': [0, 20]}], 'on_target': None}], 'abilities': []}
        self.comp = {'slot_key': slot_key(self.slot), 'monster_name': 'Frog', 'source': 'canary', 'file': identity['path'], 'source_sha256': identity['sha256'], 'block': 'attacks', 'spell_name': 'melee', 'external_url': self.fact['url'], 'external_page_sha256': self.fact['page_sha256'], 'external_status': self.fact['status'], 'all_fields_verified': False, 'field_comparisons': {k: {'source_value': v, 'status': 'unknown_external_not_published'} for k, v in self.slot['source_parameters'].items()}}
        self.validator = jsonschema.Draft202012Validator(json.loads(Path(__file__).with_name('cached-monster-spell-references.schema.json').read_text()))

    def run_bind(self, profiles=None, slots=None, facts=None, comparisons=None):
        return bind(profiles or [self.profile], slots or [self.slot], facts or [self.fact], comparisons or [self.comp])

    def test_preservation_no_effect_inference(self):
        facts, profiles, slots, targets = self.run_bind()
        self.assertEqual(facts[0]['fact']['attacks'], self.fact['attacks'])
        self.assertEqual(slots[0]['cached_comparison'], self.comp)
        self.assertEqual(slots[0]['unknown_fields'], ['effect', 'interval', 'minDamage', 'target'])
        for row in facts + profiles + slots:
            self.validator.validate(row)
        self.assertEqual(targets[0]['required_uncaptured_sources'], ['tibiawiki.com.br', 'tibia.fandom.com'])

    def test_source_hash_refusal(self):
        altered = copy.deepcopy(self.comp); altered['source_sha256'] = 'e' * 64
        with self.assertRaisesRegex(ValueError, 'source identity'):
            self.run_bind(comparisons=[altered])

    def test_parameters_nil_false_zero_and_mismatch(self):
        changed = copy.deepcopy(self.comp); changed['field_comparisons']['target']['source_value'] = True
        with self.assertRaisesRegex(ValueError, 'parameters'):
            self.run_bind(comparisons=[changed])

    def test_false_to_zero_type_change_refused(self):
        altered = copy.deepcopy(self.comp); altered['field_comparisons']['target']['source_value'] = 0
        with self.assertRaisesRegex(ValueError, 'parameters'):
            self.run_bind(comparisons=[altered])

    def test_profile_revision_drift_refused(self):
        altered = copy.deepcopy(self.profile); altered['provenance']['revision'] = 'e' * 40
        with self.assertRaisesRegex(ValueError, 'slot/profile'):
            self.run_bind(profiles=[altered])

    def test_page_hash_refusal(self):
        altered = copy.deepcopy(self.fact); altered['page_sha256'] = 'e' * 64
        with self.assertRaisesRegex(ValueError, 'page identity'):
            self.run_bind(facts=[altered])

    def test_no_fuzzy_join_or_duplicate_population(self):
        altered = copy.deepcopy(self.fact); altered['monster_name'] = 'frog'
        with self.assertRaisesRegex(ValueError, 'species population'):
            self.run_bind(facts=[altered])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.run_bind(slots=[self.slot, self.slot])

    def test_source_variants_without_slots_conserved(self):
        other = copy.deepcopy(self.profile); other['candidate_id'] = 'crystal/frog'; other['source'] = 'crystal'
        facts, profiles, slots, targets = self.run_bind(profiles=[self.profile, other])
        self.assertEqual(len(profiles), 2); self.assertEqual(profiles[1]['slot_keys'], [])
        self.assertEqual(len(facts), 1); self.assertEqual(len(slots), 1); self.assertEqual(len(targets[0]['candidate_ids']), 2)

    def test_refuse_runtime_or_verified_claim(self):
        row = self.run_bind()[2][0]
        for key in ['runtime_activation', 'source_override', 'all_fields_verified']:
            changed = copy.deepcopy(row); changed[key] = True
            with self.assertRaises(jsonschema.ValidationError):
                self.validator.validate(changed)
        changed = copy.deepcopy(self.comp); changed['all_fields_verified'] = True
        with self.assertRaisesRegex(ValueError, 'full verification'):
            self.run_bind(comparisons=[changed])


if __name__ == '__main__':
    unittest.main()
