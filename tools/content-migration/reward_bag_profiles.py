"""REWARD-CHEST-WIRE-1: reward bag lists and their native Item profiles.

Canary 47dfd51f `data/scripts/actions/items/reward_bags.lua` owns the lists and the
relative weights; Crystal ff7ede59 carries the same bag ids and lists without weights
(a uniform draw), so the id lists must match exactly. Weights are scaled to integers
(1e-5 units) and must convert exactly. BAG_YOU_DESIRE is a Soul War reward: its list is
recorded, but its Items are not admitted here and the bag stays unplayable.
"""
import argparse
import hashlib
import json
import re
import sys
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
from lower_equip_abilities_packet import canary_modifiers, canary_points
from lower_elemental_magic_modifier_packet import ALIASES

PATH = 'tools/content-schema/native-gameplay/reward-bag-profiles.json'
SCRIPT = 'data/scripts/actions/items/reward_bags.lua'
CONSTANTS = 'data/libs/functions/constants.lua'
BAGS = ('BAG_YOU_DESIRE', 'PRIMAL_BAG', 'BAG_YOU_COVET')
EXCLUDED = {'BAG_YOU_DESIRE': 'SOUL_WAR_LANE'}
WEIGHT_SCALE = Decimal(100000)
ELEMENTAL_MODIFIER_POINTS = {name.replace(' magic level', 'magiclevelpoints'): kind
                            for name, kind in ALIASES.items()}


def bag_lists(raw):
    """Each `[CONSTANT] = { { id = N, name = "...", weight = W }, ... }` block, in source order."""
    text = raw.decode()
    result = {}
    for match in re.finditer(r'\[(\w+)\] = \{\n(.*?)\n\t\},', text, re.S):
        line = text.count('\n', 0, match.start()) + 1
        rows = []
        for row in re.finditer(r'\{ id = (\d+), name = "([^"]+)"(?:, weight = ([0-9.]+))? \}', match[2]):
            rows.append({'id': int(row[1]), 'name': row[2], 'weight': row[3]})
        if len(rows) != match[2].count('{ id ='):
            raise ValueError('REWARD_BAG_ROW_SHAPE')
        result[match[1]] = (line, rows)
    if tuple(result) != BAGS:
        raise ValueError('REWARD_BAG_SET')
    return result


def constants(raw):
    found = {m[1]: int(m[2]) for m in re.finditer(r'^(\w+) = (\d+)$', raw.decode(), re.M)}
    return {name: found[name] for name in BAGS}


def key(i):
    return f'oteryn:item.tibia.i{i}'


