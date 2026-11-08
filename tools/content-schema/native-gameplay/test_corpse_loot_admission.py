"""Corpse-only native Ground admission must reach the real loot-section producer."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('corpse_full_spell_artifact', Path(__file__).with_name('build_full_spell_artifact.py'))
full = importlib.util.module_from_spec(spec)
spec.loader.exec_module(full)


class CorpseLootAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profile = next(row for row in json.loads(
            (full.ITEM_DEFINITIONS / 'spell-native-profiles.json').read_bytes())['records']
            if row['production_definition']['production_key'] == 'oteryn:item.tibia.i5969')

    def test_actual_ground_only_minotaur_corpse_reaches_production_loot_facts(self):
        _, creatures = full.read(full.CREATURE_PROFILES)
        document = json.loads(full.loot_tables(creatures))
        body = next(row for row in document['items'] if row['item']['key'] == 'oteryn:item.tibia.i5969')
        self.assertEqual(['Ground'], self.profile['admission']['legal_destinations'])
        self.assertTrue(body['materializable'], 'qualified Ground-only minotaur corpse must be mintable')
        self.assertEqual(10, body['container_capacity'])
        # No blanket materializability change to the reference-only Item/yield catalogue.
        self.assertFalse(full.item_facts()['oteryn:item.tibia.i5969']['materializable'])

    def test_actual_minotaur_loot_closure_reuses_qualified_native_inventory_facts(self):
        _, creatures = full.read(full.CREATURE_PROFILES)
        document = json.loads(full.loot_tables(creatures))
        facts = {row['item']['key']: row for row in document['items']}
        for source_id in (3056, 11472, 3274, 3264, 3410):
            with self.subTest(source_id=source_id):
                self.assertTrue(facts[f'oteryn:item.tibia.i{source_id}']['materializable'])
        self.assertEqual(200, facts['oteryn:item.tibia.i3056']['charges'])
        self.assertFalse(facts['oteryn:item.tibia.i3056']['stackable'])
        self.assertTrue(facts['oteryn:item.tibia.i11472']['stackable'])

    def produce(self, profile, *, body_is_yield=False, candidate_profile=None):
        body = {'family': 'Item', 'key': 'oteryn:item.tibia.i5969', 'revision': 'definition-r1'}
        creature = {'family': 'Creature', 'key': 'oteryn:creature.test-corpse', 'revision': 'definition-r1'}
        loot = {'family': 'Loot', 'key': 'oteryn:loot.test-corpse', 'revision': 'definition-r1'}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            items, creatures, tables = (root / name for name in ('items', 'creatures', 'loot'))
            for path in (items, creatures, tables):
                path.mkdir()
            (items / 'items-00000.json').write_text(json.dumps({'records': [{'definition': {
                'identity': body, 'materializable': False, 'semantics': {}}}]}))
            (items / 'spell-native-profiles.json').write_text(json.dumps({'records': [profile] if profile else []}))
            (creatures / 'creatures-00000.json').write_text(json.dumps({'records': [{'definition': {
                'identity': creature, 'loot': loot if body_is_yield else None}}]}))
            if body_is_yield:
                (tables / 'loot-00000.json').write_text(json.dumps({'records': [{'definition': {
                    'identity': loot, 'algorithm': 'IndependentBernoulliPpm', 'entries': [{
                        'item': body, 'min_count': 1, 'max_count': 1, 'probability_ppm': 1000000}]}}]}))
            profiles = {'records': [{'profile': {'target': creature, 'data': {'profile': {
                'details': {'corpse_item': body}}}}}]}
            with patch.object(full, 'creature_products', return_value=set()):
                result = json.loads(full.loot_tables(profiles, creatures, tables, items,
                    {"records": [candidate_profile]} if candidate_profile is not None else None))
            return result['items'][0]

    def test_fresh_candidate_item_profiles_close_loot_before_disk_registration(self):
        result = self.produce(None, candidate_profile=copy.deepcopy(self.profile))
        self.assertTrue(result['materializable'])
        self.assertEqual(result['container_capacity'], 10)
        substituted = copy.deepcopy(self.profile)
        substituted['production_binding']['target']['revision'] = 'wrong-revision'
        self.assertFalse(self.produce(None, candidate_profile=substituted)['materializable'])

    def test_source_ground_container_facts_are_reused_without_inventory_grant(self):
        result = self.produce(copy.deepcopy(self.profile))
        self.assertTrue(result['materializable'])
        self.assertEqual(10, result['container_capacity'])

    def test_reference_only_corpse_cannot_be_promoted_as_a_regular_loot_yield(self):
        result = self.produce(copy.deepcopy(self.profile), body_is_yield=True)
        self.assertFalse(result['materializable'])
        self.assertIsNone(result['container_capacity'])

    def test_unknown_or_mismatched_native_facts_keep_corpse_closed(self):
        variants = []
        for field in ('container', 'physical', 'classification'):
            row = copy.deepcopy(self.profile)
            row['semantics'][field] = {'state': 'UNKNOWN'}
            variants.append((field, row))
        for capacity in (0, 31, True):
            row = copy.deepcopy(self.profile)
            row['semantics']['container']['value']['capacity'] = {'state': 'KNOWN', 'value': capacity}
            variants.append((f'capacity-{capacity}', row))
        for field, value in (('materializable', False), ('legal_destinations', ['CharacterInventory'])):
            row = copy.deepcopy(self.profile)
            row['admission'][field] = value
            variants.append((field, row))
        row = copy.deepcopy(self.profile)
        row['production_binding']['target']['revision'] = 'wrong-revision'
        variants.append(('binding', row))
        variants.append(('missing-profile', None))
        for label, profile in variants:
            with self.subTest(label=label):
                result = self.produce(profile)
                self.assertFalse(result['materializable'])
                self.assertIsNone(result['container_capacity'])


if __name__ == '__main__':
    unittest.main()
