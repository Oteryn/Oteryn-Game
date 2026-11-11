"""Source-pinned cookbook row witnesses; ordinary checks need no Git history."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

# Accepted main includes the independently proved vial and sling-herb owners;
# every complete cooking row still has to equal this immutable predecessor.
BASE = 'f01f18582443f482b7adcfaf32cae3f9d3616876'
PACKET = 'tools/content-schema/native-gameplay/quest-batch3-item-profiles.json'
RECIPE = 'content/quests/hot-cuisine-batch3.json'
RECIPE_SOURCE = 'tools/content-schema/quest-authoring/batch3_a/native/hot_cuisine.json'
WITNESS = 'tools/content-schema/native-gameplay/m42-cooking-canonical.json'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def selected(raw, keys):
    return {r['definition']['identity']['key']: digest(json.dumps(r, sort_keys=True, separators=(',', ':')).encode())
            for r in json.loads(raw)['records'] if r['definition']['identity']['key'] in keys}


def capture(root):
    raw = (root / PACKET).read_bytes()
    recipe = (root / RECIPE).read_bytes()
    return _capture(root, raw, recipe)


def _capture(root, raw, recipe):
    recipes = json.loads(recipe)['recipes']
    keys = {i['item'] for r in recipes for i in [*r['ingredients'], r['reward']]}
    shards = {}
    for d in json.loads(raw)['canonical_inputs']:
        previous = subprocess.check_output(['git', 'show', f"{BASE}:{d['path']}"], cwd=root)
        current = (root / d['path']).read_bytes()
        # The packet names today's whole shard. The immutable predecessor
        # independently proves only the recipe rows consumed by this witness.
        if digest(current) != d['sha256']:
            raise ValueError('M42_CAPTURE_CURRENT_PIN')
        accepted = selected(previous, keys)
        if selected(current, keys) != accepted:
            raise ValueError('M42_CAPTURE_SELECTED_CANONICAL_DRIFT')
        if accepted:
            shards[d['path']] = accepted
    if set().union(*(set(rows) for rows in shards.values())) != keys:
        raise ValueError('M42_CAPTURE_COOKBOOK_CLOSURE')
    return dict(schema='OTERYN_M42_COOKING_CANONICAL/v1', baseline=BASE,
                source_packet_sha256=digest(raw), recipe_sha256=digest(recipe), shards=shards)


def refresh(root):
    """Refresh derived pins only after proving the entire accepted source packet."""
    import grizzly_shop_profiles as grizzly

    def previous(path):
        return subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=root)

    baseline = json.loads(previous(PACKET))
    packet = json.loads((root / PACKET).read_bytes())
    recipe = (root / RECIPE).read_bytes()
    if recipe != previous(RECIPE) or recipe != (root / RECIPE_SOURCE).read_bytes():
        raise ValueError('M42_REFRESH_RECIPE_PROVENANCE')
    keys = {item['item'] for row in json.loads(recipe)['recipes']
            for item in [*row['ingredients'], row['reward']]}
    if len(keys) != 72:
        raise ValueError('M42_REFRESH_COOKBOOK_CLOSURE')
    expected = baseline['canonical_inputs']
    descriptors = packet.get('canonical_inputs')
    if (not isinstance(descriptors, list) or len(descriptors) != len(expected)
            or [row.get('path') for row in descriptors] != [row['path'] for row in expected]
            or len({row['path'] for row in expected}) != len(expected)):
        raise ValueError('M42_REFRESH_CANONICAL_PATH_SET')
    normalized = copy.deepcopy(packet)
    for row, original in zip(normalized['canonical_inputs'], expected):
        row['sha256'] = original['sha256']
    # This covers source revisions, classifications, all profiles and their
    # holds, authoring/binding pins, and non-cooking scope without recapture.
    if normalized != baseline:
        raise ValueError('M42_REFRESH_PACKET_PROVENANCE')
    planned = copy.deepcopy(packet)
    for descriptor, original, replacement in zip(descriptors, expected, planned['canonical_inputs']):
        path = Path(descriptor['path'])
        if (path.is_absolute() or '..' in path.parts
                or not (root / path).resolve().is_relative_to(root.resolve())):
            raise ValueError('M42_REFRESH_CANONICAL_PATH_SET')
        before = previous(descriptor['path'])
        current = (root / path).read_bytes()
        if digest(before) != original['sha256']:
            raise ValueError('M42_REFRESH_BASE_PIN')
        # A replay may carry this exact current shard's pin, but an arbitrary
        # intermediate pin is not an accepted predecessor.
        if descriptor.get('sha256') not in (original['sha256'], digest(current)):
            raise ValueError('M42_REFRESH_CURRENT_PIN')
        def rows(raw):
            return [row for row in json.loads(raw)['records']
                    if row['definition']['identity']['key'] in keys]
        if rows(before) != rows(current):
            raise ValueError('M42_REFRESH_SELECTED_CANONICAL_DRIFT:' + descriptor['path'])
        replacement['sha256'] = digest(current)
    # Complete every provenance, row and closure check before either write.
    # The shared helper retains its independent old-pin/selected-row guards.
    captured = _capture(root, grizzly.encode(planned), recipe)
    witness = json.dumps(captured, sort_keys=True, indent=2) + '\n'
    grizzly.refresh_canonical_input_pins(root, BASE, packet_path=PACKET, keys=keys)
    (root / WITNESS).write_text(witness)


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
