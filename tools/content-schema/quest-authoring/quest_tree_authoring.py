"""Populate the data-only Quest tree from all authored source quest kinds.

Source aliases remain evidence. Canonical claims resolve only against their exact
committed provenance. This tool registers the scoped Quest family; it never declares runtime readiness.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/questlog/quests.json'
CLAIM_INDEX = 'content/interactions/reward_claims/index.json'
MANIFEST = 'tools/content-schema/quest-authoring/samples/chests/manifest.json'
READINESS = 'tools/content-schema/quest-authoring/samples/readiness/readiness.json'
GATES = 'tools/content-schema/quest-authoring/samples/doors/gates.json'
GATE_MANIFEST = 'tools/content-schema/quest-authoring/samples/doors/manifest.json'
PROGRESS = 'tools/content-schema/quest-authoring/samples/questlog/progress.json'
INTERACTIONS = 'tools/content-schema/quest-authoring/samples/interactions/interactions.json'
BUNDLE = 'tools/content-schema/quest-authoring/samples/source_migration/bundle.json'
INTERACTION_MANIFEST = 'tools/content-schema/quest-authoring/samples/interactions/manifest.json'
DIRECTORY = 'content/quests/definitions/'
REVISION = 'quest-r1'
SHARD_SIZE = 100
QUEST_KEY = re.compile(r'^oteryn:quest\.[a-z0-9_.-]+$')
CANONICAL_CLAIM_KEY = re.compile(r'^oteryn:reward-claim\.[a-z0-9_.-]+$')


def compact(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n'


def read(root, relative):
    return json.loads((root / relative).read_text(encoding='utf-8'))


def digest(root, relative):
    return hashlib.sha256((root / relative).read_bytes()).hexdigest()


def claim_catalogue(root):
    index = read(root, CLAIM_INDEX)
    return [row['definition'] for shard in index['shards'] for row in read(root, shard)['records']]


def source_identity(ref):
    return ref['key'], ref['revision']


def build_records(quests, claims, readiness_reports=None, gates=None, gate_checks=None, source_data=None):
    """No guessed identity joins, missing requirements, quantities or readiness."""
    bindings = {}
    for claim in claims:
        provenance = claim['provenance']
        source = (provenance['pilot_key'], provenance['pilot_revision'])
        if source in bindings:
            raise ValueError(f'duplicate RewardClaim source identity: {source}')
        if not CANONICAL_CLAIM_KEY.fullmatch(claim['identity']['key']):
            raise ValueError('committed RewardClaim identity is not canonical')
        bindings[source] = claim
    records, identities = [], set()
    for quest in sorted(quests, key=lambda q: (q['kind'] != 'reward_only', q['identity']['key'])):
        # The converter has one identity per wiki quest, irrespective of source namespace.
        marker = quest['identity']['key'].split(':quest/', 1)[-1].replace('/', '.')
        key = f'oteryn:quest.{marker}'
        if ':quest/' not in quest['identity']['key'] or not QUEST_KEY.fullmatch(key):
            raise ValueError(f'unsupported source quest identity: {quest["identity"]}')
        if key in identities:
            raise ValueError(f'duplicate canonical Quest identity: {key}')
        identities.add(key)
        source_refs = {'quest': {'family': 'Quest', **copy.deepcopy(quest['identity'])}, 'claims': copy.deepcopy(quest['claims'])}
        canonical, issues, unresolved = [], [], []
        if quest['kind'] != 'storyline' and quest['shown_in_quest_log']:
            issues.append({'code': 'source_kind_log_flag_conflict'})
        report = readiness_reports.get(quest['identity']['key']) if readiness_reports is not None else None
        if readiness_reports is not None and report is None:
            issues.append({'code': 'source_readiness_missing'})
        if report:
            for gap, count in sorted(report['data_gaps'].items()):
                if count:
                    issues.append({'code': 'reported_source_gap', 'field': gap, 'count': count})
        bound_gates = [g for g in (gates or []) if g.get('quest') and source_identity(g['quest']) == source_identity(quest['identity'])]
        source_refs['gates'] = [{'family': 'Gate', **copy.deepcopy(g['identity'])} for g in bound_gates]
        gate_keys = {g['identity']['key'] for g in bound_gates}
        for check in (gate_checks or []):
            if check.get('destination') in gate_keys and check['status'] == 'unresolved_semantics':
                issues.append({'code': 'gate_source_gap', 'source_key': check['destination']})
        for ref in quest['claims']:
            if ref['family'] != 'RewardClaim':
                raise ValueError(f'{key}: non-RewardClaim source reference')
            claim = bindings.get(source_identity(ref))
            if claim is None:
                unresolved.append(copy.deepcopy(ref))
                issues.append({'code': 'claim_content_missing', 'source_key': ref['key']})
                continue
            owner = claim.get('quest')
            if not owner or owner['family'] != 'Quest' or source_identity(owner) != source_identity(quest['identity']):
                raise ValueError(f'{key}: committed claim owner does not match source quest: {ref["key"]}')
            canonical.append({'family': 'RewardClaim', **copy.deepcopy(claim['identity'])})
            if claim.get('definition_profile') == 'authored_variant_v1':
                issues.append({'code': 'claim_native_lowering_missing', 'source_key': ref['key']})
                for category in sorted({hold['category'] for hold in claim['data_holds']}):
                    code = {'source': 'claim_source_data_missing', 'item': 'claim_item_semantics_missing'}[category]
                    issues.append({'code': code, 'source_key': ref['key']})
            elif claim['readiness'] != 'ready':
                code = 'claim_native_lowering_missing' if claim['readiness'] == 'waiting_implementation' else 'claim_item_semantics_missing'
                issues.append({'code': code, 'source_key': ref['key']})
        requirements = copy.deepcopy(quest.get('requirements', {}))
        for field in ('premium', 'min_level'):
            if requirements.get(field) is None:
                issues.append({'code': 'requirement_unknown', 'field': field})
        if quest['kind'] == 'reward_only' and not quest['claims']:
            raise ValueError(f'{key}: reward_only Quest has no source claims')
        record = {
            'identity': {'key': key, 'revision': REVISION},
            'display_name': quest['display_name'], 'kind': quest['kind'],
            'shown_in_quest_log': quest['shown_in_quest_log'],
            'requirements': requirements, 'requirements_from_wiki': copy.deepcopy(quest.get('requirements_from_wiki', {})),
            'requirements_unparsed': copy.deepcopy(quest.get('requirements_unparsed', {})),
            'claims': canonical, 'unresolved_claims': unresolved,
            'readiness': 'waiting_data' if issues else 'definition_ready',
            'completeness': 'NOT_ASSESSED',
            'reported_source_readiness': copy.deepcopy(report) if report else None,
            'missing_data': issues, 'source_refs': source_refs,
            'classification': 'OTS_HYPOTHESIS_ONLY',
        }
        if quest['kind'] != 'reward_only':
            issues.append({'code': 'quest_native_lowering_missing'})
            record['readiness'] = 'waiting_data'
            record['source_data'] = copy.deepcopy((source_data or {}).get(quest['identity']['key'], {
                'quest': quest, 'progress': [], 'interactions': [], 'gates': bound_gates, 'interaction_source_conflicts': [], 'reference_gaps': []}))
            for gap in record['source_data']['reference_gaps']:
                issues.append({'code': 'source_reference_missing', 'source_key': gap['target_key']})
            record['native_lowering'] = {'state': 'WAITING_IMPLEMENTATION', 'canonical_progress_refs': [],
                'canonical_interaction_refs': [], 'reason': 'QuestState tracks, bounds, transitions and requested-by bindings have not been lowered'}
        if quest.get('wiki'):
            record['wiki'] = copy.deepcopy(quest['wiki'])
        records.append({'definition': record})
    return records


def validate(records, quests, claims, readiness_reports=None, gates=None, gate_checks=None, source_data=None):
    """Validate exact regenerated content, including source ownership and missing-data truth."""
    expected = build_records(quests, claims, readiness_reports, gates, gate_checks, source_data)
    errors = []
    if records != expected:
        errors.append('Quest records differ from the source-bound Quest projection')
    return errors


def registered(project, manifest, lock, count, paths):
    """Regenerate only this family using the existing tree authoring seal pattern."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if 'Quest' not in project['migrated_families']:
        project['migrated_families'].append('Quest')
    project['next_population_families'] = [f for f in project['next_population_families'] if f != 'Quest']
    manifest['families']['Quest'] = {'records': count, 'index': DIRECTORY + 'index.json'}
    managed = {entry['path'] for entry in manifest['managed_files']}
    managed = {path for path in managed if not path.startswith(DIRECTORY)} | set(paths)
    manifest['managed_files'] = [{'path': path} for path in sorted(managed)]
    lock['family_counts']['Quest'] = count
    return project, manifest, lock


