"""Extend the first mechanic packet with bounded residual source cores."""
import argparse
import copy
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

import complete_remaining_monster_mechanics as core


def extend(repo, baseline, previous, canary, output):
    if output.exists():
        raise ValueError('candidate output must not exist')
    shutil.copytree(previous, output)  # Real copies: previous frozen packet remains immutable.
    sys.path[:0] = [str(repo / 'tools/content-schema/monster-authoring'),
                    str(repo / 'tools/content-schema/encounter-authoring')]
    import canary_batch as cb
    import validate_monster as vm
    import validate_encounter as ve
    receipt = core.read(output / 'completion.json')
    index = {r['monster']: r for r in core.read(baseline / 'population-index.json')['monsters']}
    actors, encounters, restored = {}, {}, {}

    def actor(name):
        if name not in actors:
            p = output / 'bundles' / name
            if not p.exists():
                p = baseline / 'bundles' / name
            actors[name] = {f: core.read(p / f) for f in core.FILES}
        return actors[name]

    def encounter(name):
        if name not in encounters:
            p = output / 'encounters' / name
            if not p.exists():
                p = baseline / 'encounters' / name
            if p.exists():
                encounters[name] = {f: core.read(p / f) for f in ('encounter.json', 'catalog.json', 'manifest.json')}
            else:
                encounters[name] = {'encounter.json': {'identity': {'key': core.ref('Encounter', name)['key'],
                    'revision': core.REV}, 'display_name': 'Source-backed Oteryn core: ' + name,
                    'scope': 'instance_per_party', 'participants': [{'role': name, 'creatures': [core.ref('Creature', name)]}],
                    'anchors': [], 'phases': [], 'state': {'counters': [], 'flags': [], 'timers': []},
                    'rules': [], 'outcomes': []}, 'catalog.json': {'definitions': []}, 'manifest.json': {
                    'encounter': core.ref('Encounter', name)['key'], 'classification': 'SOURCE_BACKED_OTERYN_SIMPLIFICATION',
                    'covers': {}, 'sources': [], 'entries': [], 'outcome_evidence': []}}
        return encounters[name]

    def resolve(name, field, owner, source_path, limitation):
        vals, p = actor(name), encounter(owner)
        row = next(x for x in vals['manifest.json']['entries'] if x['source_field'] == field)
        old = copy.deepcopy(row)
        row.update(status='approved_omission' if row['kind'] == 'script' else 'mapped',
                   destination='/monster/behavior/attacks', resolution='Relocated to Encounter ' +
                   p['manifest.json']['encounter'] + '; useful source core restored. ' + limitation)
        data = subprocess.check_output(['git', '-C', str(canary), 'show', core.PIN + ':' + source_path])
        src = {'kind': 'git', 'repository': 'opentibiabr/canary', 'revision': core.PIN, 'path': source_path,
               'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()}
        m = p['manifest.json']
        if src not in m['sources']:
            m['sources'].append(src)
        m['entries'].append({'source_index': m['sources'].index(src),
            'source_lines': list(range(1, len(data.splitlines()) + 1)), 'status': 'mapped',
            'destination': '/encounter/rules', 'resolution': limitation})
        m['covers'].setdefault(field.replace('events=', ''), []).append(core.ref('Creature', name)['key'])
        restored.setdefault(name, []).append({'source_field': field, 'original': old,
                                            'encounter': m['encounter'], 'partial': True})

    p = encounter('corrupted_soul')
    p['encounter.json']['rules'].append(core.rule('soul_initial_vortex',
        {'kind': 'creature_died', 'role': 'corrupted_soul'}, [{'kind': 'map_item', 'operation': 'create',
         'item': core.ref('Item', '23726'), 'at': 'death_position', 'unless_present': True}]))
    resolve('corrupted_soul', 'events=CorruptedSoul', 'corrupted_soul',
        core.QUESTS + 'forgotten_knowledge/creaturescripts_corrupted_soul.lua',
        'Initial23726vortex on death retained; cross-item23726/23727/23728 presence and upgrade/removal omitted. '
        'SOURCE_BACKED_OTERYN_SIMPLIFICATION; encounter placement unqualified.')

    p = encounter('gaz_haragoth')
    vals = actor('gaz_haragoth')
    row = next(x for x in vals['manifest.json']['entries'] if x['source_field'] == 'attacks[12]')
    schedule = cb.load_monster(canary / row['source_file'])[1]['attacks'][11]
    ability = {'family': 'Ability', 'key': 'canary:ability/spell/gaz_haragoth_summon', 'revision': core.REV}
    vals['monster.json']['behavior']['attacks'].append({'ability': ability,
        'interval_ms': schedule['interval'], 'chance_percent': schedule['chance']})
    vals['dependencies.json']['abilities'].append({'identity': {'key': ability['key'], 'revision': core.REV},
        'kind': 'spell', 'needs_direction': False, 'needs_target': False, 'range_tiles': 0,
        'encounter': core.ref('Encounter', 'gaz_haragoth')})
    trigger = {'kind': 'ability_cast', 'role': 'gaz_haragoth', 'ability': ability}
    for suffix, conditions, count in [('empty', [{'kind': 'summon_count', 'role': 'gaz_haragoth', 'op': '==', 'value': 0}], 2),
        ('one', [{'kind': 'summon_count', 'role': 'gaz_haragoth', 'op': '==', 'value': 1}], 1),
        ('grows', [{'kind': 'summon_count', 'role': 'gaz_haragoth', 'op': '>=', 'value': 2},
          {'kind': 'summon_count', 'role': 'gaz_haragoth', 'op': '<', 'value': 7},
          {'kind': 'chance_percent', 'value': 24.7525}], 1)]:
        p['encounter.json']['rules'].append(core.rule('gaz_summons_' + suffix, trigger,
            [{'kind': 'spawn', 'creature': core.ref('Creature', 'minion_of_gaz_haragoth'), 'count': count,
              'at': 'subject_position', 'owner': 'subject', 'health': 'full'},
             {'kind': 'say', 'subject': {'role': 'gaz_haragoth'}, 'text': 'Minions! Follow my call!', 'mode': 'say'}], conditions))
    resolve('gaz_haragoth', 'attacks[12]', 'gaz_haragoth', core.SPELLS + "gaz'haragoth_summon.lua",
        'Donor minimum2/cap7 retained; source growth2500/101percent rounded to24.7525percent '
        '(9901/400percent;247525ppm), explicit SOURCE_NON_GLOBAL_PROXY; rounding+0.0000248pp. '
        'Owner-summon count replaces spectator scan, '
        'mutable historical GazVariables.MinionsNow and undefined sum ownership. Explicit Oteryn simplification.')
    data = subprocess.check_output(['git', '-C', str(canary), 'show', core.PIN + ':' + core.SPELLS + 'gaz_functions.lua'])
    p['manifest.json']['sources'].append({'kind': 'git', 'repository': 'opentibiabr/canary', 'revision': core.PIN,
        'path': core.SPELLS + 'gaz_functions.lua',
        'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()})

    p = encounter('izcandar')
    e = p['encounter.json']
    for season, bounds in [('winter', [32208, 32216, 32040, 32055]), ('summer', [32198, 32207, 32039, 32055])]:
        x1, x2, y1, y2 = bounds
        anchor = 'izcandar_' + season + '_side'
        e['anchors'].append({'key': anchor, 'kind': 'area', 'description': 'Pinned seasonal side.',
            'location': {'boxes': [{'x': [x1, x2], 'y': [y1, y2], 'floor': 14}]}})
        for name in ('izcandar_the_banished', 'izcandar_champion_of_winter', 'izcandar_champion_of_summer'):
            if name == 'izcandar_champion_of_' + season:
                continue
            for event in ('creature_spawned', 'area_entered'):
                trigger = {'kind': event, 'role': name}
                if event == 'area_entered':
                    trigger.update(anchor=anchor, who='role')
                e['rules'].append(core.rule(name + '_becomes_' + season + '_' + event, trigger,
                    [{'kind': 'transform', 'role': name, 'into': core.ref('Creature', 'izcandar_champion_of_' + season),
                      'health': 'keep_absolute'}], [{'kind': 'in_anchor', 'subject': {'role': name}, 'anchor': anchor}],
                    delay={'min': 10000, 'max': 20000}))
    for name in ('izcandar_the_banished', 'izcandar_champion_of_winter', 'izcandar_champion_of_summer'):
        resolve(name, 'events=izcandarThink', 'izcandar',
            core.QUESTS + 'the_dream_courts_quest/creaturescripts_Izcandar.lua',
            'Season side chooses10–20s delayed champion transformation keeping absoluteHP. '
            'SOURCE_BACKED_OTERYN_SIMPLIFICATION: spawn/area events replace continuous think; '
            'intermediate Banished identity, quest-wide outfit storage and native role remapping are omitted.')

    p = encounter('goshnars_greed')
    e = p['encounter.json']
    spawn_anchor = next((a['key'] for a in e['anchors'] if a.get('location') ==
                         {'x': 33747, 'y': 31668, 'floor': 14}), 'soulsnatcher_source_tile')
    if not any(a['key'] == spawn_anchor for a in e['anchors']):
        e['anchors'].append({'key': spawn_anchor, 'kind': 'point', 'description': 'Pinned Soulsnatcher respawn tile.',
                             'location': {'x': 33747, 'y': 31668, 'floor': 14}})
    trigger = {'kind': 'creature_spawned', 'role': 'soulsnatcher'}
    e['rules'].extend([
        core.rule('soulsnatcher_expires_after_eight_seconds', trigger,
                  [{'kind': 'remove', 'role': 'soulsnatcher'}], delay=8000),
        core.rule('soulsnatcher_returns_after_expiry', trigger,
                  [{'kind': 'spawn', 'creature': core.ref('Creature', 'soulsnatcher'), 'role': 'soulsnatcher',
                    'count': 1, 'at': {'anchor': spawn_anchor}, 'owner': 'none', 'health': 'full'}], delay=18000)])
    src = next(x['source_file'] for x in actor('soulsnatcher')['manifest.json']['entries']
               if x['source_field'] == 'mType.onSpawn')
    for field in ('mType.onSpawn', 'mType.onThink'):
        resolve('soulsnatcher', field, 'goshnars_greed', src,
                'Eight-second lifetime then10-second fixed-tile respawn retained. Zone-wide player500–1000damage '
                'and7–9second warning effects omitted; post-removal triggering identity needs runtime qualification.')
    library = 'data-otservbr-global/lib/quests/soul_war.lua'
    data = subprocess.check_output(['git', '-C', str(canary), 'show', core.PIN + ':' + library])
    e_source = {'kind': 'git', 'repository': 'opentibiabr/canary', 'revision': core.PIN, 'path': library,
                'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()}
    if e_source not in p['manifest.json']['sources']:
        p['manifest.json']['sources'].append(e_source)

    p = encounter('the_primal_menace')
    e = p['encounter.json']
    e['state']['timers'].append({'name': 'primal_core_adds', 'duration_ms': 30000, 'repeat': True})
    trigger = {'kind': 'creature_spawned', 'role': 'the_primal_menace'}
    e['rules'].append(core.rule('primal_core_schedule_starts', trigger,
        [{'kind': 'timer', 'timer': 'primal_core_adds', 'operation': 'start'}], delay=10000))
    pool = ['emerald_tortoise_primal', 'gore_horn_primal', 'gorerilla_primal', 'headpecker_primal',
            'hulking_prehemoth_primal', 'mantosaurus_primal', 'nighthunter_primal', 'noxious_ripptor_primal',
            'sabretooth_primal', 'stalking_stalk_primal', 'sulphider_primal']
    branches = [{'weight': 1, 'actions': [{'kind': 'spawn', 'creature': core.ref('Creature', name), 'count': 1,
        'at': {'offset_tiles': 5}, 'owner': 'none', 'health': 'full'}]} for name in pool]
    actions = [{'kind': 'one_of', 'branches': copy.deepcopy(branches)} for _ in range(4)]
    e['rules'].append(core.rule('primal_initial_add_wave', trigger, actions, delay=13000))
    e['rules'].append(core.rule('primal_repeat_add_wave',
        {'kind': 'timer_elapsed', 'timer': 'primal_core_adds', 'each': 'the_primal_menace'},
        copy.deepcopy(actions), delay=3000))
    src = next(x['source_file'] for x in actor('the_primal_menace')['manifest.json']['entries']
               if x['source_field'] == 'mType.onSpawn')
    for field in ('mType.onSpawn', 'mType.onThink'):
        resolve('the_primal_menace', field, 'the_primal_menace', src,
                'Baseline10s initial/30s repeating adds plus3s telegraph delay; four independently chosen '
                'source-pool creatures within5tiles. SOURCE_NON_GLOBAL_PROXY: fixed hazard0/count4; '
                'HP/hazard scaling, pods, aged-beast conversion and warning effects are omitted.')

    p = encounter('dragon_egg')
    p['encounter.json']['rules'].append(core.rule('egg_fire_damage_heals',
        {'kind': 'damage_taken', 'role': 'dragon_egg', 'source': 'any'},
        [{'kind': 'convert_damage_to_heal', 'role': 'dragon_egg', 'damage_types': ['fire'], 'component': 'primary'}]))
    resolve('dragon_egg', 'events=DragonEggHealthChange', 'dragon_egg',
        core.QUESTS + 'forgotten_knowledge/creaturescripts_dragon_egg.lua',
        'Primary fire damage becomes healing. Healing-to-ice, fully-healed4500ice eruption and '
        'two existing Horror position/health exchanges remain omitted; useful safe typed core retained.')

    vals = actor('zamulosh')
    row = next(x for x in vals['manifest.json']['entries'] if x['source_field'] == 'defenses[2]')
    old = copy.deepcopy(row)
    schedule = cb.load_monster(canary / row['source_file'])[1]['defenses']['_list'][1]
    ability = {'family': 'Ability', 'key': 'canary:ability/spell/zamulosh_invisible', 'revision': core.REV}
    effect = {'family': 'Effect', 'key': ability['key'] + '/caster_invisibility', 'revision': core.REV}
    vals['monster.json']['behavior']['defenses'].append({'ability': ability,
        'interval_ms': schedule['interval'], 'chance_percent': schedule['chance']})
    vals['dependencies.json']['abilities'].append({'identity': {'key': ability['key'], 'revision': core.REV},
        'kind': 'spell', 'needs_direction': False, 'needs_target': False, 'range_tiles': 0, 'effects': [effect]})
    vals['dependencies.json']['effects'].append({'identity': {'key': effect['key'], 'revision': core.REV},
        'operation': 'condition', 'duration_ms': 10000,
        'condition': {'type': 'invisible', 'lifetime': 'fixed_duration'},
        'presentation': {'impact_asset_binding': 'canary.appearance:effect/teleport'}})
    asset = 'canary.appearance:effect/teleport'
    if asset not in vals['catalog.json']['assets']:
        vals['catalog.json']['assets'].append(asset)
    row.update(status='mapped', destination='/dependencies/effects/' + str(len(vals['dependencies.json']['effects']) - 1),
        resolution='SOURCE_NON_GLOBAL_PROXY:10second caster invisibility with exact2000ms/25percent schedule; '
                   'source room scan of all same-name topcreatures is omitted.')
    restored['zamulosh'] = [{'source_field': 'defenses[2]', 'original': old, 'partial': True,
                             'native_core': 'caster invisible condition10000ms'}]
    src = core.SPELLS + 'zamulosh_invisible.lua'
    data = subprocess.check_output(['git', '-C', str(canary), 'show', core.PIN + ':' + src])
    receipt['source_checks'].append({'path': src, 'repository': 'opentibiabr/canary', 'revision': core.PIN,
        'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest(),
        'sha256': hashlib.sha256(data).hexdigest()})

    p = encounter('the_time_guardian')
    e = p['encounter.json']
    e['anchors'].append({'key': 'guardian_proxy_floor14', 'kind': 'area', 'description': 'Source floor14 cast gate.',
        'location': {'boxes': [{'x': [0, 65535], 'y': [0, 65535], 'floor': 14}]}})
    vals = actor('the_time_guardian')
    if core.ref('Encounter', 'the_time_guardian') not in vals['catalog.json']['definitions']:
        vals['catalog.json']['definitions'].append(core.ref('Encounter', 'the_time_guardian'))
    for field, script in [('defenses[1]', 'time_guardian.lua'), ('defenses[2]', 'time_guardiann.lua')]:
        row = next(x for x in vals['manifest.json']['entries'] if x['source_field'] == field)
        raw = cb.load_monster(canary / row['source_file'])[1]['defenses']['_list'][int(field[-2]) - 1]
        key = 'canary:ability/spell/' + cb.slug(raw['name'])
        ability = {'family': 'Ability', 'key': key, 'revision': core.REV}
        vals['monster.json']['behavior']['defenses'].append({'ability': ability,
            'interval_ms': raw['interval'], 'chance_percent': raw['chance']})
        vals['dependencies.json']['abilities'].append({'identity': {'key': key, 'revision': core.REV},
            'kind': 'spell', 'needs_direction': False, 'needs_target': False, 'range_tiles': 0,
            'encounter': core.ref('Encounter', 'the_time_guardian')})
        trigger = {'kind': 'ability_cast', 'role': 'the_time_guardian', 'ability': ability}
        gate = [{'kind': 'in_anchor', 'subject': {'role': 'the_time_guardian'}, 'anchor': 'guardian_proxy_floor14'}]
        branches = [{'weight': 1, 'actions': [{'kind': 'transform', 'role': 'the_time_guardian',
            'into': core.ref('Creature', form), 'health': 'keep_absolute'}]}
            for form in ('the_blazing_time_guardian', 'the_freezing_time_guardian')]
        slug = cb.slug(raw['name'])
        e['rules'].append(core.rule(slug + '_proxy_phase', trigger, [{'kind': 'one_of', 'branches': branches}], gate))
        e['rules'].append(core.rule(slug + '_proxy_returns', trigger,
            [{'kind': 'transform', 'role': 'the_time_guardian', 'into': core.ref('Creature', 'the_time_guardian'),
              'health': 'keep_absolute'}], delay=30000))
        resolve('the_time_guardian', field, 'the_time_guardian', core.SPELLS + script,
            'SOURCE_NON_GLOBAL_PROXY: same-entity50/50 elemental phase retaining absoluteHP, then30s return, '
            'with source floor14 gate and exact cast schedule. Two-existing-creature position/HP exchange, '
            'fixed parked actors and spectator-dependent removal are omitted; native phase-role retention unqualified.')

    for name, vals in actors.items():
        errors = vm.validate(*(vals[f] for f in core.FILES))
        if errors:
            raise ValueError((name, errors))
        for filename, value in vals.items():
            core.write(output / 'bundles' / name / filename, value)
        row = copy.deepcopy(next((r for r in receipt['index_monsters'] if r['monster'] == name), index[name]))
        row['sha256'] = core.population.admission.bundle_digest(output / 'bundles' / name)
        row['completion_flags'] = sorted(set(row.get('completion_flags', [])) |
            {'SOURCE_TYPED_CORE_RESTORED', 'SOURCE_BACKED_OTERYN_SIMPLIFICATION',
             'SOURCE_BEHAVIOR_PARTIAL', 'GAMEPLAY_UNVERIFIED', 'ENCOUNTER_PLACEMENT_PENDING'})
        if name in ('gaz_haragoth', 'the_primal_menace', 'zamulosh', 'the_time_guardian'):
            row['completion_flags'] = sorted(set(row['completion_flags']) | {'SOURCE_NON_GLOBAL_PROXY'})
        if name == 'the_time_guardian':
            row['completion_flags'] = sorted(set(row['completion_flags']) | {'SAME_ENTITY_PHASE_PROXY'})
        receipt['index_monsters'] = [r for r in receipt['index_monsters'] if r['monster'] != name] + [row]
        old = next((a for a in receipt['actors'] if a['monster'] == name), None)
        actor_receipt = {'monster': name, 'original_bundle_digest': index[name]['sha256'], 'bundle_digest': row['sha256'],
            'restored_source_rows': (old['restored_source_rows'] if old else []) + restored[name],
            'completion_flags': row['completion_flags'], 'limitations': [x['original']['resolution'] for x in restored[name]]}
        receipt['actors'] = [a for a in receipt['actors'] if a['monster'] != name] + [actor_receipt]
    for name, vals in encounters.items():
        for dep in ve.refs(vals['encounter.json']):
            if dep not in vals['catalog.json']['definitions']:
                vals['catalog.json']['definitions'].append(dep)
        errors = ve.validate(vals['encounter.json'], vals['catalog.json'], vals['manifest.json'])
        if errors:
            raise ValueError((name, errors))
        for filename, value in vals.items():
            core.write(output / 'encounters' / name / filename, value)
        receipt['encounters'] = [r for r in receipt['encounters'] if r['encounter'] != name] + [
            {'encounter': name, 'remaining_runtime_qualification': True,
             'new_definition': not (baseline / 'encounters' / name).exists(),
             'source_bound_allowlist': vals['manifest.json']['sources']}]
    receipt['remaining'] = [r for r in receipt['remaining'] if not any(
        x['source_field'] == r['source_field'] for x in restored.get(r['monster'], []))]
    group_reasons = {
        'QUEST_SHARED_STATE_OWNER': 'Cross-party/player/global quest state and reward/reset/wall locks need quest-domain authority; no local Encounter storage substitutes accepted shared state.',
        'MEGALOMANIA_MAP_PREDICATES': 'Callbacks depend on named zone tile enumeration, safe ground409, white-tile cycling, dynamically registered cleanup/torment and mutable defense. Existing closed Encounter vocabulary lacks arbitrary tile-ground predicates and zone-scoped callback ownership.',
        'SEACREST_DEFENSIVE_MELEE': 'Source damage0 can still consume target shield block budget in Creature::blockHit; native defensive-melee schedule/armor-shield block semantics are unavailable, so no no-op claim.',
        'WALKER_TARGET_ATTRIBUTES': 'Per-target vocation dispatch chooses melee/defense, distance/defense or magic-level debuffs for7s at45–60percent. Native effect predicates cannot dispatch these source callbacks per target vocation.'}
    groups = {key: {'reason': reason, 'rows': []} for key, reason in group_reasons.items()}
    for row in receipt['remaining']:
        group = ('MEGALOMANIA_MAP_PREDICATES' if 'megalomania' in row['monster'] else
                 'WALKER_TARGET_ATTRIBUTES' if row['monster'] == 'walker' else
                 'SEACREST_DEFENSIVE_MELEE' if row['monster'] == 'seacrest_serpent' else 'QUEST_SHARED_STATE_OWNER')
        groups[group]['rows'].append({'monster': row['monster'], 'source_field': row['source_field'],
                                     'source_gap': row['source_gap']})
    receipt['remaining_classification'] = groups
    receipt['exact_parity_debt_retained'] = True
    receipt['probability_quantization'] = [{'monster': 'gaz_haragoth', 'source_percent_ratio': {'numerator': 2500, 'denominator': 101},
        'native_percent_ratio': {'numerator': 9901, 'denominator': 400}, 'native_ppm': 247525,
        'qualification': 'SOURCE_NON_GLOBAL_PROXY', 'rounding_error_percentage_points': '0.0000247524752475'}]
    receipt['source_noop_components'] = 0
    receipt['counts'] = {'actors': len(receipt['actors']),
        'restored_components': sum(len(r['restored_source_rows']) for r in receipt['actors']),
        'encounter_successors': len(receipt['encounters']), 'remaining_components': len(receipt['remaining'])}
    core.write(output / 'completion.json', receipt)
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('repo', 'baseline', 'previous', 'canary', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    print(json.dumps(extend(**vars(parser.parse_args()))['counts']))
