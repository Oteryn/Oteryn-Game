"""Check the chosen Nomads window binding against existing, pinned native inputs."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec  # noqa: E402


class NomadsWindowSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profile = json.loads((ROOT / 'docs/reference/content-prep/qmech/nomads-land-window.json').read_text())
        cls.index = json.loads((ROOT / cls.profile['map']['palette']).read_text())
        raw = (ROOT / cls.profile['map']['path']).read_bytes()
        cls.digest = hashlib.sha256(raw).hexdigest()
        cls.z, _, _, sectors = world_region_codec.decode_region(raw)
        cls.tiles = {(x, y): (house, items) for _, tiles in sectors for x, y, _, house, _, items in tiles}

    def entries(self, position):
        self.assertEqual(-self.z, position['floor'])
        house, items = self.tiles[(position['x'], position['y'])]
        self.assertEqual(house, 0)
        return [(self.index['palette'][palette]['key'], attrs) for palette, depth, attrs in items if depth == 0]

    def test_window_is_the_exact_plain_base_native_entry(self):
        window = self.profile['window']
        entries = self.entries(window['position'])
        self.assertEqual(entries[window['ordinal']][0], f"oteryn:item.tibia.i{window['appearance']}")
        self.assertEqual(self.digest, self.profile['map']['sha256'])
        self.assertEqual(self.index['source']['revision'], self.profile['map']['donor_revision'])

    def test_staging_tiles_are_clear_and_the_four_source_objects_exist(self):
        for required in self.profile['requirements']:
            self.assertEqual(self.entries(required['position']), [('oteryn:item.tibia.i7358', None)])
        for source in self.profile['loose_sources']:
            self.assertEqual(sum(key == source['item'] for key, _ in self.entries(source['position'])), source['instances'])

    def test_all_four_instances_use_admitted_movable_ground_definitions(self):
        records = json.loads((ROOT / self.profile['native_item_profile']).read_text())['records']
        by_key = {row['production_definition']['production_key']: row for row in records}
        for requirement in self.profile['requirements']:
            for expected in requirement['ground']['stacks']:
                row = by_key[expected['item']]
                self.assertTrue(row['admission']['materializable'])
                self.assertIn('Ground', row['admission']['legal_destinations'])
                self.assertEqual(row['admission']['stack_class'], 'NonStackable')
                self.assertEqual(row['production_definition']['revision_ref'], expected['revision'])
                self.assertEqual(expected['quantity'], 1)
                self.assertTrue(row['semantics']['physical']['value']['movable']['value'])


if __name__ == '__main__':
    unittest.main()