def bound_source_data(root, quests, gates):
    """Reuse SOURCE ownership joins; never turn aliases into production references."""
    from ots_readiness import join_interactions
    progress = read(root, PROGRESS)['progress']
    interactions = read(root, INTERACTIONS)['interactions']
    bundle = read(root, BUNDLE)
    conflicts = bundle['interaction_source_conflicts']
    joined = join_interactions(quests, progress, interactions)
    owners = {key: set(value) for key, value in joined.items()}
    for conflict in conflicts:
        key = conflict['interaction']
        for alternative in conflict['alternatives']:
            graph = alternative['interaction']
            projected = dict(graph, identity=dict(graph['identity'], key=key))
            owners[key].update(join_interactions(quests, progress, [projected])[key])
    result = {}
    for quest in quests:
        key = quest['identity']['key']
        tracks = {m['progress'] for m in quest.get('missions', [])}
        if quest.get('start'):
            tracks.add(quest['start']['progress'])
        selected = [p for p in progress if p['key'] in tracks or key in p['start_of']
                    or key in p.get('auxiliary_of', []) or any(m.startswith(key + '#') for m in p['missions'])]
        result[key] = {'quest': copy.deepcopy(quest), 'progress': copy.deepcopy(selected),
                       'interactions': [copy.deepcopy(i) for i in interactions if key in owners[i['identity']['key']]],
                       'gates': [copy.deepcopy(g) for g in gates if g.get('quest') and source_identity(g['quest']) == source_identity(quest['identity'])],
                       'interaction_source_conflicts': [copy.deepcopy(c) for c in conflicts if key in owners[c['interaction']]],
                       'reference_gaps': [copy.deepcopy(g) for g in bundle['reference_gaps'] if g['record_key'] == key
                                          or any(g['record_key'] == p['key'] for p in selected)
                                          or key in owners.get(g['record_key'], set())]}
    return result


