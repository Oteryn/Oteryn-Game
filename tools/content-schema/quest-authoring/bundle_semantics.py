"""Source graph closure and gap inventories, without runtime or native admission."""
from collections import Counter, defaultdict
import json
import hashlib
import re
from pathlib import Path
import source_texts

COLLECTIONS = {'quests': 'Quest', 'progress': 'Progress', 'interactions': 'Interaction',
               'gates': 'Gate', 'claims': 'RewardClaim'}


def keyed(data):
    indexes = {}
    for collection in COLLECTIONS:
        rows = data[collection]
        index = {r['key'] if collection == 'progress' else r['identity']['key']: r for r in rows}
        if len(index) != len(rows):
            raise ValueError('duplicate identity in ' + collection)
        indexes[collection] = index
    return indexes


def walk(value, path=''):
    if isinstance(value, dict):
        yield path, value
        for name, child in value.items():
            yield from walk(child, path + '/' + name)
    elif isinstance(value, list):
        for ordinal, child in enumerate(value):
            yield from walk(child, path + '/' + str(ordinal))


def reference_gaps(data):
    indexes = keyed(data)
    families = {family: indexes[name] for name, family in COLLECTIONS.items()}
    gaps = []
    def check(kind, key, field, target_type, target):
        if target not in families[target_type]:
            gaps.append({'record_type': kind, 'record_key': key, 'field': field,
                         'target_type': target_type, 'target_key': target,
                         'reason': 'reference not declared in this source bundle'})
    graphs=[(name,key,record,'') for name,records in indexes.items() for key,record in records.items()]
    graphs += [('interactions',row['interaction'],alt['interaction'],'/conflict_alternatives/'+alt['source'])
               for row in data.get('interaction_source_conflicts',[]) for alt in row['alternatives']]
    for name, key, record, prefix in graphs:
            for path, node in walk(record,prefix):
                if set(node) == {'family', 'key', 'revision'} and node['family'] in families:
                    check(COLLECTIONS[name], key, path, node['family'], node['key'])
                    target = families[node['family']].get(node['key'])
                    if target and target.get('identity', {}).get('revision') != node['revision']:
                        raise ValueError('internal reference revision mismatch: ' + key + path)
                for field, value in node.items():
                    if isinstance(value, str) and ':quest-progress/' in value and field != 'key':
                        check(COLLECTIONS[name], key, path + '/' + field, 'Progress', value)
            if name == 'progress':
                for field in ('start_of', 'auxiliary_of'):
                    for target in record.get(field, []):
                        check('Progress', key, field, 'Quest', target)
                for target in record['read_by_gates']:
                    check('Progress', key, 'read_by_gates', 'Gate', target)
                for target in record['missions']:
                    quest, _, mission = target.partition('#')
                    check('Progress', key, 'missions', 'Quest', quest)
                    if quest in indexes['quests'] and mission not in {m['key'] for m in indexes['quests'][quest].get('missions', [])}:
                        raise ValueError('progress names absent mission: ' + target)
    return sorted({json.dumps(g, sort_keys=True) for g in gaps})


def source_requester(occurrence):
    """Bind requester facts to the exact occurrence rather than the primary donor source."""
    dialogue = occurrence['dialogue']
    def keyword(value):
        if isinstance(value, dict) or (len(value.split()) <= 2 and len(value) <= 20):
            return value
        placeholders = sorted(set(re.findall(r'%[-0-9.]*[dsif]|\|[A-Z_]+\|', value)))
        return {'sha256': hashlib.sha256(value.encode('utf-8')).hexdigest(),
                'length': len(value), 'placeholders': placeholders}
    namespace = 'canary' if occurrence['source'] == 'canary' else 'crystal'
    return {'npc': namespace + ':npc/' + Path(occurrence['path']).stem,
            'keywords': [keyword(k) for k in dialogue['keywords']], 'topics': dialogue['topics']}


