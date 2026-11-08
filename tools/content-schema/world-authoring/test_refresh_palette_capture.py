"""Merged overlays must recount attributes, including present zero/empty values."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import refresh_palette_capture as refresh
import world_region_codec as codec


class RefreshPaletteCaptureTests(unittest.TestCase):
    def test_recounts_merged_region_attributes_without_changing_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            xml = b'<items><item id="100" name="fixture"/></items>'
            source = root / 'items.xml'
            source.write_bytes(xml)
            summary_path = 'tools/content-schema/world-authoring/samples/world-base-capture-v1.json'
            summary = {'source': {'files': [{'path': refresh.base.ITEMS_XML,
                       'sha256': hashlib.sha256(xml).hexdigest()}]},
                       'item_attributes': {'unique': 99}, 'bytes_on_disk': 0}
            raw = codec.encode_region(7, 0, 0, {0: codec.encode_sector_tiles(0, 0, [
                (1, 2, 0, 0, (), [(0, 0, {'unique': 0, 'text': ''}),
                                  (0, 1, {'unique': 123})])])})
            region = 'content/world/placements/region-z07-x000-y000.b3'
            index = {'regions': [{'path': region, 'sha256': hashlib.sha256(raw).hexdigest()}],
                     'palette': [{'key': 'oteryn:item.tibia.i100', 'source_item_id': 100,
                                  'provisional': False}], 'totals': {'items': 2}}
            for path, data in ((summary_path, json.dumps(summary).encode()),
                               ('content/world/placements/index.json', json.dumps(index).encode()),
                               (region, raw)):
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
            result = json.loads(refresh.outputs(source, root)[summary_path])
            self.assertEqual(result['item_attributes'], {'text': 1, 'unique': 2})
            self.assertEqual(result['bytes_on_disk'], len(raw))
            self.assertEqual(result['source'], summary['source'])
            self.assertEqual((root / region).read_bytes(), raw)
