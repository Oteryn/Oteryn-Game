#!/usr/bin/env python3
"""Stage fully resolved Canary monster bundles for WorldProject/v2 admission.

Implements OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §2-§5: selects the admissible bundles, maps
their provisional `canary:` keys to Oteryn production keys and their Canary item ids to Oteryn Item
keys, and writes Reference records, declarative authoring profiles and source identity bindings in
the exact serde shapes the game server parses. The materializer
(`apps/game-server/examples/materialize_content_world_project_v2.rs`) consumes the staged file.

Usage:
  python creature_admission_stage.py --bundles <population_census --bundles dir> \
      --item-map <export_reference_item_identity_map output> --out <staged.json> [--pilot]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from decimal import Decimal
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
INDEX = ROOT / 'tools/content-schema/monster-authoring/samples/population-bundles-canary-47dfd51f.json'
ENCOUNTERS = ROOT / 'tools/content-schema/encounter-authoring/samples'
BUNDLE_FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
SCHEMA = 'OTERYN_CREATURE_ADMISSION_STAGED/v1'
REVISION = 'definition-r1'
CANARY_REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
ITEM_ALLOCATION_SHA256 = 'ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966'
# Protected Item rekeys applied on top of the allocation map (the allocation keeps the old key).
ITEM_REKEYS = (ROOT / 'docs/agents/evidence/OTV2-20260927-r7-p04-gold-coin.json',)
REFERENCE = ROOT / 'content/world/definitions/reference.json'
# Admission §2: a pilot covering each profile shape (shared spell, inline condition, summons,
# voices, variants, chain, invisible and familiar appearance, skipped loot entry, bosstiary).
PILOT = ('rat', 'dragon', 'dragon_lord', 'demon', 'warlock', 'orc_shaman', 'bonebeast', 'hydra',
         'nightmare', 'ghoul', 'undead_dragon', 'frost_dragon', 'water_elemental', 'hellhound',
         'plaguesmith', 'massive_fire_elemental', 'serpent_spawn', 'wyrm', 'juggernaut', 'grim_reaper',
         'mawhawk',  # a loot entry that skips later drops of the same item
         # summoned by the pilot monsters above, so that the pilot closes over its references
         'fire_elemental', 'snake', 'stone_golem', 'clay_guardian')
NONPRODUCTION = ('fixture', 'synthetic', 'evidence', 'test-only')


class StageError(Exception):
    pass


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode('utf-8')


def bundle_digest(directory: Path) -> str:
    digest = hashlib.sha256()
    for name in BUNDLE_FILES:
        data = (directory / name).read_bytes()
        digest.update(f'{name}\0{len(data)}\0'.encode('ascii') + data)
    return digest.hexdigest()


def ppm(percent: Any) -> int:
    value = Decimal(str(percent)) * 10_000
    if value != value.to_integral_value() or not 0 <= value <= 1_000_000:
        raise StageError(f'percent {percent} is not an exact ppm value (D1)')
    return int(value)


def nonproduction(key: str) -> bool:
    lower = key.lower()
    return any(segment == 'test' for segment in re.split(r'[:./_-]', lower)) or any(word in lower for word in NONPRODUCTION)


class Mapper:
    def __init__(self, item_map: dict[int, str]):
        self.item_map = item_map

    def key(self, family: str, key: str) -> str:
        if not key.startswith('canary:'):
            raise StageError(f'unexpected key namespace {key}')
        namespace, _, path = key[len('canary:'):].partition('/')
        if family == 'Item':
            if namespace != 'item' or not path.isdigit():
                raise StageError(f'unexpected Item key {key}')
            native = self.item_map.get(int(path))
            if native is None:
                raise StageError(f'Item {path} is not in the Oteryn Item registry')
            return native
        prefix = {'Creature': 'creature', 'Presentation': 'presentation', 'Behavior': 'behavior', 'Loot': 'loot',
                  'Ability': 'ability', 'Effect': 'effect', 'Formula': 'formula'}[family]
        dotted = path.replace('/', '.')
        if family == 'Creature':
            native = f'oteryn:creature.{dotted}'
        elif dotted.startswith('spell.'):
            native = f'oteryn:{prefix}.{dotted}'
        else:
            native = f'oteryn:{prefix}.creature.{dotted}'
        if nonproduction(native) or not re.fullmatch(r'[a-z0-9:._/-]+', native):
            raise StageError(f'{native} is not a production key')
        return native

    def ref(self, reference: dict[str, str]) -> dict[str, str]:
        return {'family': reference['family'], 'key': self.key(reference['family'], reference['key']), 'revision': REVISION}

    def identity(self, family: str, identity: dict[str, str]) -> dict[str, str]:
        return {'family': family, 'key': self.key(family, identity['key']), 'revision': REVISION}


def asset(binding: str) -> str:
    return binding


def effect_presentation(value: dict | None) -> dict | None:
    if not value:
        return None
    return {key: value[key] for key in ('impact_asset_binding', 'projectile_asset_binding', 'path_asset_binding') if key in value}


def condition(value: dict, mapper: Mapper) -> dict:
    result: dict[str, Any] = {'condition_type': value['type'], 'lifetime': {'fixed_duration': 'FixedDuration',
                                                                          'damage_schedule': 'DamageSchedule'}[value['lifetime']]}
    dot = value.get('damage_over_time')
    if dot:
        first = {'after_interval': 'AfterInterval', 'immediate': 'Immediate'}[dot['first_tick']]
        if dot['tick_profile'] == 'fixed':
            result['damage_over_time'] = {'tick_profile': 'Fixed', 'first_tick': first, 'ticks': [
                {'count': tick['count'], 'interval_ms': tick['interval_ms'], 'amount': tick['amount']} for tick in dot['fixed_ticks']]}
        elif dot['tick_profile'] == 'decreasing':
            decreasing = {'tick_profile': 'Decreasing', 'first_tick': first,
                          'total_minimum': dot['total_damage_range']['minimum'], 'total_maximum': dot['total_damage_range']['maximum'],
                          'tick_interval_ms': dot['tick_interval_ms']}
            if dot['initial_tick']['mode'] == 'fixed':
                decreasing['initial_tick_amount'] = dot['initial_tick']['amount']
            result['damage_over_time'] = decreasing
        else:
            geometric = dot['geometric']
            result['damage_over_time'] = {'tick_profile': 'Geometric', 'first_tick': first,
                                          'base_minimum': geometric['base_range']['minimum'],
                                          'base_maximum': geometric['base_range']['maximum'], 'factor': geometric['factor'],
                                          'tick_counts': sorted(geometric['tick_counts']),
                                          'tick_interval_ms': geometric['tick_interval_ms']}
    if 'speed_formula' in value:
        result['speed_formula'] = mapper.ref(value['speed_formula'])
    if value.get('attribute_modifiers'):
        result['attribute_modifiers'] = [{'attribute': modifier['attribute'],
                                          'mode': {'percent_of_base': 'PercentOfBase', 'add': 'Add'}[modifier['mode']],
                                          'value': modifier['value']} for modifier in value['attribute_modifiers']]
    return result


def inline_effect(effect: dict, mapper: Mapper) -> dict:
    op = effect['operation']
    if op == 'condition':
        operation: dict[str, Any] = {'operation': 'Condition', 'condition': condition(effect['condition'], mapper)}
        if 'duration_ms' in effect:
            operation['duration_ms'] = effect['duration_ms']
    elif op == 'appearance_transform':
        target = effect['appearance_transform']
        operation = {'operation': 'AppearanceTransform', 'duration_ms': effect['duration_ms']}
        for field in ('creature', 'item'):
            if field in target:
                operation[field] = mapper.ref(target[field])
    elif op == 'create_item':
        operation = {'operation': 'CreateItem', 'item': mapper.ref(effect['created_item'])}
    elif op == 'presentation_only':
        operation = {'operation': 'PresentationOnly'}
    elif op == 'remove_condition':
        operation = {'operation': 'RemoveCondition', 'condition': effect['removed_condition']}
    elif op == 'remove_items':
        removed = effect['removed_items']
        operation = {'operation': 'RemoveItems', 'items': [mapper.ref(item) for item in removed['items']],
                     'selection': {'first_listed_per_tile': 'FirstListedPerTile', 'top_item_first_tile': 'TopItemFirstTile'}[removed['selection']]}
    elif op == 'summon_creature':
        summon = effect['summon']
        operation = {'operation': 'SummonCreature', 'creatures': sorted((mapper.ref(c) for c in summon['creatures']),
                                                                         key=lambda r: (r['family'], r['key'], r['revision'])),
                     'count_mode': {'fill_to_limit': 'FillToLimit', 'fixed': 'Fixed'}[summon['count_mode']],
                     'count': summon['count'], 'only_below_summons': summon['only_below_summons'], 'owned': summon['owned'],
                     'max_offset_tiles': summon['max_offset_tiles']}
    else:
        raise StageError(f'effect operation {op} is executable, not inline')
    result = {'key': mapper.key('Effect', effect['identity']['key']), 'operation': operation}
    presentation = effect_presentation(effect.get('presentation'))
    if presentation:
        result['presentation'] = presentation
    return result


def area(value: dict) -> dict:
    if 'length_tiles' in value:
        return {'shape': 'Beam', 'length_tiles': value['length_tiles'], 'spread_tiles': value['spread_tiles']}
    if 'radius_tiles' in value:
        return {'shape': 'Circle', 'radius_tiles': value['radius_tiles']}
    matrix = {'shape': 'Matrix', 'north': value['matrix']['north']}
    if value['matrix'].get('diagonal'):
        matrix['diagonal'] = value['matrix']['diagonal']
    return matrix


def profile(target: dict, kind: str, data: Any) -> dict:
    return {'target': target, 'data': {'kind': kind, 'profile': data}}


class Stage:
    def __init__(self, mapper: Mapper):
        self.mapper = mapper
        self.records: dict[tuple[str, str], dict] = {}
        self.profiles: dict[tuple[str, str], dict] = {}
        self.bindings: list[dict] = []

    def add(self, store: dict, family: str, key: str, value: dict, owner: str) -> None:
        existing = store.get((family, key))
        if existing is not None and canonical(existing) != canonical(value):
            raise StageError(f'{family} {key} differs between bundles ({owner})')
        store[(family, key)] = value

    def stage_dependencies(self, dependencies: dict, owner: str) -> None:
        m = self.mapper
        effects = {effect['identity']['key']: effect for effect in dependencies['effects']}
        for formula in dependencies['formulas']:
            identity = m.identity('Formula', formula['identity'])
            self.add(self.records, 'Formula', identity['key'], {'kind': 'Formula', 'identity': identity}, owner)
            kind = formula['kind']
            if kind == 'range':
                data = {'formula': 'Range', **formula['magnitude']}
            elif kind == 'melee_attack_skill':
                data = {'formula': 'MeleeAttackSkill', **formula['melee']}
            elif kind == 'speed_modifier':
                data = {'formula': 'SpeedModifier', **formula['speed']}
            else:
                data = {'formula': 'CasterMagnitude'}
            self.add(self.profiles, 'Formula', identity['key'], profile(identity, 'Formula', data), owner)
        for effect in dependencies['effects']:
            if effect['operation'] not in ('damage', 'heal'):
                continue
            identity = m.identity('Effect', effect['identity'])
            self.add(self.records, 'Effect', identity['key'], {
                'kind': 'Effect', 'identity': identity, 'client_projection': 'ServerOnly',
                'effect_family': 'Damage' if effect['operation'] == 'damage' else 'Heal',
                'formula': m.ref(effect['formula'])}, owner)
            data: dict[str, Any] = {'damage_type': effect['damage_type']}
            if effect.get('mitigated_by'):
                data['mitigated_by'] = sorted({'armor': 'Armor', 'shield': 'Shield'}[value] for value in effect['mitigated_by'])
            if 'affects' in effect:
                affects = effect['affects']
                data['affects'] = {'kind': {'masterless_monsters': 'MasterlessMonsters', 'non_player_side': 'NonPlayerSide',
                                            'player_side': 'PlayerSide', 'named_creatures': 'NamedCreatures'}[affects['kind']],
                                   'top_creature_only': affects['top_creature_only'],
                                   'excludes_caster_name': affects['excludes_caster_name'],
                                   'includes_caster': affects['includes_caster']}
                if 'creatures' in affects:
                    data['affects']['creatures'] = sorted((m.ref(c) for c in affects['creatures']),
                                                          key=lambda r: (r['family'], r['key'], r['revision']))
            presentation = effect_presentation(effect.get('presentation'))
            if presentation:
                data['presentation'] = presentation
            self.add(self.profiles, 'Effect', identity['key'], profile(identity, 'Effect', data), owner)
        for ability in dependencies['abilities']:
            identity = m.identity('Ability', ability['identity'])
            executable, ordered = [], []
            for reference in ability.get('effects', []):
                effect = effects[reference['key']]
                if effect['operation'] in ('damage', 'heal'):
                    mapped = m.ref(reference)
                    executable.append(mapped)
                    ordered.append({'kind': 'Executable', 'effect': mapped})
                else:
                    ordered.append({'kind': 'Inline', 'effect': inline_effect(effect, m)})
            self.add(self.records, 'Ability', identity['key'], {'kind': 'Ability', 'identity': identity, 'effects': executable}, owner)
            details: dict[str, Any] = {'kind': {'melee': 'Melee', 'spell': 'Spell'}[ability['kind']],
                                       'range_tiles': ability['range_tiles'], 'needs_target': ability['needs_target'],
                                       'needs_direction': ability['needs_direction']}
            if 'area' in ability:
                details['area'] = area(ability['area'])
            if ordered:
                details['effects'] = ordered
            if 'variants' in ability:
                details['variants'] = [m.ref(variant) for variant in ability['variants']]
            if 'path_requirement' in ability:
                details['path_requirement'] = ability['path_requirement']
            if 'chain' in ability:
                details['chain'] = dict(ability['chain'])
            for cue in ('cast_cue', 'impact_cue'):
                if cue in ability.get('audio', {}):
                    details[cue] = ability['audio'][cue]
            self.add(self.profiles, 'Ability', identity['key'], profile(identity, 'Ability', {'details': details}), owner)
        if dependencies['documents'] or dependencies['loot_tables']:
            raise StageError(f'{owner}: documents and nested loot tables are outside wave A')

    def stage_monster(self, monster: dict, file: str) -> str:
        m = self.mapper
        creature, behavior, presentation = monster['creature'], monster['behavior'], monster['presentation']
        owner = creature['identity']['key']
        identity = m.identity('Creature', creature['identity'])
        presentation_ref, behavior_ref = m.ref(creature['presentation']), m.ref(creature['behavior'])
        loot = monster.get('loot')
        loot_ref = m.ref(creature['loot']) if 'loot' in creature else None
        record = {'kind': 'Creature', 'identity': identity, 'client_projection': 'ClientSafe',
                  'presentation': presentation_ref, 'behavior': behavior_ref, 'loot': loot_ref}
        self.add(self.records, 'Creature', identity['key'], record, owner)
        self.add(self.records, 'Presentation', presentation_ref['key'],
                 {'kind': 'Generic', 'identity': presentation_ref, 'client_projection': 'ClientSafe'}, owner)
        self.add(self.records, 'Behavior', behavior_ref['key'],
                 {'kind': 'Generic', 'identity': behavior_ref, 'client_projection': 'ServerOnly'}, owner)
        if loot_ref:
            entries, skips = [], []
            for entry in loot['entries']:
                if entry['min_count'] < 1 or 'instance_attributes' in entry or 'contents_loot' in entry:
                    raise StageError(f'{owner}: loot entry outside the Reference Loot contract')
                entries.append({'item': m.ref(entry['item']), 'min_count': entry['min_count'], 'max_count': entry['max_count'],
                                'probability_ppm': ppm(entry['probability_percent'])})
                skips.append({'skip_later_same_item_after_success': entry['skip_later_same_item_after_success']})
            self.add(self.records, 'Loot', loot_ref['key'], {'kind': 'Loot', 'identity': loot_ref,
                                                             'algorithm': 'IndependentBernoulliPpm', 'entries': entries}, owner)
            if any(skip['skip_later_same_item_after_success'] for skip in skips):
                self.add(self.profiles, 'Loot', loot_ref['key'], profile(loot_ref, 'Loot', {'entries': skips}), owner)
        self.add(self.profiles, 'Creature', identity['key'], profile(identity, 'Creature', self.creature_profile(monster)), owner)
        self.add(self.profiles, 'Behavior', behavior_ref['key'], profile(behavior_ref, 'Behavior', self.behavior_profile(behavior)), owner)
        self.add(self.profiles, 'Presentation', presentation_ref['key'],
                 profile(presentation_ref, 'Presentation', self.presentation_profile(presentation)), owner)
        self.bindings.append({'source_key': 'oteryn:source.canary', 'source_revision': CANARY_REVISION,
                              'identity_namespace': 'canary/monster-file', 'external_id': file,
                              'target': identity, 'disposition': 'EXACT'})
        return identity['key']

    def creature_profile(self, monster: dict) -> dict:
        m = self.mapper
        creature, behavior = monster['creature'], monster['behavior']
        stats = creature['stats']
        if stats['max_health'] != stats['initial_health']:
            raise StageError(f"{creature['identity']['key']}: initial health differs from maximum health")
        schedule_refs = {json.dumps(m.ref(s['ability']), sort_keys=True) for s in behavior['attacks'] + behavior['defenses']}
        result: dict[str, Any] = {
            'health': stats['max_health'], 'experience': stats['experience'], 'speed': stats['speed'], 'armor': stats['armor'],
            'resistances': sorted(({'damage_type': r['damage_type'], 'percent': r['reduction_percent']} for r in creature['resistances']),
                                  key=lambda r: r['damage_type']),
            'immunities': sorted(creature['immunities']['damage_types']),
            'abilities': sorted((json.loads(ref) for ref in schedule_refs), key=lambda r: (r['family'], r['key'], r['revision'])),
        }
        if 'mitigation_percent' in stats:
            result['mitigation'] = stats['mitigation_percent']
        bestiary = creature.get('bestiary')
        if bestiary:
            result['bestiary'] = {'difficulty': bestiary['difficulty'], 'occurrence': bestiary['occurrence'],
                                  'kill_thresholds': bestiary['kill_thresholds'], 'charm_points': bestiary['charm_points']}
        summoning = creature['summoning']
        if summoning['is_familiar']:
            familiar = summoning['familiar']
            if familiar['duration_ms'] % 1000:
                raise StageError('familiar duration is not whole seconds')
            result['familiar'] = {'vocation': familiar['vocation'], 'summon_ability': m.ref(familiar['summon_ability']),
                                  'duration_seconds': familiar['duration_ms'] // 1000, 'mana_cost': familiar['mana_cost']}
            if 'owner_speed_bonus' in familiar:
                result['familiar']['owner_speed_bonus'] = familiar['owner_speed_bonus']
        details: dict[str, Any] = {
            'display_name': creature['display_name'], 'inspection': creature['inspection']['description'],
            'defense': stats['defense'], 'critical_chance_ppm': ppm(stats['critical_chance_percent']),
            'flags': creature['flags'],
            'summoning': {'summonable': summoning['summonable'], 'convinceable': summoning['convinceable'],
                          'is_familiar': summoning['is_familiar']},
            'system_eligibility': creature['system_eligibility'],
            'spawn_eligibility': {'period': {'all': 'All', 'day': 'Day', 'night': 'Night'}[creature['spawn_eligibility']['period']],
                                  'ignore_period_underground': creature['spawn_eligibility']['ignore_period_underground'],
                                  'blocked_by_nearby_players': creature['spawn_eligibility']['blocked_by_nearby_players']},
        }
        if 'mana_cost' in summoning:
            details['summoning']['mana_cost'] = summoning['mana_cost']
        forms = creature.get('name_forms', {})
        for field in ('article', 'plural'):
            if field in forms:
                details[field] = forms[field]
        if creature['immunities']['conditions']:
            details['condition_immunities'] = sorted(creature['immunities']['conditions'])
        if bestiary:
            details['bestiary'] = {'class': bestiary['class'], 'taxonomy': bestiary['taxonomy']}
            for field in ('stars', 'locations'):
                if field in bestiary:
                    value = bestiary[field]
                    details['bestiary'][field] = value.strip() if isinstance(value, str) else value
        if 'bosstiary' in creature:
            details['bosstiary'] = creature['bosstiary']
        for field in ('corpse_item', 'soul_core_item', 'reward_encounter'):
            if field in creature:
                details[field] = m.ref(creature[field])
        if 'encyclopedia' in creature:
            details['encyclopedia_document'] = m.ref(creature['encyclopedia']['description_document'])
        if 'death_residue' in creature:
            residue = {'item': m.ref(creature['death_residue']['item'])}
            if 'fluid_type' in creature['death_residue']:
                residue['fluid_type'] = creature['death_residue']['fluid_type']
            details['death_residue'] = residue
        for field in ('damage_reflection', 'healing_from_damage'):
            if creature[field]:
                details[field] = sorted(creature[field], key=lambda r: r['damage_type'])
        result['details'] = details
        return result

    def behavior_profile(self, behavior: dict) -> dict:
        m = self.mapper
        movement, targeting = behavior['movement'], behavior['targeting']

        def schedule(value: dict) -> dict:
            result = {'ability': m.ref(value['ability']), 'interval_ms': value['interval_ms'], 'chance_ppm': ppm(value['chance_percent'])}
            for field in ('magnitude', 'range_tiles'):
                if field in value:
                    result[field] = value[field]
            return result

        result: dict[str, Any] = {
            'movement': {'can_walk': movement['can_walk'], 'pass_through': movement['pass_through'], 'pushable': movement['pushable'],
                         'push_items': movement['push_items'], 'push_creatures': movement['push_creatures'],
                         'walks_on_energy': movement['field_permissions']['energy'],
                         'walks_on_fire': movement['field_permissions']['fire'],
                         'walks_on_poison': movement['field_permissions']['poison']},
            'targeting': {'hostile': targeting['hostile'], 'can_target': targeting['can_target'],
                          'sense_invisible': targeting['sense_invisible'], 'target_distance_tiles': targeting['target_distance_tiles'],
                          'static_attack_chance_ppm': ppm(targeting['static_attack_chance_percent']),
                          'flee_health': targeting['flee_health']},
        }
        if 'change_target' in targeting:
            result['targeting']['change_target'] = {'interval_ms': targeting['change_target']['interval_ms'],
                                                    'chance_ppm': ppm(targeting['change_target']['chance_percent'])}
        if 'strategy_weights' in targeting:
            result['targeting']['strategy_weights'] = targeting['strategy_weights']
        for field in ('attacks', 'defenses'):
            if behavior[field]:
                result[field] = [schedule(value) for value in behavior[field]]
        if 'voices' in behavior:
            voices = behavior['voices']
            result['voices'] = {'interval_ms': voices['interval_ms'], 'chance_ppm': ppm(voices['chance_percent']),
                                'entries': [{'text': entry['text'].strip(), 'mode': {'say': 'Say', 'yell': 'Yell'}[entry['mode']]}
                                            for entry in voices['entries']]}
        if 'summons' in behavior:
            summons = behavior['summons']
            result['summons'] = {'max_summons': summons['max_summons'], 'entries': [
                {'creature': m.ref(entry['creature']), 'interval_ms': entry['interval_ms'],
                 'chance_ppm': ppm(entry['chance_percent']), 'count': entry['count']} for entry in summons['entries']]}
        if 'periodic_audio' in behavior:
            audio = behavior['periodic_audio']
            result['periodic_audio'] = {'interval_ms': audio['interval_ms'], 'chance_ppm': ppm(audio['chance_percent']),
                                        'cue_ids': sorted(audio['cue_ids'])}
        if 'faction_and_preferences' in behavior:
            faction = behavior['faction_and_preferences']
            result['faction'] = {'faction': faction['faction'], 'enemy_factions': sorted(faction['enemy_factions']),
                                 'prefer_player': faction['prefer_player'], 'prefer_master': faction['prefer_master']}
        if behavior['event_bindings']:
            raise StageError('behaviour event bindings are outside wave A')
        return result

    def presentation_profile(self, presentation: dict) -> dict:
        appearance = presentation['appearance']
        result: dict[str, Any] = {'light_level': presentation['light']['level']}
        if 'asset_binding' in appearance:
            result['asset_binding'] = asset(appearance['asset_binding'])
        if 'selection' in appearance:
            result['selection'] = {'owner_familiar_look': 'OwnerFamiliarLook', 'invisible': 'Invisible'}[appearance['selection']]
        slots = {'head': 'Head', 'body': 'Body', 'legs': 'Legs', 'feet': 'Feet', 'mount_head': 'MountHead',
                 'mount_body': 'MountBody', 'mount_legs': 'MountLegs', 'mount_feet': 'MountFeet', 'addon': 'Addon',
                 'mount': 'Mount', 'familiar': 'Familiar', 'wing': 'Wing', 'aura': 'Aura', 'effect': 'Effect', 'shader': 'Shader'}
        order = list(slots)
        for field, binding_field in (('palette_bindings', 'palette_binding'), ('attachment_bindings', 'asset_binding'),
                                     ('visual_effect_bindings', 'asset_binding')):
            if appearance[field]:
                result[field] = [{'slot': slots[value['slot']], 'asset_binding': value[binding_field]}
                                 for value in sorted(appearance[field], key=lambda v: order.index(v['slot']))]
        if 'color_binding' in presentation['light']:
            result['light_color_binding'] = presentation['light']['color_binding']
        if presentation['audio']['event_bindings']:
            events = ['cast', 'impact', 'death', 'periodic']
            result['audio_bindings'] = [{'event': value['event'].capitalize(), 'cue_id': value['cue_id'],
                                         'asset_binding': value['asset_binding']}
                                        for value in sorted(presentation['audio']['event_bindings'],
                                                            key=lambda v: (events.index(v['event']), v['cue_id']))]
        if 'variant_label' in presentation:
            result['variant_label'] = presentation['variant_label']
        if 'status_marker' in presentation:
            result['status_marker'] = presentation['status_marker'].capitalize()
        return result


def definition_refs(value: Any):
    """Yield every non-Item typed definition reference as (family, key)."""
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            if value['family'] != 'Item':
                yield value['family'], value['key']
            return
        for child in value.values():
            yield from definition_refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from definition_refs(child)


def encounter_covered() -> set[str]:
    covered: set[str] = set()
    for manifest in sorted(ENCOUNTERS.glob('*/manifest.json')):
        for creatures in json.loads(manifest.read_text(encoding='utf-8'))['covers'].values():
            covered.update(creatures)
    return covered


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--bundles', type=Path, required=True)
    parser.add_argument('--item-map', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--pilot', action='store_true')
    args = parser.parse_args()

    item_export = json.loads(args.item_map.read_text(encoding='utf-8'))
    if item_export['allocation_digest_sha256'] != ITEM_ALLOCATION_SHA256:
        raise StageError('Item identity map allocation drifted')
    item_map = {row['source_item_id']: row['native_key'] for row in item_export['records']}
    rekeys = []
    for path in ITEM_REKEYS:
        evidence = json.loads(path.read_text(encoding='utf-8'))['source_identity']
        source_id, old, new = evidence['source_item_id'], evidence['current_native_key'], evidence['target_native_key']
        if item_map.get(source_id) != old:
            raise StageError(f'Item rekey {path.name} does not match the allocation map')
        item_map[source_id] = new
        rekeys.append({'source_item_id': source_id, 'from': old, 'to': new,
                       'evidence_sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    registered = {record['identity']['key'] for record in json.loads(REFERENCE.read_text(encoding='utf-8'))['records']
                  if record['identity']['family'] == 'Item'}
    if not set(item_map.values()) <= registered:
        raise StageError('Item identity map names keys absent from content/world')
    mapper = Mapper(item_map)
    index = json.loads(INDEX.read_text(encoding='utf-8'))
    covered = encounter_covered()
    candidates: dict[str, tuple[dict, dict, dict, set, set]] = {}
    deferred: dict[str, list] = {'encounter': [], 'unregistered_items': [], 'reference_loot_contract': [],
                                 'unresolved_reference': []}
    for row in index['monsters']:
        if args.pilot and row['monster'] not in PILOT:
            continue
        directory = args.bundles / row['monster']
        if bundle_digest(directory) != row['sha256']:
            raise StageError(f"{row['monster']}: bundle digest differs from the census index")
        monster = json.loads((directory / 'monster.json').read_text(encoding='utf-8'))
        dependencies = json.loads((directory / 'dependencies.json').read_text(encoding='utf-8'))
        key = monster['creature']['identity']['key']
        if key in covered:
            deferred['encounter'].append(row['monster'])
            continue
        missing = sorted({int(item) for item in re.findall(r'canary:item/(\d+)', json.dumps([monster, dependencies]))}
                         - set(mapper.item_map))
        if missing:
            deferred['unregistered_items'].append({'monster': row['monster'], 'items': missing})
            continue
        if any(entry['min_count'] < 1 for entry in (monster.get('loot') or {}).get('entries', [])):
            deferred['reference_loot_contract'].append(row['monster'])
            continue
        probe = Stage(mapper)
        probe.stage_dependencies(dependencies, key)
        probe.stage_monster(monster, row['file'])
        produced = {(value['identity']['family'], value['identity']['key']) for value in probe.records.values()}
        referenced = set(definition_refs([probe.records, probe.profiles])) - produced
        candidates[row['monster']] = (row, monster, dependencies, produced, referenced)

    # Admit only a closed set: every non-Item reference must resolve to a record of an admitted monster.
    admitted_set = set(candidates)
    while True:
        available = set().union(*(candidates[name][3] for name in admitted_set)) if admitted_set else set()
        dropped = {name: sorted(candidates[name][4] - available) for name in admitted_set
                   if not candidates[name][4] <= available}
        if not dropped:
            break
        for name, keys in dropped.items():
            admitted_set.discard(name)
            deferred['unresolved_reference'].append({'monster': name, 'references': [key for _, key in keys]})
    deferred['unresolved_reference'].sort(key=lambda value: value['monster'])

    stage = Stage(mapper)
    admitted = []
    for name, (row, monster, dependencies, _, _) in candidates.items():
        if name not in admitted_set:
            continue
        key = monster['creature']['identity']['key']
        probe = Stage(mapper)
        probe.records, probe.profiles = dict(stage.records), dict(stage.profiles)
        probe.stage_dependencies(dependencies, key)
        probe.stage_monster(monster, row['file'])
        stage.records, stage.profiles, stage.bindings = probe.records, probe.profiles, stage.bindings + probe.bindings
        admitted.append(name)
    if args.pilot and len(admitted) != len(PILOT):
        raise StageError(f'pilot admitted {len(admitted)} of {len(PILOT)}')

    order = lambda item: (item[0][0], item[0][1])  # noqa: E731
    staged = {
        'schema': SCHEMA,
        'wave': 'pilot' if args.pilot else 'A',
        'source': {'repository': index['source']['repository'], 'revision': CANARY_REVISION,
                   'census_index_sha256': hashlib.sha256(INDEX.read_bytes()).hexdigest(),
                   'item_allocation_sha256': ITEM_ALLOCATION_SHA256, 'item_rekeys': rekeys},
        'counts': {'creatures': len(admitted), 'records': len(stage.records), 'profiles': len(stage.profiles),
                   'deferred_encounter': len(deferred['encounter']),
                   'deferred_unregistered_items': len(deferred['unregistered_items']),
                   'deferred_reference_loot_contract': len(deferred['reference_loot_contract']),
                   'deferred_unresolved_reference': len(deferred['unresolved_reference'])},
        'records': [value for _, value in sorted(stage.records.items(), key=order)],
        'authoring_profiles': [value for _, value in sorted(stage.profiles.items(), key=order)],
        'source_identity_bindings': sorted(stage.bindings, key=lambda b: b['external_id']),
        'deferred': deferred,
    }
    args.out.write_bytes(canonical(staged))
    print(json.dumps(staged['counts']))


if __name__ == '__main__':
    main()
