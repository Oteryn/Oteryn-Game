"""Finite chosen-recipe corrections; never change original SOURCE or its holds."""
import copy
import hashlib
import json

PATH = 'tools/content-schema/quest-authoring/samples/recipe-followup/corrections.json'
SHA256 = 'c0966551c2fd1a240fff0ea08a78fa7388d509dcf9678334cd33b9fc1141bad0'


def digest(value):
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
    return hashlib.sha256(raw.encode()).hexdigest()


def apply(root, rows):
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('chosen followup packet differs')
    packet = json.loads(raw)
    if (packet['runtime_enabled'] is not False or packet['source_holds_preserved'] is not True
            or packet['schema'] != 'OTERYN_CHOSEN_RECIPE_FOLLOWUP/v1'):
        raise ValueError('followup scope differs')
    result = copy.deepcopy(rows)
    selected = [r for r in result if r['identity']['key'] == packet['canonical_key']]
    if len(selected) != 1 or digest(selected[0]['recipe']) != packet['baseline_recipe_sha256']:
        raise ValueError('followup recipe fence differs')
    row = selected[0]
    change, = packet['changes']
    if (change['path'], change['old_value'], change['new_value']) != ('/stages/0/kind', 'talk', 'use'):
        raise ValueError('unqualified followup change')
    for ref in change['evidence_refs']:
        source = (root / ref['path']).read_bytes()
        if hashlib.sha256(source).hexdigest() != ref['file_sha256']:
            raise ValueError('followup evidence file differs')
        facts = [r for r in json.loads(source)['records']
                 if r['canonical_key'] == ref['canonical_key']]
        if len(facts) != 1 or facts[0]['facts'][ref['fact_index']]['evidence'] != ref['evidence']:
            raise ValueError('followup evidence span differs')
    stage = row['recipe']['stages'][0]
    if stage['kind'] != change['old_value'] or stage['targets'] != ['Task Board']:
        raise ValueError('followup target differs')
    stage['kind'] = change['new_value']
    return result, {'path': PATH, 'sha256': SHA256}
