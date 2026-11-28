"""Pinned NPC recipes close native custody without changing existing profiles."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import authored_npc_item_profiles as profiles

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/items/native-manifest.json'


class AuthoredNpcCustody(unittest.TestCase):
    def generated(self):
        return {PROFILE: (ROOT / PROFILE).read_bytes(),
                MANIFEST: b'{"item_profiles":{"sha256":"pending"}}'}

    def admit(self, generated):
        profiles.admit(ROOT, generated, PROFILE, MANIFEST,
                       lambda row: row['production_definition']['production_key'],
                       lambda document: json.dumps(document, sort_keys=True).encode())

    def test_complete_recipe_has_exact_native_custody_and_retains_existing_rows(self):
        generated = self.generated()
        before = {row['production_definition']['production_key']: row
                  for row in json.loads(generated[PROFILE])['records']}
        self.admit(generated)
        after = {row['production_definition']['production_key']: row
                 for row in json.loads(generated[PROFILE])['records']}
        for key, row in before.items():
            self.assertEqual(after[key], row)
        for source in (*range(11545, 11553), 11701):
            row = after[f'oteryn:item.tibia.i{source}']
            self.assertTrue(row['admission']['materializable'])
            self.assertIn('CharacterInventory', row['admission']['legal_destinations'])

    def test_npc_shop_shopping_bag_has_exact_native_custody(self):
        generated = self.generated()
        self.admit(generated)
        row, = [item for item in json.loads(generated[PROFILE])['records']
                if item['production_definition']['production_key'] == 'oteryn:item.tibia.i21411']
        self.assertEqual(row['production_binding']['external_id'], '21411')
        self.assertEqual(row['admission']['stack_class'], 'NonStackable')
        self.assertIn('CharacterInventory', row['admission']['legal_destinations'])
        self.assertEqual(row['semantics']['container']['value']['capacity']['value'], 20)
        self.assertEqual(row['semantics']['physical']['value']['weight']['value'], 1800)

    def test_postman_handouts_and_cael_shop_have_exact_native_custody(self):
        generated = self.generated()
        self.admit(generated)
        rows = {r['production_binding'].get('external_id'): r
                for r in json.loads(generated[PROFILE])['records']}
        for identifier in ('3216', '3220', '10216', '10288'):
            with self.subTest(identifier=identifier):
                row = rows.get(identifier)
                self.assertIsNotNone(row)
                self.assertEqual(row['production_definition']['production_key'],
                                 f'oteryn:item.tibia.i{identifier}')
                self.assertTrue(row['admission']['materializable'])
                self.assertIn('CharacterInventory', row['admission']['legal_destinations'])

    def test_final_postman_promotion_completes_the_quest(self):
        document = json.loads(
            (ROOT / 'content/dialogues/quest-rules-supplemental/postman.json').read_bytes())
        promotion, = [rule for rule in document['rules']
                      if rule['key'] == 'oteryn:quest-npc/postman/kevin/rank_5']
        self.assertTrue(promotion.get('completes'))
        self.assertTrue(any(effect.get('value') == 5 and effect['track'].endswith('/rank')
                            for effect in promotion['effects'] if effect['kind'] == 'set'))

    def test_changed_authoring_pin_refuses_without_any_generated_mutation(self):
        generated = self.generated()
        before = copy.deepcopy(generated)
        original = profiles.hashlib.sha256
        class WrongDigest:
            def hexdigest(self):
                return '0' * 64
        with patch.object(profiles.hashlib, 'sha256', return_value=WrongDigest()):
            with self.assertRaisesRegex(ValueError, 'AUTHORING_PIN'):
                self.admit(generated)
        self.assertEqual(generated, before)
        self.assertIs(profiles.hashlib.sha256, original)


if __name__ == '__main__':
    unittest.main()
