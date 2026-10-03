"""Compile pinned SOURCE container recipes without native admission."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

LOCAL = Path(__file__).resolve().parent
M_SHA = '503bba9e9a6a96f807c40c519bd3c83ed40a9ded'
INPUT_SHA256 = '75090228200e2c12b2b5599d0ea94849d1e40d4a3ab8a49707958085e1a8ba1c'
PACKET_BLOB = '5631547afff0db9ca9f3e400d455f994aebda4df'


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n'


def indexed(rows, key):
    out = {}
    for row in rows:
        identity = key(row)
        if identity in out:
            raise ValueError('duplicate input identity or role: ' + str(identity))
        out[identity] = row
    return out


def known(field):
    return field.get('value') if isinstance(field, dict) and field.get('state') == 'KNOWN' else None


def fact(item, group, name):
    return known((known((item or {}).get('semantics', {}).get(group, {})) or {}).get(name, {}))


def positive(value):
    return {'state': 'KNOWN', 'value': value} if type(value) is int and value > 0 else {'state': 'UNKNOWN'}


def compile_snapshot(snapshot, digest):
    provenance = snapshot['provenance']
    if (snapshot['schema'], snapshot['classification'], provenance['repository'], provenance['revision'], provenance['pull_request'], provenance['upstream_candidate']) != ('OTERYN_SOURCE_CONTAINER_INPUT/v1', 'OTS_HYPOTHESIS_ONLY', 'Oteryn/Oteryn-Game', M_SHA, 1469, 'UNMERGED_SOURCE_EVIDENCE'):
        raise ValueError('unmerged source provenance changed')
    if provenance['variant_packet']['blob_sha1'] != PACKET_BLOB or len(digest) != 64:
        raise ValueError('source packet witness changed')
    indexed(provenance['item_shards'], lambda s: s['path'])
    items = indexed(snapshot['items'], lambda d: d['identity']['key'])
    witnesses = indexed([i for s in provenance['item_shards'] for i in s['definitions']], lambda i: i['key'])
    if set(items) != set(witnesses) or any(items[k]['identity'] != witnesses[k] for k in items):
        raise ValueError('Item reference inventory changed')
    fluid_evidence = snapshot['source_fluid_evidence']
    fluid_roles = indexed(fluid_evidence['roles'], lambda r: (r['source_claim']['key'], r['placement_index'], r['reward_index']))
    wiki_rows = indexed(snapshot['source_quantity_wiki_evidence']['receipts'], lambda r: r['source_claim_key'])
    originals = indexed(snapshot['records'], lambda r: r['source_claim']['identity']['key'])
    output, used_fluids = [], set()
    for key in sorted(originals):
        record = originals[key]
        if record['native_lowering']['status'] != 'WAITING_IMPLEMENTATION':
            raise ValueError('source cannot claim native admission')
        indexed(record['source_claim']['placements'], lambda p: tuple(p['position'][k] for k in ('x', 'y', 'z')))
        for role in record['item_bindings'] + record['charge_dispositions']:
            pi, qi = role['placement_index'], role['reward_index']
            if type(pi) is not int or pi < 0 or (qi is not None and (type(qi) is not int or qi < 0)):
                raise ValueError('invalid source role index')
        bindings = indexed(record['item_bindings'], lambda b: (b['placement_index'], b['reward_field'], b['reward_index']))
        charges = indexed(record['charge_dispositions'], lambda c: (c['placement_index'], c['reward_field'], c['reward_index']))
        expected = set()
        for pi, p in enumerate(record['source_claim']['placements']):
            reward = p['reward']
            for field in ('items', 'random_one_of'):
                expected.update((pi, field, qi) for qi in range(len(reward.get(field, []))))
            expected.add((pi, 'container', None)) if 'container' in reward else None
            expected.add((pi, 'written_text', None)) if reward.get('written_text', {}).get('item') else None
        if set(bindings) != expected or any(c not in expected for c in charges):
            raise ValueError('source role inventory changed')
        for (pi, field, qi), binding in bindings.items():
            source = record['source_claim']['placements'][pi]['reward'][field]
            source = source[qi]['item'] if qi is not None else source['item'] if field == 'written_text' else source
            ref = binding['canonical_item']
            if binding['source_item'] != source or (ref is not None and (items.get(ref['key'], {}).get('identity') != ref or ref['family'] != 'Item' or ref['key'] != 'oteryn:item.tibia.i' + source['key'].split(':item/', 1)[1])):
                raise ValueError('source or canonical reference changed')
        placements = []
        for pi, placement in enumerate(record['source_claim']['placements']):
            source = placement['reward']
            if 'container' not in source:
                continue
            root = bindings[(pi, 'container', None)]['canonical_item']
            definition = items.get(root['key']) if root else None
            capacity = fact(definition, 'container', 'capacity')
            capacity_known = positive(capacity)['state'] == 'KNOWN'
            gaps, constraints, entries, children = set(), [], [], []
            def native_facts(ref, item):
                if item and item.get('materializable') is not True:
                    constraints.append({'kind': 'ITEM_NOT_MATERIALIZABLE', 'item': copy.deepcopy(ref)})
                own_capacity = fact(item, 'container', 'capacity')
                if type(own_capacity) is int and own_capacity > 20:
                    constraints.append({'kind': 'CONTAINER_CAPACITY_ABOVE_ADMITTED_MAXIMUM', 'value': own_capacity, 'maximum': 20, 'item': copy.deepcopy(ref)})
            native_facts(root, definition)
            if root is None: gaps.add('ITEM_REFERENCE_UNKNOWN')
            if not capacity_known: gaps.add('CONTAINER_CAPACITY_UNKNOWN')
            if source.get('random_one_of'): gaps.add('CONTAINER_RANDOM_RECIPE_UNRESOLVED')
            for qi, quantity in enumerate(source['items']):
                if set(quantity) != {'item', 'count'}: raise ValueError('uncovered nested source role')
                binding = bindings[(pi, 'items', qi)]
                ref, raw = binding['canonical_item'], quantity['count']
                item = items.get(ref['key']) if ref else None
                if type(raw) is not int or raw < 1 or binding['argument']['raw_count_argument'] != raw:
                    raise ValueError('source argument changed')
                native_facts(ref, item)
                charge = charges.get((pi, 'items', qi))
                amount, executed, basis, fluid = raw, raw, 'SOURCE_LITERAL', None
                role_id = (key, pi, qi)
                if role_id in fluid_roles:
                    proof = fluid_roles[role_id]
                    if proof['source_raw_argument'] != raw or proof['source_fluid'] != {'subtype': 1, 'symbol': 'WATER'} or ref['key'] != 'oteryn:item.tibia.i' + str(proof['item_id']) or proof['project_position'] != placement['position']:
                        raise ValueError('fluid witness or carrier changed')
                    used_fluids.add(role_id)
                    amount, executed, basis, fluid = 1, 1, 'SOURCE_FLUID_SUBTYPE', copy.deepcopy(proof['source_fluid'])
                    constraints.append({'kind': 'FLUID_INSTANCE_NATIVE_ADMISSION_UNRESOLVED', 'item': copy.deepcopy(ref)})
                elif binding['argument']['kind'] == 'CHARGES_SUBTYPE':
                    if charge is None or charge['source_raw_argument'] != raw: raise ValueError('charge witness missing')
                    default = fact(item, 'charges', 'count')
                    amount, executed, basis = 1, 1, 'SOURCE_CHARGES_SUBTYPE'
                    if charge['definition_charges'] != positive(default) or charge['normalized_quantity'] != positive(1 if raw == default else None):
                        raise ValueError('charge disposition changed')
                    if raw != default: gaps.add('SOURCE_CHARGE_MISMATCH' if positive(default)['state'] == 'KNOWN' else 'CHARGE_DEFINITION_UNKNOWN')
                elif charge is not None or ((item or {}).get('stack_class') == 'NonStackable' and fact(item, 'charges', 'count') is not None):
                    raise ValueError('charged definition lacks source disposition')
                stack = (item or {}).get('stack_class')
                proven_max = fact(item, 'stack', 'stack_max')
                maximum = 1 if stack == 'NonStackable' else (100 if proven_max is None else proven_max) if stack == 'StackCapable' else None
                if type(maximum) is not int or not 1 <= maximum <= 100:
                    gaps.add('ITEM_STACK_SEMANTICS_UNKNOWN'); amount = None
                elif stack == 'StackCapable' and raw > maximum:
                    executed = maximum
                    wiki = wiki_rows.get(key, {})
                    if wiki.get('item_id_from_source') == int(ref['key'].rsplit('i', 1)[1]) and wiki.get('target_quantity') == raw and wiki.get('source_recipe_corroboration') == 'EXPLICIT_WIKI_QUANTITY_200':
                        basis = 'SOURCE_LITERAL_CORROBORATED_BY_PINNED_WIKI'
                    else:
                        gaps.add('SOURCE_STACK_COUNT_CLAMP_CONFLICT'); amount = None; basis = 'CONFLICTING_SOURCE_REWARDS'
                elif stack == 'NonStackable' and raw > 1 and charge is None and fluid is None:
                    gaps.add('SOURCE_NONSTACK_COUNT_CONFLICT'); amount = None; executed = 1
                if ref is None: gaps.add('ITEM_REFERENCE_UNKNOWN'); amount = None
                entry = {'source_reward_index': qi, 'item': copy.deepcopy(ref), 'source_raw_argument': raw, 'source_executed_quantity': positive(executed), 'quantity_basis': basis, 'normalized_quantity': positive(amount)}
                if fluid is not None: entry['source_fluid'] = fluid
                entries.append(entry)
                if amount is None: continue
                if len(children) + (amount + maximum - 1) // maximum > 499:
                    gaps.add('RECIPE_EXCEEDS_COMPILER_BOUND'); continue
                while amount:
                    count = min(amount, maximum)
                    child = {'item': copy.deepcopy(ref), 'count': count}
                    if positive(fact(item, 'container', 'capacity'))['state'] == 'KNOWN': child['contents'] = []
                    if fluid is not None: child['source_fluid'] = copy.deepcopy(fluid)
                    children.append(child); amount -= count
            if capacity_known and len(children) > capacity: gaps.add('CONTENTS_EXCEED_DEFINITION_CAPACITY')
            if len(children) > 20: constraints.append({'kind': 'CONTAINER_ENTRIES_ABOVE_ADMITTED_MAXIMUM', 'value': len(children), 'maximum': 20})
            tree = {'state': 'UNKNOWN'} if gaps else {'state': 'KNOWN', 'value': {'item': copy.deepcopy(root), 'count': 1, 'contents': children}}
            status = 'CONFLICT' if gaps & {'SOURCE_CHARGE_MISMATCH', 'CONTENTS_EXCEED_DEFINITION_CAPACITY', 'SOURCE_STACK_COUNT_CLAMP_CONFLICT', 'SOURCE_NONSTACK_COUNT_CONFLICT'} else 'WAITING_DATA' if gaps else 'AUTHORED'
            placements.append({'placement_index': pi, 'source_position': copy.deepcopy(placement['position']), 'container': copy.deepcopy(root), 'definition_capacity': positive(capacity), 'entries': entries, 'item_tree': tree, 'data_status': status, 'source_checks': sorted(gaps), 'native_constraints': sorted(constraints, key=lambda h: json.dumps(h, sort_keys=True))})
        if not placements: raise ValueError('non-container record in snapshot')
        output.append({'original_record': copy.deepcopy(record), 'placements': placements, 'native_lowering': copy.deepcopy(record['native_lowering'])})
    if used_fluids != set(fluid_roles): raise ValueError('fluid source role inventory changed')
    rows = [p for r in output for p in r['placements']]
    return {'schema': 'OTERYN_SOURCE_CONTAINER_COMPILED/v1', 'classification': 'OTS_HYPOTHESIS_ONLY', 'scope': 'SOURCE_RECIPES_ONLY_NO_NATIVE_ADMISSION', 'input_sha256': digest, 'provenance': copy.deepcopy(provenance), 'source_fluid_evidence': copy.deepcopy(fluid_evidence), 'source_quantity_wiki_evidence': copy.deepcopy(snapshot['source_quantity_wiki_evidence']), 'records': output, 'summary': {'records': len(output), 'placements': len(rows), 'logical_roles': sum(len(p['entries']) for p in rows), **{s: sum(p['data_status'] == s for p in rows) for s in ('AUTHORED', 'WAITING_DATA', 'CONFLICT')}}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, default=LOCAL / 'samples/container-source/input.json')
    parser.add_argument('--output', type=Path, default=LOCAL / 'samples/container-source/compiled.json')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--verify-upstream', action='store_true')
    args = parser.parse_args()
    raw = args.input.read_bytes(); digest = hashlib.sha256(raw).hexdigest()
    if digest != INPUT_SHA256: raise ValueError('immutable input bytes differ from pinned snapshot')
    snapshot = json.loads(raw)
    if args.verify_upstream:
        from container_source_verify import verify_sources
        verify_sources(snapshot)
    result = compile_snapshot(snapshot, digest)
    import jsonschema
    jsonschema.Draft202012Validator(json.loads((LOCAL / 'container_source.schema.json').read_text())).validate(result)
    text = encoded(result)
    if args.check:
        if args.output.read_text(encoding='utf-8') != text: raise ValueError('compiled output differs from full offline recomputation')
    else: args.output.write_text(text, encoding='utf-8')


if __name__ == '__main__': main()
