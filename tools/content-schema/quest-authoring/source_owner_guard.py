"""Exact SOURCE association correction for two reviewed symbolic tracks only."""
import copy

STEAL = 'canary:quest-progress/quest/u8_2/the_thieves_guild_quest/steal_from_thieves'
GRAVE = 'canary:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission05'
SIDE = 'oteryn:quest.steal_from_thieves_quest'
GUILD = 'oteryn:quest.the_thieves_guild_quest'
DREFIA = 'oteryn:quest.the_gravedigger_of_drefia_quest'
CORES = {SIDE, GUILD, DREFIA}


def track_effect(track):
    value = copy.deepcopy(track)
    for field in ('auxiliary_of', 'owner_basis', 'note'):
        value.pop(field, None)
    checks = value.get('source_checks', {})
    checks.pop('owner', None)
    if not checks:
        value.pop('source_checks', None)
    return value


def reviewed_pair(root, change, descriptor):
    """Return copies with ONLY approved association snapshots factored out."""
    from source_fix_guard import read_json, verify_file
    if change['key'] not in CORES:
        return change['old_core'], change['new_core']
    verify_file(root, descriptor)
    packet = read_json(root / descriptor['path'])
    if (packet['schema'] != 'OTERYN_REVIEWED_SOURCE_OWNER_ASSOCIATIONS/v1'
            or packet['classification'] != 'OTS_HYPOTHESIS_ONLY'
            or packet['native_runtime_admission'] is not False):
        raise ValueError('owner correction cannot admit Native')
    pairs = packet['approved_core_digests']
    if len(pairs) != 3 or {r['key'] for r in pairs} != CORES:
        raise ValueError('owner correction requires exact three reviewed cores')
    if not any((r['key'], r['from_digest'], r['to_digest']) ==
               (change['key'], change['from_digest'], change['to_digest']) for r in pairs):
        raise ValueError('unapproved owner core pair')
    for field in ('curation', 'evidence'):
        verify_file(root, packet[field])
    curation = read_json(root / packet['curation']['path'])
    additions = curation['additions']
    paths = {STEAL.split(':quest-progress/')[1], GRAVE.split(':quest-progress/')[1]}
    if set(additions) != paths:
        raise ValueError('owner curation must contain only two symbolic tracks')
    if packet['relocated_graph_key'] != 'canary:interaction/others/actions_steal_from_thieves':
        raise ValueError('only the exact StealFromThieves graph can relocate')
    snapshots = {r['old']['key']: r for r in packet['tracks']}
    if len(packet['tracks']) != 2 or set(snapshots) != {STEAL, GRAVE}:
        raise ValueError('owner snapshots must contain exactly two tracks')
    expected = {STEAL: (['canary:quest/the_thieves_guild_quest'], ['canary:quest/steal_from_thieves_quest']),
                GRAVE: ([], ['canary:quest/the_gravedigger_of_drefia_quest'])}
    for key, row in snapshots.items():
        old, new = row['old'], row['new']
        if (track_effect(old) != track_effect(new) or new['key'] != key
                or (old['auxiliary_of'], new['auxiliary_of']) != expected[key]
                or old['owner_basis'] != ('UNKNOWN' if key == GRAVE else 'mission track prefix')
                or new['owner_basis'] != ('exclusive curated NPC writers' if key == GRAVE else 'track_owners.json')
                or (key == GRAVE and not old.get('source_checks', {}).get('owner', '').startswith('UNKNOWN'))
                or 'owner' in new.get('source_checks', {})):
            raise ValueError('owner correction changed effects/provenance or other ownership')
        entry = additions[key.split(':quest-progress/')[1]]
        if entry['wiki_quest'] != ('Steal From Thieves Quest' if key == STEAL else 'The Gravedigger of Drefia Quest'):
            raise ValueError('owner curation belongs to another title')
        occurrences = [o for t in new['transitions'] for o in t['source_occurrences']]
        if len(occurrences) != (6 if key == STEAL else 2):
            raise ValueError('owner write inventory changed')
        proof_fields = ('source', 'repository', 'revision', 'path', 'line', 'blob_sha256', 'line_sha256')
        facts = {tuple(o[f] for f in proof_fields) for o in occurrences}
        witnesses = entry['owner_evidence']
        if len(witnesses) != len(facts) or {tuple(w[f] for f in proof_fields) for w in witnesses} != facts:
            raise ValueError('owner evidence is not an observed write')
    key = change['key']
    membership = {'old': {SIDE: None, GUILD: STEAL, DREFIA: None},
                  'new': {SIDE: STEAL, GUILD: None, DREFIA: GRAVE}}
    result = []
    for side in ('old', 'new'):
        core = copy.deepcopy(change[side + '_core'])
        progress = core['source_data']['progress']
        selected = [p for p in progress if p['key'] in snapshots]
        target = membership[side][key]
        expected_tracks = [] if target is None else [snapshots[target][side]]
        if selected != expected_tracks:
            raise ValueError('owner track membership or occurrence snapshot differs')
        core['source_data']['progress'] = [p for p in progress if p['key'] not in snapshots]
        if (side, key) in (('old', GUILD), ('new', SIDE)):
            graphs = core['source_data']['interactions']
            graph = packet['relocated_graph']
            if graphs.count(graph) != 1 or graph['identity']['key'] != packet['relocated_graph_key']:
                raise ValueError('owner graph relocation must preserve exact source snapshot')
            core['source_data']['interactions'] = [g for g in graphs if g != graph]
        result.append(core)
    return tuple(result)
