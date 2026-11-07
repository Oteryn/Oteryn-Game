"""Plain collectible admission retains canonical facts and exact reviewed proofs."""
import hashlib
import json
import unittest

import grizzly_shop_profiles as accepted
import quest_collectible_profiles as author


class QuestCollectibleAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = author.capture()

    def test_own_namespace_plain_policy_and_preserved_canonical_known_facts(self):
        entry, = self.packet['entries']
        profile = entry['profile']
        self.assertEqual(profile['production_binding']['source_key'], 'oteryn:source.oteryn')
        self.assertEqual(profile['production_binding']['identity_namespace'], 'oteryn/quest_collectible_id')
        self.assertEqual(profile['admission'], dict(materializable=True,stack_class='NonStackable',legal_destinations=['CharacterInventory','Ground']))
        for group in ['container','charges','temporal']:
            self.assertEqual(profile['semantics'][group], dict(state='NOT_APPLICABLE'))
        definitions = [r['definition'] for path in (author.ROOT/'content/items/definitions').glob('items-*.json')
                       for r in json.loads(path.read_bytes())['records']
                       if r['definition']['identity']['key']=='oteryn:item.tibia.i44179']
        original, = definitions
        self.assertFalse(accepted.known_conflict(original['semantics'],profile['semantics']))
        self.assertEqual(profile['semantics']['physical']['value']['weight'],dict(state='KNOWN',value=1000))
        self.assertIn('10oz',entry['assumption'])

    def test_native_record_and_generated_proof_match_exactly(self):
        raw = (author.ROOT/author.PACKET).read_bytes()
        self.assertEqual(json.loads(raw),self.packet)
        self.assertIn(hashlib.sha256(raw).hexdigest(),(author.ROOT/author.PIN).read_text())
        native = json.loads((author.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        for entry in self.packet['entries']:
            actual, = [row for row in native['records'] if row['production_definition']==entry['profile']['production_definition']]
            self.assertEqual(actual,entry['profile'])


if __name__ == '__main__':
    unittest.main()