def tree_validator():
    """Resolve the existing typed SOURCE schemas offline, with no copied vocabulary."""
    import jsonschema
    from referencing import Registry, Resource
    here = Path(__file__).parent
    registry = Registry()
    for name in ('quest_content', 'quest_progress', 'interaction', 'quest_bundle', 'quest_authored', 'quest_completion'):
        schema = json.loads((here / (name + '.schema.json')).read_text(encoding='utf-8'))
        registry = registry.with_resource(schema['$id'], Resource.from_contents(schema))
    schema = json.loads((here / 'quest_tree.schema.json').read_text(encoding='utf-8'))
    return jsonschema.Draft202012Validator(schema, registry=registry)


def expected_files(root, include_registration=False):
    quests = read(root, SOURCE)['quests']
    claims = claim_catalogue(root)
    reports = {q['quest']: q for q in read(root, READINESS)['quests']}
    gates = read(root, GATES)['gates']
    records = build_records(quests, claims, reports, gates, read(root, GATE_MANIFEST)['entries'], bound_source_data(root, quests, gates))
    from quest_completion_authoring import attach
    records = attach(root, records)
    from authored_quest_authoring import build_records as authored_records
    records.extend(authored_records(root))
    files, shards = {}, []
    for number, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start:start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f'{DIRECTORY}quests-{start:05d}-{end:05d}.json'
        shards.append(path)
        files[path] = compact({'schema': 'OTERYN_QUEST_TREE_SHARD/v1', 'family': 'Quest', 'shard': {'index': number, 'start': start, 'end': end, 'count': len(chunk)}, 'records': chunk})
    missing = Counter(issue['code'] for row in records for issue in row['definition']['missing_data'])
    by_kind = Counter(q['kind'] for q in quests)
    files[DIRECTORY + 'index.json'] = compact({
        'schema': 'OTERYN_FAMILY_INDEX/v1', 'family': 'Quest', 'population_state': 'POPULATED',
        'record_count': len(records), 'shards': shards, 'scope': 'source_quests_and_explicit_oteryn_recipes',
        'classification': 'SOURCE_AND_CHOSEN_OTERYN_DATA',
        'source_classification': 'OTS_HYPOTHESIS_ONLY',
        'readiness': dict(sorted(Counter(row['definition']['readiness'] for row in records).items())),
        'missing_data': dict(sorted(missing.items())),
        'chosen_source_completion': {'records': 242, 'scope': 'CHOSEN_OTERYN_RECIPE_ONLY', 'native_admission': 'WAITING_IMPLEMENTATION'},
        'definition_profiles': {'donor_source': len(quests), 'oteryn_authored_v1': len(records) - len(quests)},
        'catalogue_coverage': {'source_records': len(quests), 'included_records': len(quests), 'excluded_by_kind': {}, 'included_by_kind': dict(sorted(by_kind.items())), 'complete': True},
        'runtime_readiness': 'NOT_ASSESSED',
        'readiness_scope': 'definition_fields_and_known_source_gaps_only',
        'quest_completeness': 'NOT_ASSESSED',
        'authoring_source': {'path': SOURCE, 'sha256': digest(root, SOURCE)},
        'authoring_sources': [{'path': path, 'sha256': digest(root, path)} for path in (SOURCE, CLAIM_INDEX, MANIFEST, READINESS, GATES, GATE_MANIFEST, INTERACTION_MANIFEST, PROGRESS, INTERACTIONS, BUNDLE,
            'tools/content-schema/quest-authoring/ots_readiness.py',
            'tools/content-schema/quest-authoring/quest_tree_authoring.py',
            *[f'tools/content-schema/quest-authoring/{n}.schema.json' for n in ('quest_tree', 'quest_content', 'quest_progress', 'interaction', 'quest_bundle')],
            *read(root, CLAIM_INDEX)['shards'],
            'tools/content-schema/quest-authoring/quest_authored.schema.json',
            'tools/content-schema/quest-authoring/authored_quest_authoring.py',
            'tools/content-schema/quest-authoring/samples/authored68/selection.json',
            'tools/content-schema/quest-authoring/samples/authored68/baseline-rollout.json',
            'tools/content-schema/quest-authoring/samples/authored68/recipes.json',
            'tools/content-schema/quest-authoring/samples/authored68/wiki-access.json',
            'tools/content-schema/quest-authoring/quest_completion_authoring.py',
            'tools/content-schema/quest-authoring/quest_completion.schema.json',
            'tools/content-schema/quest-authoring/source_fix_guard.py',
            'tools/content-schema/quest-authoring/samples/completion242/source-fix-receipt.json',
            'tools/content-schema/quest-authoring/samples/completion242/source-fix-approval.json',
            'tools/content-schema/quest-authoring/samples/completion242/selection.json',
            'tools/content-schema/quest-authoring/samples/completion242/baseline-core-digests.json',
            'tools/content-schema/quest-authoring/samples/completion242/recipes.json',
            'tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json')],
        'source_refs': read(root, MANIFEST)['sources'],
        'contract': 'docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md',
    })
    if include_registration:
        values = registered(read(root, 'content/project.json'), read(root, 'content/manifest.json'), read(root, 'content/content.lock.json'), len(records), list(files))
        for path, value in zip(('content/project.json', 'content/manifest.json', 'content/content.lock.json'), values):
            files[path] = json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
    validator = tree_validator()
    for path in shards:
        validator.validate(json.loads(files[path]))
    return files


