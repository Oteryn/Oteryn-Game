"""Validate finite Oteryn quest recipes and project DATA, without Native admission."""
import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

from jsonschema import Draft202012Validator

DIRECTORY = 'tools/content-schema/quest-authoring/samples/authored68/'
SPECIFICATIONS = 'tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json'


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def identity(title):
    slug = re.sub('[^a-z0-9]+', '_', title.lower()).strip('_')
    return {'key': 'oteryn:quest.authored.' + slug, 'revision': 'authored-r1'}


def validate_recipes(selection, recipes, specifications, schema):
    titles = selection['titles']
    if len(titles) != 68 or len(set(titles)) != 68 or selection['selection_count'] != 68:
        raise ValueError('selection must contain exactly 68 unique titles')
    if [r['wiki_title'] for r in recipes] != titles:
        raise ValueError('recipes must cover exact ordered selection')
    specs = {s['wiki_title']: s for s in specifications['entries']}
    keys = [identity(t)['key'] for t in titles]
    if len(keys) != len(set(keys)):
        raise ValueError('authored identity collision')
    validator = Draft202012Validator(schema['$defs']['recipe'])
    for recipe in recipes:
        validator.validate(recipe)
        title = recipe['wiki_title']
        if recipe['source_refs'] != specs[title]['source_refs']:
            raise ValueError('source provenance substitution: ' + title)
        stages = recipe['stages']
        stage_keys = [s['key'] for s in stages]
        if len(stage_keys) != len(set(stage_keys)):
            raise ValueError('duplicate stage: ' + title)
        if len({s['objective'] for s in stages}) != len(stages):
            raise ValueError('repeated objective shell: ' + title)
        if stages[-1]['kind'] != 'complete' or any(s['kind'] == 'complete' for s in stages[:-1]):
            raise ValueError('completion must be the sole terminal stage: ' + title)
        # This profile deliberately admits a finite linear first release only.
        for index, stage in enumerate(stages):
            expected = [stage_keys[index + 1]] if index + 1 < len(stages) else []
            if stage['next'] != expected:
                raise ValueError('unreachable, cyclic or premature terminal stage: ' + title)
        for reward in recipe['reward_intents']:
            if reward['kind'] == 'none' and (reward['count'] != 0 or len(recipe['reward_intents']) != 1):
                raise ValueError('none reward must be exclusive and zero')
            if reward['kind'] != 'none' and reward['count'] < 1:
                raise ValueError('reward count must be positive')
    return recipes


def build_records(root):
    directory = root / DIRECTORY
    selection = read(directory / 'selection.json')
    baseline = directory / 'baseline-rollout.json'
    if digest(baseline) != selection['rollout_sha256']:
        raise ValueError('selection baseline digest mismatch')
    expected = sorted(r['wiki_title'] for r in read(baseline)['records'] if r['binding_scope'] == 'unbound')
    if selection['titles'] != expected:
        raise ValueError('selection does not equal frozen unbound titles')
    path = directory / 'recipes.json'
    schema = read(root / 'tools/content-schema/quest-authoring/quest_authored.schema.json')
    recipes = validate_recipes(selection, read(path)['recipes'], read(root / SPECIFICATIONS), schema)
    receipt = read(directory / 'wiki-access.json')
    fields = ('provider', 'pageid', 'revid', 'content_sha256')
    expected_refs = {tuple(ref[k] for k in fields) for r in recipes for ref in r['source_refs']}
    recorded_refs = {tuple(ref[k] for k in fields) for ref in receipt['sources']}
    if (recorded_refs != expected_refs or len(receipt['sources']) != len(expected_refs)
            or receipt['selection_sha256'] != digest(directory / 'selection.json')
            or any(ref['public_http_status'] != 200 or ref['cached_body_hash_verified'] is not True
                   or ref['access_method'] != 'remote_desktop_browser' for ref in receipt['sources'])):
        raise ValueError('recorded browser access receipt does not cover exact source pins')
    records = []
    for recipe in recipes:
        definition = {
            'identity': identity(recipe['wiki_title']), 'display_name': recipe['wiki_title'],
            'definition_profile': 'oteryn_authored_v1', 'kind': 'storyline',
            'readiness': 'waiting_native_bindings', 'data_complete': True,
            'completeness_scope': 'CHOSEN_OTERYN_RECIPE_ONLY',
            'classification': 'OTERYN_AUTHORED_APPROXIMATION',
            'missing_data': [{'code': 'quest_native_lowering_missing'},
                             {'code': 'authored_trigger_and_delivery_bindings_missing'}],
            'native_lowering': {'state': 'WAITING_IMPLEMENTATION', 'canonical_progress_refs': [],
                               'canonical_interaction_refs': [], 'canonical_reward_refs': []},
            'runtime_enabled': False, 'recipe': copy.deepcopy(recipe),
            'provenance': {'path': DIRECTORY + 'recipes.json', 'sha256': digest(path)},
        }
        Draft202012Validator(schema).validate(definition)
        records.append({'definition': definition})
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    records = build_records(args.root)
    print(f'Authored recipes: {len(records)} complete chosen DATA recipes; Native admission held')


if __name__ == '__main__':
    main()
