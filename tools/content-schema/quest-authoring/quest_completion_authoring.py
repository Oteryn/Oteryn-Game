"""Attach chosen recipes to existing SOURCE identities without erasing fidelity holds."""
import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

DIRECTORY = 'tools/content-schema/quest-authoring/samples/completion242/'
SPECIFICATIONS = 'tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json'


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def validate_journey(recipe):
    stages = recipe['stages']
    keys = [s['key'] for s in stages]
    if len(keys) != len(set(keys)) or len({s['objective'] for s in stages}) != len(stages):
        raise ValueError('duplicate stage or objective')
    if stages[-1]['kind'] != 'complete' or any(s['kind'] == 'complete' for s in stages[:-1]):
        raise ValueError('completion must be sole terminal stage')
    for index, stage in enumerate(stages):
        expected = [keys[index + 1]] if index + 1 < len(keys) else []
        if stage['next'] != expected:
            raise ValueError('cyclic, disconnected or premature terminal stage')
    for reward in recipe['reward_intents']:
        if reward['kind'] == 'none':
            if reward['count'] != 0 or len(recipe['reward_intents']) != 1:
                raise ValueError('none reward must be exclusive and zero')
        elif reward['count'] < 1:
            raise ValueError('reward count must be positive')


def completion_records(root, source_records):
    here = root / 'tools/content-schema/quest-authoring'
    directory = root / DIRECTORY
    selection = read(directory / 'selection.json')
    baseline = read(directory / 'baseline-core-digests.json')
    selected = selection['records']
    if selection['selection_count'] != 242 or len(selected) != 242:
        raise ValueError('completion selection must contain exactly 242 records')
    selected_by_key = {r['identity']['key']: r for r in selected}
    if len(selected_by_key) != 242 or set(baseline) != set(selected_by_key):
        raise ValueError('duplicate or mismatched completion selection')
    source = {r['definition']['identity']['key']: r['definition'] for r in source_records}
    if {k for k, d in source.items() if d['readiness'] == 'waiting_data'} != set(selected_by_key):
        raise ValueError('completion must cover exact waiting SOURCE definitions')
    payload_path = directory / 'recipes.json'
    payload = read(payload_path)
    if payload['base_head'] != selection['base_head'] or payload['runtime_enabled'] is not False:
        raise ValueError('completion baseline or runtime admission differs')
    rows = payload['records']
    if [r['identity'] for r in rows] != [r['identity'] for r in selected]:
        raise ValueError('recipes must cover exact ordered SOURCE selection')
    specifications = {r['wiki_title']: r for r in read(root / SPECIFICATIONS)['entries']}
    catalogue = read(here / 'samples/catalogue/catalogue.json')['quests']
    linked_titles = {}
    for entry in catalogue:
        keys = {r['identity']['key'] for r in entry['authored_candidates']}
        if entry.get('family_representation') and not keys:
            keys.add(entry['family_representation']['target_key'])
        for key in keys:
            linked_titles.setdefault(key, set()).add(entry['wiki_title'])
    git_sources = {canonical(r) for r in read(here / 'samples/questlog/manifest.json')['sources'] if r['kind'] == 'git'}
    validator = Draft202012Validator(read(here / 'quest_completion.schema.json'))
    completed = {}
    for row in rows:
        validator.validate(row)
        key = row['identity']['key']
        original = source[key]
        if digest(original) != baseline[key]:
            raise ValueError('original SOURCE core changed: ' + key)
        if row['source_identity'] != original['source_refs']['quest']:
            raise ValueError('SOURCE owner or revision substitution: ' + key)
        recipe = row['recipe']
        if recipe['wiki_title'] != original['display_name']:
            raise ValueError('recipe belongs to another SOURCE definition')
        expected_refs = specifications.get(recipe['wiki_title'], {}).get('source_refs', [])
        if recipe['source_refs'] != expected_refs:
            raise ValueError('wiki evidence substitution: ' + key)
        if sorted(row['covered_wiki_titles']) != sorted(linked_titles.get(row['source_identity']['key'], [])):
            raise ValueError('false or missing family coverage: ' + key)
        routes = row['title_stage_keys']
        stage_kinds = {s['key']: s['kind'] for s in recipe['stages']}
        if set(routes) != set(row['covered_wiki_titles']) or any(not set(keys) <= set(stage_kinds) for keys in routes.values()):
            raise ValueError('missing or invalid per-title route mapping: ' + key)
        order = {s['key']: i for i, s in enumerate(recipe['stages'])}
        if any(keys != sorted(keys, key=order.get) for keys in routes.values()):
            raise ValueError('family routes must follow actual stage order')
        if len(routes) > 1:
            if len({tuple(keys) for keys in routes.values()}) != len(routes):
                raise ValueError('family titles must have distinct individualized routes')
            if any(all(stage_kinds[k] == 'complete' for k in keys) for keys in routes.values()):
                raise ValueError('family route cannot consist only of generic completion')
        if (not expected_refs and not row['donor_source_refs']) or any(canonical(r) not in git_sources for r in row['donor_source_refs']):
            raise ValueError('missing or substituted donor evidence: ' + key)
        validate_journey(recipe)
        completed[key] = {
            'profile': 'chosen_source_completion_v1', 'chosen_data_complete': True,
            'readiness': 'waiting_native_bindings', 'source_fidelity': 'ORIGINAL_HOLDS_PRESERVED',
            'runtime_enabled': False, 'source_definition_sha256': baseline[key],
            'payload': copy.deepcopy(row),
            'provenance': {'path': DIRECTORY + 'recipes.json',
                           'sha256': hashlib.sha256(payload_path.read_bytes()).hexdigest()},
        }
    return completed


def attach(root, source_records):
    completed = completion_records(root, source_records)
    result = copy.deepcopy(source_records)
    for row in result:
        definition = row['definition']
        if definition['identity']['key'] in completed:
            definition['oteryn_recipe'] = completed[definition['identity']['key']]
    return result


def main():
    import argparse
    import quest_tree_authoring as tree
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    expected = tree.expected_files(root)
    index = json.loads(expected[tree.DIRECTORY + 'index.json'])
    count = sum('oteryn_recipe' in r['definition'] for path in index['shards']
                for r in json.loads(expected[path])['records'])
    if count != 242:
        raise ValueError('completion projection must contain exactly 242 supplements')
    print('242 chosen recipes complete; original SOURCE holds and Native non-admission preserved')


if __name__ == '__main__':
    main()
