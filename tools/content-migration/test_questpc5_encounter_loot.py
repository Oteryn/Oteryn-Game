"""Owned loot refresh preserves unrelated lanes and refuses incomplete admission."""
import json
from pathlib import Path
import unittest

import questpc5_encounter_loot as admission

ROOT = Path(__file__).resolve().parents[2]


class EncounterLootTests(unittest.TestCase):
    def inputs(self):
        return {'profiles': (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes(),
                'manifest': json.dumps({'loot_tables': {'sha256': 'before'}}).encode()}

    def apply(self, generated):
        admission.apply(ROOT, generated, 'profiles', 'manifest',
                        lambda doc: json.dumps(doc, sort_keys=True, separators=(',', ':')).encode())
        return json.loads(generated[admission.PATH])

    def test_exact_closures_preserve_source_drops_and_unrelated_facts(self):
        before = json.loads((ROOT / admission.PATH).read_bytes())
        generated = self.inputs()
        after = self.apply(generated)
        self.assertEqual(before['tables'], after['tables'])
        self.assertEqual(before['creature_loot'], after['creature_loot'])
        selected = {f'oteryn:item.tibia.i{x}' for x in admission.CORPSES}
        for table in before['tables']:
            if table['identity']['key'] in {'oteryn:loot.creature.' + n for n in ('troll', 'dragon', 'minotaur_guard')}:
                selected.update(e['item']['key'] for e in table['entries'])
        for old, new in zip(before['items'], after['items'], strict=True):
            if old['item']['key'] not in selected:
                self.assertEqual(old, new)
            else:
                self.assertTrue(new['materializable'])
        self.assertEqual(after, self.apply(generated))

    def test_missing_or_conflicting_corpse_refuses(self):
        for variant in ('missing', 'capacity'):
            generated = self.inputs()
            profiles = json.loads(generated['profiles'])
            row = next(r for r in profiles['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i5960')
            if variant == 'missing':
                profiles['records'].remove(row)
            else:
                row['semantics']['container']['value']['capacity']['value'] = 30
            generated['profiles'] = json.dumps(profiles).encode()
            with self.subTest(variant=variant), self.assertRaises(ValueError):
                self.apply(generated)


if __name__ == '__main__':
    unittest.main()
