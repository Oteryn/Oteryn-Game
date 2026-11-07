"""Fill missing optional Creature metadata only from identity-bound Wiki facts.

No morphology guesses, familiar dynamic-speed constants, or empty-cell zeros.
"""
import argparse
import gzip
import hashlib
import json
from collections import Counter
from pathlib import Path

BASELINE_SHA = '1b87ba6bee339bbd0e485cecaf7d8862607a8bf473f8f5f44221142b79afa1c5'
BR_SHA = '785509ec404bbd47b9e799711546cff8a2389168d419eae7f61927338863854a'
COVERAGE_SHA = 'e45602f8641eccdc4a5f0c9fc5a0cd78d5ce4ac35eafd2009941f49ea17e098a'
BOSS_RULES = {'bane': ([25, 100, 300], [5, 15, 30]),
              'archfoe': ([5, 20, 60], [10, 30, 60]),
              'nemesis': ([1, 3, 5], [10, 30, 60])}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    path = Path(path)
    if path.suffix == '.gz':
        with gzip.open(path, 'rt', encoding='utf-8') as stream:
            return json.load(stream)
    return json.loads(path.read_text())


def fact(page, field, line=None):
    source = {key: page[key] for key in ('url', 'revision_id', 'content_sha256', 'method')}
    source.update(source_file=page['page_title'], source_field=field,
                  source_line=line or page.get('field_lines', {}).get(field, 1))
    if field in page.get('fields', {}):
        source['fields'] = {field: {'raw': page['fields'][field], 'source_line': source['source_line']}}
    return source


def patch_actor(slug, document, page, helper, *, variant=False):
    """A complete category record requires both the actor category and stage rules."""
    creature = document['creature']
    if variant or page is None:
        return []
    patches = []
    difficulty = {'inofensivo': (0, 'harmless'), 'trivial': (1, 'trivial'), 'fácil': (2, 'easy'), 'médio': (3, 'medium'), 'difícil': (4, 'hard'), 'desafiador': (5, 'challenging')}.get(page.get('fields', {}).get('dificuldade', '').strip().lower())
    b = creature.get('bestiary', {})
    if b and 'stars' not in b and difficulty and b['difficulty'] == difficulty[1]:
        patches.append({'monster': slug, 'file': 'monster.json', 'pointer': '/creature/bestiary/stars',
                        'expected_present': False, 'expected_value': None, 'value': difficulty[0],
                        'source': fact(page, 'dificuldade'),
                        'supporting_sources': [{'repository': 'Oteryn/Oteryn-Game',
                            'revision': '740c80e6c4f6d7c5156433c376ce823c4e9371e3',
                            'source_file': 'tools/content-schema/monster-authoring/canary_batch.py',
                            'source_line': 240, 'source_field': 'DIFFICULTY enum'}],
                        'reason': 'Encode the actual Wiki difficulty using the existing accepted project DIFFICULTY0..5 enum; this is a representation mapping, not an independently observed Global star icon count.'})
    category = page.get('fields', {}).get('bosstype', '').strip().lower()
    if 'bosstiary' in creature or category not in BOSS_RULES:
        return patches
    kills, points = BOSS_RULES[category]
    value = {'category': category}
    for stage, kill, point in zip(('prowess', 'expertise', 'mastery'), kills, points):
        value[stage + '_kills'] = kill
        value[stage + '_points'] = point
    return patches + [{'monster': slug, 'file': 'monster.json', 'pointer': '/creature/bosstiary',
             'expected_present': False, 'expected_value': None, 'value': value,
             'source': fact(page, 'bosstype'), 'supporting_sources': [fact(helper, 'bosstype stage switch', 17)],
             'reason': 'Actual same-actor Wiki Bosstiary category; all six stage values explicitly documented by the captured Infobox template.'}]


