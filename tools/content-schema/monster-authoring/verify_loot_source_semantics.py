"""Independent D1/D15/D22/D32 loot value evidence; no runtime or Item admission.

Uses the source evaluator only to read Lua declarations. Probability/count/identity
expectations are computed independently of Converter, Mapper and source coverage.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

from canary_batch import blob_id, load_monster

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from client_map_reader import fields

SOURCES = {'opentibiabr/canary': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
           'zimbadev/crystalserver': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bundle_digest(directory):
    digest = hashlib.sha256()
    for name in ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'):
        data = (directory / name).read_bytes()
        digest.update(f'{name}\0{len(data)}\0'.encode('ascii') + data)
    return digest.hexdigest()


def half_even(numerator, denominator):
    if numerator < 0 or denominator <= 0:
        raise ValueError('probability ratio must be nonnegative with positive denominator')
    quotient, remainder = divmod(numerator, denominator)
    return quotient + (2 * remainder > denominator or (2 * remainder == denominator and quotient % 2 == 1))


def exact_ppm(percent):
    value = Fraction(percent) * 10000
    if value.denominator != 1 or not 0 <= value <= 1000000:
        raise ValueError('D1 probability is outside the exact 1ppm grid')
    return value.numerator


def source_values(entry):
    low, high = entry.get('minCount', 1), entry.get('maxCount', 1)
    if not isinstance(low, int) or not isinstance(high, int) or low < 0 or high < max(low, 1):
        raise ValueError('unsupported source count range')
    probability = Fraction(min(int(entry.get('chance', 0)), 100000) * 10)
    if low == 0:
        probability *= Fraction(high, high + 1)
        low = 1
    return low, high, half_even(probability.numerator, probability.denominator)


def wiki_values(source, chance, kills):
    low, high, probability = source_values(source)
    if (not chance or chance.get('status') not in ('CONSISTENT', 'DIFF')
            or chance.get('confidence') != 'estimate' or chance.get('times', 0) < 10):
        return low, high, probability, 'SOURCE_KEPT_' + (chance or {}).get('status', 'NO_STATISTICS')
    if kills <= 0 or not 0 <= chance['times'] <= kills:
        raise ValueError('invalid wiki observations')
    observed = [int(value) for value in chance['wiki_amount'].split('-')]
    observed_low, observed_high = observed[0], observed[-1]
    if observed_low < low or observed_high > high:
        low, high = observed_low, observed_high
    probability = half_even(chance['times'] * 1000000, kills)
    return low, high, probability, 'APPROVED_WIKI_ESTIMATE'


def item_table(checkout):
    names = {}
    for number, wire, appearance in fields((checkout / 'data/items/appearances.dat').read_bytes()):
        if number != 1 or wire != 2:
            continue
        values = {key: value for key, _, value in fields(appearance) if key in (1, 4)}
        if values.get(4):
            names[values[1]] = values[4].decode('utf-8', 'replace')
    attributes = {}
    for node in ET.parse(checkout / 'data/items/items.xml').getroot().iter('item'):
        ids = [int(node.get('id'))] if node.get('id') else range(int(node.get('fromid')), int(node.get('toid')) + 1)
        for number in ids:
            if node.get('name'):
                names[number] = node.get('name')
            attributes[number] = {a.get('key').lower(): a.get('value') for a in node.findall('attribute')}
    by_name = defaultdict(set)
    for number, name in names.items():
        by_name[name.lower()].add(number)
    return names, by_name, attributes


def source_item(entry, by_name):
    # The donor registrar gives a declared name priority over an accompanying id.
    if 'name' in entry:
        candidates = by_name.get(entry['name'].lower(), set())
        if len(candidates) != 1:
            raise ValueError('source item name is not unique')
        return next(iter(candidates))
    number = entry.get('id')
    if not isinstance(number, int) or number < 1:
        raise ValueError('source ItemID is absent or invalid')
    return number


def wiki_items(item, title, names, by_name, attributes):
    page = item.get('item_page') or {}
    variants = page.get('variants', [])
    chosen = [v for v in variants if title.lower() in v.get('dropped_by', [])]
    if variants:
        pages = chosen if len(chosen) == 1 else variants
        ids = {number for page in pages for number in page['item_ids']}
        if len(chosen) == 1:
            return ids
    else:
        ids = set(page.get('item_ids', []))
    candidates = set(by_name.get(item['name'].lower(), set()))
    narrowed = candidates & ids
    if len(narrowed) == 1:
        return narrowed
    if len(candidates) == 1 and (not ids or candidates <= ids):
        return candidates
    if len(ids) == 1 and ids <= names.keys():
        return ids
    pool = narrowed or candidates
    equipped = {int(attributes[number]['transformequipto']) for number in pool
                if 'transformequipto' in attributes.get(number, {})}
    if len(pool - equipped) == 1:
        return pool - equipped
    if ids and ids <= names.keys() and (not candidates or candidates == ids or variants):
        return ids
    raise ValueError('wiki ItemID remains ambiguous or unavailable')


def native_projection(stage, identity_map, rekeys, source_binding, entries):
    """Compare explicit source bindings and the qualified caller's Item map, never item names."""
    mapping = {row['source_item_id']: (row['native_key'], row['native_revision']) for row in identity_map['records']}
    for rekey in rekeys:
        fact = rekey['source_identity']
        if mapping[fact['source_item_id']][0] != fact['current_native_key']:
            raise ValueError('protected rekey does not match the supplied Item map')
        mapping[fact['source_item_id']] = fact['target_native_key'], fact['target_revision']
    identity = lambda ref: (ref.get('family'), ref.get('key'), ref.get('revision'))
    targets = {identity(row['target']) for row in stage['source_identity_bindings']
               if all(row.get(field) == source_binding[field] for field in
                      ('source_key', 'source_revision', 'identity_namespace', 'external_id'))
               and row['disposition'] == 'EXACT' and row['target']['family'] == 'Creature'}
    if len(targets) != 1:
        return {'status': 'UNKNOWN_NOT_ADMITTED_OR_UNBOUND_SOURCE_ROOT'}
    records = defaultdict(list)
    for row in stage['records']:
        records[identity(row['identity'])].append(row)
    creatures = records[next(iter(targets))]
    if len(creatures) != 1 or creatures[0].get('kind') != 'Creature':
        return {'status': 'MISMATCH_CREATURE_IDENTITY'}
    creature = creatures[0]
    loot_ref = creature.get('loot') or {}
    loot_records = records[identity(loot_ref)]
    if (loot_ref.get('family') != 'Loot' or len(loot_records) != 1
            or loot_records[0].get('kind') != 'Loot'):
        return {'status': 'UNKNOWN_NO_NATIVE_LOOT_RECORD'}
    loot = loot_records[0]
    profiles = [row for row in stage['authoring_profiles']
                if row['data']['kind'] == 'Loot' and row['target']['key'] == loot_ref['key']]
    if profiles and (len(profiles) != 1 or identity(profiles[0]['target']) != identity(loot_ref)):
        return {'status': 'MISMATCH_LOOT_PROFILE_IDENTITY'}
    skips = (profiles[0]['data']['profile']['entries'] if profiles else
             [{'skip_later_same_item_after_success': False}] * len(entries))
    if len(entries) != len(loot['entries']) or len(skips) != len(entries):
        return {'status': 'MISMATCH_ENTRY_COUNT'}
    valid, missing = True, set()
    for index, (authored, native) in enumerate(zip(entries, loot['entries'])):
        number = int(authored['item']['key'].rsplit('/', 1)[1])
        if number not in mapping:
            missing.add(number)
        else:
            key, revision = mapping[number]
            valid &= native['item'] == {'family': 'Item', 'key': key, 'revision': revision}
        valid &= (native['probability_ppm'] == exact_ppm(authored['probability_percent'])
                  and (native['min_count'], native['max_count']) == (authored['min_count'], authored['max_count'])
                  and skips[index]['skip_later_same_item_after_success'] == authored['skip_later_same_item_after_success'])
    status = 'MISMATCH_NATIVE_PROJECTION' if not valid else (
        'UNKNOWN_ITEMS_ABSENT_FROM_CALLER_MAP' if missing else 'PROVEN_CALLER_ITEM_MAP_AND_SOURCE_BINDING_PROJECTION')
    return {'status': status, 'native_loot_key': loot['identity']['key'], 'entries': len(entries),
            'item_ids_absent_from_caller_map': sorted(missing)}


