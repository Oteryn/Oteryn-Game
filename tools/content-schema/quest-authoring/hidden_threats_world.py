"""Compile the exact source-key unlocks, preserving ordinary door ownership."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
CAPSULE = 'tools/content-schema/quest-authoring/samples/native-rewards/hidden-threats-doors.json'
SHA256 = '7a8b37a76cfd650eefc9bc6a4c3beeb34c5029da957c7472b94ba7d0fe579879'
OUTPUT = 'content/quests/hidden-threats-callbacks.json'


def facts(root=ROOT):
    raw = (root / CAPSULE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('HIDDEN_THREATS_DOORS_SOURCE_PIN')
    data = json.loads(raw)
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    from world_region_codec import decode_region
    index = json.loads((root / 'content/world/placements/index.json').read_bytes())
    bindings = json.loads((root / 'tools/content-schema/world-authoring/samples/quest-door-bindings.json').read_bytes())['bindings']
    for door in data['doors']:
        raw = (root / door['map_path']).read_bytes()
        if hashlib.sha256(raw).hexdigest() != door['map_sha256']:
            raise ValueError('HIDDEN_THREATS_DOORS_MAP_PIN')
        x, y, _ = door['position']
        tile, = [t for _, tiles in decode_region(raw)[3] for t in tiles if t[:2] == (x, y)]
        roots = [[index['palette'][i]['source_item_id'], depth, attrs] for i, depth, attrs in tile[-1]]
        matching = [r for r in bindings if all(r[key] == door[key] for key in ['position', 'action', 'track', 'item', 'table_line', 'storage_line'])]
        if tile[3] != 0 or roots != door['roots'] or len(matching) != 1:
            raise ValueError('HIDDEN_THREATS_DOORS_NATIVE_SHAPE')
    return data


def document(root=ROOT):
    data = facts(root)
    callbacks = []
    for door in data['doors']:
        x, y, z = door['position']
        callbacks.append(dict(
            catalogue_layer=1,
            key=f"oteryn:quest-callback/hidden-threats/works-door0{door['number']}",
            quest='oteryn:quest.tibia_tales', trigger='use',
            selector=dict(position=dict(x=x, y=y, floor=-z), unique=None, action=door['action'], appearance=None),
            lever=None, reversible=False, passages=[], teleport=None,
            storage=[dict(source_track=door['track'].replace('oteryn:', 'canary:', 1), track=door['track'], minimum=-1, maximum=1, set=1, set_if=-1)],
            offerings=[], item_on_target=dict(item=f"oteryn:item.tibia.i{data['key']}", count=1, retain=True, replacement=None, target_replacement=None),
            return_on_failure=False, failure_message=None,
            success_message='The forged key unlocks the door.',
            sources=[f"canary:{data['source']['revision']}:{data['source']['path']}"]))
    return dict(schema=1, callbacks=callbacks)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = json.dumps(document(), indent=2) + '\n'
    output = ROOT / OUTPUT
    if args.check:
        if output.read_text() != expected:
            raise SystemExit('Hidden Threats source callbacks require regeneration')
    else:
        output.write_text(expected)
    print('PASS Hidden Threats retained-key callbacks=3')


if __name__ == '__main__':
    main()