def validate_relations(data):
    indexes = keyed(data)
    linked = {(key, c['quest']['key']) for key, c in indexes['claims'].items()
              if c.get('quest') and c['quest']['key'] in indexes['quests']}
    listed = {(ref['key'], key) for key, q in indexes['quests'].items()
              for ref in q['claims'] if ref['key'] in indexes['claims']}
    if linked != listed:
        raise ValueError('non-reciprocal source quest/claim links')
    tracks = indexes['progress']
    for key, track in tracks.items():
        visited, current = set(), key
        while current in tracks and tracks[current].get('alias_of'):
            if current in visited:
                raise ValueError('progress alias cycle: ' + key)
            visited.add(current)
            current = tracks[current]['alias_of']
        if track.get('alias_of'):
            target=track['alias_of']
            if target.split(':quest-progress/',1)[1]!=key.split(':quest-progress/',1)[1]:
                raise ValueError('progress alias conflates distinct source paths: '+key)
            if target in tracks and (track['transitions']!=tracks[target]['transitions'] or track['writes']!=tracks[target]['writes']):
                raise ValueError('progress alias write evidence differs from original: '+key)
        occurrences=[o for t in track['transitions'] for o in t['source_occurrences']]
        counts=Counter(o['source'] for o in occurrences)
        if any(counts[source]!=count for source,count in track['writes'].items()):
            raise ValueError('source occurrences do not cover every observed write: '+key)
        for source,count in track['writes'].items():
            ordinals=[o['occurrence'] for o in occurrences if o['source']==source]
            if sorted(ordinals)!=list(range(1,count+1)):
                raise ValueError('duplicate or missing source occurrence ordinal: '+key)
        for transition in track['transitions']:
            write = transition['write']
            if transition['key'] != write['key'] or set(transition['sources']) != set(write['servers']):
                raise ValueError('transition source/write descriptor mismatch: ' + key)
            if not transition['script']:
                raise ValueError('transition missing source script: ' + key)
            occurrences=transition['source_occurrences']
            if {o['source'] for o in occurrences}!=set(transition['sources']):
                raise ValueError('source occurrence provider mismatch: '+key)
            for occurrence in occurrences:
                local_write=occurrence['write']
                if {k:v for k,v in local_write.items() if k not in ('servers','requested_by')}!={k:v for k,v in write.items() if k not in ('servers','requested_by')}:
                    raise ValueError('source occurrence effect/context mismatch: '+key)
                if local_write['servers']!=[occurrence['source']]:
                    raise ValueError('source occurrence namespace mismatch: '+key)
                if local_write['owner']=='npc' and local_write.get('requested_by')!=source_requester(occurrence):
                    raise ValueError('source occurrence requester context mismatch: '+key)
                if re.sub(r'^(data-otservbr-global|data-global|data-crystal|data)/','',occurrence['path'])!=transition['script']:
                    raise ValueError('source occurrence script mismatch: '+key)
            for source,primary in transition['sources'].items():
                if not any(o['source']==source and all(o.get(k)==v for k,v in primary.items()) for o in occurrences):
                    raise ValueError('primary source missing from occurrences: '+key)
    return reference_gaps(data)


