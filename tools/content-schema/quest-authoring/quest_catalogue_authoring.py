#!/usr/bin/env python3
"""Offline wiki inventory; authored candidate coverage is not runtime readiness."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
from urllib.parse import quote

import jsonschema


def read(path):
    return json.loads(path.read_text())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate_semantic(row):
    semantic = row['source_fields'].get('semantic_enrichment')
    if semantic is None:
        return
    require(semantic['source_revision_sha256'] == row['content_sha256'], 'stale semantic source hash')
    payload = {k: v for k, v in semantic.items() if k != 'semantic_sha256'}
    fingerprint = hashlib.sha256(json.dumps(payload, sort_keys=True, ensure_ascii=False,
                                           separators=(',', ':')).encode()).hexdigest()
    require(fingerprint == semantic['semantic_sha256'], 'stale semantic payload hash')
    require(semantic['runtime_promotion'] is False and semantic['source_field_listing_complete'] is False,
            'false complete semantic listing or runtime promotion')
    for ref in row['source_fields']['reward_entity_references']:
        require(not ref['entity'].split(':', 1)[0].lower() in ('image', 'file', 'arquivo', 'ficheiro'),
                'media reference is not a reward')
    for expression in semantic['prerequisite_expressions']:
        require(expression['execution_semantics'] == 'UNKNOWN', 'unbound source prerequisite execution')
        for node in expression['nodes']:
            if node['kind'] == 'level_at_least':
                require(node['applies_to'] in ('alternative_entity', 'mission_or_addon', 'full_completion',
                                               'world', 'source_requirement_list'), 'missing level scope')
                require(expression['scope'] != 'entity_only' or node['applies_to'] == 'alternative_entity',
                        'entity threshold promoted to quest requirement')


def fresh_sources(facts, wiki):
    fresh = facts.get('fresh_wiki')
    if fresh is None:
        return {}, {}
    for row in fresh['pages'] + fresh['revision_manifest']:
        require(all(row.get(k) is not None for k in ('provider', 'pageid', 'revid', 'content_sha256')),
                'missing fresh revision provenance')
    key = lambda row: (row['provider'], row['pageid'], row['revid'])
    pages = {key(row): row for row in fresh['pages']}
    require(len(pages) == len(fresh['pages']), 'duplicate fresh source revision')
    manifest = {key(row): row for row in fresh['revision_manifest']}
    require(len(manifest) == len(fresh['revision_manifest']) and pages.keys() == manifest.keys(),
            'missing or duplicate fresh revision provenance')
    fingerprints = []
    for identity, row in pages.items():
        require(row.get('content_sha256') == manifest[identity]['content_sha256'], 'stale content hash')
        require(row.get('revision_timestamp') and row.get('url') and row.get('target_cut')
                and row.get('access_method') in ('remote_desktop_browser', 'normal_web'), 'missing fresh provenance')
        require(row['source_fields'].get('listing_complete') is False, 'false complete source listing')
        validate_semantic(row)
        fingerprints.append(':'.join(map(str, identity)) + ':' + row['content_sha256'])
    actual_digest = hashlib.sha256(('\n'.join(sorted(fingerprints)) + '\n').encode()).hexdigest()
    require(actual_digest == fresh['snapshot_pages_digest'], 'stale source snapshot digest')
    linked = defaultdict(list)
    seen = set()
    for link in fresh['title_links']:
        title = link['wiki_title']
        identity = key(link)
        require(title in wiki and identity in pages, 'stale fresh title link')
        require((title, link['role']) not in seen, 'ambiguous fresh title link')
        seen.add((title, link['role']))
        if link['role'] == 'base':
            require(identity[0] == 'tibia_fandom' and identity[1:]
                    == (wiki[title]['pageid'], wiki[title]['revid']), 'stale pinned base revision')
        linked[title].append({'relation': link['role'], 'mapping_basis': link['mapping_basis'], **pages[identity]})
    counts = Counter(row['provider'] for row in pages.values())
    summary = {'pages': len(pages), 'pages_by_provider': dict(sorted(counts.items())),
               'title_links_by_role': dict(sorted(Counter(x['role'] for x in fresh['title_links']).items())),
               'source_target_cuts': fresh['source_target_cuts'], 'snapshot_pages_digest': actual_digest,
               'semantic_pages': sum('semantic_enrichment' in p['source_fields'] for p in pages.values()),
               'missing_pages': len(fresh['unavailable_pages']), 'runtime_promotion': False}
    return linked, summary


def build(coverage, quests, readiness, links, facts, provenance):
    rows = coverage['quests']
    titles = [row['title'] for row in rows]
    require(len(titles) == len(set(titles)), 'duplicate wiki title')
    for row in rows:
        require(all(row.get(k) is not None for k in ('pageid', 'revid', 'rev_timestamp', 'in_quest_log')),
                'missing wiki provenance: ' + row['title'])
    require(len(provenance) >= 4 and all(p.get('sha256') and p.get('path') for p in provenance),
            'missing input provenance')
    require(all(not Path(p['path']).is_absolute() for p in provenance), 'absolute input path')
    authored = defaultdict(list)
    for quest in quests:
        authored[quest['display_name']].append(quest)
    keyed = {quest['identity']['key']: quest for quest in quests}
    require(len(keyed) == len(quests), 'duplicate authored key')
    ready = {row['quest']: row for row in readiness['quests']}
    wiki = {row['title']: row for row in rows}
    fresh, acquisition_summary = fresh_sources(facts, wiki)
    family = {}
    for link in links['links']:
        title = link['wiki_title']
        require(title not in family and title not in authored, 'ambiguous family link: ' + title)
        require(title in wiki and link['wiki_pageid'] == wiki[title]['pageid']
                and link['wiki_revid'] == wiki[title]['revid'], 'stale wiki link: ' + title)
        require(link.get('scope') == 'partial_named_missions', 'false complete family scope')
        target = keyed.get(link['target_key'])
        require(target is not None, 'stale family target: ' + title)
        actual = {m['key'] for m in target.get('missions', [])}
        require(link['mission_keys'] and set(link['mission_keys']) <= actual,
                'stale family mission: ' + title)
        require(link.get('basis') and link.get('evidence'), 'missing family provenance')
        family[title] = link
    additions = {}
    for fact in facts.get('npc_fact_candidates', []):
        source = fact.get('source', {})
        require(source.get('provider') and source.get('revid') and source.get('raw_page_sha256')
                and source.get('snapshot_sha256') and source.get('npc_line_indexes'), 'missing NPC fact provenance')
    for fact in facts['source_backed_authoring']:
        title = fact['curator_entry']['title']
        require(title not in additions, 'duplicate source-backed title')
        require(title in wiki and fact['wiki']['revid'] == wiki[title]['revid'], 'stale authoring evidence')
        require(fact.get('sources') and all(s.get('repository') and s.get('revision')
                and s.get('path') and s.get('blob_sha256') for s in fact['sources']),
                'missing source-backed provenance')
        additions[title] = fact
    fresh_pages = facts.get('fresh_wiki', {}).get('pages', [])
    source_keys = {(r['provider'], r['pageid'], r['revid'], r['content_sha256']) for r in fresh_pages}
    for overlap in facts.get('source_identity_overlaps', []):
        involved = overlap['titles']
        require(len(involved) >= 2 and len(involved) == len(set(involved))
                and set(involved) <= wiki.keys(), 'stale or duplicate identity overlap')
        markers = {(e['provider'], e['pageid'], e['revid'], e['content_sha256']) for e in overlap['evidence']}
        require(markers <= source_keys and len(markers) >= 2, 'stale identity overlap provenance')
        for title in involved:
            require(any(e['provider'] == 'tibia_fandom' and (e['pageid'], e['revid'])
                        == (wiki[title]['pageid'], wiki[title]['revid']) for e in overlap['evidence']),
                    'missing pinned overlap title provenance')
    output = []
    for row in sorted(rows, key=lambda r: r['title']):
        title = row['title']
        require(all(row.get(k) is not None for k in ('pageid', 'revid', 'rev_timestamp', 'in_quest_log')),
                'missing wiki provenance: ' + title)
        matches = authored.get(title, [])
        checks = []
        if len(matches) > 1:
            checks.append({'kind': 'duplicate_title', 'keys': [q['identity']['key'] for q in matches]})
        for quest in matches:
            if quest['shown_in_quest_log'] != row['in_quest_log']:
                checks.append({'kind': 'quest_log_conflict', 'key': quest['identity']['key'],
                               'wiki_reference_in_quest_log': row['in_quest_log'],
                               'ots_shown_in_quest_log': quest['shown_in_quest_log']})
        references = fresh.get(title, [])
        for reference in references:
            if reference['relation'] != 'crosscheck':
                continue
            br = reference['source_fields']
            values = {'level': (row.get('lvl'), br['level']),
                      'premium': (row.get('premium'), {'sim': 'yes', 'não': 'no', 'nao': 'no'}.get(br['premium'], br['premium']))}
            for field, (cached, other) in values.items():
                if cached and other and cached != other:
                    checks.append({'kind': 'structured_source_difference', 'field': field,
                                   'fandom_value': cached, 'br_value': other,
                                   'fandom_target_cut': '2026-09-27', 'br_target_cut': reference['target_cut'],
                                   'classification': 'CONFLICT', 'resolution': 'Preserve scoped fields; align dates and meanings before promotion'})
        for overlap in facts.get('source_identity_overlaps', []):
            if title in overlap['titles']:
                require(overlap.get('evidence') and all(e.get('revid') and e.get('content_sha256') for e in overlap['evidence']), 'missing identity overlap provenance')
                checks.append({'kind': 'source_identity_overlap', **overlap})
        state = ('conflict' if checks else 'directly_authored' if matches
                 else 'represented_by_family' if title in family else 'unrepresented')
        projection = []
        for quest in matches:
            evidence = ready.get(quest['identity']['key'])
            projection.append({'identity': quest['identity'], 'kind': quest['kind'],
                               'ots_shown_in_quest_log': quest['shown_in_quest_log'],
                               'readiness_data': {'classification': 'DERIVED' if evidence else 'UNKNOWN',
                                                  'features': evidence.get('features', []) if evidence else [],
                                                  'data_gaps': evidence.get('data_gaps') if evidence else None,
                                                  'runtime_readiness': 'UNKNOWN'}})
        source = {'provider': 'tibia_fandom', 'pageid': row['pageid'], 'revid': row['revid'],
                  'revision_timestamp': row['rev_timestamp'], 'retrieved_at': coverage['wiki']['retrieved_at'],
                  'url': 'https://tibia.fandom.com/wiki/' + quote(title.replace(' ', '_')) + '?oldid=' + str(row['revid']),
                  'classification': 'DERIVED', 'target_cut': '2026-09-27', 'target_continuity': 'DERIVED',
                  'continuity_basis': 'Protected exact-revision snapshot retrieved on target date; not fresh verification.'}
        output.append({'wiki_title': title, 'source': source,
                       'source_facts': {k: row.get(k) for k in ('in_quest_log', 'premium', 'lvl', 'implemented')},
                       'coverage_state': state, 'authored_candidates': projection,
                       'family_representation': family.get(title), 'source_checks': checks,
                       'source_backed_authoring': additions.get(title),
                       'fresh_sources': references,
                       'global_rewards': {'classification': 'UNKNOWN'},
                       'global_prerequisites_beyond_cached_fields': {'classification': 'UNKNOWN'}})
    if acquisition_summary:
        field_counts = Counter()
        for row in output:
            refs = {r['relation']: r['source_fields'] for r in row['fresh_sources']}
            base, spoiler = refs.get('base', {}), refs.get('spoiler', {})
            field_counts['fandom_reward_entity_lists'] += bool(base.get('reward_entity_references'))
            field_counts['fandom_requirement_entity_lists'] += bool(spoiler.get('requirement_entity_references'))
            field_counts['any_requirement_entity_lists'] += any(r.get('requirement_entity_references') for r in refs.values())
            field_counts['plain_numeric_fandom_min_level'] += bool(base.get('level') and base['level'].isdigit())
            field_counts['fandom_min_level_unknown'] += not bool(base.get('level') and base['level'].isdigit())
            field_counts['structured_source_differences'] += sum(c['kind'] == 'structured_source_difference' for c in row['source_checks'])
        acquisition_summary['field_coverage'] = dict(sorted(field_counts.items()))
    return {'schema': 'OTERYN_QUEST_CATALOGUE/v1', 'target_cut': '2026-09-27',
            'scope': 'complete_source_inventory; candidate coverage only; no runtime promotion',
            'input_provenance': provenance,
            'historical_coverage_ots_sources': coverage['servers'],
            'summary': {'source_titles': len(output), 'authored_records': len(quests),
                        'coverage_states': dict(sorted(Counter(r['coverage_state'] for r in output).items())),
                        'quest_log_conflicts': sum(c['kind'] == 'quest_log_conflict' for r in output for c in r['source_checks']),
                        'duplicate_title_checks': sum(c['kind'] == 'duplicate_title' for r in output for c in r['source_checks'])},
            'fresh_acquisition_summary': acquisition_summary,
            'source_access_checks': facts.get('fresh_wiki', {}).get('access_checks', []),
            'unavailable_source_pages': facts.get('fresh_wiki', {}).get('unavailable_pages', []),
            'npc_fact_candidates': facts.get('npc_fact_candidates', []), 'quests': output}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parents = Path(__file__).resolve().parents
    parser.add_argument('--root', type=Path, default=parents[3] if len(parents) > 3 else Path.cwd())
    parser.add_argument('--curated', type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    base = args.root / 'tools/content-schema/quest-authoring'
    paths = [base / 'samples/quest-coverage-2026-09-27.json', base / 'samples/questlog/quests.json',
             base / 'samples/readiness/readiness.json', args.curated / 'wiki_quest_links.json',
             args.curated / 'wiki_quest_facts.json', base / 'samples/questlog/manifest.json']
    provenance = [{'path': 'tools/content-schema/quest-authoring/' +
                   (str(p.relative_to(base)) if p.is_relative_to(base) else p.name), 'sha256': digest(p)} for p in paths]
    result = build(read(paths[0]), read(paths[1])['quests'], read(paths[2]), read(paths[3]), read(paths[4]), provenance)
    result['authored_ots_sources'] = [{k: s[k] for k in ('repository', 'revision', 'path')}
                                     for s in read(paths[5])['sources'] if s.get('kind') == 'git']
    jsonschema.Draft202012Validator(read(args.curated / 'quest_catalogue.schema.json')).validate(result)
    serialized = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
    output = args.output or base / 'samples/catalogue/catalogue.json'
    if args.check:
        require(output.exists() and output.read_text() == serialized, 'catalogue differs: rebuild required')
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(serialized)
    print(json.dumps(result['summary'], sort_keys=True))


if __name__ == '__main__':
    main()
