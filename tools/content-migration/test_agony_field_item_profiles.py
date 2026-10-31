import json
from pathlib import Path
import unittest

import agony_field_item_profiles as agony

ROOT = Path(__file__).resolve().parents[2]
PROFILES = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'


def key(row):
    return row['production_definition']['production_key']


class AgonyFieldItemProfilesTest(unittest.TestCase):
    def generated(self):
        document = json.loads((ROOT / PROFILES).read_bytes())
        document['records'] = [r for r in document['records'] if key(r) != agony.KEY]
        return {PROFILES: json.dumps(document).encode(), MANIFEST: (ROOT / MANIFEST).read_bytes()}

    def admit(self, generated):
        agony.admit(ROOT, generated, PROFILES, MANIFEST,
                    lambda r: int(key(r).split('.i')[-1]),
                    lambda value: (json.dumps(value, sort_keys=True) + '\n').encode())

    def test_the_crystal_agony_field_is_admitted_once_with_its_source_profile(self):
        generated = self.generated()
        self.admit(generated)
        [row] = [r for r in json.loads(generated[PROFILES])['records'] if key(r) == agony.KEY]
        field = row['attributes']['field_condition']
        self.assertEqual(field['source']['server'], 'crystal')
        self.assertEqual(field['condition']['element'], 'agony')
        self.assertEqual(sum(e['amount'] for e in field['condition']['entries']), 33930)
        self.assertEqual(row['semantics']['temporal']['value']['duration']['value'], 248_000)
        self.assertEqual(row['semantics']['temporal']['value']['decay_target'], agony.NA)
        self.assertEqual(row['semantics']['classification']['value']['item_type']['value'],
                         'MAGIC_FIELD')
        self.assertEqual((row['attributes']['blocks_movement'], row['attributes']['blocks_projectile']),
                         (False, False))
        self.assertEqual(row['production_binding']['disposition'], 'EXACT')
        self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
        # The regenerated provider holds exactly this record.
        [committed] = [r for r in json.loads((ROOT / PROFILES).read_bytes())['records']
                       if key(r) == agony.KEY]
        self.assertEqual(committed, row)
        with self.assertRaises(ValueError):
            self.admit(generated)

    def test_the_official_appearance_is_unmovable_and_walkable(self):
        facts = agony.physical(ROOT)
        self.assertEqual({k: facts[k] for k in ('movable', 'pickupable', 'block_solid',
                                                 'block_projectile', 'has_height', 'stackable')},
                         {'movable': False, 'pickupable': False, 'block_solid': False,
                          'block_projectile': False, 'has_height': False, 'stackable': False})


if __name__ == '__main__':
    unittest.main()
