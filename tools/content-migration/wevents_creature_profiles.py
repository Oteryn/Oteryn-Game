#!/usr/bin/env python3
"""Produce explicit authored event actors without revising historical definitions."""
import copy
import hashlib
import json
import re

POLICY = 'content/quests/world-event-creature-policy.json'
PREFIX = 'oteryn:creature.wevents.'


def candidates(root, creatures, presentations):
    raw = (root / POLICY).read_bytes()
    policy = json.loads(raw)
    if policy.get('schema') != 1 or policy.get('classification') != 'OTERYN_AUTHORED_POLICY':
        raise ValueError('WEVENTS_CREATURE_POLICY')
    actors = policy['actors']
    if not actors or len(actors) > 16 or len({row['name'] for row in actors}) != len(actors):
        raise ValueError('WEVENTS_CREATURE_MEMBERS')
    by_key = {r['profile']['target']['key']: r for r in creatures['records']}
    baseline = by_key[policy['baseline']]
    cues = {r['target']['key']: r for r in presentations['records']}
    baseline_cue = cues[baseline['presentation']['key']]
    records, additions = [], []
    policy_sha = hashlib.sha256(raw).hexdigest()
    for actor in actors:
        name = actor['name']
        if not re.fullmatch(r'[a-z][a-z0-9_]{0,63}', name) or not 1 <= actor['health'] <= 1_000_000:
            raise ValueError('WEVENTS_CREATURE_ID_OR_HEALTH')
        source = actor['source']
        if source and (not re.fullmatch('[a-f0-9]{40}', source['revision'])
                       or not re.fullmatch('[a-f0-9]{64}', source['sha256']) or source['line'] < 1):
            raise ValueError('WEVENTS_CREATURE_SOURCE')
        record = copy.deepcopy(baseline)
        record.pop('attack_definitions', None)
        record['profile']['target']['key'] = PREFIX + name
        record['behavior']['target']['key'] = 'oteryn:behavior.creature.wevents.' + name
        profile = record['profile']['data']['profile']
        behavior = record['behavior']['data']['profile']
        behavior['attacks'] = []
        behavior['defenses'] = []
        behavior.pop('voices', None)
        profile.update(health=actor['health'], experience=0, armor=actor['armor'],
                       speed=actor['source_speed'] // 2, abilities=[], resistances=[],
                       mitigation={'numerator': 0, 'denominator': 1})
        details = profile['details']
        details.update(display_name=actor['display_name'], inspection=actor['display_name'],
                       defense=actor['defense'], bosstiary=None, corpse_item=None,
                       death_residue=None, condition_immunities=[])
        details['system_eligibility'] = dict(exclusive_prey=False, forge=False, prey=False, reward_boss=True)
        kind, value = actor['melee']['kind'], actor['melee']['value']
        if kind == 'passive':
            record.pop('monster_melee', None)
            behavior['movement']['can_walk'] = False
            behavior['targeting'].update(can_target=False, hostile=False)
        else:
            if kind == 'reuse':
                melee = copy.deepcopy(by_key[value]['monster_melee'])
            elif kind in {'range', 'formula'} and source:
                melee = copy.deepcopy(baseline['monster_melee'])
                maximum = value if kind == 'range' else (value[0] * value[1] + 10 * value[1] + 19) // 20
                if not 0 < maximum <= 100_000:
                    raise ValueError('WEVENTS_CREATURE_MELEE')
                melee.update(minimum=0, maximum=maximum, source_repository=source['repository'],
                             source_revision=source['revision'], source_path=source['path'],
                             source_sha256=source['sha256'], interval_ms=2000, chance_ppm=1_000_000)
            else:
                raise ValueError('WEVENTS_CREATURE_MELEE_KIND')
            record['monster_melee'] = melee
            behavior['attacks'] = [dict(ability=copy.deepcopy(melee['ability']),
                                       interval_ms=melee['interval_ms'], chance_ppm=melee['chance_ppm'])]
            profile['abilities'] = [copy.deepcopy(melee['ability'])]
        if actor['look'] is not None:
            asset = 'canary.appearance:outfit/' + str(actor['look'])
            cue = next((r for r in cues.values() if r['data']['profile'].get('asset_binding') == asset), None)
            if cue is None:
                cue = copy.deepcopy(baseline_cue)
                cue['target']['key'] = 'oteryn:presentation.creature.wevents.' + name
                cue['data']['profile'].update(asset_binding=asset, light_level=0)
                cue['data']['profile'].pop('light_color_binding', None)
                additions.append(cue)
                cues[cue['target']['key']] = cue
            record['presentation'] = copy.deepcopy(cue['target'])
        provenance = {'policy': POLICY, 'sha256': policy_sha, 'rule': policy['policy'], 'source': source}
        record['source_qualification'] = 'OTERYN_AUTHORED_POLICY ' + json.dumps(provenance, sort_keys=True, separators=(',', ':'))
        records.append(record)
    return records, additions


def admit(root, generated, creature_path, presentation_path, manifest_path, canonical_bytes):
    creatures = json.loads(generated[creature_path])
    presentations = json.loads(generated[presentation_path])
    records, cues = candidates(root, creatures, presentations)
    existing = {r['profile']['target']['key'] for r in creatures['records']}
    if any(row['profile']['target']['key'] in existing for row in records):
        raise ValueError('WEVENTS_CREATURE_DUPLICATE')
    creatures['records'].extend(records)
    creatures['records'].sort(key=lambda r: r['profile']['target']['key'].encode())
    presentations['records'].extend(cues)
    presentations['records'].sort(key=lambda r: r['target']['key'].encode())
    generated[creature_path] = canonical_bytes(creatures)
    generated[presentation_path] = canonical_bytes(presentations)
    manifest = json.loads(generated[manifest_path])
    for key, path in [('creature_profiles', creature_path), ('presentation_profiles', presentation_path)]:
        manifest[key]['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated[manifest_path] = canonical_bytes(manifest)
