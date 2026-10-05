"""Finite proven direct storage-read effects; preserve Source, guards and native bindings."""
import copy
import hashlib
import json
from pathlib import Path

PACKET = 'tools/content-schema/quest-authoring/samples/state-effect-refinements/refinements.json'
APPROVED_SHA256 = '129f24ec69346e24c4b7e2240c44868c0bcc5776c968f237d7f6a31fd24fce1c'


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def apply_packet(quests, packet):
    import jsonschema
    schema = json.loads(Path(__file__).with_name('effect_refinements.schema.json').read_text())
    jsonschema.validate(packet, schema)
    if packet['schema'] != 'OTERYN_DIRECT_STORAGE_EFFECT_REFINEMENTS/v1' or packet['source_datums_unchanged'] is not True:
        raise ValueError('Wrong refinement scope')
    result = copy.deepcopy(quests)
    transitions = {t['key']: t for q in result for t in q['transitions']}
    if len(packet['changes']) != 5:
        raise ValueError('Finite refinement selection differs')
    seen = set()
    for change in packet['changes']:
        key = change['transition_key']
        if key in seen or key not in transitions:
            raise ValueError('Duplicate or missing effect refinement')
        seen.add(key)
        transition = transitions[key]
        if digest(transition) != change['baseline_sha256']:
            raise ValueError('Transition baseline changed')
        effect = transition['effects'][0]
        if len(transition['effects']) != 1 or effect['from_exact'] is not True or effect['effect'] != {'kind': 'COMPUTED'}:
            raise ValueError('Cannot promote an inexact guard')
        replacement = copy.deepcopy(transition)
        replacement['effects'][0]['effect'] = change['effect']
        if change['effect']['kind'] != 'ADD' or replacement != change['replacement']:
            raise ValueError('Effect-only refinement changed protected data')
        transition['effects'][0]['effect'] = copy.deepcopy(change['effect'])
    return result


def apply(root, quests):
    path = Path(root) / PACKET
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != APPROVED_SHA256:
        raise ValueError('Unapproved state effect refinement packet')
    return apply_packet(quests, json.loads(raw)), {'path': PACKET, 'sha256': APPROVED_SHA256}
