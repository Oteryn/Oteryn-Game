"""Generate qualification plans, never gameplay receipts or activation authority."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path


def read_json(path):
    with path.open('rb') as handle:
        raw = handle.read(8 * 1024 * 1024 + 1)
    if len(raw) > 8 * 1024 * 1024:
        raise ValueError('input exceeds native bound')
    return json.loads(raw), hashlib.sha256(raw).hexdigest()


def build(catalog, selection, digest, expected_count=246):
    if catalog.get('schema') != 'OTERYN_EXECUTABLE_SPELL_CATALOG/v1':
        raise ValueError('wrong executable catalog schema')
    if selection.get('schema') != 'OTERYN_SPELL_SOURCE_SELECTION/v1' or selection.get('catalog_sha256') != digest:
        raise ValueError('source selection catalog digest mismatch')
    bundles = catalog['bundles']
    if len(bundles) != expected_count:
        raise ValueError('catalog coverage changed: review expected count')
    by_key = {}
    groups = defaultdict(list)
    for bundle in bundles:
        spell = bundle['bundle']['spell']
        key = spell['identity']['key']
        if key in by_key:
            raise ValueError('duplicate spell identity')
        by_key[key] = bundle
        if spell['carrier'] == 'instant':
            groups[spell['words'].lower()].append(key)
    selections = {}
    for row in selection['selections']:
        selected = row['selected']['key']
        if selected not in by_key:
            raise ValueError('selected identity absent')
        words = by_key[selected]['bundle']['spell']['words'].lower()
        if words in selections or selected not in groups[words] or len(groups[words]) < 2:
            raise ValueError('invalid alias selection')
        alternatives = {x['key'] for x in row['alternatives']}
        if alternatives != set(groups[words]) - {selected}:
            raise ValueError('alias alternative coverage mismatch')
        selections[words] = selected
    inactive = set()
    for words, keys in groups.items():
        if len(keys) < 2:
            continue
        if words in selections:
            selected = selections[words]
        else:
            # Rust selects the first canonical identity only for equivalent aliases.
            signatures = []
            for key in keys:
                spell = dict(by_key[key]['bundle']['spell'])
                for field in ('identity', 'name', 'library_text', 'reference_spell_id'):
                    spell.pop(field, None)
                signatures.append(spell)
            if any(s != signatures[0] for s in signatures[1:]):
                raise ValueError('ambiguous incantation without source selection')
            selected = sorted(keys, key=lambda x: x.encode())[0]
        inactive.update(set(keys) - {selected})
    entries = []
    for index, key in enumerate(sorted(by_key, key=lambda x: x.encode()), 1):
        bundle = by_key[key]
        s = bundle['bundle']['spell']
        requirements, costs, target = s['requirements'], s['costs'], s['targeting']
        execution = s['execution']
        if len(execution) != 1:
            raise ValueError('unrecognized execution closure')
        family = next(iter(execution))
        native = execution.get('native_behavior', {}).get('key')
        owners = []
        fixtures = ['admitted_current_actor_and_content', 'eligible_vocation_level_and_learning',
                    'sufficient_mana_soul', 'expired_spell_and_group_cooldowns']
        if requirements.get('premium'):
            owners.append('platform_premium_entitlement_producer_not_composed_in_qualified_node')
        if requirements.get('wheel_unlock'):
            fixtures.append('eligible_current_wheel_owner_unlock')
        if target['parameter'] != 'none':
            owners.append('parameter_cast_negotiated_connection_and_client_route')
            fixtures.append('source_valid_parameter_' + target['parameter'])
        if native == 'house_access':
            owners.append('physical_world_house_interior_owner_not_composed')
        if native in ('equipment_attack', 'delayed_strike', 'owned_field_buff', 'creature_appearance'):
            owners.append('physical_cast_dispatcher_missing_for_' + native)
        if native == 'locate_message' and execution['native_behavior']['parameters'].get('source') != 'online_player':
            owners.append('physical_cast_dispatcher_missing_for_nearest_fiendish')
        if native == 'party_buff':
            fixtures.append('current_party_membership_and_party_owner_snapshot')
        if native in ('familiar_summon', 'acquire_summon', 'companion_haste'):
            fixtures.append('current_creature_population_reservation_and_ownership')
        if native in ('equipment_attack',) or s.get('needs_weapon'):
            fixtures.append('source_eligible_current_equipment_and_skills')
        if family == 'conjure' or native in ('tile_item_operation', 'random_item_grant', 'owned_field_buff'):
            fixtures.append('actual_item_owner_inventory_capacity_and_required_reagents')
        if s['carrier'] == 'rune':
            fixtures.append('current_rune_item_and_charge_owner')
        if target.get('aggressive'):
            fixtures.append('non_protection_zone_and_valid_combat_target_facts')
        if target.get('needs_target'):
            fixtures.append('real_current_target_in_source_range_floor_and_line_of_sight')
        if target.get('needs_direction') or target.get('target_or_direction'):
            fixtures.append('current_facing_and_occupied_effect_footprint')
        refusals = [{'condition': 'immediate_repeat_after_success', 'expected': 'CoolingDown'}]
        for field, condition, disposition in (
            ('mana', 'mana_below_required_cost', 'NotEnoughMana'),
            ('soul', 'soul_below_required_cost', 'NotEnoughSoul'),
        ):
            if isinstance(costs.get(field), int) and costs[field] > 0:
                refusals.append({'condition': condition, 'expected': disposition})
        if requirements.get('level', 0) > 1:
            refusals.append({'condition': 'level_below_required', 'expected': 'LevelTooLow'})
        if len(requirements['vocations']) < 10:
            refusals.append({'condition': 'vocation_outside_allowed_set', 'expected': 'NotAvailable'})
        if s.get('rune', {}).get('magic_level', 0) > 0:
            refusals.append({'condition': 'magic_level_below_rune_requirement', 'expected': 'MagicLevelTooLow'})
        if requirements.get('wheel_unlock'):
            refusals.append({'condition': 'current_wheel_unlock_absent', 'expected': 'Rejected'})
        if requirements.get('premium'):
            refusals.append({'condition': 'current_free_entitlement', 'expected': 'NotAvailable'})
        if requirements.get('learning_required'):
            refusals.append({'condition': 'spell_not_learned', 'expected': 'NotAvailable'})
        if target.get('needs_target'):
            refusals.append({'condition': 'required_target_absent', 'expected': 'TargetRequired'})
        if key in inactive:
            refusals = [{'condition': 'unselected_source_alias_index', 'expected': 'NotAvailable'}]
        status = ('inactive_source_alias' if key in inactive else
                  'requires_runtime_owner_integration' if owners else 'executable_path_requires_fixture')
        entries.append({
            'key': key, 'revision': s['identity']['revision'], 'name': s['name'],
            'canonical_book_index': index, 'selected': key not in inactive,
            'carrier': s['carrier'], 'rune': s.get('rune'), 'requirements': requirements, 'costs': costs,
            'cooldown_ms': s['cooldown_ms'], 'groups': s['groups'], 'targeting': target,
            'execution_family': family, 'native_behavior': native,
            'dependencies': bundle['catalog']['definitions'],
            'source_identities': bundle['source_identities'],
            'planning_status': status, 'runtime_owner_requirements': owners,
            'transport_modes_to_test': (['none'] if target.get('self_target') else
                                        ['attack', 'position'] if target.get('needs_target') else
                                        ['none', 'attack', 'position']),
            'parameter_transport': target['parameter'],
            'positive': {'expected': 'Cast', 'fixtures': fixtures, 'execute': key not in inactive},
            'refusals': refusals,
            'assertions': ['refusal_leaves_cost_cooldown_and_world_unchanged',
                           'success_commits_cost_and_cooldown_once',
                           'observe_real_target_or_world_effect_not_only_disposition'],
            'execution_evidence': None,
        })
    return {'schema': 'OTERYN_SPELL_QUALIFICATION_SCENARIOS/v1',
            'catalog_sha256': digest, 'catalog_revision': catalog['revision'],
            'summary': {'definitions': len(entries), 'selected': sum(e['selected'] for e in entries),
                        'planning_status': dict(Counter(e['planning_status'] for e in entries)),
                        'executed_by_this_generator': 0},
            'refusal_setup': 'isolate one failing precondition; keep all earlier gates satisfied',
            'book_index_scope': 'only this exact catalog generation; verify loaded book before live use',
            'activation_authority': False, 'scenarios': entries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('catalog', type=Path)
    parser.add_argument('selection', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    catalog, digest = read_json(args.catalog)
    selection, selection_digest = read_json(args.selection)
    result = build(catalog, selection, digest)
    result['source_selection_sha256'] = selection_digest
    with args.output.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(result['summary']))


if __name__ == '__main__':
    main()
