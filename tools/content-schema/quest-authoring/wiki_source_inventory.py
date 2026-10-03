"""Deterministic SOURCE specification inventory; historical OTS presence is evidence.

No network, native vocabulary admission, execution assumptions or repository writes.
"""
import argparse
import copy
import hashlib
import json
from collections import Counter
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def fingerprint(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()


def currently_unbound(catalogue):
    return [q for q in catalogue['quests'] if not q.get('authored_candidates') and not q.get('family_representation')]


def new_specs(catalogue, previous, coverage):
    old_titles = {e['wiki_title'] for e in previous['entries']}
    historical = {e['title']:e for e in coverage['quests']}
    candidates = [q for q in currently_unbound(catalogue) if q['wiki_title'] not in old_titles]
    entries = []
    for row in candidates:
        title = row['wiki_title']
        sources = row['fresh_sources']
        if not sources or not all(s['available'] for s in sources):
            raise ValueError('Unavailable source must be handled explicitly: '+title)
        for source in sources:
            if not source.get('revid') or not source.get('content_sha256'):
                raise ValueError('Missing exact source revision: '+title)
            semantic = source['source_fields']['semantic_enrichment']
            if semantic['source_revision_sha256'] != source['content_sha256']:
                raise ValueError('Stale source semantic hash: '+title)
        entries.append(dict(
            wiki_title=title,
            historical_donor_status={'canary':historical[title]['canary'],'crystal':historical[title]['crystalserver']},
            source_refs=[{k:s[k] for k in ['provider','pageid','revid','content_sha256','url','relation']} for s in sources],
            source_specification=[dict(relation=s['relation'],provider=s['provider'],semantic_enrichment=copy.deepcopy(s['source_fields']['semantic_enrichment'])) for s in sources],
            source_fields_with_provenance=[{k:s[k] for k in ['provider','relation','pageid','revid','content_sha256','url','target_cut','access_method']} | dict(classification='DERIVED',fields={k:copy.deepcopy(s['source_fields'][k]) for k in ['premium','level','recommended_level','level_note','quest_log','implemented','team','location_entity_references','mission_heading_references','quest_entity_references']}) for s in sources],
            migration_status='BLOCKED_UNKNOWN_BINDINGS',definition_complete=False,runtime_readiness='UNKNOWN',
            unresolved=[dict(field_group=x,classification='UNKNOWN') for x in [
                'Canonical quest and mission identity/binding',
                'Item/outfit/mount/achievement IDs and delivery semantics',
                'Trigger/placement/NPC source identity',
                'Progress/state/cooldown/world-event semantics',
                'Quest Log publication policy']],
            available_model_projection='Pinned structured source rewards, requirements and headings; no executable journey.',
        ))
    return entries


def merge_inventory(catalogue, previous, added):
    by_title = {e['wiki_title']:copy.deepcopy(e) for e in previous['entries']}
    if len(by_title) != len(previous['entries']):
        raise ValueError('Duplicate previous specification')
    for entry in added:
        if entry['wiki_title'] in by_title:
            raise ValueError('Duplicate supplement specification')
        by_title[entry['wiki_title']] = copy.deepcopy(entry)
    required = {q['wiki_title'] for q in currently_unbound(catalogue)}
    if set(by_title) != required:
        raise ValueError('Incomplete current unbound inventory or extraneous title')
    order = [q['wiki_title'] for q in catalogue['quests']]
    entries = [by_title[t] for t in order if t in by_title]
    if any(e['definition_complete'] is not False or e['runtime_readiness']!='UNKNOWN' for e in entries):
        raise ValueError('Source inventory cannot assert native/runtime completeness')
    return dict(schema=previous['schema'],titles=len(entries),entries=entries,no_runtime_promotion=True)


def add_unparsed_holds(entries):
    for entry in entries:
        for source in entry['source_specification']:
            ref = next(r for r in entry['source_refs'] if r['provider']==source['provider'] and r['relation']==source['relation'])
            for hold in source['semantic_enrichment']['unparsed_requirements']:
                entry['unresolved'].append(dict(field_group='Source requirement not parsed',classification='UNKNOWN',reason=hold['reason'],evidence=copy.deepcopy(ref)|dict(line=hold['line'],line_sha256=hold['line_sha256'])))


def generate(samples):
    for filename, expected in read(samples / 'inventory-inputs.json')['inputs'].items():
        if Path(filename).name != filename or hashlib.sha256((samples / filename).read_bytes()).hexdigest() != expected:
            raise ValueError('Pinned inventory input digest differs: ' + filename)
    catalogue = read(samples / 'selection-input.json')
    previous = read(samples / 'previous25.json')
    coverage = read(samples / 'historical-status.json')
    selection = read(samples / 'selection-receipt.json')
    if set(selection['selected_titles']) != {q['wiki_title'] for q in catalogue['quests']}:
        raise ValueError('Pinned selected field snapshot differs')
    added = new_specs(catalogue, previous, coverage)
    add_unparsed_holds(added)
    return merge_inventory(catalogue, previous, added)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--samples', type=Path, default=Path(__file__).parent / 'samples/wiki-source-specs')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = args.samples / 'source-specs-105.json'
    content = json.dumps(generate(args.samples), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if output.read_text() != content:
            raise SystemExit('Pinned SOURCE inventory differs')
    else:
        output.write_text(content)
    print('Pinned SOURCE inventory passed; current source bindings do not change snapshot selection')
