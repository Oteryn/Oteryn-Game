"""Source-pinned cookbook row witnesses; ordinary checks need no Git history."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

BASE = 'ed788c4bb1e2b420c8bfbd7a9bd68e36ea8ebfeb'
PACKET = 'tools/content-schema/native-gameplay/quest-batch3-item-profiles.json'
RECIPE = 'content/quests/hot-cuisine-batch3.json'
WITNESS = 'tools/content-schema/native-gameplay/m42-cooking-canonical.json'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def selected(raw, keys):
    return {r['definition']['identity']['key']: digest(json.dumps(r, sort_keys=True, separators=(',', ':')).encode())
            for r in json.loads(raw)['records'] if r['definition']['identity']['key'] in keys}


def capture(root):
    raw = (root / PACKET).read_bytes()
    recipe = (root / RECIPE).read_bytes()
    recipes = json.loads(recipe)['recipes']
    keys = {i['item'] for r in recipes for i in [*r['ingredients'], r['reward']]}
    shards = {}
    for d in json.loads(raw)['canonical_inputs']:
        previous = subprocess.check_output(['git', 'show', f"{BASE}:{d['path']}"], cwd=root)
        # The descriptor pins current canonical input bytes. The immutable
        # predecessor separately pins the selected cookbook records below.
        current = (root / d['path']).read_bytes()
        if digest(current) != d['sha256']:
            raise ValueError('M42_CAPTURE_CURRENT_INPUT_PIN')
        accepted = selected(previous, keys)
        if selected(current, keys) != accepted:
            raise ValueError('M42_CAPTURE_SELECTED_CANONICAL_DRIFT')
        if accepted:
            shards[d['path']] = accepted
    if set().union(*(set(rows) for rows in shards.values())) != keys:
        raise ValueError('M42_CAPTURE_COOKBOOK_CLOSURE')
    return dict(schema='OTERYN_M42_COOKING_CANONICAL/v1', baseline=BASE,
                source_packet_sha256=digest(raw), recipe_sha256=digest(recipe), shards=shards)


def verify(root, descriptor, keys):
    witness = json.loads((root / WITNESS).read_bytes())
    if (witness['schema'] != 'OTERYN_M42_COOKING_CANONICAL/v1'
            or witness['source_packet_sha256'] != digest((root / PACKET).read_bytes())
            or witness['recipe_sha256'] != digest((root / RECIPE).read_bytes())):
        raise ValueError('M42_CANONICAL_WITNESS_INPUT_DRIFT')
    current = (root / descriptor['path']).read_bytes()
    if selected(current, keys) != witness['shards'].get(descriptor['path'], {}):
        raise ValueError('NPC_RECIPE_CANONICAL_DRIFT')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--refresh', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if args.refresh:
        (root / WITNESS).write_text(json.dumps(capture(root), sort_keys=True, indent=2) + '\n')
    else:
        recipes = json.loads((root / RECIPE).read_bytes())['recipes']
        keys = {i['item'] for r in recipes for i in [*r['ingredients'], r['reward']]}
        for descriptor in json.loads((root / PACKET).read_bytes())['canonical_inputs']:
            verify(root, descriptor, keys)
    print('PASS exact M42 cookbook canonical witnesses')
