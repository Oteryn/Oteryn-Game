"""Crafting overlays retain established native facts and reject real conflicts."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

import qready1_craft_item_profiles as craft
import register_spell_families as registry


ROOT = Path(__file__).resolve().parents[2]


class CraftProfileJoin(unittest.TestCase):
    def generated(self):
        packet = json.loads((ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes())
        records = [r for r in packet['records']
                   if r['production_definition']['production_key'] in {
                       'oteryn:item.tibia.i21506', 'oteryn:item.tibia.i21507'}]
        self.assertEqual(len(records), 2)
        return {'profiles': json.dumps({'records': records}).encode(),
                'manifest': b'{"item_profiles":{}}'}

    def admit(self, generated):
        craft.admit(ROOT, generated, 'profiles', 'manifest',
                    registry.provider_order, registry.spell_import.canonical_bytes)

    def test_actual_shared_oil_and_dust_facts_survive_crafting_admission(self):
        generated = self.generated()
        before = json.loads(generated['profiles'])['records']
        self.admit(generated)
        after = json.loads(generated['profiles'])['records']
        self.assertEqual(len(after), len(craft.IDS))
        for record in before:
            self.assertIn(record, after)
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         hashlib.sha256(generated['profiles']).hexdigest())

    def test_established_weight_name_binding_and_custody_conflicts_still_refuse(self):
        for field in ('weight', 'name', 'binding', 'custody', 'description'):
            with self.subTest(field=field):
                generated = self.generated()
                packet = json.loads(generated['profiles'])
                record = packet['records'][0]
                if field == 'weight':
                    record['semantics']['physical']['value']['weight']['value'] += 1
                elif field == 'name':
                    record['semantics']['presentation']['value']['name']['value'] += ' changed'
                elif field == 'binding':
                    record['production_binding']['external_id'] = '1'
                elif field == 'custody':
                    record['admission']['legal_destinations'] = ['Ground']
                else:
                    record['semantics']['presentation']['value']['description']['value'] += ' changed'
                generated['profiles'] = json.dumps(packet).encode()
                predecessor = copy.deepcopy(generated)
                with self.assertRaisesRegex(ValueError, 'CONFLICT'):
                    self.admit(generated)
                self.assertEqual(generated, predecessor)


if __name__ == '__main__':
    unittest.main()
