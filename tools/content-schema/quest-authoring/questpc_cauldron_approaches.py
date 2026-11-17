"""The chosen personal contribution starts at the retained Green Claw cauldron."""
from pathlib import Path

QUEST = 'oteryn:quest.authored.bewitched'
REFERENCE = 'cauldron-approach:green-claw-swamp'
PLACEMENTS = 'content/world/placements/region-z12-x127-y124.b3'
ADAPTATION = ('Replace rotating ingredient lists, eight-hour global phases and world '
              'favour points with a fixed daily contribution.')


def resolve(root, quest, stage, reference, *, definition):
    recipe = definition.get('recipe', {})
    first = recipe.get('stages', [{}])[0]
    if ((quest, stage, reference) != (QUEST, 's1', REFERENCE)
            or definition['identity']['key'] != QUEST
            or recipe.get('repeat', {}).get('kind') != 'daily'
            or ADAPTATION not in recipe.get('adaptations', [])
            or (first.get('key'), first.get('kind'), first.get('count'), first.get('objective'))
            != ('s1', 'explore', 1, 'Reach the witch cauldron beneath Green Claw Swamp')):
        raise ValueError('Not the chosen personal cauldron survey')
    refs = recipe.get('source_refs', [])
    source = 'https://tibia.fandom.com/index.php?curid=50859&oldid=1190948'
    if not any(r.get('url') == source and r.get('content_sha256') ==
               'c27620a33a5938ab3d61a97b046147d5cffea9ac131acb8d8956132931a78987' for r in refs):
        raise ValueError('Pinned personal cauldron source changed')
    from questpc_native_anchors import NativeAnchors
    native = NativeAnchors(root)
    raw = (Path(root) / PLACEMENTS).read_bytes()
    z, rx, ry, table = native.codec.parse_region(raw)
    if (z, rx, ry) != (12, 127, 124):
        raise ValueError('Pinned cauldron region changed')
    found = []
    for local, offset, length in table:
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        if (sx, sy) != (32732 // 32, 31935 // 32):
            continue
        payload = native.codec.zstandard.ZstdDecompressor().decompress(
            raw[offset:offset + length], max_output_size=native.codec.MAX_SECTOR_BYTES)
        for x, y, _, house, _, items in native.codec.decode_sector(payload, sx, sy):
            if (x, y) == (32732, 31935) and house == 0:
                found = [native.palette[index]['source_item_id'] for index, _, _ in items]
    if found != [7272, 12023]:
        raise ValueError('Pinned huge cauldron landmark changed')
    return (dict(x=32733, y=31935, floor=-12), 7272,
            [source, PLACEMENTS,
             'https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