def derive(data, evidence):
    indexes = keyed(data)
    missing = [json.loads(row) for row in reference_gaps(data)]
    by_quest = defaultdict(list)
    unassigned = []
    def add(owners, kind, record, reason):
        gap = {'kind': kind, 'record': record, 'reason': reason}
        owners = set(owners) & indexes['quests'].keys()
        if not owners:
            unassigned.append(gap)
        for owner in owners:
            by_quest[owner].append(gap)
    progress_owners = {}
    for key, track in indexes['progress'].items():
        owners = set(track.get('start_of', []) + track.get('auxiliary_of', []))
        owners.update(m.split('#', 1)[0] for m in track['missions'])
        owners.update(indexes['gates'][g]['quest']['key'] for g in track['read_by_gates']
                      if g in indexes['gates'] and indexes['gates'][g].get('quest'))
        progress_owners[key] = owners
        for field, reason in track.get('source_checks', {}).items():
            add(owners, 'source_progress_unknown', key + '/' + field, reason)
        represented=Counter(o['source'] for t in track['transitions'] for o in t['source_occurrences'])
        for source,count in track['writes'].items():
            if represented[source]<count:
                add(owners,'source_write_evidence_gap',key+'/'+source,
                    str(count-represented[source])+' source writes lack distinct line descriptors after transition coalescing')
        if any(t['write'].get('computed') for t in track['transitions']):
            add(owners, 'source_effect_not_closed', key,
                'computed source effect preserved; no closed native effect is inferred')
    interaction_owners = defaultdict(set)
    for key, interaction in indexes['interactions'].items():
        for _, node in walk(interaction):
            for value in node.values():
                if isinstance(value, str) and value in progress_owners:
                    interaction_owners[key].update(progress_owners[value])
            if node.get('family') == 'RewardClaim' and node.get('key') in indexes['claims']:
                claim = indexes['claims'][node['key']]
                if claim.get('quest'):
                    interaction_owners[key].add(claim['quest']['key'])
        for issue in interaction['unresolved']:
            add(interaction_owners[key], 'source_interaction_gap', key, issue['reason'])
    for conflict in data.get('interaction_source_conflicts',[]):
        owners=set(interaction_owners[conflict['interaction']])
        for alternative in conflict['alternatives']:
            for _,node in walk(alternative['interaction']):
                for value in node.values():
                    if isinstance(value,str) and value in progress_owners:owners.update(progress_owners[value])
                if node.get('family')=='RewardClaim' and node.get('key') in indexes['claims']:
                    claim=indexes['claims'][node['key']]
                    if claim.get('quest'):owners.add(claim['quest']['key'])
        interaction_owners[conflict['interaction']].update(owners)
        add(owners,'source_coverage_hold',conflict['interaction'],'CONFLICT: full donor alternative interaction graphs preserved; no native effect chosen')
    for gap in source_texts.unresolved_gaps(data):
        add(gap['owners'],'source_text_unknown',gap['record'],gap['reason'])
    for hold in evidence['coverage_holds']:
        add([hold['quest']], 'source_coverage_hold', hold['record'], hold['reason'])
    for hold in evidence['deferred_owners']:
        add([hold['quest']] if hold['quest'] else [], 'source_owner_deferred', hold['record'], hold['reason'])
    reported = {r['quest']: r for r in evidence['reported_readiness']}
    for key in indexes['quests']:
        row = reported.get(key)
        if row is None:
            add([key], 'source_readiness_missing', key, 'No source readiness row; data coverage UNKNOWN')
        else:
            for field, count in row['data_gaps'].items():
                if count:
                    add([key], 'reported_source_gap', key + '/' + field, str(count) + ' source gaps reported')
    for gap in missing:
        key, family = gap['record_key'], gap['record_type']
        owners = ([key] if family == 'Quest' else progress_owners.get(key, []) if family == 'Progress'
                  else interaction_owners.get(key, []) if family == 'Interaction'
                  else [indexes['gates'][key]['quest']['key']] if family == 'Gate' and indexes['gates'][key].get('quest')
                  else [indexes['claims'][key]['quest']['key']] if family == 'RewardClaim' and indexes['claims'][key].get('quest') else [])
        add(owners, 'source_reference_gap', key + gap['field'], gap['target_key'])
    for row in data['wiki_catalogue']['quests']:
        owners = [q['identity']['key'] for q in row['authored_candidates']]
        for check in row['source_checks']:
            add(owners, 'wiki_source_conflict', row['wiki_title'], check['kind'])
    dedupe = lambda rows: [json.loads(row) for row in sorted({json.dumps(r, sort_keys=True) for r in rows})]
    quests = [{'quest': key, 'runtime_readiness': 'UNKNOWN', 'coverage_completeness': 'NOT_ASSESSED',
               'reported_source_readiness': reported.get(key), 'gaps': dedupe(by_quest[key])}
              for key in sorted(indexes['quests'])]
    return {'reference_gaps': missing, 'quest_gaps': quests, 'unassigned_gaps': dedupe(unassigned),
            'summary': {**{name: len(data[name]) for name in COLLECTIONS},
                        'wiki_quests': len(data['wiki_catalogue']['quests']),
                        'quests_with_reported_gaps': sum(bool(q['gaps']) for q in quests),
                        'reference_gaps': len(missing), 'unassigned_gaps': len(dedupe(unassigned)),
                        'progress_source_transitions': sum(len(t['transitions']) for t in data['progress'] if not t.get('alias_of')),
                        'source_write_occurrences': sum(len(e['source_occurrences']) for t in data['progress'] if not t.get('alias_of') for e in t['transitions'])}}