def validate_source_packet(packet, root):
    """Resolve the existing Quest vocabulary locally, without network retrieval."""
    import jsonschema
    from referencing import Registry, Resource
    schema_path = Path(__file__).with_name('quest_source_packet.schema.json')
    schema = json.loads(schema_path.read_text(encoding='utf-8'))
    source_schema = read(root, 'tools/content-schema/quest-authoring/quest_content.schema.json')
    registry = Registry().with_resource(source_schema['$id'], Resource.from_contents(source_schema))
    jsonschema.Draft202012Validator(schema, registry=registry).validate(packet)
    source_quests = [record['source_quest'] for record in packet['records']]
    identities = [quest['identity']['key'] for quest in source_quests]
    if len(identities) != len(set(identities)):
        raise ValueError('source packet contains duplicate Quest identities')
    if packet['record_count'] != len(source_quests):
        raise ValueError('source packet record_count differs from actual records')
    if packet['by_kind'] != dict(sorted(Counter(q['kind'] for q in source_quests).items())):
        raise ValueError('source packet by_kind differs from actual records')
    if packet['source_schema_sha256'] != digest(root, packet['source_schema']):
        raise ValueError('source packet Quest schema digest differs from its local binding')
    for record in packet['records']:
        report = record['reported_source_readiness']
        quest = record['source_quest']
        if report and (report['quest'] != quest['identity']['key'] or report['kind'] != quest['kind']):
            raise ValueError('source packet readiness report does not bind to its quest')


