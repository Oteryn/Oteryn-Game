"""Restore bounded donor mechanic cores without executing donor callbacks.

The resulting Encounter documents require explicit encounter placement/runtime
qualification. Partial cores retain their limitations; they never imply Global parity.
"""
import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

import complete_creature_dependencies as population

FILES = population.FILES
PIN = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'
SPELLS = 'data-otservbr-global/scripts/spells/monster/'
QUESTS = 'data-otservbr-global/scripts/quests/'


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def ref(family, slug):
    return {'family': family, 'key': f'canary:{family.lower()}/{slug}', 'revision': REV}


def rule(key, trigger, actions, conditions=None, delay=None):
    value = {'key': key, 'trigger': trigger, 'conditions': conditions or [], 'actions': actions}
    if delay:
        value['delay_ms'] = delay
    return value


def prepare(repo, baseline, canary, inventory, output):
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be empty')
    if baseline.resolve() == output.resolve() or baseline.resolve() in output.resolve().parents:
        raise ValueError('output would modify baseline')
    sys.path[:0] = [str(repo / 'tools/content-schema/monster-authoring'),
                    str(repo / 'tools/content-schema/encounter-authoring')]
    import canary_batch as cb
    import validate_monster as vm
    import validate_encounter as ve
    old_index = read(baseline / 'population-index.json')
    index = {r['monster']: r for r in old_index['monsters']}
    actors, encounters, restored, limits = {}, {}, {}, {}
    evidence = []

    def source(path):
        data = subprocess.check_output(['git', '-C', str(canary), 'show', PIN + ':' + path])
        if (canary / path).exists() and (canary / path).read_bytes() != data:
            raise ValueError('dirty donor input ' + path)
        item = {'kind': 'git', 'repository': 'opentibiabr/canary', 'revision': PIN, 'path': path,
                'blob_sha1': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()}
        evidence.append({**item, 'sha256': hashlib.sha256(data).hexdigest()})
        return item, data.decode()

    def actor(name):
        if name not in actors:
            actors[name] = {f: read(baseline / 'bundles' / name / f) for f in FILES}
        return actors[name]

    def encounter(name, owner):
        if name not in encounters:
            p = baseline / 'encounters' / name
            if p.exists():
                encounters[name] = {f: read(p / f) for f in ('encounter.json', 'catalog.json', 'manifest.json')}
            else:
                encounters[name] = {
                    'encounter.json': {'identity': {'key': ref('Encounter', name)['key'], 'revision': REV},
                        'display_name': 'Oteryn source-backed core: ' + name, 'scope': 'instance_per_party',
                        'participants': [], 'anchors': [], 'phases': [],
                        'state': {'counters': [], 'flags': [], 'timers': []}, 'rules': [], 'outcomes': []},
                    'catalog.json': {'definitions': []},
                    'manifest.json': {'encounter': ref('Encounter', name)['key'],
                        'classification': 'SOURCE_BACKED_OTERYN_SIMPLIFICATION', 'covers': {},
                        'sources': [], 'entries': [], 'outcome_evidence': []}}
        packet = encounters[name]
        e = packet['encounter.json']
        if not any(owner == p['role'] for p in e['participants']):
            e['participants'].append({'role': owner, 'creatures': [ref('Creature', owner)]})
        return packet

    def cover(name, field, packet, source_path, explanation, partial=None):
        vals = actor(name)
        entries = [x for x in vals['manifest.json']['entries'] if x['source_field'] == field]
        if len(entries) != 1:
            raise ValueError('ambiguous source field ' + name + ':' + field)
        old = copy.deepcopy(entries[0])
        entries[0].update(status='approved_omission' if old['kind'] == 'script' else 'mapped',
                          destination='/monster/behavior/attacks',
                          resolution='Relocated to Encounter ' + packet['manifest.json']['encounter'] +
                          '; existing typed core restored. ' + explanation)
        m = packet['manifest.json']
        s, text = source(source_path)
        if s not in m['sources']:
            m['sources'].append(s)
        m['entries'].append({'source_index': m['sources'].index(s),
                             'source_lines': list(range(1, len(text.splitlines()) + 1)), 'status': 'mapped',
                             'destination': '/encounter/rules', 'resolution': explanation})
        event = field.replace('events=', '') if field.startswith('events=') else field
        m['covers'].setdefault(event, []).append(ref('Creature', name)['key'])
        restored.setdefault(name, []).append({'source_field': field, 'original': old,
                                              'encounter': m['encounter'], 'partial': bool(partial)})
        if partial:
            limits.setdefault(name, []).append(partial)

    def cast(name, field, script, builder, partial=None, owner=None):
        vals = actor(name)
        row = next(x for x in vals['manifest.json']['entries'] if x['source_field'] == field)
        src = row['source_file']
        source(src)
        raw = cb.load_monster(canary / src, [])[1]
        section, number = field.split('[')
        rows = raw[section]
        rows = rows if isinstance(rows, list) else rows['_list']
        position = int(number[:-1])
        schedule = rows[position - 1] if isinstance(rows, list) else rows[position]
        slug = cb.slug(schedule['name'])
        ability_ref = {'family': 'Ability', 'key': 'canary:ability/spell/' + slug, 'revision': REV}
        packet = encounter(owner or name, name)
        e = packet['encounter.json']
        typed = {'ability': ability_ref, 'interval_ms': schedule['interval'],
                 'chance_percent': schedule.get('chance', 100)}
        vals['monster.json']['behavior'][section].append(typed)
        vals['dependencies.json']['abilities'].append({'identity': {'key': ability_ref['key'], 'revision': REV},
            'kind': 'spell', 'needs_direction': False, 'needs_target': bool(schedule.get('target', False)),
            'range_tiles': schedule.get('range', 0), 'encounter': ref('Encounter', owner or name)})
        if ref('Encounter', owner or name) not in vals['catalog.json']['definitions']:
            vals['catalog.json']['definitions'].append(ref('Encounter', owner or name))
        builder(e, {'kind': 'ability_cast', 'role': name, 'ability': ability_ref})
        cover(name, field, packet, SPELLS + script,
              'Pinned cast interval/chance preserved; encounter actions encode the selected source core.', partial)
        packet['manifest.json']['covers'].setdefault('spell.' + schedule['name'], []).append(ref('Creature', name)['key'])

    def nuke(name, field, script, damage, low, high, radius, delay, effect, remove=False):
        def build(e, trigger):
            key = name + '_explosion'
            e.setdefault('abilities', []).append({'key': key, 'area': {'shape': 'circle', 'radius': radius},
                'damage': {'damage_type': damage, 'min': low, 'max': high},
                'affects': {'players': True, 'creatures': []}, 'effect': 'canary.appearance:effect/' + effect})
            actions = [{'kind': 'cast', 'encounter_ability': key, 'at': 'subject_position'}]
            if remove:
                actions.append({'kind': 'remove', 'role': name})
            if name in ('gaz_haragoth', 'lady_tenebris'):
                _, text = source(SPELLS + script)
                messages = re.findall(r'creature:say\("([^"]+)"', text)
                if messages:
                    e['rules'].append(rule(key + '_warning', trigger,
                        [{'kind': 'say', 'subject': {'role': name}, 'text': messages[-1], 'mode': 'yell'}]))
                    if len(messages) > 1:
                        actions.insert(0, {'kind': 'say', 'subject': {'role': name},
                                           'text': messages[0], 'mode': 'yell'})
            e['rules'].append(rule(key + '_cast', trigger, actions, delay=delay))
        cast(name, field, script, build,
             'SOURCE_BACKED_OTERYN_SIMPLIFICATION: players-only circle replaces donor tile/vocation callbacks; '
             'non-player hits, identity/target retention and ancillary source actions remain unqualified. '
             + ('Plagirath retains source1500earth core at caster circle; source target-distance20, '
                '10soutfit299 and retained target-position predicate are omitted.' if name == 'plagirath' else ''))

    nuke('gaz_haragoth', 'attacks[10]', "gaz'haragoth_death.lua", 'energy', 30000, 30000, 6, 5000, 'purpleenergy')
    nuke('lady_tenebris', 'attacks[5]', 'tenebris_ultimate.lua', 'death', 2200, 2500, 7, 4000, 'mortarea')
    nuke('eruption_of_destruction', 'attacks[2]', 'eruption_of_destruction_explosion.lua',
         'fire', 4000, 6000, 6, 7000, 'firearea', True)
    nuke('charging_outburst', 'attacks[6]', 'outburst_explode.lua', 'energy', 1500, 2000, 6, None, 'purpleenergy')
    nuke('plagirath', 'attacks[7]', 'plagirath_bog.lua', 'earth', 1500, 1500, 4, 10000, 'poison')

    def mazoran(e, trigger):
        e['anchors'].append({'key': 'mazoran_summon_room', 'kind': 'area', 'description': 'Pinned summon rectangle.',
            'location': {'boxes': [{'x': [33576, 33593], 'y': [32684, 32695], 'floor': 14}]}})
        e['rules'].append(rule('mazoran_summons_four', trigger,
            [{'kind': 'spawn', 'creature': ref('Creature', 'rage_of_mazoran'), 'count': 4,
              'at': {'random_in': 'mazoran_summon_room'}, 'owner': 'subject', 'health': 'full'}],
            [{'kind': 'summon_count', 'role': 'mazoran', 'op': '<', 'value': 4}]))
    cast('mazoran', 'defenses[3]', 'mazoran_fire.lua', mazoran,
         'Source3s warning/lava transformation/revert and forced placement are omitted; only four-summon core restored.')

    def generator(e, trigger):
        e['rules'].append(rule('generator_releases_energy', trigger,
            [{'kind': 'spawn', 'creature': ref('Creature', 'energy_pulse'), 'count': 1,
              'at': 'subject_position', 'owner': 'none', 'health': 'full'},
             {'kind': 'say', 'subject': {'role': 'glooth_generator'},
              'text': 'The fully charged generator explodes in a blast!', 'mode': 'yell'},
             {'kind': 'remove', 'role': 'glooth_generator'}], delay=14000))
    cast('glooth_generator', 'defenses[1]', 'glooth-generator_summon.lua', generator,
         'Source delayed identity retention and forced-placement failure are unqualified.')

    # Existing sapling combat stays byte-for-byte; only the missing delayed removal is added.
    p = encounter('carnisylvan_sapling', 'carnisylvan_sapling')
    p['encounter.json']['rules'].append(rule('sapling_removed_after_cast',
        {'kind': 'ability_cast', 'role': 'carnisylvan_sapling',
         'ability': actor('carnisylvan_sapling')['monster.json']['behavior']['attacks'][0]['ability']},
        [{'kind': 'remove', 'role': 'carnisylvan_sapling'}], delay=1))
    cover('carnisylvan_sapling', 'attacks[1]', p, SPELLS + 'sapling_explode.lua',
          'Fire combat unchanged; remove caster one millisecond after its cast.',
          'Delayed identity/Encounter placement requires native qualification.')
    actor('carnisylvan_sapling')['catalog.json']['definitions'].append(ref('Encounter', 'carnisylvan_sapling'))

    p = encounter('wormling', 'wormling')
    e = p['encounter.json']
    e['abilities'] = [{'key': 'wormling_death_blast', 'area': {'shape': 'square', 'radius': 1},
        'damage': {'damage_type': 'earth', 'min': 750, 'max': 750},
        'affects': {'players': True, 'creatures': []}, 'effect': 'canary.appearance:effect/hitbypoison'}]
    e['rules'].append(rule('wormling_death_blast', {'kind': 'creature_died', 'role': 'wormling'},
        [{'kind': 'cast', 'encounter_ability': 'wormling_death_blast', 'at': 'death_position'}]))
    cover('wormling', 'events=WormlingDeath', p, QUESTS + 'feaster_of_souls/creaturescripts_wormling_death.lua',
          'Death-position square earth blast 750.',
          'Source top-creature-per-tile restriction simplified to players in area; placement qualification pending.')

    p = encounter('dragon_egg', 'dragon_egg')
    p['encounter.json']['rules'].append(rule('egg_prevents_death', {'kind': 'lethal_damage', 'role': 'dragon_egg'},
        [{'kind': 'prevent_death', 'role': 'dragon_egg'}]))
    cover('dragon_egg', 'events=DragonEggPrepareDeath', p, QUESTS + 'forgotten_knowledge/creaturescripts_dragon_egg.lua',
          'Accepted prevent-death core; this actor is deliberately non-lethal.',
          'Source adds one HP rather than native prevent-death floor; healing/linked-boss exchange remains omitted.')

    p = encounter('leiden', 'leiden')
    p['encounter.json']['rules'].append(rule('leiden_player_damage_heals',
        {'kind': 'damage_taken', 'role': 'leiden', 'source': 'player'},
        [{'kind': 'convert_damage_to_heal', 'role': 'leiden'}]))
    cover('leiden', 'events=LeidenHeal', p, QUESTS + 'cults_of_tibia/creaturescripts_leiden_heal.lua',
          'Player damage becomes healing using existing encounter primitive.',
          'Source copies primary to secondary and extra addHealth; native conversion keeps each component.')

    # Owner-authorized single-pool content policy: accepted Q6a damage trigger,
    # source-backed unless-present creation, unresolved Global stacking retained.
    p = encounter('soul_war_taint_zones', 'cloak_of_terror')
    p['encounter.json']['rules'].append(rule('cloak_of_terror_bleeds',
        {'kind': 'damage_taken', 'role': 'cloak_of_terror', 'source': 'player'},
        [{'kind': 'map_item', 'operation': 'create', 'at': 'subject_position', 'unless_present': True,
          'item': ref('Item', '33854'), 'interaction': 'canary:interaction/blood_of_cloak_of_terror'}]))
    cover('cloak_of_terror', 'events=CloakOfTerrorHealthLoss', p, QUESTS + 'soul_war/soul_war_mechanics.lua',
          'SOURCE_BACKED_OTERYN_SIMPLIFICATION: accepted Q6a player damage leaves one initial pool; '
          'existing33854 suppresses another33854, matching donor item check (decayed pools can coexist).',
          'GLOBAL_POOL_STACKING_UNVERIFIED; interaction-domain step-in/decay runtime qualification remains pending.')
    for entry in p['manifest.json']['entries']:
        if entry['status'] == 'unresolved_semantics' and 'SW-6' in entry['resolution']:
            entry.update(status='approved_omission', resolution='Creation core is restored under explicit Oteryn '
                'single-initial-pool policy; Global stacking and pool interaction runtime remain unqualified.')

    p = encounter('faceless_bane', 'faceless_bane')
    e = p['encounter.json']
    e['state']['counters'].append({'name': 'faceless_lives', 'initial': 0})
    e['rules'].append(rule('faceless_heals_and_summons',
        {'kind': 'health_crossed', 'role': 'faceless_bane', 'percent': 20},
        [{'kind': 'heal', 'subject': {'role': 'faceless_bane'}, 'amount': 'full'},
         {'kind': 'counter', 'counter': 'faceless_lives', 'operation': 'add', 'value': 1}] +
        [{'kind': 'spawn', 'creature': ref('Creature', n), 'count': 1,
          'at': 'subject_position', 'owner': 'none', 'health': 'full'}
         for n in ('burster_spectre', 'gazer_spectre', 'ripper_spectre')],
        [{'kind': 'counter_compare', 'counter': 'faceless_lives', 'op': '<=', 'value': 3}]))
    cover('faceless_bane', 'events=facelessThink', p,
          QUESTS + 'the_dream_courts_quest/creaturescripts_facelessBane.lua',
          'At20percent heals fully and spawns all three spectres, at most four lives.',
          'Quest global storage and dynamically registered immunity are deliberately omitted; '
          'per-encounter lives replace mutable global storage.')

    # Retain an instance-local eight-second transformation core; original fixed
    # GreedMonsters coordinates, zone-wide damage and boss upgrades stay flagged.
    for name, next_name in [('weak_soul', 'strong_soul'), ('strong_soul', 'powerful_soul'),
                            ('powerful_soul', 'weak_soul')]:
        p = encounter('goshnars_greed', name)
        e = p['encounter.json']
        e['rules'].append(rule(name + '_matures', {'kind': 'creature_spawned', 'role': name},
            [{'kind': 'transform', 'role': name, 'into': ref('Creature', next_name), 'health': 'full'}], delay=8000))
        src = next(x['source_file'] for x in actor(name)['manifest.json']['entries']
                   if x['source_field'] == 'mType.onThink')
        for field in ('mType.onSpawn', 'mType.onThink'):
            cover(name, field, p, src, 'Spawn initiates an eight-second full-health next-soul transformation.',
                  'SOURCE_BACKED_OTERYN_SIMPLIFICATION: current-tile transform replaces fixed-position '
                  'create/remove. Shared source timer and PowerfulSoul boss upgrades are omitted; '
                  'native phase/role retention remains unqualified.')

    p = encounter('the_baron_from_below', 'the_baron_from_below')
    e = p['encounter.json']
    e['anchors'].extend([
        {'key': 'hungry_baron_tile', 'kind': 'point', 'description': 'Pinned hungry boss tile.',
         'location': {'x': 33648, 'y': 32300, 'floor': 15}},
        {'key': 'baron_food_tile', 'kind': 'point', 'description': 'Pinned food tile.',
         'location': {'x': 33647, 'y': 32300, 'floor': 15}}])
    e['state']['timers'].append({'name': 'baron_hunger', 'duration_ms': 40000, 'repeat': True})
    e['rules'].append(rule('baron_starts_hunger', {'kind': 'creature_spawned', 'role': 'the_baron_from_below'},
        [{'kind': 'timer', 'timer': 'baron_hunger', 'operation': 'start'}]))
    for trigger, suffix in [({'kind': 'creature_spawned', 'role': 'the_baron_from_below'}, 'initial'),
                            ({'kind': 'timer_elapsed', 'timer': 'baron_hunger', 'each': 'the_baron_from_below'}, 'cycle')]:
        e['rules'].append(rule('baron_hungry_' + suffix, trigger,
            [{'kind': 'transform', 'role': 'the_baron_from_below',
              'into': ref('Creature', 'the_hungry_baron_from_below'), 'health': 'keep_absolute'},
             {'kind': 'teleport', 'who': {'role': 'the_baron_from_below'}, 'to': 'hungry_baron_tile'},
             {'kind': 'spawn', 'creature': ref('Creature', 'organic_matter'), 'role': 'organic_matter',
              'count': 1, 'at': {'anchor': 'baron_food_tile'}, 'owner': 'none', 'health': 'full'}], delay=30000))
        e['rules'].append(rule('baron_returns_' + suffix, trigger,
            [{'kind': 'transform', 'role': 'the_baron_from_below',
              'into': ref('Creature', 'the_baron_from_below'), 'health': 'keep_absolute'}], delay=40000))
    cover('the_baron_from_below', 'events=TheBaronFromBelowThink', p,
          QUESTS + 'dangerous_depth/creaturescripts_the_baron_from_below.lua',
          'Thirty-second hunger, preserving absoluteHP, fixed teleport and organic spawn; returns after10seconds.',
          'Conditional food consumption/heal/adds and spectator scan remain omitted; timer role identity unqualified.')

    def vortex(e, trigger):
        _, text = source(SPELLS + 'charge_vortex.lua')
        positions = re.findall(r'Position\((\d+),\s*(\d+),\s*(\d+)\)', text.split('local function')[0])
        if len(positions) != 11:
            raise ValueError('charge vortex destination census changed')
        branches = []
        for number, xyz in enumerate(positions):
            anchor = 'vortex_tile_' + str(number)
            x, y, floor = map(int, xyz)
            e['anchors'].append({'key': anchor, 'kind': 'point', 'description': 'Pinned vortex tile.',
                                 'location': {'x': x, 'y': y, 'floor': floor}})
            branches.append({'weight': 1, 'actions': [{'kind': 'map_item', 'operation': 'transform',
                'item': ref('Item', '23049'), 'into': ref('Item', '22894'), 'anchor': anchor,
                'revert_after_ms': 10000}]})
        e['rules'].append(rule('anomaly_charges_vortex', trigger, [{'kind': 'one_of', 'branches': branches}]))
    cast('charged_anomaly', 'attacks[6]', 'charge_vortex.lua', vortex,
         'Source transforms arbitrary current ground; selected core transforms expected23049 only. '
         'Ground identity and map placement require qualification.')

    def ragiaz(e, trigger):
        e['anchors'].append({'key': 'ragiaz_regeneration_tile', 'kind': 'point',
            'description': 'Pinned Ragiaz regeneration tile.', 'location': {'x': 33487, 'y': 32333, 'floor': 14}})
        e['anchors'].append({'key': 'ragiaz_floor13', 'kind': 'area', 'description': 'Source floor13 gate.',
            'location': {'boxes': [{'x': [0, 65535], 'y': [0, 65535], 'floor': 13}]}})
        e['rules'].append(rule('ragiaz_regenerates', trigger,
            [{'kind': 'teleport', 'who': {'role': 'ragiaz'}, 'to': 'ragiaz_regeneration_tile'},
             {'kind': 'heal', 'subject': {'role': 'ragiaz'}, 'amount': 1000}],
            [{'kind': 'in_anchor', 'subject': {'role': 'ragiaz'}, 'anchor': 'ragiaz_floor13'}]))
    cast('ragiaz', 'defenses[3]', 'ragiaz_transform.lua', ragiaz,
         'Source floor13 gate retained; existing capsule/current-position exchange omitted. Teleport/heal core retained.')

    def fairy_heal(e, trigger):
        e['state']['flags'].append({'name': 'fairy_regeneration_cooldown', 'initial': False})
        e['state']['timers'].append({'name': 'fairy_regeneration_reset', 'duration_ms': 30000, 'repeat': False})
        conditions = [{'kind': 'health_percent', 'role': 'professor_maxxen', 'op': '<', 'value': 10},
                      {'kind': 'flag', 'flag': 'fairy_regeneration_cooldown', 'value': False}]
        e['rules'].append(rule('fairy_heal_starts', trigger,
            [{'kind': 'flag', 'flag': 'fairy_regeneration_cooldown', 'value': True},
             {'kind': 'timer', 'timer': 'fairy_regeneration_reset', 'operation': 'start'}], conditions))
        # The delayed event is gated by the cast's initial cooldown state, then
        # observed at10s. A separate timer prevents later casts scheduling heals.
        e['state']['timers'].append({'name': 'fairy_delayed_heal', 'duration_ms': 10000, 'repeat': False})
        e['rules'][-1]['actions'].append({'kind': 'timer', 'timer': 'fairy_delayed_heal', 'operation': 'start'})
        e['rules'].append(rule('fairy_heal_finishes', {'kind': 'timer_elapsed', 'timer': 'fairy_delayed_heal'},
            [{'kind': 'heal', 'subject': {'role': 'professor_maxxen'}, 'amount': {'min': 7500, 'max': 8000}}]))
        e['rules'].append(rule('fairy_cooldown_ends', {'kind': 'timer_elapsed', 'timer': 'fairy_regeneration_reset'},
            [{'kind': 'flag', 'flag': 'fairy_regeneration_cooldown', 'value': False}]))
    cast('professor_maxxen', 'defenses[5]', 'glooth_fairy_healing.lua', fairy_heal,
         'Oteryn instance flag30s replaces CONDITION_REGENERATION SUBID88888; tiny regeneration and '
         'native retained-caster timer identity remain unqualified.')

    # Every audited omission gets a durable answer, including genuine owner gaps.
    audit = read(inventory)
    remaining = []
    for row in audit['remaining_whole_rows'] + audit['remaining_partial_action_rows']:
        if not any(x['source_field'] == row['source_field'] for x in restored.get(row['monster'], [])):
            remaining.append({**row, 'status': 'FLAGGED_RUNTIME_OR_DOMAIN_FOLLOWUP',
                              'architecture_issue': 162, 'not_claimed_implemented': True})
    receipt = {'schema': 'OTERYN_REMAINING_MECHANIC_CORES/v1', 'runtime_qualified': False,
               'baseline_index_sha256': hashlib.sha256((baseline / 'population-index.json').read_bytes()).hexdigest(),
               'actors': [], 'encounters': [], 'index_monsters': [], 'source_checks': evidence,
               'remaining': remaining, 'decisions_for_162': [
                   'SW6 adopts reversible Oteryn single33854-pool creation; Global stacking and step-in runtime unverified.',
                   'Ferumbras reset is quest-wide shared state over player/global storage and fixed map items; '
                   'requires accepted quest-domain outcome contract, not an invented local Encounter reset.']}
    for name, vals in actors.items():
        original = {f: read(baseline / 'bundles' / name / f) for f in FILES}
        if vals['monster.json']['creature']['stats'] != original['monster.json']['creature']['stats']:
            raise ValueError('changed stats ' + name)
        errors = vm.validate(*(vals[f] for f in FILES))
        if errors:
            raise ValueError((name, errors))
        dest = output / 'bundles' / name
        for filename, value in vals.items():
            write(dest / filename, value)
        row = copy.deepcopy(index[name])
        row['sha256'] = population.admission.bundle_digest(dest)
        flags = set(row.get('completion_flags', [])) | {'SOURCE_TYPED_CORE_RESTORED', 'GAMEPLAY_UNVERIFIED', 'ENCOUNTER_PLACEMENT_PENDING'}
        if limits.get(name):
            flags |= {'SOURCE_BEHAVIOR_PARTIAL', 'SOURCE_BACKED_OTERYN_SIMPLIFICATION'}
        if name == 'cloak_of_terror':
            flags.discard('SW6_POOL_CREATION_POLICY_NOT_IMPLEMENTED')
            flags.add('GLOBAL_POOL_STACKING_UNVERIFIED')
            row['resolved_completion_flags'] = ['SW6_POOL_CREATION_POLICY_NOT_IMPLEMENTED']
        row['completion_flags'] = sorted(flags)
        receipt['index_monsters'].append(row)
        receipt['actors'].append({'monster': name, 'original_bundle_digest': index[name]['sha256'],
            'bundle_digest': row['sha256'], 'restored_source_rows': restored[name],
            'completion_flags': row['completion_flags'], 'limitations': limits.get(name, [])})
    for name, vals in encounters.items():
        from validate_encounter import refs
        definitions = vals['catalog.json']['definitions']
        for dependency in refs(vals['encounter.json']):
            if dependency not in definitions:
                definitions.append(dependency)
        errors = ve.validate(vals['encounter.json'], vals['catalog.json'], vals['manifest.json'])
        if errors:
            raise ValueError((name, errors))
        for filename, value in vals.items():
            write(output / 'encounters' / name / filename, value)
        receipt['encounters'].append({'encounter': name, 'remaining_runtime_qualification': True,
            'new_definition': not (baseline / 'encounters' / name).exists(),
            'source_bound_allowlist': vals['manifest.json']['sources']})
    receipt['counts'] = {'actors': len(actors), 'restored_components': sum(map(len, restored.values())),
                         'encounter_successors': len(encounters), 'remaining_components': len(remaining)}
    write(output / 'completion.json', receipt)
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('repo', 'baseline', 'canary', 'inventory', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(**vars(args))['counts']))