def capture(canary, crystal):
    script = (canary / SCRIPT).read_bytes()
    other_script = (crystal / SCRIPT).read_bytes()
    canary_constants, crystal_constants = (canary / CONSTANTS).read_bytes(), (crystal / CONSTANTS).read_bytes()
    xml_raw = (canary / 'data/items/items.xml').read_bytes()
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    lists, other = bag_lists(script), bag_lists(other_script)
    ids = constants(canary_constants)
    if ids != constants(crystal_constants):
        raise ValueError('REWARD_BAG_CONSTANT_DRIFT')
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    candidates = {ids[name] for name in BAGS if name not in EXCLUDED} | {
        r['id'] for name in BAGS if name not in EXCLUDED for r in lists[name][1]}
    physical = {r['server_item_id']: r for r in source.policies(appearance_raw, xml_raw, candidates)}
    bindings = binding_index(BINDINGS.read_bytes())
    profiles = {i: source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(candidates)}
    for i, record in profiles.items():
        attrs = source.attrs(xml[i])
        # Reuse the equipment lowering's signed points, suppressions and whole-group
        # holds. Its elemental successor also supports these seven magic bonuses.
        elemental = {k: attrs[k] for k in ELEMENTAL_MODIFIER_POINTS if k in attrs}
        _, modifiers = canary_modifiers({k: v for k, v in attrs.items() if k not in elemental})
        if modifiers is None and elemental:
            modifiers = []
        if not isinstance(modifiers, list):
            continue
        for attr_name, raw in elemental.items():
            points = canary_points(raw)
            if points is None:
                break
            modifiers.append({'kind': ELEMENTAL_MODIFIER_POINTS[attr_name],
                              'parameter': source.K({'kind': 'SIGNED_POINTS', 'value': points}),
                              'evaluation_phase': source.U, 'priority': source.U, 'target_domain': source.U})
        else:
            record['semantics']['skill_modifiers'] = source.K({
                'modifiers': source.K(sorted(modifiers, key=lambda m: m['kind']))})

    def leaf(i):
        # The CRAFT-1 owner mints exactly one non-stackable, non-container leaf per Use.
        record = profiles[i]
        return (record['admission']['stack_class'] == 'NonStackable'
                and record['semantics']['container'] == source.NA
                and 'CharacterInventory' in record['admission']['legal_destinations'])

    bags = []
    admitted = set()
    for name in BAGS:
        line, rows = lists[name]
        if [(r['id'], r['name']) for r in rows] != [(r['id'], r['name']) for r in other[name][1]]:
            raise ValueError('REWARD_BAG_LIST_DRIFT')
        if any(r['weight'] is not None for r in other[name][1]) or any(r['weight'] is None for r in rows):
            raise ValueError('REWARD_BAG_WEIGHT_SHAPE')
        rewards = []
        for row in rows:
            weight = Decimal(row['weight']) * WEIGHT_SCALE
            if weight != weight.to_integral_value() or weight <= 0:
                raise ValueError('REWARD_BAG_WEIGHT_SCALE')
            rewards.append({'item': key(row['id']), 'name': row['name'], 'weight': int(weight)})
        reason = EXCLUDED.get(name)
        if reason is None:
            blocked = [r['id'] for r in rows if not leaf(r['id'])]
            if blocked:
                reason = 'NON_LEAF_REWARD:' + ','.join(map(str, blocked))
        if reason is None:
            admitted |= {ids[name]} | {r['id'] for r in rows}
        bags.append({'constant': name, 'bag': key(ids[name]), 'source_line': line,
                     'crystal_source_line': other[name][0], 'playable': reason is None,
                     'excluded_reason': reason, 'rewards': rewards})
    records = [profiles[i] for i in sorted(admitted)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, admitted)
    return {'schema': 'OTERYN_NATIVE_REWARD_BAG_PROFILES/v1',
            'producer': 'tools/content-migration/reward_bag_profiles.py',
            'weight_scale': int(WEIGHT_SCALE),
            'sources': [{'reference': name, 'path': path, 'sha256': hashlib.sha256(data).hexdigest()}
                        for name, path, data in (
                            ('canary-47dfd51f', SCRIPT, script),
                            ('canary-47dfd51f', CONSTANTS, canary_constants),
                            ('canary-47dfd51f', 'data/items/items.xml', xml_raw),
                            ('canary-47dfd51f', 'data/items/appearances.dat', appearance_raw),
                            ('crystal-ff7ede59', SCRIPT, other_script),
                            ('crystal-ff7ede59', CONSTANTS, crystal_constants))],
            'bags': bags,
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    expected = sorted({b['bag'] for b in overlay['bags'] if b['playable']}
                      | {r['item'] for b in overlay['bags'] if b['playable'] for r in b['rewards']},
                      key=lambda k: int(k.rsplit('i', 1)[1]))
    if (overlay['schema'] != 'OTERYN_NATIVE_REWARD_BAG_PROFILES/v1'
            or [r['production_definition']['production_key'] for r in records] != expected):
        raise ValueError('REWARD_BAG_PROFILE_SET')
    document = json.loads(generated[profile_path])
    if set(expected) & {r['production_definition']['production_key'] for r in document['records']}:
        raise ValueError('REWARD_BAG_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('REWARD_BAG_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
