"""Oteryn lifecycle bindings for five standalone, once-only source chest quests."""

BINDINGS = {
    'kv/adoreduh': 'oteryn:quest.adorned_uh_rune_quest',
    'kv/barbarianaxe1': 'oteryn:quest.barbarian_axe_quest',
    'kv/battleaxe': 'oteryn:quest.battle_axe_quest',
    'kv/bloodherb': 'oteryn:quest.blood_herb_quest',
    'kv/captain_iglues': 'oteryn:quest.captain_iglues_treasure_quest',
}


def append(quests, records):
    definitions = {r['identity']['key']: r for r in records}
    existing = {q['quest'] for q in quests}
    for marker, key in BINDINGS.items():
        if key not in definitions:
            raise ValueError('Missing owned standalone quest: ' + key)
        if key in existing:
            raise ValueError('Standalone quest already has a progress programme: ' + key)
        track = 'oteryn:quest-progress/' + marker + '/claimed'
        quests.append({
            'quest': key, 'source_quest': definitions[key]['identity'],
            'completion': 'standalone_source_claim',
            'tracks': [{'key': track, 'quest': key, 'initial': 0, 'min': 0, 'max': 1,
                        'bounds_basis': 'once_only_source_claim',
                        'source_key': 'oteryn:reward-claim/' + marker}],
            'transitions': [{
                'key': 'oteryn:quest-transition/' + marker + '/chest',
                'quest': key, 'completes': True,
                'effects': [{'track': track, 'from': {'op': 'EQ', 'value': 0},
                             'from_exact': True, 'effect': {'kind': 'SET', 'value': 1}}],
                'requested_by': None,
                'source': {'key': 'chest', 'owner': 'chest',
                           'basis': 'OTERYN_STANDALONE_ONCE_CLAIM_COMPLETION',
                           'claim': 'oteryn:reward-claim.' + marker.replace('/', '.')}}],
        })