def verify(args):
    inputs = {str(path): sha(path) for path in (args.index, args.wiki_canary, args.wiki_crystal)}
    census = {row['monster']: row for row in json.loads(args.index.read_text())['monsters']}
    captures = {repository: {row['monster']: row for row in json.loads(path.read_text())['monsters']}
                for repository, path in [('opentibiabr/canary', args.wiki_canary), ('zimbadev/crystalserver', args.wiki_crystal)]}
    tables, blobs, source_pins = {}, {}, {}
    for repository, checkout in [('opentibiabr/canary', args.canary), ('zimbadev/crystalserver', args.crystal)]:
        revision = SOURCES[repository]
        tree = subprocess.check_output(['git', '-C', str(checkout), 'ls-tree', '-r', revision]).decode()
        blobs[repository] = {line.split('\t', 1)[1]: line.split()[2] for line in tree.splitlines()}
        pins = {}
        for path in ('data/items/appearances.dat', 'data/items/items.xml', 'data/scripts/lib/register_monster_type.lua',
                     'src/lua/functions/creatures/monster/loot_functions.cpp', 'src/items/items.cpp'):
            data = (checkout / path).read_bytes()
            if blob_id(data) != blobs[repository].get(path):
                raise ValueError('donor source changed: ' + path)
            pins[path] = sha(checkout / path)
        source_pins[repository] = {'revision': revision, 'files': pins}
        tables[repository] = item_table(checkout)
    leaves, unknown, counts, bundle_pins, source_cache, native_rows = [], [], Counter(), {}, {}, []
    native_inputs = None
    if args.staged or args.item_map:
        if not args.staged or not args.item_map:
            raise ValueError('native projection requires both exact stage and qualified Item-map inputs')
        paths = [args.staged, args.item_map, *args.item_rekey]
        inputs.update({str(path): sha(path) for path in paths})
        native_inputs = tuple(json.loads(path.read_text()) for path in paths[:2]) + ([json.loads(path.read_text()) for path in paths[2:]],)
    for directory in sorted(args.bundles.iterdir()):
        name = directory.name
        if name not in census:
            continue
        if bundle_digest(directory) != census[name]['sha256']:
            raise ValueError(name + ': bundle differs from exact census pin')
        monster = json.loads((directory / 'monster.json').read_text(), parse_float=Decimal)
        manifest = json.loads((directory / 'manifest.json').read_text())
        bundle_pins[name] = {file.name: sha(file) for file in directory.iterdir() if file.suffix == '.json'}
        entries = monster.get('loot', {}).get('entries', [])
        counts['bundle_loot_entries'] += len(entries)
        if native_inputs and entries:
            binding = dict(census[name].get('binding') or {
                'source_key': 'oteryn:source.canary', 'source_revision': SOURCES['opentibiabr/canary'],
                'identity_namespace': 'canary/monster-file', 'external_id': census[name]['file']})
            if 'source_revision' not in binding:
                wiki_authored = ROOT / 'tools/content-schema/monster-authoring/samples/wiki-authored-2026-09-27.json'
                inputs[str(wiki_authored)] = sha(wiki_authored)
                binding['source_revision'] = 'tibiawiki-wiki-authored-creature-' + sha(wiki_authored)[:16]
            native_rows.append({'monster': name, **native_projection(*native_inputs, binding, entries)})
        proven = set()
        candidates = [e for e in manifest['entries'] if re.fullmatch(r'loot\[\d+\]', e.get('source_field', ''))
                      and e.get('status') == 'mapped' and e.get('destination')]
        for declaration in candidates:
            source = manifest['sources'][declaration['source_index']]
            if source.get('repository') not in SOURCES:
                continue
            repository = source['repository']
            checkout = args.canary if repository == 'opentibiabr/canary' else args.crystal
            leaf = {'monster': name, 'source_field': declaration['source_field'], 'source': source,
                    'source_file': declaration['source_file'], 'destination': declaration['destination']}
            try:
                path = checkout / declaration['source_file']
                if source['revision'] != SOURCES[repository] or blob_id(path.read_bytes()) != blobs[repository].get(declaration['source_file']):
                    raise ValueError('monster donor differs from exact source pin')
                if path not in source_cache:
                    _, raw, _ = load_monster(path, errors=[])
                    source_cache[path] = raw, sha(path)
                raw, source_sha = source_cache[path]
                position = int(re.search(r'\d+', declaration['source_field'])[0]) - 1
                item = raw['loot'][position]
                index = int(declaration['destination'].rsplit('/', 1)[1])
                target = entries[index]
                number = source_item(item, tables[repository][1])
                record = captures[repository].get(name, {})
                chance = next((c for offset, c in enumerate(record.get('loot_chances', []))
                               if c.get('position', offset) == position), None)
                expected = wiki_values(item, chance, record.get('loot_statistics', {}).get('kills', 0))
                low, high, probability, state = expected
                actual = target['min_count'], target['max_count'], exact_ppm(target['probability_percent'])
                valid = (actual == (low, high, probability) and target['item']['key'] == f'canary:item/{number}'
                         and target['skip_later_same_item_after_success'] == bool(item.get('unique', False)))
                leaf.update(proven_status='PROVEN' if valid else 'MISMATCH', expected_item_id=number,
                            expected_ppm=probability, expected_min_count=low, expected_max_count=high,
                            wiki_state=state, expected_skip_later_same_item_after_success=bool(item.get('unique', False)),
                            actual=actual, source_sha256=source_sha, wiki_capture_status=record.get('status', 'NO_CAPTURE'))
                proven.add(index)
            except (ValueError, KeyError, IndexError, FileNotFoundError) as error:
                leaf.update(proven_status='UNKNOWN', reason=str(error))
            leaves.append(leaf)
        additions = [e for e in manifest['entries'] if e.get('status') == 'mapped'
                     and re.fullmatch(r'/monster/loot/entries/\d+', e.get('destination', ''))
                     and e.get('source_field', '').startswith('Loot2.')]
        for declaration in additions:
            index = int(declaration['destination'].rsplit('/', 1)[1])
            target = entries[index]
            leaf = {'monster': name, 'source_field': declaration['source_field'], 'destination': declaration['destination']}
            try:
                repository = next(s['repository'] for s in manifest['sources'] if s.get('repository') in SOURCES)
                record = captures[repository][name]
                stats = record['loot_statistics']
                item = next(i for i in stats['items'] if i.get('line') == declaration['source_line'])
                numbers = wiki_items(item, record['wiki_title'], *tables[repository])
                probability = half_even(item['times'] * 1000000, stats['kills'] * len(numbers))
                amount = [int(value) for value in item['amount'].split('-')]
                number = int(target['item']['key'].rsplit('/', 1)[1])
                valid = (number in numbers and exact_ppm(target['probability_percent']) == probability
                         and (target['min_count'], target['max_count']) == (amount[0], amount[-1])
                         and target['skip_later_same_item_after_success'] is False)
                leaf.update(proven_status='PROVEN' if valid else 'MISMATCH', expected_item_ids=sorted(numbers),
                            expected_ppm=probability, expected_min_count=amount[0], expected_max_count=amount[-1],
                            wiki_state='APPROVED_WIKI_ADDITION' if item['times'] >= 10 else 'LOW_CONFIDENCE_WIKI_ADDITION',
                            wiki_source=manifest['sources'][declaration['source_index']])
                proven.add(index)
            except (ValueError, KeyError, IndexError, StopIteration) as error:
                leaf.update(proven_status='UNKNOWN', reason=str(error))
            leaves.append(leaf)
        unknown.extend({'monster': name, 'entry': index, 'reason': 'no independently supported source declaration pairing'}
                       for index in set(range(len(entries))) - proven)
    counts.update(Counter(row['proven_status'] for row in leaves))
    return {'scope': 'Independent authored loot source/D1/D15/D22/D32 value proof; not Lua RNG parity, canonical Item admission or Actor runtime readiness.',
            'helper_sha256': sha(Path(__file__)), 'input_sha256': inputs, 'source_pins': source_pins,
            'bundle_file_pins': bundle_pins, 'counts': dict(counts), 'leaves': leaves, 'unpaired_entries': unknown,
            'native_projection': native_rows,
            'canonical_item_identity_admission': 'UNKNOWN_DELEGATED_TO_ITEM_GUARD', 'runtime_qualified': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('bundles', 'index', 'wiki-canary', 'wiki-crystal', 'canary', 'crystal', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('staged', 'item-map'):
        parser.add_argument('--' + name, type=Path)
    parser.add_argument('--item-rekey', type=Path, action='append', default=[])
    args = parser.parse_args()
    report = verify(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'counts': report['counts'], 'unpaired_entries': len(report['unpaired_entries'])}))


if __name__ == '__main__':
    main()
