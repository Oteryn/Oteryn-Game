"""Assemble reproducible per-quest binding evidence from existing local inputs.

This is authoring reference data, not a runtime catalogue or completion verdict.
No network acquisition and no Source hold or Native admission mutation occurs.
"""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import tempfile

import jsonschema

DIRECTORY = 'tools/content-schema/quest-authoring/samples/binding_packets/'
OUTPUTS = ('source/crosswalk.json', 'rewards/perquest-352.json',
           'rewards/reward-only-105.json', 'rewards/reward-holds.json',
           'rewards/requirement-choices.json', 'rewards/chosen-reward-item-candidates.json',
           'npc/quest-npc-dialogue-links.json', 'npc/authored68-npc-links.json',
           'recipes/supplementary-index.json')


def read(path):
    return json.loads(path.read_bytes())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_index(root, scratch, manifest):
    schema = read(Path(__file__).resolve().with_name('quest_binding_index.schema.json'))
    jsonschema.Draft202012Validator(schema).validate(manifest)
    if manifest['summary']['definitions_with_substantive_joins'] != sum(
            bool(row['substantive_join_relations']) for row in manifest['records']):
        raise ValueError('substantive join count differs from records')
    output_paths = {entry['path']: entry for entry in manifest['outputs']}
    if set(output_paths) != {DIRECTORY + relative for relative in OUTPUTS}:
        raise ValueError('unexpected binding output membership')
    for path, entry in output_paths.items():
        if digest(scratch / path.removeprefix(DIRECTORY)) != entry['sha256']:
            raise ValueError('binding output digest differs: ' + path)
    seen = set()
    documents = {}
    digests = {}

    def document(path):
        if path not in documents:
            documents[path] = read(path)
        return documents[path]

    def cached_digest(path):
        if path not in digests:
            digests[path] = digest(path)
        return digests[path]

    for record in manifest['records']:
        key = record['quest']['key']
        if key in seen:
            raise ValueError('duplicate canonical binding identity')
        seen.add(key)
        witness = record['definition']
        path = (root / witness['path']).resolve()
        if not path.is_relative_to(root.resolve()):
            raise ValueError('definition witness escapes repository')
        if cached_digest(path) != witness['sha256']:
            raise ValueError('stale definition witness')
        definition = dereference(document(path), witness['json_pointer'])
        if definition['identity'] != record['quest']:
            raise ValueError('definition belongs to another quest')
        for ref in record['supplement_refs']:
            if ref['path'] not in output_paths:
                raise ValueError('unregistered binding reference')
            value = dereference(document(scratch / ref['path'].removeprefix(DIRECTORY)),
                                ref['json_pointer'])
            owner = value.get('quest', {}).get('key', value.get('quest_key',
                              value.get('canonical_key')))
            if owner != key:
                raise ValueError('supplement belongs to another quest')
    return manifest


def dereference(value, pointer):
    if not pointer.startswith('/'):
        raise ValueError('invalid JSON pointer')
    for part in pointer.split('/')[1:]:
        part = part.replace('~1', '/').replace('~0', '~')
        if isinstance(value, list):
            if re.fullmatch(r'0|[1-9][0-9]*', part) is None:
                raise ValueError('invalid JSON pointer array index')
            value = value[int(part)]
        else:
            value = value[part]
    return value


def substantive_relations(reward, npc, recipe, plain, source):
    relations = []
    if recipe:
        targets = [target for stage in recipe['stages'] for target in stage['target_evidence']]
        targets += recipe['requirements']['prerequisite_evidence'] + recipe['rewards']
        if any(target['evidence_refs'] for target in targets):
            relations.append('CHOSEN_TARGET_TO_PINNED_FACT')
        if any(target['identity_candidates'] for target in targets):
            relations.append('CHOSEN_TARGET_TO_CANONICAL_IDENTITY_CANDIDATE')
    if any(binding.get('proposed_ref') for binding in reward['chosen_reward_bindings']):
        relations.append('CHOSEN_REWARD_TO_ITEM_DEFINITION_CANDIDATE')
    if any(link.get('npc') and link.get('dialogue_record_present')
           for link in npc['recipe_stage_npc_links']):
        relations.append('CHOSEN_NPC_TARGET_TO_NPC_AND_DIALOGUE')
    if plain and any(item.get('item_witness') for claim in plain['claims']
                     for item in claim['item_rewards']):
        relations.append('EXISTING_REWARD_CLAIM_TO_CANONICAL_ITEM')
    if source and source['source_reward_bindings'] and source['native_reward_refs']:
        relations.append('SOURCE_REWARD_PROGRAM_TO_CANONICAL_CLAIM')
    return relations


