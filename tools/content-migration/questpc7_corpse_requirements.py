"""Finite source corpse coverage for every active QuestPC7 death objective.

Unknown/new/changed objective identities fail instead of silently disappearing.
Closed entries retain the source uncertainty and grant no Item admission.
"""
import json

PATH = 'tools/content-schema/native-gameplay/questpc7-corpse-objective-coverage.json'
DOCUMENT = 'content/quests/questpc7-death-objectives.json'
MAX_OBJECTIVES = 128


def roots(root, creatures):
    coverage = json.loads((root / PATH).read_bytes())
    document = json.loads((root / DOCUMENT).read_bytes())
    if (set(coverage) != {'schema', 'objective_document', 'objectives'}
            or coverage['schema'] != 1 or coverage['objective_document'] != DOCUMENT
            or document.get('schema') != 1
            or not 1 <= len(document['objectives']) <= MAX_OBJECTIVES
            or not 1 <= len(coverage['objectives']) <= MAX_OBJECTIVES):
        raise ValueError('QUESTPC7_CORPSE_COVERAGE_SCHEMA')
    declared = {row['objective']: row for row in coverage['objectives']}
    actual = {row['key']: row for row in document['objectives']}
    if (len(declared) != len(coverage['objectives'])
            or len(actual) != len(document['objectives']) or declared.keys() != actual.keys()):
        raise ValueError('QUESTPC7_CORPSE_OBJECTIVE_COVERAGE')
    selected = set()
    native = None
    for key, objective in actual.items():
        row = declared[key]
        if (set(row) != {'objective', 'creature', 'corpse', 'disposition', 'reason'}
                or row['creature'] != objective['creature'] or not row['reason']):
            raise ValueError('QUESTPC7_CORPSE_OBJECTIVE_BINDING')
        profile = creatures.get(objective['creature'])
        if (profile is None or profile['target'] != {
                'family': 'Creature', 'key': row['creature'], 'revision': 'definition-r1'}):
            raise ValueError('QUESTPC7_CORPSE_CREATURE_BINDING')
        corpse = profile['data']['profile']['details'].get('corpse_item')
        if corpse != row['corpse']:
            raise ValueError('QUESTPC7_CORPSE_SOURCE_BODY_CHANGED')
        if row['disposition'] == 'closed':
            continue
        if corpse is None:
            raise ValueError('QUESTPC7_CORPSE_SOURCE_ABSENT')
        if row['disposition'] == 'existing':
            if native is None:
                raw = json.loads((root / 'content/items/definitions/spell-native-profiles.json').read_bytes())
                native = {r['production_definition']['production_key']: r for r in raw['records']}
            existing = native.get(corpse['key'])
            if (existing is None or existing['production_binding']['target'] != corpse
                    or existing['production_definition'] != {'family': corpse['family'],
                        'production_key': corpse['key'], 'revision_ref': corpse['revision']}
                    or existing['admission']['materializable'] is not True):
                raise ValueError('QUESTPC7_CORPSE_EXISTING_OWNER_CHANGED')
        elif row['disposition'] == 'qualify':
            import quest_corpse_profiles
            selected.add(quest_corpse_profiles.item_id(corpse))
        else:
            raise ValueError('QUESTPC7_CORPSE_COVERAGE_DISPOSITION')
    return selected
