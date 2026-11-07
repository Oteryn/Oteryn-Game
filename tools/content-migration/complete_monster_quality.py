"""Qualify prepared monster roles and retained non-Global balance estimates.

Produces composable annotations and index flag additions, never overwrites bundles.
Registration proves a broad donor creature role, not the absence of boss/quest roles.
Uncertain loot remains uncertain after structural qualification.
"""
import argparse
import copy
from collections import Counter
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import re
import subprocess

FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def digest(folder):
    result = hashlib.sha256()
    for name in FILES:
        data = (folder / name).read_bytes()
        result.update(f'{name}\0{len(data)}\0'.encode() + data)
    return result.hexdigest()


def registration_evidence(source, cache):
    """Verify source against its pinned Git tree before interpreting registration."""
    path = source['path']
    if Path(path).is_absolute() or '..' in Path(path).parts:
        raise ValueError('unsafe donor source path')
    data = subprocess.check_output(['git', '-C', str(cache), 'show',
                                    source['revision'] + ':' + path])
    blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
    if blob != source['blob_sha1']:
        raise ValueError('classification donor blob mismatch')
    for number, line in enumerate(data.decode().splitlines(), 1):
        if re.search(r'Game\.createMonsterType\s*\(', line):
            return {'kind': 'source_lua_line', 'source': path,
                    'repository': source['repository'], 'revision': source['revision'],
                    'line': number, 'value': line.strip(),
                    'qualification': 'Pinned donor registers a monster actor. Broad inferred creature role is nonexclusive; boss and special role refinements remain possible.'}
    return None


def qualify_loot(monster, catalog, manifest):
    """Check meaningful admission invariants without upgrading wiki certainty."""
    definitions = {(r['family'], r['key'], r['revision']) for r in catalog['definitions']}
    entries = monster.get('loot', {}).get('entries', [])
    low = []
    for row in manifest['entries']:
        if 'LOW_CONFIDENCE_WIKI_ADDITION' in row.get('resolution', ''):
            match = re.fullmatch(r'/monster/loot/entries/(\d+)', row.get('destination', ''))
            if not match or row.get('status') != 'mapped':
                raise ValueError('low-confidence loot lacks a mapped exact entry')
            index = int(match[1])
            if index >= len(entries):
                raise ValueError('loot evidence points outside entries')
            source = manifest['sources'][row['source_index']]
            if source.get('kind') != 'mediawiki' or not all(k in source for k in
                    ('page_id', 'revision_id', 'content_sha256')):
                raise ValueError('wiki loot evidence is not revision-pinned')
            low.append({'entry_index': index, 'item': entries[index]['item'],
                        'wiki_source': source, 'resolution': row['resolution']})
    for entry in entries:
        reference = entry['item']
        if (reference['family'], reference['key'], reference['revision']) not in definitions:
            raise ValueError('loot item lacks exact catalog definition')
        chance = Fraction(str(entry['probability_percent']))
        if not 0 < chance <= 100 or (chance * 10000).denominator != 1:
            raise ValueError('loot probability cannot be admitted at exact ppm')
        if not (type(entry['min_count']) is int and type(entry['max_count']) is int
                and 1 <= entry['min_count'] <= entry['max_count']):
            raise ValueError('invalid loot stack count range')
    if len({r['entry_index'] for r in low}) != len(low):
        raise ValueError('duplicated low-confidence source pairing')
    return low


def dark_template(monster, manifest):
    stats = monster['creature']['stats']
    evidence = []
    for field in ('speed', 'armor', 'defense'):
        destination = '/monster/creature/stats/' + field
        matches = [r for r in manifest['entries'] if r.get('destination') == destination
                   and r.get('status') == 'mapped' and 'template' in r.get('resolution', '').lower()]
        if not matches:
            raise ValueError('Dark Merudri template stat lacks explicit provenance: ' + field)
        row = matches[0]
        evidence.append({'field': field, 'value': stats[field], 'source': manifest['sources'][row['source_index']],
                         'source_file': row['source_file'], 'source_line': row['source_line'],
                         'resolution': row['resolution']})
    return {'qualification': 'NON_GLOBAL_TEMPLATE_BALANCE', 'global_parity': False,
            'values_preserved': True, 'evidence': evidence,
            'reason': 'Retain the existing Dark Knight donor template as practical Oteryn balance; no Global verification is claimed.'}