def assemble(root, scratch):
    here = Path(__file__).resolve().parent
    scripts = here / 'binding_packets'
    for lane in ('source', 'rewards', 'npc', 'recipes'):
        (scratch / lane).mkdir()
    env = {**os.environ, 'QUEST_BINDING_ROOT': str(root), 'QUEST_BINDING_OUT': str(scratch),
           'PYTHONDONTWRITEBYTECODE': '1'}
    subprocess.run([sys.executable, str(scripts / 'source.py'), str(root),
                    str(scratch / 'source/crosswalk.json')], check=True, env=env)
    for name in ('rewards', 'npc', 'recipes', 'recipes_compact', 'verify_recipes'):
        subprocess.run([sys.executable, str(scripts / (name + '.py'))], check=True, env=env)
    source = read(scratch / OUTPUTS[0])
    rewards = read(scratch / OUTPUTS[1])
    npc = read(scratch / 'npc/quest-npc-dialogue-links.json')
    recipes = read(scratch / 'recipes/supplementary-index.json')
    source_keys = {r['quest_key']: n for n, r in enumerate(source['quests'])}
    npc_keys = {r['quest']['key']: n for n, r in enumerate(npc['quests'])}
    recipe_keys = {r['canonical_key']: n for n, r in enumerate(recipes['records'])}
    plain = {r['quest']['key']: r for r in read(scratch / 'rewards/reward-only-105.json')['records']}
    records = []
    for n, row in enumerate(rewards['records']):
        key = row['quest']['key']
        refs = [{'path': DIRECTORY + OUTPUTS[1], 'json_pointer': f'/records/{n}'},
                {'path': DIRECTORY + 'npc/quest-npc-dialogue-links.json',
                 'json_pointer': f'/quests/{npc_keys[key]}'}]
        if key in source_keys:
            refs.append({'path': DIRECTORY + OUTPUTS[0], 'json_pointer': f'/quests/{source_keys[key]}'})
        if key in recipe_keys:
            refs.append({'path': DIRECTORY + 'recipes/supplementary-index.json',
                         'json_pointer': f'/records/{recipe_keys[key]}'})
        relations = substantive_relations(row, npc['quests'][npc_keys[key]],
                      recipes['records'][recipe_keys[key]] if key in recipe_keys else None,
                      plain.get(key), source['quests'][source_keys[key]] if key in source_keys else None)
        records.append({'substantive_join_relations': relations, 'quest': row['quest'], 'display_name': row['name'],
                        'definition': row['witness'], 'supplement_refs': refs,
                        'new_source_hold_closure': False, 'runtime_admission': 'NOT_ASSESSED'})
    keys = [r['quest']['key'] for r in records]
    if len(keys) != 352 or len(set(keys)) != 352 or set(npc_keys) != set(keys):
        raise ValueError('binding packet canonical coverage differs')
    if len(source_keys) != 284 or len(recipe_keys) != 310:
        raise ValueError('binding packet source/recipe scope differs')
    if recipes['summary']['source_fidelity_resolved'] != 0 or recipes['summary']['runtime_admitted'] != 0:
        raise ValueError('identity mention evidence promoted to execution')
    outputs = []
    for relative in OUTPUTS:
        path = scratch / relative
        # Keep one deterministic compact packet, not repeated pretty-printed copies.
        path.write_text(json.dumps(read(path), ensure_ascii=False, sort_keys=True,
                                   separators=(',', ':')) + '\n', encoding='utf-8')
        outputs.append({'path': DIRECTORY + relative, 'sha256': digest(path)})
    manifest = {'schema': 'OTERYN_QUEST_BINDING_EVIDENCE_INDEX/v1',
                'scope': 'DATA_SUPPLEMENT_ONLY_NOT_QUEST_COMPLETENESS_OR_NATIVE_ADMISSION',
                'summary': {'definitions_indexed': 352,
                            'definitions_with_substantive_joins': sum(bool(r['substantive_join_relations']) for r in records), 'donor_crosswalks': 284,
                            'chosen_recipe_evidence_rows': 310,
                            'recipe_stages': recipes['summary']['stage_count'],
                            'recipe_identity_candidates': recipes['summary']['stage_identity_candidates'],
                            'reward_identity_candidates': sum('proposed_ref' in r for r in
                                read(scratch / 'rewards/chosen-reward-item-candidates.json')['records']),
                            'closed_original_source_holds': 0, 'runtime_readiness': 'NOT_ASSESSED'},
                'outputs': outputs, 'records': records}
    validate_index(root, scratch, manifest)
    (scratch / 'index.json').write_text(json.dumps(manifest, ensure_ascii=False, sort_keys=True,
                                                   separators=(',', ':')) + '\n', encoding='utf-8')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    with tempfile.TemporaryDirectory(prefix='quest-binding-authoring-') as temporary:
        scratch = Path(temporary)
        manifest = assemble(root, scratch)
        for relative in (*OUTPUTS, 'index.json'):
            target = root / DIRECTORY / relative
            expected = (scratch / relative).read_bytes()
            if args.check:
                if not target.is_file() or target.read_bytes() != expected:
                    raise ValueError('stale binding evidence: ' + relative)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(expected)
        print(json.dumps(manifest['summary'], sort_keys=True))


if __name__ == '__main__':
    main()
