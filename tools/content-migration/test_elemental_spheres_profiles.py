"""All altar families preserve nonportable bodies and exact source bindings."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import elemental_spheres_profiles as elemental
import register_spell_families as registry

ROOT = Path(__file__).resolve().parents[2]


class ElementalSpheresProfiles(unittest.TestCase):
    def overlay(self):
        return json.loads((ROOT / elemental.PATH).read_bytes())

    def generated(self):
        return {'profiles': b'{"records":[]}', 'manifest': b'{"item_profiles":{}}'}

    def test_all_altars_have_exact_source_facts_and_only_soils_are_portable(self):
        records = {r['production_definition']['production_key']: r for r in self.overlay()['records']}
        self.assertEqual(set(records), {f'oteryn:item.tibia.i{i}' for i in (940, 941, 944, 945, 7737, 7738, 7739, 7740, 7741, 7742, 7743, 7744, 7745, 7746, 7748, 8106, 8139, 8140, 8141)})
        for key, record in records.items():
            self.assertEqual(record['admission']['materializable'], key in {f'oteryn:item.tibia.i{i}' for i in (940, 941, 944, 945)})
        for item, weight, capacity in ((8106, 40000, 15), (8139, 10000, 14), (8140, 10000, 14), (8141, 10000, 11)):
            body = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(body['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(body['semantics']['physical']['value'], {
                'weight': {'state': 'KNOWN', 'value': weight},
                'movable': {'state': 'KNOWN', 'value': True},
                'pickupable': {'state': 'KNOWN', 'value': False}})
            self.assertEqual(body['semantics']['container']['value']['capacity'], {'state': 'KNOWN', 'value': capacity})
        corpse = records['oteryn:item.tibia.i8141']
        self.assertEqual(corpse['admission']['legal_destinations'], ['Ground'])
        self.assertEqual(corpse['semantics']['container']['value']['capacity'], {'state': 'KNOWN', 'value': 11})
        self.assertEqual(corpse['semantics']['physical']['value'], {
            'weight': {'state': 'KNOWN', 'value': 10000},
            'movable': {'state': 'KNOWN', 'value': True},
            'pickupable': {'state': 'KNOWN', 'value': False}})

    def test_admit_updates_the_manifest_hash_without_duplicate_keys(self):
        generated = self.generated()
        elemental.admit(ROOT, generated, 'profiles', 'manifest', registry.provider_order, registry.spell_import.canonical_bytes)
        records = json.loads(generated['profiles'])['records']
        self.assertEqual(len(records), 19)
        self.assertEqual(len({r['production_definition']['production_key'] for r in records}), 19)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'], hashlib.sha256(generated['profiles']).hexdigest())

    def test_existing_profile_cannot_be_rebound(self):
        record = copy.deepcopy(self.overlay()['records'][0])
        record['production_binding']['target']['key'] = 'oteryn:item.tibia.i8141'
        generated = self.generated()
        generated['profiles'] = json.dumps({'records': [record]}).encode()
        with self.assertRaisesRegex(ValueError, 'PROFILE_CONFLICT'):
            elemental.admit(ROOT, generated, 'profiles', 'manifest', registry.provider_order, registry.spell_import.canonical_bytes)

    def test_missing_record_and_portable_corpse_fail_closed(self):
        for operation in ('missing', 'portable'):
            with tempfile.TemporaryDirectory(prefix='.elemental-profile-test-', dir=ROOT) as directory:
                root = Path(directory)
                target = root / elemental.PATH
                target.parent.mkdir(parents=True)
                overlay = self.overlay()
                if operation == 'missing':
                    overlay['records'].pop()
                else:
                    next(r for r in overlay['records'] if r['production_definition']['production_key'].endswith('.i8141'))['admission']['materializable'] = True
                target.write_text(json.dumps(overlay))
                with self.assertRaises(ValueError):
                    elemental.admit(root, self.generated(), 'profiles', 'manifest', registry.provider_order, registry.spell_import.canonical_bytes)


if __name__ == '__main__':
    unittest.main()
