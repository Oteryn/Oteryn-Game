"""WCONTRIB native custody and creature-loot admission."""
import hashlib
import json
from pathlib import Path
import unittest

import world_contribution_item_profiles as tool

ROOT = Path(__file__).resolve().parents[2]


class WorldContributionItemProfiles(unittest.TestCase):
    def test_native_provider_materializes_medicine_and_corpse(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {row['production_definition']['production_key']: row for row in document['records']}
        for identifier in (3111, 4176, 4177, 4178, 6018, 12517, 12557, 12558, 12559, 12560, 12561, 12562, 12563):
            key = f'oteryn:item.tibia.i{identifier}'
            self.assertTrue(key in rows, key)
            self.assertTrue(rows[key]['admission']['materializable'])
        self.assertEqual(rows['oteryn:item.tibia.i12517']['admission']['stack_class'], 'StackCapable')
        self.assertEqual(rows['oteryn:item.tibia.i12517']['semantics']['physical']['value']['weight']['value'], 1200)

    def test_corpse_decay_chain_preserves_source_flags_and_timers(self):
        packet = json.loads((ROOT / 'tools/content-schema/native-gameplay/world-contribution-item-profiles.json').read_bytes())
        rows = {int(row['production_binding']['external_id']): row for row in packet['records']}
        self.assertEqual(sorted(rows), [3111, 4176, 4177, 4178, 6018, 12517, 12557, 12558, 12559, 12560, 12561, 12562, 12563])
        self.assertEqual(rows[6018]['admission']['legal_destinations'], ['Ground'])
        for identifier in (4176, 4177, 4178, 12517):
            self.assertEqual(rows[identifier]['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
        for identifier, duration, target in ((6018, 10000, 4176), (4176, 300000, 4177), (4177, 300000, 4178), (4178, 60000, None)):
            temporal = rows[identifier]['semantics']['temporal']['value']
            self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': duration})
            if target is None:
                self.assertEqual(temporal['decay_target'], {'state': 'NOT_APPLICABLE'})
            else:
                self.assertEqual(temporal['decay_target']['value']['key'], f'oteryn:item.tibia.i{target}')
        self.assertEqual(rows[4176]['semantics']['container']['value']['capacity']['value'], 10)
        self.assertEqual(rows[4178]['semantics']['container'], {'state': 'NOT_APPLICABLE'})

    def test_registration_adds_only_the_closed_cohort_and_rejects_duplicates(self):
        generated = {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}
        canonical = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()
        order = lambda row: row['production_definition']['production_key']
        tool.admit(ROOT, generated, 'profiles', 'manifest', order, canonical)
        self.assertEqual(len(json.loads(generated['profiles'])['records']), 13)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'], hashlib.sha256(generated['profiles']).hexdigest())
        with self.assertRaisesRegex(ValueError, 'WCONTRIB_PROFILE_DUPLICATE'):
            tool.admit(ROOT, generated, 'profiles', 'manifest', order, canonical)

    def test_actual_fishing_targets_and_outputs_have_native_profiles(self):
        profiles = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {int(row['production_binding']['external_id']): row for row in profiles['records']}
        required = {3111, 12557, 12558, 12559, 12560, 12561, 12562, 12563}
        self.assertEqual(sorted(required - rows.keys()), [])
        for identifier in (3111, 12557):
            self.assertEqual(rows[identifier]['admission'], {'materializable': True, 'stack_class': 'StackCapable', 'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(rows[identifier]['semantics']['stack']['value']['stack_max'], {'state': 'KNOWN', 'value': 100})
        for identifier in range(12558, 12564):
            self.assertEqual(rows[identifier]['admission'], {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']})
            self.assertEqual(rows[identifier]['semantics']['physical']['value']['pickupable'], {'state': 'KNOWN', 'value': False})
            self.assertEqual(rows[identifier]['semantics']['physical']['value']['movable'], {'state': 'KNOWN', 'value': False})
            self.assertEqual(rows[identifier]['semantics']['classification']['value']['item_type'], {'state': 'KNOWN', 'value': 'TRASH_HOLDER'})
        for identifier in range(12558, 12561):
            temporal = rows[identifier]['semantics']['temporal']['value']
            self.assertEqual(temporal['duration']['value'], 1200000)
            self.assertEqual(temporal['decay_target']['value']['key'], f'oteryn:item.tibia.i{identifier + 3}')
        for identifier in range(12561, 12564):
            self.assertEqual(rows[identifier]['semantics']['temporal'], {'state': 'NOT_APPLICABLE'})

    def test_loot_admission_matches_exact_native_provider(self):
        profiles = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {row['production_definition']['production_key']: row for row in profiles['records']}
        loot = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        items = {row['item']['key']: row for row in loot['items']}
        for identifier in (6018, 12517, 3111, 12557):
            key = f'oteryn:item.tibia.i{identifier}'
            self.assertTrue(items[key]['materializable'])
            self.assertTrue(rows[key]['admission']['materializable'])
        native = ROOT / 'content/spells.manifest.json'
        manifest = json.loads(native.read_bytes())
        self.assertEqual(manifest['item_profiles']['sha256'], hashlib.sha256((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes()).hexdigest())


if __name__ == '__main__':
    unittest.main()
