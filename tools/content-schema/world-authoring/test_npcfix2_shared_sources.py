import json
import unittest

import npcfix2_shared_sources as sources
import quest_access_bindings as registry
import world_region_codec as codec


class CapturedSharedSources(unittest.TestCase):
    def fixture(self, children=None):
        x, y = 32609, 32244
        local = ((y // 32) % 8) * 8 + (x // 32) % 8
        items = [(0, 0, None), (1, 0, None)] + (children or [])
        raw = codec.encode_region(7, 127, 125, {
            local: codec.encode_sector_tiles(x // 32, y // 32, [(x, y, 0, 0, (), items)])
        })
        packet = {'bindings': [{'position': [x, y, 7], 'root_item': 3634,
                               'children': [{'item': 2970, 'key_number': 3301, 'quantity': 1}]}]}
        outputs = {
            sources.REGION: raw,
            registry.INDEX: registry.canonical({'regions': [{'path': sources.REGION, 'items': len(items)}],
                'palette': [{'source_item_id': n} for n in [100, 3634, 2970]],
                'totals': {'items': len(items)}, 'source': {}}),
            registry.SUMMARY: registry.canonical({'totals': {'items': len(items)},
                'source': {}, 'item_attributes': {}, 'bytes_on_disk': len(raw)}),
        }
        return outputs, packet

    def test_captured_key_is_nested_numbered_and_overlay_is_idempotent(self):
        outputs, packet = self.fixture()
        sources.overlay_outputs(outputs, require_all=True, packet=packet)
        items = codec.decode_region(outputs[sources.REGION])[3][0][1][0][5]
        self.assertEqual(items[-1], (2, 1, {'action': 3301}))
        self.assertEqual(json.loads(outputs[registry.INDEX])['totals']['items'], 3)
        first = outputs.copy()
        sources.overlay_outputs(outputs, require_all=True, packet=packet)
        self.assertEqual(outputs, first)

    def test_existing_different_contents_are_not_replaced(self):
        outputs, packet = self.fixture([(2, 1, {'action': 999})])
        with self.assertRaisesRegex(ValueError, 'CONTENT_CONFLICT'):
            sources.overlay_outputs(outputs, require_all=True, packet=packet)

    def test_unrelated_crop_does_not_gain_source_metadata(self):
        outputs, packet = self.fixture()
        packet['bindings'][0]['position'][0] += 1
        before = outputs.copy()
        self.assertEqual(sources.overlay_outputs(outputs, packet=packet), [])
        self.assertEqual(outputs, before)

    def test_missing_root_is_not_silently_accepted(self):
        outputs, packet = self.fixture()
        packet['bindings'][0]['position'][0] += 1
        with self.assertRaisesRegex(ValueError, 'POSITION_MISSING'):
            sources.overlay_outputs(outputs, require_all=True, packet=packet)


    def test_real_outlaw_source_overlay_contains_exact_numbered_keys(self):
        index = json.loads((sources.ROOT / registry.INDEX).read_bytes())
        palette = {r['source_item_id']: n for n, r in enumerate(index['palette'])}
        paths = [registry.INDEX, registry.SUMMARY, sources.REGION]
        outputs = {path: (sources.ROOT / path).read_bytes() for path in paths}
        sources.overlay_outputs(outputs, require_all=True)
        tiles = { (x, y): items for _, sector in codec.decode_region(
            outputs[sources.REGION])[3]
            for x, y, _, _, _, items in sector }
        for x, y, number, item in sources.SPECS:
            with self.subTest(position=(x,y), number=number):
                self.assertIn((palette[item], 1, {'action': number}), tiles[(x,y)])


    def test_committed_outlaw_key_placements_match_the_source_packet(self):
        index = json.loads((sources.ROOT / registry.INDEX).read_bytes())
        palette = {r['source_item_id']: n for n, r in enumerate(index['palette'])}
        tiles = {(x, y): items for _, sector in codec.decode_region(
            (sources.ROOT / sources.REGION).read_bytes())[3]
            for x, y, _, _, _, items in sector}
        for x, y, number, item in sources.SPECS:
            with self.subTest(position=(x, y), number=number):
                self.assertIn((palette[item], 1, {'action': number}), tiles[(x, y)])


if __name__ == '__main__':
    unittest.main()
