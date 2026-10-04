"""Prepare source-bound loot repairs and explicit practical-balance qualifications.

Never modifies its input population. Fresh small samples do not replace robust
reference probabilities. New wiki-listed items need an admitted exact ItemID;
missing rates use a disclosed, population-calibrated Oteryn rarity estimate.
"""
import argparse
from collections import Counter, defaultdict
import csv
from decimal import Decimal
from fractions import Fraction
import gzip
import hashlib
import json
from pathlib import Path
import re
import statistics
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'content-schema/monster-authoring'))
from verify_loot_source_semantics import item_table, half_even

BUCKETS = ('lootcomum', 'lootincomum', 'lootsemiraro', 'lootraro', 'lootmuitoraro')


def read(path):
    path = Path(path)
    return json.load(gzip.open(path, 'rt')) if path.suffix == '.gz' else json.loads(path.read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def csv_rows(path):
    return list(csv.DictReader(gzip.open(path, 'rt')))


def norm(value):
    return re.sub(r'\s+', ' ', value.replace('_', ' ')).strip().casefold()


def source(page, field, api='https://tibiawiki.com.br/api.php'):
    line = page.get('field_lines', {}).get(field)
    if not isinstance(line, int):
        line = next((i for i, value in enumerate(page.get('content', '').splitlines(), 1)
                     if re.match(r'^\|\s*' + re.escape(field) + r'\s*=', value)), 1)
    return {'kind': 'mediawiki', 'url': page['url'], 'rev': page['revision_id'],
            'sha': page['content_sha256'], 'api': api, 'title': page['page_title'],
            'page_id': page['page_id'], 'revision_id': page['revision_id'],
            'content_sha256': page['content_sha256'], 'source_file': page['page_title'],
            'source_line': line,
            'source_field': 'infobox.' + field}


def verify_page(page):
    if 'content' in page and hashlib.sha256(page['content'].encode()).hexdigest() != page['content_sha256']:
        raise ValueError('captured wiki bytes differ from their digest')


def item_index(packet):
    result = {}
    for page in packet['pages']:
        verify_page(page)
        result[norm(page['page_title'])] = page
    for redirect in packet.get('redirects', []):
        target = result.get(norm(redirect['to']))
        if target:
            result[norm(redirect['from'])] = target
    return result


def resolve_item(title, pages, by_name, admitted):
    """Never allocate an ID or choose a variant by resemblance."""
    page = pages.get(norm(title))
    if page:
        raw = page.get('fields', {}).get('itemid', '')
        if not raw and 'content' in page:
            match = re.search(r'^\|\s*itemid\s*=\s*([^\n]*)', page['content'], re.M | re.I)
            raw = match[1] if match else ''
        if raw:
            numbers = {int(v) for v in re.findall(r'\d+', raw)}
            if numbers:
                return numbers & admitted, page, 'WIKI_ITEMID'
        name = page.get('fields', {}).get('name', page['page_title'])
    else:
        name = title
    numbers = by_name.get(norm(name), set())
    if len(numbers) == 1:
        return numbers & admitted, page, 'UNIQUE_ADMITTED_DONOR_NAME'
    return set(), page, 'UNRESOLVED_ITEM_ID'


def observations(loot_fields):
    for bucket, text in loot_fields.items():
        if bucket not in BUCKETS:
            continue
        for match in re.finditer(r'\[\[([^\]|#]+)(?:\|[^\]]+)?\]\]', text):
            # Conditional grants are kept outside ordinary monster loot.
            segment = text[match.end():].split(',', 1)[0]
            conditional = bool(re.search(r'primeira|apenas|somente|quest|miss[aã]o', segment, re.I))
            prefix = text[:match.start()].split(',')[-1]
            quantity = re.search(r'(\d+)\s*[-–]\s*(\d+)\s*$', prefix)
            if quantity:
                low, high = map(int, quantity.groups())
                amount = (max(1, low), high)
                amount_kind = 'WIKI_RANGE_NO_DROP_ZERO_NORMALIZED'
            else:
                single = re.search(r'(\d+)\s*$', prefix)
                amount = (int(single[1]), int(single[1])) if single else (1, 1)
                amount_kind = 'WIKI_EXPLICIT_COUNT' if single else 'OTERYN_SINGLE_UNIT_ASSUMPTION'
            yield match[1].strip(), bucket, amount, amount_kind, conditional


def calibrate(rows, monsters, names, by_name):
    samples = defaultdict(list)
    for row in rows:
        monster = monsters[row['monster']]
        by_id = {int(e['item']['key'].rsplit('/', 1)[1]): e for e in monster.get('loot', {}).get('entries', [])}
        for title, bucket, _, _, conditional in observations(json.loads(row['loot_fields_raw'])):
            ids = by_name.get(norm(title), set())
            if not conditional and len(ids) == 1 and next(iter(ids)) in by_id:
                value = by_id[next(iter(ids))]['probability_percent']
                samples[bucket].append(int(Fraction(str(value)) * 10000))
    return {'schema': 'OTERYN_LOOT_RARITY_CALIBRATION/v1', 'method': 'median of existing authored exact-ppm entries, unique donor item-name match, minimum 20 samples per rarity',
            'global_parity': False, 'buckets': {bucket: {'samples': len(samples[bucket]),
                'estimated_ppm': max(1, round(statistics.median(samples[bucket]))) if len(samples[bucket]) >= 20 else None}
                for bucket in BUCKETS}}


def make_packet(baseline, audit, wiki, item_wiki, item_map, output, en_item_wiki=None):
    for p in (baseline, audit, wiki, item_wiki, item_map, output):
        if not isinstance(p, Path):
            raise TypeError('paths must be pathlib Paths')
    if output.resolve() == baseline.resolve() or baseline.resolve() in output.resolve().parents:
        raise ValueError('output would modify the input population')
    index = read(baseline / 'population-index.json')
    summary = read(audit / 'summary.json')
    if sha(baseline / 'population-index.json') != summary['population_index_sha256']:
        raise ValueError('loot audit and population differ')
    receipt = read(audit / 'audit-ready.json')
    for filename in ('summary.json', 'entries.csv.gz', 'fresh-br-itemsets.csv.gz', 'fresh-en-numeric.csv.gz'):
        if sha(audit / filename) != receipt['files_sha256'][filename]:
            raise ValueError('audit input has changed: ' + filename)
    rows = csv_rows(audit / 'fresh-br-itemsets.csv.gz')
    original = {r['monster']: read(baseline / 'bundles' / r['monster'] / 'monster.json') for r in index['monsters']}
    actor_flags = defaultdict(set)
    def flag(name, *values):
        actor_flags[name].update(values)
    inherited_estimates, inherited_low = set(), set()
    entries = csv_rows(audit / 'entries.csv.gz')
    for row in entries:
        if row['wiki_confirmation_status'] in ('CACHED_OBSERVATION_ESTIMATE_MATCH', 'CACHED_LOW_CONFIDENCE_ESTIMATE_MATCH'):
            flag(row['monster'], 'WIKI_LOOT_ESTIMATE')
            inherited_estimates.add(row['monster'])
        if row['wiki_confirmation_status'] == 'CACHED_LOW_CONFIDENCE_ESTIMATE_MATCH':
            flag(row['monster'], 'LOW_CONFIDENCE_WIKI_LOOT')
            inherited_low.add(row['monster'])
    for row in csv_rows(audit / 'fresh-en-numeric.csv.gz'):
        if row['status'].startswith('FRESH_COMMUNITY_ESTIMATE_DIFFERENCE'):
            flag(row['monster'], 'FRESH_WIKI_LOOT_ESTIMATE_DIFFERS_FROM_REFERENCE')
        if row['status'] == 'UNVERIFIED_INVALID_OBSERVATION':
            flag(row['monster'], 'INVALID_COMMUNITY_LOOT_OBSERVATION_IGNORED')
    names, by_name, _ = item_table(Path('/workspace/monster-reference-sources/canary'))
    admitted = {r['source_item_id'] for r in read(item_map)['records']}
    ipages = item_index(read(item_wiki))
    if en_item_wiki:
        ipages.update(item_index(read(en_item_wiki)))
    pages = {norm(p['page_title']): p for p in read(wiki)['pages']}
    calibration = calibrate(rows, original, names, by_name)
    calibration['population_index_sha256'] = sha(baseline / 'population-index.json')
    output.mkdir(parents=True, exist_ok=False)
    ledger = output / 'loot-rarity-calibration.json'
    ledger.write_text(json.dumps(calibration, indent=2) + '\n')
    patches, unresolved, resolved_aliases = [], [], []
    resolved_actor_flags = {}
    by_index = {r['monster']: r for r in index['monsters']}
    def patch(name, file, pointer, before, after, evidence, reason, present=True):
        patches.append({'monster': name, 'file': file, 'pointer': pointer,
                        'expected_present': present, 'expected_value': before, 'value': after,
                        'source': evidence, 'reason': reason})
    for row in rows:
        name = row['monster']
        monster = original[name]
        wiki_page = next((p for p in pages.values() if p['content_sha256'] == row['wiki_sha256']), None)
        if row['fresh_wiki_itemset_status'] == 'CONFLICT_EXPLICIT_NO_LOOT':
            if not wiki_page or norm(wiki_page['fields'].get('name', '')) != norm(monster['creature']['display_name']):
                raise ValueError('no-loot correction lacks exact actor identity')
            evidence = source(wiki_page, 'loot')
            patch(name, 'monster.json', '/loot/entries', monster['loot']['entries'], [], evidence,
                  'Fresh exact-identity WikiBR explicitly states Nenhum; preserve the selected Loot identity and catalog, remove inherited ordinary drops.')
            stale = {'WIKI_LOOT_ESTIMATE', 'LOW_CONFIDENCE_WIKI_LOOT', 'FRESH_WIKI_LOOT_ESTIMATE_DIFFERS_FROM_REFERENCE'}
            actor_flags[name].difference_update(stale)
            inherited_stale = sorted(stale & set(by_index[name].get('completion_flags', [])))
            if inherited_stale:
                resolved_actor_flags[name] = inherited_stale
            flag(name, 'WIKI_CONFIRMED_NO_LOOT', 'LEGACY_LOOT_DATA_REMOVED')
            if name in ('dawn_bat', 'neutral_deepling_warrior'):
                flag(name, 'PRE_INTRODUCTION_LOOT_STATISTICS_REJECTED')
            continue
        if row['fresh_wiki_itemset_status'] != 'UNVERIFIED_ITEM_NAME_DIFFERENCE':
            continue
        existing = {int(e['item']['key'].rsplit('/', 1)[1]) for e in monster.get('loot', {}).get('entries', [])}
        wanted = set(json.loads(row['only_wiki']))
        observations_by_name = {norm(t): (t, b, q, k, c) for t, b, q, k, c in observations(json.loads(row['loot_fields_raw']))}
        for title in sorted(wanted):
            ids, item_page, identity_kind = resolve_item(title, ipages, by_name, admitted)
            if ids & existing:
                resolved_aliases.append({'monster': name, 'wiki_item': title, 'item_ids': sorted(ids & existing), 'method': identity_kind})
                flag(name, 'WIKI_LOOT_ITEM_ALIAS_RESOLVED')
                continue
            observation = observations_by_name.get(title)
            if len(ids) != 1 or not observation or observation[4] or not wiki_page:
                unresolved.append({'monster': name, 'wiki_item': title, 'reason': 'conditional/unparsed presence or absent/ambiguous admitted ItemID', 'wiki_url': row['wiki_url'], 'item_ids': sorted(ids),
                    'declared_wiki_item_ids': sorted({int(v) for v in re.findall(r'\d+', item_page.get('fields', {}).get('itemid', ''))}) if item_page else [],
                    'item_identity_source': source(item_page, 'itemid' if identity_kind == 'WIKI_ITEMID' else 'name', item_page.get('url', '').split('/wiki/')[0] + '/api.php') if item_page else None})
                flag(name, 'WIKI_LOOT_ITEM_PRESENCE_UNVERIFIED')
                continue
            _, bucket, amount, amount_kind, _ = observation
            if 'loot' not in monster:
                unresolved.append({'monster': name, 'wiki_item': title, 'reason': 'source has no ordinary loot table; retain source summon/phase context pending exact drop applicability', 'wiki_url': row['wiki_url'], 'item_ids': sorted(ids)})
                flag(name, 'WIKI_LOOT_CONTEXT_CONFLICT_SOURCE_EMPTY_RETAINED')
                continue
            ppm = calibration['buckets'][bucket]['estimated_ppm']
            if ppm is None or amount[0] > amount[1]:
                unresolved.append({'monster': name, 'wiki_item': title, 'reason': 'insufficient rarity calibration or unsupported count range', 'wiki_url': row['wiki_url']})
                continue
            number = next(iter(ids))
            ref = {'family': 'Item', 'key': 'canary:item/' + str(number), 'revision': 'canary-47dfd51f'}
            evidence = {'kind': 'oteryn_balance_estimate', 'ledger_sha256': sha(ledger),
                'qualification': 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE', 'evidence_classification': 'DERIVED', 'global_parity': False,
                'owner_acceptance': '2026-10-02 owner authorized maximal practical monster completion with flagged non-Global balance; coordinator approved rarity medians with >=20 existing samples.',
                'source_file': ledger.name, 'source_line': 1, 'source_field': 'loot-rarity.' + bucket,
                'item_presence_source': source(wiki_page, bucket), 'item_identity_method': identity_kind,
                'item_identity_source': source(item_page, 'itemid' if identity_kind == 'WIKI_ITEMID' else 'name', item_page.get('url', '').split('/wiki/')[0] + '/api.php') if item_page else None}
            entry = {'item': ref, 'min_count': amount[0], 'max_count': amount[1],
                     'probability_percent': ppm / 10000, 'skip_later_same_item_after_success': False}
            patch(name, 'monster.json', '/loot/entries/-', None, entry, evidence,
                  f'Wiki-listed admitted ItemID {number}; Oteryn {bucket} median from {calibration["buckets"][bucket]["samples"]} existing samples; quantity {amount_kind}; no Global rate asserted.', False)
            catalog = read(baseline / 'bundles' / name / 'catalog.json')
            if ref not in catalog['definitions']:
                patch(name, 'catalog.json', '/definitions/-', None, ref, evidence, 'Declare the exact pre-admitted Item reference; allocate no identity.', False)
            existing.add(number)
            flag(name, 'WIKI_LOOT_ESTIMATE', 'OWNER_ACCEPTED_NON_GLOBAL_LOOT_ESTIMATE', 'LOOT_RARITY_MEDIAN_BALANCE', 'GAMEPLAY_UNVERIFIED')
            if amount_kind == 'OTERYN_SINGLE_UNIT_ASSUMPTION':
                flag(name, 'LOOT_QUANTITY_SINGLE_UNIT_ESTIMATE')
        if json.loads(row['only_prepared']):
            flag(name, 'WIKI_LOOT_ITEMSET_DIFFERENCE_RETAINED_SOURCE')
    counts = {'inherited_low_confidence_actors_examined': len(inherited_low),
              'low_confidence_actors_qualified': sum('LOW_CONFIDENCE_WIKI_LOOT' in f for f in actor_flags.values()),
              'statistical_estimate_actors_qualified': sum('WIKI_LOOT_ESTIMATE' in f for f in actor_flags.values()),
              'inherited_statistical_estimate_actors': len(inherited_estimates),
              'inherited_missing_estimate_flags_repaired': sum(r['monster'] in inherited_estimates and 'WIKI_LOOT_ESTIMATE' not in r.get('completion_flags', []) for r in index['monsters']),
              'inherited_missing_low_confidence_flags_repaired': sum(r['monster'] in inherited_low and 'LOW_CONFIDENCE_WIKI_LOOT' not in r.get('completion_flags', []) for r in index['monsters']),
              'no_loot_tables_corrected': sum(p['pointer'] == '/loot/entries' for p in patches),
              'loot_entries_added': sum(p['pointer'] == '/loot/entries/-' for p in patches),
              'wiki_aliases_resolved': len(resolved_aliases), 'unresolved_wiki_items': len(unresolved)}
    packet = {'schema': 'OTERYN_MONSTER_FIELD_PATCH_PACKET/v1', 'lane': 'loot-quality',
              'baseline_index_sha256': sha(baseline / 'population-index.json'),
              'audit_sha256': sha(audit / 'summary.json'), 'item_map_sha256': sha(item_map),
              'calibration_sha256': sha(ledger), 'patches': patches,
              'actor_flags': {n: sorted(v) for n, v in sorted(actor_flags.items())},
              'resolved_actor_flags': resolved_actor_flags,
              'resolved_aliases': resolved_aliases, 'unresolved': unresolved, 'counts': counts,
              'reference_rate_policy': 'Existing source/reference probabilities stay unchanged; fresh community drift is flagged. New confirmed items use disclosed median rarity estimates only.',
              'runtime_qualification': False}
    (output / 'field-patches.json').write_text(json.dumps(packet, indent=2, ensure_ascii=False) + '\n')
    (output / 'summary.json').write_text(json.dumps(counts, indent=2) + '\n')
    return packet


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for arg in ('baseline', 'audit', 'wiki', 'item-wiki', 'item-map', 'out'):
        p.add_argument('--' + arg, type=Path, required=True)
    p.add_argument('--en-item-wiki', type=Path)
    a = p.parse_args()
    print(json.dumps(make_packet(a.baseline, a.audit, a.wiki, a.item_wiki, a.item_map, a.out, a.en_item_wiki)['counts']))


if __name__ == '__main__':
    main()
