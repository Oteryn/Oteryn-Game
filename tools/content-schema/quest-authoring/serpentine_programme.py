"""Selected finite tower trial; retain the source route and its one pearl claim."""
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.serpentine_tower_quest'
TRACK = 'oteryn:quest-progress/serpentine-trial/stage'
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
DONORS = [
    ('data-global/scripts/quests/others/actions_serpentine_tower_torch.lua', 'ec8c4bbf2c1643e5cf1cdf499f51a1ecc4ebba7bb4b3fca46ec82193207d648d', '3-5'),
    ('data-global/scripts/quests/others/actions_serpentine_tower_lever.lua', 'a0f4d171cb73739843e14021b5e87d2113753a816b213f4db218836f3e854c07', '3-34'),
    ('data-global/scripts/quests/white_pearl/movements_white_pearl.lua', '216b2d0a1111217874c28c31ef14501df70f96c9bf5e2f48975b56da5995d66f', '1-46'),
]


def p(x, y, z):
    return dict(x=x, y=y, floor=-z)


def documents():
    from qengine_access import callback
    path = 'content/quests/definitions/quests-00200-00299.json'
    raw = (ROOT / path).read_bytes()
    records = json.loads(raw)['records']
    definition = records[0]['definition']
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (definition['identity']['key'] != QUEST
            or recipe['requirements']['min_level'] != 0
            or not recipe['requirements']['premium']
            or recipe['requirements']['prerequisites'] != []
            or recipe['repeat']['kind'] != 'once'
            or [(s['kind'], s['count']) for s in recipe['stages']] != [('explore', 1), ('use', 1), ('use', 1), ('complete', 1)]
            or [(r['item']['key'], r['count']) for r in recipe['reward_intents']] != [('oteryn:item.tibia.i3026', 1)]):
        raise ValueError('SERPENTINE_SELECTED_RECIPE_DRIFT')
    sources = [f'oteryn:selected-recipe:{path}:/records/0/definition:sha256:{hashlib.sha256(raw).hexdigest()}']
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        for name, digest, lines in DONORS:
            row, = [r for r in manifest['files'] if r['source'] == 'crystalserver' and r['revision'] == PIN and r['path'] == name]
            donor = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            if row['sha256'] != digest or hashlib.sha256(donor).hexdigest() != digest:
                raise ValueError('SERPENTINE_DONOR_DRIFT: ' + name)
            sources.append(f'crystal:{PIN}:{name}:{lines}:sha256:{digest}')
    sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
    import world_region_codec
    palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
    cells = {}
    for z in [7, 8, 9]:
        region = f'content/world/placements/region-z{z:02}-x129-y128.b3'
        world = (ROOT / region).read_bytes()
        sources.append(f'oteryn:source-map:{region}:sha256:{hashlib.sha256(world).hexdigest()}')
        for _, tiles in world_region_codec.decode_region(world)[3]:
            for x, y, flags, house, zones, items in tiles:
                cells[(x, y, z)] = [palette[item]['source_item_id'] for item, depth, attrs in items if depth == 0]
    expected = {
        (33147,32863,7): [423], (33148,32864,7): [423,1949],
        (33150,32864,7): [423,1949], (33151,32861,7): [1316,1306,2928],
        (33150,32862,7): [423,2472], (33145,32865,7): [423,2306,2698,2820],
        (33151,32866,8): [422,1345], (33152,32866,8): [415,2772],
    }
    expected.update({(x,y,9): [422,2129] for x in [33148,33149] for y in [32867,32868]})
    if any(cells.get(at) != items for at, items in expected.items()):
        raise ValueError('SERPENTINE_SOURCE_GEOMETRY_DRIFT')
    rows = []

    def add(name, trigger, at, appearance, before, after, **effects):
        row = callback('authored-serpentine-' + name, 'serpentine_tower_quest', trigger,
                       'selected-finite-observation-trial:' + name, source_pin='oteryn-authored:',
                       at=at, appearance=appearance, **effects)
        row['sources'] += sources
        row['storage'] = [dict(track=TRACK, minimum=before, maximum=before, set=after)]
        row['premium_required'] = True
        rows.append(row)
        return row

    add('explore', 'step_in', p(33147,32863,7), 423, -1, 0)
    for name, at, action, destination in [
        ('entry', p(33148,32864,7), 5630, p(33145,32863,7)),
        ('return', p(33150,32864,7), 5631, p(33147,32864,7)),
    ]:
        row = callback('authored-serpentine-' + name, 'serpentine_tower_quest', 'step_in',
                       'selected-finite-observation-trial:' + name, source_pin='oteryn-authored:',
                       at=at, appearance=1949, action=action, teleport=destination)
        row['sources'] += sources
        row['storage'] = [dict(track=TRACK, minimum=0, maximum=3, set=0, read_only=True)]
        if name == 'entry':
            row['remote_teleport'] = dict(position=p(33145,32862,7), appearance=3465, destination=p(33151,32864,7))
        rows.append(row)
    add('torch', 'use', p(33151,32861,7), 2928, 0, 1, action=5632,
        passages=[dict(position=p(33151,32866,8), appearance=1345, replacement=None)])
    add('lever', 'use', p(33152,32866,8), 2772, 1, 2, action=5633, lever=[2772,2773],
        passages=[dict(position=p(x,y,9), appearance=2129, replacement=None) for x in [33148,33149] for y in [32867,32868]])
    # The existing research paper is an authored observation contact. Its text and
    # the retained source map remain unchanged; this does not solve the mystery.
    add('observation', 'use', p(33145,32865,7), 2820, 2, 3)['completes'] = True
    gate = dict(claim='oteryn:reward-claim.quest.u7_3.serpentine_tower.white_pearl', quest=QUEST,
                conditions=[dict(track=TRACK, minimum=3, maximum=3)], sources=sources)
    return rows, gate
