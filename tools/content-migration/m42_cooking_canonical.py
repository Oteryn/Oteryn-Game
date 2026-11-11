"""Source-pinned cookbook row witnesses; ordinary checks need no Git history."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

BASE = '856a420d64cc0ea5c9e8cb2a54742b45c49b9914'
PACKET_BASE = 'f01f18582443f482b7adcfaf32cae3f9d3616876'
PACKET = 'tools/content-schema/native-gameplay/quest-batch3-item-profiles.json'
RECIPE = 'content/quests/hot-cuisine-batch3.json'
AUTHORED = 'tools/content-schema/quest-authoring/batch3_a/native/hot_cuisine.json'
WITNESS = 'tools/content-schema/native-gameplay/m42-cooking-canonical.json'
REFINER_INPUTS = (
    'docs/agents/evidence/OTV2-20261001-quest-reward-item-admission.json',
    'docs/agents/evidence/OTV2-20261001-quest-interaction-item-admission.json',
)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def selected(raw, keys):
    return {r['definition']['identity']['key']: digest(json.dumps(r, sort_keys=True, separators=(',', ':')).encode())
            for r in json.loads(raw)['records'] if r['definition']['identity']['key'] in keys}


def accepted_packet(root):
    return subprocess.check_output(['git', 'show', f'{PACKET_BASE}:{PACKET}'], cwd=root)


def require_refiner_inputs(root, packet):
    pins = {entry['path']: entry['sha256'] for entry in packet['accepted_inputs']}
    # These retained admission packets are source inputs, unlike the separately
    # generated native provider also named by accepted_inputs.
    for path in REFINER_INPUTS:
        if digest((root / path).read_bytes()) != pins[path]:
            raise ValueError('M42_CAPTURE_REFINER_INPUT:' + path)


def qualified_selected(root, raw, keys):
    rows = {}
    for original in json.loads(raw)['records']:
        key = original['definition']['identity']['key']
        if key not in keys:
            continue
        row = copy.deepcopy(original)
        if key == 'oteryn:item.tibia.i2874':
            # Replay only this accepted empty-fluid refinement on the fixed
            # historical row. The recipe's instance-subtype hold stays sealed.
            require_refiner_inputs(root, json.loads(accepted_packet(root)))
            import quest_reward_item_semantics as reward
            reward.enrich(row['definition'], *reward.load_admissions(root)[key])
        elif key == 'oteryn:item.tibia.i3738':
            import grizzly_shop_profiles as shop
            import native_goal_loot_items as loot
            shop_keys = {entry['production_definition']['production_key']
                         for entry in json.loads((root / shop.PATH).read_bytes())['records']}
            if 3738 not in loot.selected(root) or key in shop_keys:
                raise ValueError('M42_CAPTURE_NATIVE_LOOT_SOURCE_SCOPE')
            loot.apply_admissions([row['definition']], root)
        rows[key] = digest(json.dumps(row, sort_keys=True, separators=(',', ':')).encode())
    return rows


def capture(root, *, packet_bytes=None):
    raw = (root / PACKET).read_bytes() if packet_bytes is None else packet_bytes
    recipe = (root / RECIPE).read_bytes()
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
        accepted = qualified_selected(root, previous, keys)
        if selected(current, keys) != accepted:
            raise ValueError('M42_CAPTURE_SELECTED_CANONICAL_DRIFT')
        if accepted:
            shards[d['path']] = accepted
    if set().union(*(set(rows) for rows in shards.values())) != keys:
        raise ValueError('M42_CAPTURE_COOKBOOK_CLOSURE')
    return dict(schema='OTERYN_M42_COOKING_CANONICAL/v1', baseline=BASE,
                source_packet_sha256=digest(raw), recipe_sha256=digest(recipe), shards=shards)


def refresh(root):
    from grizzly_shop_profiles import encode, plan_canonical_input_pins
    from source_relocations import read_pinned

    previous = accepted_packet(root)
    packet = json.loads(previous)
    current = json.loads((root / PACKET).read_bytes())
    sealed = copy.deepcopy(packet)
    if len(current['canonical_inputs']) != len(sealed['canonical_inputs']):
        raise ValueError('M42_CAPTURE_PACKET_SEAL')
    for expected, observed in zip(sealed['canonical_inputs'], current['canonical_inputs']):
        pin = observed.get('sha256')
        if (observed.get('path') != expected['path'] or not isinstance(pin, str)
                or len(pin) != 64 or any(c not in '0123456789abcdef' for c in pin)):
            raise ValueError('M42_CAPTURE_PACKET_SEAL')
        expected['sha256'] = pin
    if encode(sealed) != (root / PACKET).read_bytes():
        raise ValueError('M42_CAPTURE_PACKET_SEAL')

    recipe = (root / RECIPE).read_bytes()
    authored = [entry for entry in packet['authoring_inputs'] if entry['path'] == AUTHORED]
    if (len(authored) != 1 or recipe != (root / AUTHORED).read_bytes()
            or digest(recipe) != authored[0]['sha256']):
        raise ValueError('M42_CAPTURE_RECIPE')
    binding = packet['binding_input']
    if read_pinned(root, binding['path'], binding['sha256']) is None:
        raise ValueError('M42_CAPTURE_BINDING')
    require_refiner_inputs(root, packet)

    # Always start from the immutable packet's pins, including on a second run.
    # A current matching pin must not bypass the full classification row proof.
    planned = plan_canonical_input_pins(
        root, PACKET_BASE, packet_path=PACKET, packet_bytes=previous,
        keys={entry['key'] for entry in packet['classification']})
    witness = capture(root, packet_bytes=planned)
    (root / PACKET).write_bytes(planned)
    (root / WITNESS).write_text(json.dumps(witness, sort_keys=True, indent=2) + '\n')


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
        refresh(root)
    else:
        recipes = json.loads((root / RECIPE).read_bytes())['recipes']
        keys = {i['item'] for r in recipes for i in [*r['ingredients'], r['reward']]}
        for descriptor in json.loads((root / PACKET).read_bytes())['canonical_inputs']:
            verify(root, descriptor, keys)
    print('PASS exact M42 cookbook canonical witnesses')