def assess(document, page, variant, patched):
    """Typed non-applicability is explicitly scoped to this prepared contract."""
    c = document['creature']
    fields = page.get('fields', {}) if page else {}
    rows = []
    def record(field, present, reason, status='UNAVAILABLE'):
        rows.append({'field': field, 'status': 'PRESENT' if present else status, 'reason': reason})
    record('name_forms.article', 'article' in c.get('name_forms', {}), 'No exact actor article cell; do not infer morphology from the display name.')
    record('name_forms.plural', 'plural' in c.get('name_forms', {}), 'Captured actor Wiki does not provide plural; source registrar has no plural setter. Never append s heuristically.')
    for field in ('description',):
        record('inspection.' + field, field in c.get('inspection', {}), 'Existing source inspection retained; encyclopedia narrative belongs to Document lane.')
    for field in ('prey', 'exclusive_prey', 'forge', 'reward_boss'):
        record('system_eligibility.' + field, field in c.get('system_eligibility', {}), 'Existing pinned donor/default value retained; Wiki absence is not an eligibility proof.')
    b = c.get('bestiary', {})
    for field in ('stars', 'locations'):
        record('bestiary.' + field, field in b, 'Missing Wiki cell or separately transcluded location; generic Creature class is not a Bestiary membership proof.')
    record('bosstiary', 'bosstiary' in c, 'Shared-title variant requires own identity evidence.' if variant else 'Empty bosstype does not establish non-applicability.', 'PATCHED' if patched else 'UNAVAILABLE')
    summoning = c['summoning']
    record('summoning.mana_cost', 'mana_cost' in summoning, 'Prepared schema forbids mana_cost when both summonable and convinceable are false; this is typed applicability, not a Global claim.', 'NA_PREPARED_CONTRACT' if not summoning['summonable'] and not summoning['convinceable'] else 'UNAVAILABLE')
    record('summoning.familiar.owner_speed_bonus', 'owner_speed_bonus' in summoning.get('familiar', {}), 'Familiar owner-relative dynamic speed cannot be represented by a fixed bonus.' if summoning['is_familiar'] else 'No familiar subrecord is permitted for a non-familiar.', 'UNREPRESENTABLE_DYNAMIC_SOURCE' if summoning['is_familiar'] else 'NA_PREPARED_CONTRACT')
    record('stats.mitigation_percent', 'mitigation_percent' in c['stats'], 'Existing direct-source or qualified accepted estimate retained.')
    return rows


