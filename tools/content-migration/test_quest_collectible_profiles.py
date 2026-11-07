"""Plain collectible admission retains canonical facts and exact reviewed proofs."""
import hashlib
import copy
import tempfile
from pathlib import Path
import json
import unittest

import grizzly_shop_profiles as accepted
import quest_collectible_profiles as author


class QuestCollectibleAdmission(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = author.capture()
        # Preserve the original unknown-canonical fallback regression even after
        # ITEMHOLD-Q supplies the real selected collectible's exact native facts.
        item, _, _, _, recipe = author.SELECTIONS[0]
        selected = [r['definition'] for path in (author.ROOT/'content/items/definitions').glob('items-*.json')
                    for r in json.loads(path.read_bytes())['records']
                    if r['definition']['identity']['key'] == f'oteryn:item.tibia.i{item}']
        cls.fallback_definition = copy.deepcopy(selected[0])
        cls.fallback_definition['materializable'] = False
        cls.fallback_definition['semantics']['physical'] = {'state': 'UNKNOWN'}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in [author.CLIENT, recipe]:
                target = root/relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.symlink_to(author.ROOT/relative)
            definitions = root/'content/items/definitions'
            definitions.mkdir(parents=True)
            (definitions/'items-00000-00000.json').write_bytes(author.encoded(
                {'records': [{'definition': cls.fallback_definition}]}))
            cls.fallback_packet = author.capture(root)

    def test_own_namespace_plain_policy_and_preserved_canonical_known_facts(self):
        entry, = self.fallback_packet['entries']
        profile = entry['profile']
        self.assertEqual(profile['production_binding']['source_key'], 'oteryn:source.oteryn')
        self.assertEqual(profile['production_binding']['identity_namespace'], 'oteryn/quest_collectible_id')
        self.assertEqual(profile['admission'], dict(materializable=True,stack_class='NonStackable',legal_destinations=['CharacterInventory','Ground']))
        for group in ['container','charges','temporal']:
            self.assertEqual(profile['semantics'][group], dict(state='NOT_APPLICABLE'))
        original = self.fallback_definition
        self.assertFalse(accepted.known_conflict(original['semantics'],profile['semantics']))
        self.assertEqual(profile['semantics']['physical']['value']['weight'],dict(state='KNOWN',value=1000))
        self.assertIn('10oz',entry['assumption'])

    def test_native_record_and_generated_proof_match_exactly(self):
        raw = (author.ROOT/author.PACKET).read_bytes()
        self.assertEqual(json.loads(raw),self.packet)
        self.assertIn(hashlib.sha256(raw).hexdigest(),(author.ROOT/author.PIN).read_text())
        native = json.loads((author.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        self.assertEqual(self.packet['entries'], [], 'canonical native ownership supersedes fallback admission')
        bottle, = [row for row in native['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i44179']
        self.assertEqual(bottle['production_binding']['source_key'], 'oteryn:source.tibiawiki')
        self.assertEqual(bottle['production_binding']['disposition'], 'EXACT')
        self.assertEqual(bottle['semantics']['physical']['value']['weight'], dict(state='KNOWN', value=60))
        self.assertEqual(bottle['admission'], dict(materializable=True,stack_class='NonStackable',legal_destinations=['CharacterInventory','Ground']))
        for entry in self.packet['entries']:
            actual, = [row for row in native['records'] if row['production_definition']==entry['profile']['production_definition']]
            self.assertEqual(actual,entry['profile'])


if __name__ == '__main__':
    unittest.main()
