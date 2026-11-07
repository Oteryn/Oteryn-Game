"""Admit the WVAR Mamma Longlegs recipe into the existing native providers.

This is an explicitly authored successor, not a claim that the donor defines
Mamma. Its source facts are retained separately from the conservative combat
template, and it does not duplicate Giant Spider loot or Bestiary eligibility.
"""
from __future__ import annotations

import copy
import hashlib
import json

CREATURES = 'content/creatures/definitions/spell-native-profiles.json'
PRESENTATIONS = 'content/presentations/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
CREATURE = 'oteryn:creature.mamma_longlegs'
BEHAVIOR = 'oteryn:behavior.creature.mamma_longlegs'
PRESENTATION = 'oteryn:presentation.creature.mamma_longlegs'
SOURCE = 'imports/tibiawiki/facts/creatures/creatures-00500-00999.json'
SOURCE_SHA256 = '29a7b8a32c96f48b65ab43d1906a8af48cc278f10649df9d24b434bca48bb7b0'


def canonical(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode()


def admit(root, generated):
    facts = json.loads((root / SOURCE).read_bytes())['records']
    source = next(row for row in facts if row.get('source_title') == 'Mamma Longlegs')
    if (source['source_page_id'], source['source_revision_id'], source['source_raw_utf8_sha256']) != (
            '25603', 407575, SOURCE_SHA256):
        raise ValueError('WVAR_MAMMA_SOURCE_CHANGED')
    observed = source['observed_fields']
    bosses = json.loads((root / 'imports/cipsoft-staticdata/bosses/bosses-00000-00249.json').read_bytes())['records']
    official = next(row for row in bosses if row.get('source_id') == 901)
    if official['name'] != 'Mamma Longlegs' or official['look']['hex'] != '08d001120808001000180020001800':
        raise ValueError('WVAR_MAMMA_APPEARANCE_CHANGED')
    creatures = json.loads(generated[CREATURES])
    original = next(row for row in creatures['records'] if row['profile']['target']['key'] == 'oteryn:creature.giant_spider')
    mamma = copy.deepcopy(original)
    mamma['profile']['target']['key'] = CREATURE
    mamma['presentation']['key'] = PRESENTATION
    profile = mamma['profile']['data']['profile']
    profile['health'], profile['experience'] = int(observed['hp']), int(observed['exp'])
    profile.pop('bestiary', None)
    details = profile['details']
    details['display_name'], details['inspection'] = observed['name'], 'Mamma Longlegs'
    details['article'] = ''
    details.pop('bestiary', None)
    details['condition_immunities'] = ['invisible']
    details['system_eligibility'] = {'exclusive_prey': False, 'forge': False, 'prey': False, 'reward_boss': False}
    mamma['behavior']['target']['key'] = BEHAVIOR
    movement = mamma['behavior']['data']['profile']['movement']
    movement['pushable'] = False
    movement['push_items'] = True
    # Keep the independently sourced Giant Spider attack/effect/formula bodies;
    # give the changed Creature and Behavior their own exact closure identities.
    closure = mamma['attack_definitions']
    for reference in closure['records']:
        if reference['identity']['key'] == 'oteryn:creature.giant_spider':
            reference['identity'] = copy.deepcopy(mamma['profile']['target'])
            reference['behavior'] = copy.deepcopy(mamma['behavior']['target'])
            reference['presentation'] = copy.deepcopy(mamma['presentation'])
            reference.pop('loot', None)
        elif reference['identity']['key'] == 'oteryn:behavior.creature.giant_spider':
            reference['identity'] = copy.deepcopy(mamma['behavior']['target'])
    closure['profiles'] = [
        copy.deepcopy(mamma['profile']) if row['target']['key'] == 'oteryn:creature.giant_spider'
        else copy.deepcopy(mamma['behavior']) if row['target']['key'] == 'oteryn:behavior.creature.giant_spider'
        else row for row in closure['profiles']
    ]
    creatures['records'] = [row for row in creatures['records'] if row['profile']['target']['key'] != CREATURE]
    creatures['records'].append(mamma)
    presentations = json.loads(generated[PRESENTATIONS])
    presentation = copy.deepcopy(next(row for row in presentations['records'] if row['target']['key'] == 'oteryn:presentation.creature.giant_spider'))
    presentation['target']['key'] = PRESENTATION
    presentation['data']['profile']['asset_binding'] = 'canary.appearance:outfit/208'
    presentations['records'] = [row for row in presentations['records'] if row['target']['key'] != PRESENTATION]
    presentations['records'].append(presentation)
    generated[CREATURES], generated[PRESENTATIONS] = canonical(creatures), canonical(presentations)
    manifest = json.loads(generated[MANIFEST])
    for key, path in [('creature_profiles', CREATURES), ('presentation_profiles', PRESENTATIONS)]:
        manifest[key]['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated[MANIFEST] = canonical(manifest)
