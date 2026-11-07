"""Compile source-world operations after verifying their source and native map pins."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]
CAPSULE = 'tools/content-schema/quest-authoring/samples/native-rewards/source-world-objects.json'
WORLD = 'content/quests/source-world-callbacks.json'
CORPUS = 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
CORPUS_SHA = 'f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277'
SOURCE_SHAS = {
    'life_ring': 'f24ab1e72f54048662b0cd436129ff69657f80fbbb359154f6502b194f4893f9',
    'mintwallin': '61944b37d0a86201ad45759b7abddf2e7b80389d4b0a04efe4a54db257d4742c',
    'hidden_threats': '0e00845e4ce1d3cf40dba3139210f86bc96dcb0f8f3242345a4a01aa3badc9b4',
}


def read(root, path):
    return json.loads((root / path).read_bytes())


def verify(root, capsule):
    if hashlib.sha256((root / CORPUS).read_bytes()).hexdigest() != CORPUS_SHA:
        raise ValueError('SOURCE_WORLD_CORPUS_PIN')
    with tarfile.open(root / CORPUS) as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        sources = [(capsule['worlds'][key]['source'], sha) for key, sha in SOURCE_SHAS.items()]
        for key in ['life_ring', 'mintwallin']:
            sources.append((capsule['worlds'][key]['reward_source'], '0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca'))
        for source, sha in sources:
            matches = [r for r in manifest['files'] if r['revision'] == source['revision']
                       and r['sha256'] == sha and source['url'] ==
                       f"https://raw.githubusercontent.com/{r['repository']}/{r['revision']}/{r['path']}"]
            if source['sha256'] != sha or len(matches) != 1:
                raise ValueError('SOURCE_WORLD_SOURCE_CUSTODY')
            raw = corpus.extractfile(matches[0]['cache_path']).read()
            if hashlib.sha256(raw).hexdigest() != sha:
                raise ValueError('SOURCE_WORLD_SOURCE_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    from world_region_codec import decode_region
    palette = read(root, 'content/world/placements/index.json')['palette']
    regions = {}

    def check(value):
        if isinstance(value, list):
            for row in value:
                check(row)
        elif isinstance(value, dict):
            if 'map' in value:
                path = value['map']
                if (not path.startswith('content/world/placements/region-')
                        or '..' in Path(path).parts or not path.endswith('.b3')):
                    raise ValueError('SOURCE_WORLD_MAP_PATH')
                if path not in regions:
                    raw = (root / path).read_bytes()
                    regions[path] = (hashlib.sha256(raw).hexdigest(), decode_region(raw)[3])
                sha, sectors = regions[path]
                x, y, _ = value['position']
                tiles = [t for _, rows in sectors for t in rows if t[:2] == (x, y)]
                if sha != value['map_sha256'] or len(tiles) != 1 or tiles[0][3] != 0:
                    raise ValueError('SOURCE_WORLD_NATIVE_PIN')
                roots = [[palette[i]['source_item_id'], attrs]
                         for i, depth, attrs in tiles[0][5] if depth == 0]
                if value['house'] != 0 or roots != value['roots']:
                    raise ValueError('SOURCE_WORLD_NATIVE_SHAPE')
            for row in value.values():
                check(row)
    check(capsule)
    terrain = capsule['worlds']['life_ring']['native_terrain']
    if [(row['source_item_id'], row['kind'], row['walkable'], row['ground_speed']) for row in terrain] != [
            (4598, 'ground', False, 0), (4788, 'border', True, None), (5770, 'ground', True, 90)]:
        raise ValueError('SOURCE_WORLD_BRIDGE_TERRAIN')
    for row in terrain:
        path = root / row['path']
        if (not row['path'].startswith('content/world/terrain/terrain-') or '..' in path.parts
                or hashlib.sha256(path.read_bytes()).hexdigest() != row['sha256']):
            raise ValueError('SOURCE_WORLD_TERRAIN_PIN')
        definitions = [record.get('definition', record) for record in json.loads(path.read_bytes())['records']]
        found = [definition for definition in definitions if definition['identity']['key'] == row['key']]
        if len(found) != 1 or any(found[0].get(key, {}).get('value') != row[key] for key in ['kind', 'walkable', 'ground_speed']):
            raise ValueError('SOURCE_WORLD_TERRAIN_SHAPE')
    for key in ['life_ring', 'mintwallin']:
        lever = capsule['worlds'][key]['native_lever']
        x, y, _ = lever['position']
        _, sectors = regions[lever['map']]
        levers = [(tile[0], tile[1]) for _, rows in sectors for tile in rows
                  if abs(tile[0] - x) <= 10 and abs(tile[1] - y) <= 10
                  and any(depth == 0 and palette[index]['source_item_id'] == 2772
                          for index, depth, _ in tile[5])]
        if levers != [(x, y)]:
            raise ValueError('SOURCE_WORLD_NATIVE_LEVER_AMBIGUOUS')
    definitions = {}
    for path in (root / 'content/interactions/reward_claims').glob('*.json'):
        for row in json.loads(path.read_bytes()).get('records', []):
            definition = row.get('definition', {})
            key = definition.get('identity', {}).get('key')
            if key:
                definitions.setdefault(key, []).append(definition)
    for world in capsule['worlds'].values():
        for claim in world.get('native_claims', []):
            matches = definitions.get(claim['claim'], [])
            if len(matches) != 1 or len(matches[0]['placements']) != 1:
                raise ValueError('SOURCE_WORLD_NATIVE_CLAIM_OWNER')
            placement = matches[0]['placements'][0]
            pos = placement['source_binding']['project_position']
            if (placement['source_binding'] != claim['source_binding']
                    or placement['appearance_tibia_id'] != claim['appearance']
                    or claim['native']['position'] != [pos['x'], pos['y'], pos['z']]
                    or [claim['appearance'], None] not in claim['native']['roots']):
                raise ValueError('SOURCE_WORLD_NATIVE_CLAIM_BINDING')


def position(values):
    x, y, z = values
    return dict(x=x, y=y, floor=-z)


def source_ref(source):
    prefix = f"https://raw.githubusercontent.com/zimbadev/crystalserver/{source['revision']}/"
    if not source['url'].startswith(prefix):
        raise ValueError('SOURCE_WORLD_SOURCE_REPOSITORY')
    return f"crystal:{source['revision']}:" + source['url'][len(prefix):]


def callbacks(capsule, root=ROOT):
    life = capsule['worlds']['life_ring']
    bridge = [r['position'] for r in life['bridge']]
    if (bridge != [[x, y, 10] for y in [32231, 32232] for x in [32410, 32411, 32412]]
            or life['native_lever']['position'] != [32413, 32230, 10]
            or life['eject_to'] != [32409, 32231, 10]
            or (life['source_water'], life['native_water'], life['source_bridge']) != (4610, 4598, 5770)
            or any([4598, None] not in r['roots'] for r in life['bridge'])
            or [2772, None] not in life['native_lever']['roots']):
        raise ValueError('SOURCE_WORLD_LIFE_RING_CONTRACT')
    rows = [dict(
        key='oteryn:quest-callback/life-ring-bridge', quest='oteryn:quest.life_ring_quest',
        trigger='use', selector=dict(position=position(life['native_lever']['position']),
                                     unique=None, action=None, appearance=2772),
        lever=[2772, 2773], reversible=True,
        eject_occupants=dict(positions=[position(p) for p in bridge],
                             destination=position(life['eject_to'])),
        passages=[dict(position=position(p), appearance=4598, replacement=5770) for p in bridge],
        teleport=None, storage=[], offerings=[], return_on_failure=False,
        failure_message=None, sources=[source_ref(life['source'])],
    )]
    mint = capsule['worlds']['mintwallin']
    if (mint['native_lever']['position'] != [32600, 32105, 14]
            or [row['position'] for row in mint['remove']] != [[x, 32104, 14] for x in range(32593, 32602)]
            or [row['position'] for row in mint['create']] != [[32592, y, 14] for y in [32105, 32106]]
            or mint['create_appearance'] != 1270 or mint['remove_appearance'] != 1271
            or any([1271, None] not in row['roots'] for row in mint['remove'])):
        raise ValueError('SOURCE_WORLD_MINTWALLIN_CONTRACT')
    # Reuse the concurrently qualified source callback and its object owner.
    existing = [row for row in read(root, 'content/quests/world-callbacks.json')['callbacks']
                if row['key'] == 'oteryn:quest-callback/mintwallin-cyclops-lever']
    if (len(existing) != 1 or existing[0]['selector'] != dict(position=None, unique=None, action=30008, appearance=None)
            or not existing[0]['objects']['consume_trigger']
            or len(existing[0]['objects']['mutations']) != 11):
        raise ValueError('SOURCE_WORLD_MINTWALLIN_EXISTING_OWNER')
    hidden = capsule['worlds']['hidden_threats']
    expected_positions = [[33079,32014,13], [33079,32015,13], [33080,32015,13], [33032,32050,13], [33032,32051,13]]
    if ([row['position'] for row in hidden['positions']] != expected_positions
            or hidden['source_track_owner'] != 'oteryn:quest.tibia_tales'
            or (hidden['from'], hidden['to']) != (2,3)):
        raise ValueError('SOURCE_WORLD_HIDDEN_THREATS_CONTRACT')
    rows.append(dict(
        key='oteryn:quest-transition/quest/u11_50/hidden_threats/quest_line/movement_1',
        quest=hidden['source_track_owner'], trigger='step_out',
        selector=dict(position=None, positions=[position(p) for p in expected_positions],
                      unique=None, action=None, appearance=None),
        lever=None, reversible=False, passages=[], teleport=None,
        storage=[dict(track=hidden['track'], minimum=2, maximum=2, set=3)],
        required_claims=hidden['required_once_claims'], offerings=[], return_on_failure=False,
        failure_message=None, sources=[source_ref(hidden['source'])],
    ))
    return rows


def outputs(root=ROOT):
    capsule = read(root, CAPSULE)
    if capsule['schema'] != 'OTERYN_QUEST_SOURCE_OBJECTS/v1':
        raise ValueError('SOURCE_WORLD_SCHEMA')
    verify(root, capsule)
    rows = callbacks(capsule, root)
    # Runtime bindings have their own shard. Reviewed source transcriptions and
    # the source-fix approval's immutable inputs remain byte-for-byte unchanged.
    if len(rows) != len({row['key'] for row in rows}):
        raise ValueError('SOURCE_WORLD_CALLBACK_OWNER')
    return {WORLD: dict(schema=1, callbacks=sorted(rows, key=lambda row: row['key']))}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, value in outputs().items():
        path = ROOT / name
        text = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        if args.check:
            if not path.exists() or path.read_text() != text:
                raise ValueError(f'SOURCE_WORLD_GENERATION_DRIFT: {name}')
        elif not path.exists() or path.read_text() != text:
            path.write_text(text)
    print('source-world callbacks verified' if args.check else 'source-world callbacks generated')