def source_catalogue_packet(root):
    """All source quest kinds in full original shape; no canonical admission claims."""
    source = read(root, SOURCE)
    reports = {q['quest']: q for q in read(root, READINESS)['quests']}
    packet = {
        'schema': 'OTERYN_QUEST_SOURCE_MIGRATION_PACKET/v1',
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'record_count': len(source['quests']),
        'by_kind': dict(sorted(Counter(q['kind'] for q in source['quests']).items())),
        'canonical_admission': 'NOT_ASSESSED', 'quest_completeness': 'NOT_ASSESSED',
        'source_schema': 'tools/content-schema/quest-authoring/quest_content.schema.json',
        'source_schema_sha256': digest(root, 'tools/content-schema/quest-authoring/quest_content.schema.json'),
        'source_refs': [
            {'path': path, 'sha256': digest(root, path)} for path in
            (SOURCE, READINESS, GATES, GATE_MANIFEST, INTERACTION_MANIFEST,
             'tools/content-schema/quest-authoring/samples/questlog/progress.json',
             'tools/content-schema/quest-authoring/samples/interactions/interactions.json',
             'tools/content-schema/quest-authoring/quest_content.schema.json',
             'tools/content-schema/quest-authoring/samples/questlog/manifest.json',
             'tools/content-schema/quest-authoring/samples/chests/claims.json',
             'tools/content-schema/quest-authoring/samples/chests/manifest.json')
        ],
        'records': [
            {'source_quest': copy.deepcopy(q), 'reported_source_readiness': copy.deepcopy(reports.get(q['identity']['key']))}
            for q in source['quests']
        ],
    }
    validate_source_packet(packet, root)
    return packet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['content'], nargs='?', default='content')
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--output', type=Path, help='output root for Quest shards/index and regenerated family registries')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--source-packet', type=Path, help='optional full source catalogue packet; no canonical readiness')
    args = parser.parse_args()
    files = expected_files(args.root, include_registration=True)
    output = args.output or args.root
    differences = []
    managed = set(files)
    for old in (output / DIRECTORY).glob('quests-*.json'):
        if not re.fullmatch(r'quests-[0-9]{5}-[0-9]{5}\.json', old.name):
            continue
        if old.relative_to(output).as_posix() not in managed:
            if args.check:
                differences.append(old.relative_to(output).as_posix())
            else:
                old.unlink()
    for relative, text in files.items():
        path = output / relative
        if args.check:
            if not path.exists() or path.read_text(encoding='utf-8') != text:
                differences.append(relative)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding='utf-8')
    if differences:
        parser.exit(1, 'Quest tree differences: ' + ', '.join(differences) + '\n')
    if args.source_packet:
        packet = compact(source_catalogue_packet(args.root))
        if args.check:
            if not args.source_packet.exists() or args.source_packet.read_text(encoding='utf-8') != packet:
                parser.exit(1, 'Quest source packet differs from current inputs\n')
        else:
            args.source_packet.parent.mkdir(parents=True, exist_ok=True)
            args.source_packet.write_text(packet, encoding='utf-8')
    print(f'Quest tree: {len(files)-4} shards; check={args.check}; family registries regenerated')


if __name__ == '__main__':
    main()
