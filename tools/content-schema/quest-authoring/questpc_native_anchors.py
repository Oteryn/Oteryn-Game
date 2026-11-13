"""Validate imported EXPLORE selectors against retained native placement sectors."""
import json
from pathlib import Path
import sys


class NativeAnchors:
    def __init__(self, root):
        self.root = Path(root)
        codec_path = str(self.root / 'tools/content-schema/world-authoring')
        if codec_path not in sys.path:
            sys.path.insert(0, codec_path)
        import world_region_codec
        self.codec = world_region_codec
        self.palette = json.loads(
            (self.root / 'content/world/placements/index.json').read_text())['palette']
        self.sectors = {}

    def validate(self, at, appearance):
        self.require(at, [appearance])

    def require(self, at, entries):
        x, y, z = at['x'], at['y'], -at['floor']
        sector = (z, x // 32, y // 32)
        if sector not in self.sectors:
            rx, ry = x // 256, y // 256
            path = self.root / f'content/world/placements/region-z{z:02}-x{rx}-y{ry}.b3'
            if not path.is_file():
                raise ValueError('Missing native EXPLORE region: ' + str(path))
            raw = path.read_bytes()
            floor, region_x, region_y, table = self.codec.parse_region(raw)
            if (floor, region_x, region_y) != (z, rx, ry):
                raise ValueError('Wrong native EXPLORE region identity: ' + str(path))
            tiles = {}
            for local, offset, length in table:
                sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
                if (sx, sy) != sector[1:]:
                    continue
                payload = self.codec.zstandard.ZstdDecompressor().decompress(
                    raw[offset:offset + length], max_output_size=self.codec.MAX_SECTOR_BYTES)
                for tx, ty, _, house, _, items in self.codec.decode_sector(payload, sx, sy):
                    ground = [self.palette[index]['source_item_id']
                              for index, depth, _ in items if depth == 0]
                    tiles[(tx, ty)] = (house, ground)
            self.sectors[sector] = tiles
        tile = self.sectors[sector].get((x, y))
        if tile is None or tile[0] != 0 or tile[1] != entries:
            raise ValueError('Missing, private or changed native EXPLORE ground: ' + str(at))