def run(baseline, wiki, variants_file, output, supplement=None, candidates=None, donor=None):
    baseline, wiki, output = map(Path, (baseline, wiki, output))
    if digest(baseline / 'population-index.json') != BASELINE_SHA:
        raise ValueError('wrong baseline population index')
    if digest(wiki / 'pages.json.gz') != BR_SHA or digest(wiki / 'actor-page-coverage.json.gz') != COVERAGE_SHA:
        raise ValueError('wrong frozen Wiki corpus')
    index = load(baseline / 'population-index.json')
    pages = {p['page_title']: p for p in load(wiki / 'pages.json.gz')['pages']}
    coverage = {a['monster']: a for a in load(wiki / 'actor-page-coverage.json.gz')['actors']}
    variants = set(load(variants_file)['shared_title_variant_unverified_actors'])
    helper = pages['Predefinição:Infobox Criatura']
    patches, actors, counts = [], [], Counter()
    residuals, extra_flags = [], {}
    candidate_rows = {}
    if candidates:
        for candidate in load(candidates):
            if candidate['pointer'] in ('/creature/summoning/mana_cost', '/creature/resistances'):
                candidate_rows.setdefault(candidate['monster'], []).append(candidate)
    supplemental_pages = {}
    if supplement:
        raw_packet = load(supplement)
        for raw in json.loads(raw_packet['text']).get('query', {}).get('pages', []):
            if 'revisions' not in raw:
                continue
            revision = raw['revisions'][0]
            content = revision['slots']['main']['content']
            page = {'page_title': raw['title'], 'page_id': raw['pageid'],
                    'revision_id': revision['revid'], 'content_sha256': hashlib.sha256(content.encode()).hexdigest(),
                    'url': 'https://tibiawiki.com.br/wiki/' + __import__('urllib.parse', fromlist=['quote']).quote(raw['title'].replace(' ', '_'), safe=''),
                    'method': raw_packet['method'], 'retrieved_at': raw_packet['retrieved_at'], 'content': content}
            supplemental_pages[raw['title']] = page
    for row in index['monsters']:
        slug = row['monster']
        document = load(baseline / 'bundles' / slug / 'monster.json')
        cov = coverage[slug]
        variant = slug in variants or cov.get('skip_generated_variant', False)
        page = pages.get(cov.get('resolved_title')) if cov['status'] == 'FRESH_PAGE_CAPTURED' else None
        if page and (page['revision_id'] != cov['page_revision_id'] or page['content_sha256'] != cov['page_content_sha256']):
            raise ValueError('actor page identity proof mismatch: ' + slug)
        actor_patches = patch_actor(slug, document, page, helper, variant=variant)
        location_page = supplemental_pages.get('Predefinição:Locations')
        c = document['creature']
        if not variant and page and location_page and 'bestiary' in c and 'locations' not in c['bestiary']:
            import re
            title = page['page_title']
            match = re.search(r'^[ \t]*\|[ \t]*' + re.escape(title) + r'\s*=\s*(.+)$', location_page['content'], re.M)
            if match:
                raw = match[1].strip()
                value = re.sub(r'\{\{Loc\|([^{}|]+)(?:\|[^{}]*)?\}\}', r'\1', raw)
                value = re.sub(r'\[\[([^\[\]|]+)(?:\|([^\[\]]+))?\]\]', lambda m: m[2] or m[1], value)
                if value and '{{' not in value and '[[' not in value:
                    line = location_page['content'][:match.start()].count('\n') + 1
                    proof = fact(location_page, title, line)
                    proof['fields'] = {title: {'raw': raw, 'source_line': line}}
                    actor_patches.append({'monster': slug, 'file': 'monster.json', 'pointer': '/creature/bestiary/locations',
                                          'expected_present': False, 'expected_value': None, 'value': value, 'source': proof,
                                          'supporting_sources': [fact(page, 'name'), fact(helper, 'Locations transclusion', 539)],
                                          'reason': 'Exact actor entry in the actual transcluded Wiki Locations template; formatting-only link removal.'})
        for candidate in candidate_rows.get(slug, []):
            import subprocess, re
            proof = candidate['proof']
            raw = subprocess.check_output(['git', '-C', str(donor), 'show', proof['revision'] + ':' + proof['file']])
            if hashlib.sha256(raw).hexdigest() != proof['file_sha256'] or proof.get('partial_errors'):
                raise ValueError('unverified donor candidate: ' + slug)
            receipt = {'monster': slug, 'pointer': candidate['pointer'], 'source_value': candidate['source_value'], 'proof': proof}
            if candidate['pointer'].endswith('mana_cost'):
                summoning = c['summoning']
                if not summoning['summonable'] and not summoning['convinceable']:
                    receipt.update(status='NA_PREPARED_CONTRACT', reason='Both accepted summoning eligibility booleans are false; schema explicitly forbids mana_cost. Raw donor unused cost retained as evidence.')
                else:
                    raise ValueError('unexpected enabled summoning missing cost: ' + slug)
            else:
                kinds = {'PHYSICAL': 'physical', 'ENERGY': 'energy', 'EARTH': 'earth', 'FIRE': 'fire', 'HOLY': 'holy', 'DEATH': 'death', 'ICE': 'ice'}
                values = []
                reasons = []
                for element in candidate['source_value']:
                    code = element['type'].removeprefix('@COMBAT_').removesuffix('DAMAGE')
                    kind = kinds.get(code)
                    if not kind:
                        reasons.append('unknown damage type held')
                        continue
                    wiki_cell = page.get('fields', {}).get(kind + 'DmgMod') if page and not variant else None
                    immune = kind in c['immunities']['damage_types']
                    if wiki_cell or immune:
                        reasons.append(kind + ': retained actual Wiki resistance/immunity; donor value does not override it')
                    elif isinstance(element['percent'], int):
                        values.append({'damage_type': kind, 'reduction_percent': {'numerator': element['percent'], 'denominator': 1}})
                if values:
                    line = next((i for i,l in enumerate(raw.decode().splitlines(),1) if re.search(r'\.elements\s*=', l)), 1)
                    source = {'repository': proof['repository'], 'revision': proof['revision'], 'source_file': proof['file'], 'source_line': line, 'source_field': 'elements', 'file_sha256': proof['file_sha256']}
                    actor_patches.append({'monster': slug, 'file': 'monster.json', 'pointer': '/creature/resistances', 'expected_present': True, 'expected_value': c['resistances'], 'value': c['resistances'] + values, 'source': source, 'reason': 'Exact pinned donor resistances with no actual same-actor Wiki cell; preserve separate OTS qualification.'})
                    extra_flags[slug] = ['SOURCE_OTS_RESISTANCE_GLOBAL_UNVERIFIED']
                    receipt.update(status='PATCHED_PINNED_DONOR', filled_damage_types=[v['damage_type'] for v in values])
                else:
                    receipt.update(status='ALREADY_REPRESENTED_OR_WIKI_SUPERSEDED', reason='; '.join(reasons))
            residuals.append(receipt)
        patches.extend(actor_patches)
        fields = assess(document, page, variant, any(p['pointer'] == '/creature/bosstiary' for p in actor_patches))
        applied = {p['pointer'].removeprefix('/creature/').replace('/', '.') for p in actor_patches}
        for field in fields:
            if field['field'] in applied:
                field['status'] = 'PATCHED'
                field['reason'] = 'Positive source proof included in guarded packet.'
        counts.update(field['status'] for field in fields)
        actors.append({'monster': slug, 'variant_preserved': variant, 'wiki_page': fact(page, 'actual actor page') if page else None, 'fields': fields})
    packet = {'schema': 'OTERYN_MONSTER_FIELD_PATCH/v1', 'lane': 'optional-wiki-fields',
              'baseline_index_sha256': BASELINE_SHA, 'patches': patches, 'actor_flags': extra_flags,
              'resolved_actor_flags': {}, 'source_snapshot_sha256': BR_SHA,
              'global_parity_asserted': False}
    output.mkdir(parents=True, exist_ok=True)
    if supplemental_pages:
        with gzip.open(output / 'supplemental-pages.json.gz', 'wt', encoding='utf-8') as stream:
            json.dump({'pages': list(supplemental_pages.values())}, stream, ensure_ascii=False)
    (output / 'residual-source-decisions.json').write_text(json.dumps({'decisions': residuals}, ensure_ascii=False, indent=2) + '\n')
    (output / 'field-patches.json').write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    with gzip.open(output / 'applicability.json.gz', 'wt', encoding='utf-8') as stream:
        json.dump({'actors': actors}, stream, ensure_ascii=False)
    summary = {'actors': len(actors), 'patches': len(patches), 'patched_actors': len({p['monster'] for p in patches}),
               'status_counts': dict(counts), 'packet_sha256': digest(output / 'field-patches.json'),
               'baseline_index_sha256': BASELINE_SHA, 'source_snapshot_sha256': BR_SHA,
               'coverage_sha256': COVERAGE_SHA, 'qualified_variants_preserved': sum(a['variant_preserved'] for a in actors),
               'limitations': ['Empty Wiki fields never imply zero or Global non-applicability.',
                               'No Creature description replaced by encyclopedia lore.',
                               'Missing plural and dynamic familiar speed remain explicit unavailable/unrepresentable receipts.']}
    (output / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True)
    parser.add_argument('--wiki', required=True)
    parser.add_argument('--variants', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--supplement')
    parser.add_argument('--candidates')
    parser.add_argument('--donor')
    args = parser.parse_args()
    print(json.dumps(run(args.baseline, args.wiki, args.variants, args.output, args.supplement, args.candidates, args.donor)))