def complete(population, annotations, caches):
    index = read(population / 'population-index.json')
    packet = copy.deepcopy(read(annotations))
    if packet['population_index_sha256'] != sha(population / 'population-index.json'):
        raise ValueError('annotations belong to another population')
    names = {r['monster'] for r in index['monsters']}
    if names != set(packet['annotations']):
        raise ValueError('annotations do not cover this population')
    changes, flag_additions, loot_receipts = [], {}, []
    for row in index['monsters']:
        name = row['monster']
        if Path(name).name != name or name in ('', '.', '..'):
            raise ValueError('unsafe bundle name')
        folder = population / 'bundles' / name
        if digest(folder) != row['sha256']:
            raise ValueError('bundle digest mismatch: ' + name)
        monster, manifest = read(folder / 'monster.json'), read(folder / 'manifest.json')
        annotation = packet['annotations'][name]
        if any(r['role'] == 'unknown' for r in annotation['roles']):
            evidence = next((e for source in annotation['source_files']
                             if source['repository'] in caches
                             if (e := registration_evidence(source, caches[source['repository']]))), None)
            if evidence:
                annotation['roles'] = [{'role': 'creature', 'confidence': 'inferred', 'evidence': [evidence]}]
                changes.append(name)
                flag_additions.setdefault(name, []).append('SOURCE_REGISTERED_ROLE_INFERRED')
        if 'LOW_CONFIDENCE_WIKI_LOOT' in row.get('completion_flags', []):
            low = qualify_loot(monster, read(folder / 'catalog.json'), manifest)
            if not low:
                raise ValueError('low-confidence flag lacks source-paired entries: ' + name)
            loot_receipts.append({'monster': name, 'entries': low,
                                  'global_parity': False, 'uncertainty_flag_preserved': True})
            flag_additions.setdefault(name, []).append('WIKI_LOOT_STRUCTURALLY_QUALIFIED')
    dark = None
    if 'dark_merudri' in names:
        folder = population / 'bundles/dark_merudri'
        dark = dark_template(read(folder / 'monster.json'), read(folder / 'manifest.json'))
        flag_additions.setdefault('dark_merudri', []).extend(['NON_GLOBAL_TEMPLATE_BALANCE', 'SOURCE_TEMPLATE_GLOBAL_UNVERIFIED', 'GAMEPLAY_UNVERIFIED'])
    remaining = sorted(n for n, a in packet['annotations'].items()
                       if any(r['role'] == 'unknown' for r in a['roles']))
    packet['summary']['roles'] = dict(sorted(Counter(
        r['role'] + ':' + r['confidence']
        for a in packet['annotations'].values() for r in a['roles']).items()))
    # The donor evidence packet predates accepted mitigation completion. Drop its
    # obsolete absent-mitigation subgroup metrics; the rebuilt catalogue counts
    # current values directly from the matching bundles.
    packet['summary'].pop('unknown_mitigation_no_encyclopedia', None)
    packet['summary'].pop('unknown_group_roles', None)
    receipt = {'schema': 'OTERYN_MONSTER_QUALITY_COMPLETION/v1',
               'population_index_sha256': sha(population / 'population-index.json'),
               'input_annotations_sha256': sha(annotations), 'role_refinements': sorted(changes),
               'remaining_unknown_roles': remaining, 'dark_merudri': dark,
               'low_confidence_loot_creatures': len(loot_receipts),
               'low_confidence_loot_entries': sum(len(r['entries']) for r in loot_receipts),
               'loot_receipts': loot_receipts, 'flag_additions': flag_additions,
               'runtime_verified': False, 'global_parity': False}
    packet.setdefault('limitations', []).append('Registered donor actors receive a broad inferred creature role; this does not exclude an unconfirmed boss or mechanic role.')
    return packet, receipt


def apply_flag_additions(index, receipt):
    """Compose quality flags without discarding earlier source uncertainty."""
    result = copy.deepcopy(index)
    by_name = {r['monster']: r for r in result['monsters']}
    if not set(receipt['flag_additions']) <= set(by_name):
        raise ValueError('quality flags reference a missing monster')
    for name, flags in receipt['flag_additions'].items():
        row = by_name[name]
        row['completion_flags'] = sorted(set(row.get('completion_flags', [])) | set(flags))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('population', 'annotations', 'canary', 'crystal', 'out'):
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists() or args.out.resolve() == args.population.resolve() or args.population.resolve() in args.out.resolve().parents:
        raise ValueError('output must be new and outside the input population')
    packet, receipt = complete(args.population, args.annotations,
                               {'opentibiabr/canary': args.canary, 'zimbadev/crystalserver': args.crystal})
    args.out.mkdir(parents=True)
    for filename, value in [('classification-source-evidence.json', packet), ('quality-completion.json', receipt)]:
        (args.out / filename).write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps({k: receipt[k] for k in ('low_confidence_loot_creatures', 'low_confidence_loot_entries')} |
                     {'roles_refined': len(receipt['role_refinements']), 'unknown_roles': len(receipt['remaining_unknown_roles'])}))


if __name__ == '__main__':
    main()
