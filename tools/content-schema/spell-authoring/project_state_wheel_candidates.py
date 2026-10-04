"""R55 real base-Spell projections using accepted S21/S5/S6/S23 rules.

Every current source header stays byte-exact. Optional Wheel/Crystal stance
branches remain source evidence and explicit dependency/normalization limits.
True source Wheel unlock gates and unsupported native timelines are never erased.
"""
import argparse
import copy
import gzip
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace

import convert_spells
import import_source_player_bundles as base
import project_player_control_candidates as controls
import validate_spell

HERE = Path(__file__).resolve().parent
BASE = Path('docs/reference/spells/r28-source-closure/player-source-bundles')
WORKLIST = Path('docs/reference/spells/current-source-gap-worklist.json')
OUTPUT = Path('docs/reference/spells/r55-source-closure')
REVISION = 'source-player-r55'
CACHE = Path('/workspace/spell-source-closure/generated-monsters-r28-complete/all-registered-spells.json')
SNAPSHOTS = Path('/workspace/spells-r22-monster-import-current/source-inputs')
WHEEL_BASE = {'energy_beam.lua', 'great_energy_beam.lua', 'front_sweep.lua', 'strong_ice_wave.lua'}


def sha(raw): return hashlib.sha256(raw).hexdigest()
def canonical(value): return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
def row_id(key): return sha(key.encode())[:16]
def base_row(repo, key): return Path(repo) / BASE / key.split('/')[0] / row_id(key)


def rebind(value, mapping):
    if isinstance(value, list): return [rebind(child, mapping) for child in value]
    if not isinstance(value, dict): return copy.deepcopy(value)
    result = {key: rebind(child, mapping) for key, child in value.items()}
    if 'key' in result and 'revision' in result and result['key'] in mapping:
        result['key'] = mapping[result['key']]; result['revision'] = REVISION
    return result


def reader_projection(repo, spell):
    spell = copy.deepcopy(spell)
    spell['requirements'].pop('vocation_display_flags', None)
    spell['targeting'].setdefault('allow_on_self', True)
    spell['targeting'].setdefault('check_floor', True)
    # All selected projections are explicit vocational player attack spells.
    if 'none' in spell['requirements']['vocations']: raise ValueError('unqualified None vocation domain')
    return spell


def normalized_formula(callback):
    formula = callback['formula']
    if formula.get('status') != 'resolved': raise ValueError('unresolved base formula: ' + formula.get('error', 'unknown'))
    notes = set(); bounds = [convert_spells.convert_expr(formula[key], notes) for key in ['minimum', 'maximum']]
    mags = [{'op': 'abs', 'args': [bound]} for bound in bounds]
    return {'kind': 'player_expression', 'inputs': 'skill' if callback['kind'] == 'CALLBACK_PARAM_SKILLVALUE' else 'level_magic',
            'minimum': {'op': 'min', 'args': copy.deepcopy(mags)}, 'maximum': {'op': 'max', 'args': copy.deepcopy(mags)}}


def counterpart_projection(repo, key, facts, index):
    counterpart = key.replace('crystal-summer-current', 'canary-main-current')
    if counterpart not in index or index[counterpart]['status'] != 'CANDIDATE_SCHEMA_VALID':
        raise ValueError('S21 counterpart has no complete standard candidate')
    row = base_row(repo, counterpart)
    spell = json.loads((row / 'spell.json').read_text())['spell']; deps = json.loads((row / 'dependencies.json').read_text())
    if 'ability' not in spell['execution'] or not deps['abilities'] or not deps['effects']:
        raise ValueError('S21 counterpart not a standard Ability')
    mapping = {spell['identity']['key']: 'candidate:spell/source/' + key.split('/')[0] + '/' + row_id(key)}
    for family, values in deps.items():
        for position, value in enumerate(values): mapping[value['identity']['key']] = mapping[spell['identity']['key']] + '/' + family + '/' + str(position)
    spell, deps = rebind(spell, mapping), rebind(deps, mapping)
    # The accepted S5 normalizer, not a new formula engine.
    for formula in deps['formulas']:
        if formula.get('kind') == 'player_expression':
            for bound in ['minimum', 'maximum']: formula[bound] = convert_spells.convert_expr(formula[bound], set())
    return reader_projection(repo, spell), deps, counterpart


def cached_base_projection(repo, key, facts, caches, converters):
    fact = facts[key]; raw = fact['source_callback_facts']; donor = key.split('-')[0]
    cached = caches[(donor, raw['file'])]
    if cached['provenance']['sha256'] != fact['source_sha256'] or cached['provenance']['revision'] != fact['source_revision']:
        raise ValueError('cached Combat source identity differs')
    info = cached['conversion']
    if not info.get('reference_capture_complete'): raise ValueError('cached base declarations incomplete')
    combat = copy.deepcopy(info['reference_combats']['0'])
    callback = next((value for value in raw['combats'][0].get('callbacks', []) if 'formula' in value), None)
    if callback is None: raise ValueError('base needs one exact value formula callback')
    if set(combat['callbacks']) - {'CALLBACK_PARAM_LEVELMAGICVALUE', 'CALLBACK_PARAM_SKILLVALUE'}:
        raise ValueError('base includes unrepresented target/chain callback')
    header = json.loads((base_row(repo, key) / 'source-header.json').read_text())
    defaults, proofs = base.default_fields(Path('/workspace/spell-sources') / donor, fact['source_revision'])
    prefix = 'candidate:spell/source/' + key.split('/')[0] + '/' + row_id(key)
    spell = base.fill_header(header['spell'], defaults, {'key': prefix, 'revision': REVISION}, raw['registrar'])
    converter = converters[donor]; converter.pending_definitions = set()
    deps = {'abilities': [], 'effects': [], 'formulas': []}; notes = []
    converter.combat_ability(prefix + '/ability', combat,
                            {'needs_target': spell['targeting']['needs_target'] or spell['targeting']['target_or_direction'],
                             'needs_direction': spell['targeting']['needs_direction']},
                            spell['targeting'].get('range_tiles', 0), deps, lambda binding: binding, notes)
    player_formula = prefix + '/formula'
    normalized = normalized_formula(callback)
    replaced = 0
    for effect in deps['effects']:
        if effect.get('formula', {}).get('key') == base.canary_batch.CASTER_MAGNITUDE:
            effect['formula'] = {'family': 'Formula', 'key': player_formula, 'revision': REVISION}; replaced += 1
    if replaced != 1: raise ValueError('base formula does not bind exactly one health Effect')
    deps['formulas'] = [formula for formula in deps['formulas'] if formula['identity']['key'] != base.canary_batch.CASTER_MAGNITUDE]
    deps['formulas'].append({'identity': {'key': player_formula, 'revision': REVISION}, **normalized})
    # The monster converter uses its own revision; rebind all local definitions exactly.
    mapping = {value['identity']['key']: value['identity']['key'] for values in deps.values() for value in values}
    deps = rebind(deps, mapping)
    if donor == 'crystal':
        def visual_prefix(value):
            if isinstance(value, str): return value.replace('canary.appearance:', 'crystal.appearance:')
            if isinstance(value, list): return [visual_prefix(child) for child in value]
            if isinstance(value, dict): return {name: visual_prefix(child) for name, child in value.items()}
            return value
        deps = visual_prefix(deps)
    spell['execution'] = {'ability': {'family': 'Ability', 'key': prefix + '/ability', 'revision': REVISION}}
    return reader_projection(repo, spell), deps, key


def block_reason(row):
    name = row['registration_key'].rsplit('/', 1)[1].split('#')[0]
    if name in {'great_death_beam.lua', 'executioners_throw.lua', 'ice_burst.lua', 'terra_burst.lua'}:
        return 'True Wheel unlock remains representable but no complete source-correct base grade geometry/formula/state dependency projection is built here; grade branch remains explicit source evidence.'
    if name.startswith('avatar_') or name in {'divine_empowerment.lua', 'divine_grenade.lua', 'mass_spirit_mend.lua'}:
        return 'Source-correct full native parameters and dependency closure not mapped here; canonical template copying cannot establish source completeness. Strict67 identity is a separate runtime capability limit.'
    if name in {'mass_healing.lua'}:
        return 'Targetcreature callback filters party/player/summon and rolls per target; plain area heal cannot preserve exact callback owner semantics.'
    if name == 'magic_shield.lua':
        return 'Dynamic max-mana capacity and condition owner required; no complete standard Effect models source capacity-before-condition timeline.'
    if name == 'sap_strength.lua':
        return 'Accepted S24 removes canonical Sap Strength; current source remains archived, no active executable placeholder emitted.'
    if row['baseline_lane'] == 'stance' and '/support/' in row['registration_key']:
        return 'Persistent custom Crystal stance/aura/secondary party effects lack a complete source-correct accepted parameter/branch closure here; native strict67 is a separate runtime capability limit.'
    if name in {'energy_wave.lua', 'sharpshooter.lua'}:
        return 'Current stance/familiar/Wheel conditional behavior needs existing native owner; no safe complete standard base candidate currently qualified.'
    return 'No exact supported standard base projection qualified; source behavior retained.'


def build(repo):
    repo = Path(repo)
    worklist = json.loads((repo / WORKLIST).read_text())
    cohort = [row for row in worklist['records'] if row['baseline_lane'] in {'wheel', 'stance', 'chain'}]
    if len(cohort) != 64 or len({row['registration_key'] for row in cohort}) != 64: raise ValueError('requires exact64 lane cohort')
    facts = {row['registration_key']: row for row in map(json.loads, gzip.decompress((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    index = {row['registration_key']: row for row in json.loads((repo / BASE / 'import-summary.json').read_text())['records_index']}
    caches = {(row['source'], row['provenance']['path']): row for row in json.loads(CACHE.read_text())}
    converters = {donor: base.canary_batch.Converter(SNAPSHOTS / donor, {}, {}, {}, {}) for donor in ['canary', 'crystal']}
    for donor, converter in converters.items(): converter.spell_scripts = SimpleNamespace(enums=base.spell_scripts.engine_enums(SNAPSHOTS / donor))
    candidates, audit = [], []
    for row in cohort:
        key = row['registration_key']; name = key.rsplit('/', 1)[1].split('#')[0]
        reason = block_reason(row)
        try:
            if row['baseline_lane'] == 'stance' and '/attack/' in key:
                spell, deps, selected = counterpart_projection(repo, key, facts, index); policy = 'S21 canonical Canary source + S5; raw Crystal custom stance preserved separately'
            elif row['baseline_lane'] == 'wheel' and name in WHEEL_BASE:
                # Prefer accepted Canary source for Crystal, including effect/area/formula declarations.
                selected_key = key.replace('crystal-summer-current', 'canary-main-current')
                if key.startswith('crystal') and index.get(selected_key, {}).get('status') == 'CANDIDATE_SCHEMA_VALID':
                    spell, deps, selected = counterpart_projection(repo, key, facts, index)
                else:
                    spell, deps, selected = cached_base_projection(repo, key, facts, caches, converters)
                policy = 'S6 level-unlocked base Spell; optional Wheel/Beam Mastery augment separate; S5 formula'
            else:
                raise ValueError(reason)
            bundle = {'spell': spell}; catalog = {'definitions': []}
            errors = validate_spell.validate(bundle, deps, catalog)
            if errors: raise ValueError('target schema: ' + ' | '.join(errors[:3]))
            controls.validate_reader_shape(repo, bundle, deps)
            original = base_row(repo, key)
            receipt = json.loads((original / 'receipt.json').read_text())
            if receipt['status'] != 'BLOCKED': raise ValueError('requires original blocked source receipt')
            remaining = [{'source_field': 'source.optional_augment_or_Crystal_stance', 'reason': 'Full raw source remains in existing R38. Canonical base uses accepted policy; Wheel augment/state owner execution remains separate/unqualified.'},
                         {'source_field': 'source.policy_normalized_values', 'reason': policy},
                         {'source_field': 'source.runtime_provider_and_assets', 'reason': 'Reader structural shape checked; this exact bundle/native runtime provider/assets not executed or activated.'}]
            receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[], dependencies={family: len(values) for family, values in deps.items()},
                           schema_and_semantic_validation_errors=[], native_execution_qualified=False,
                           item_owner_bindings_required=[], remaining_mechanics=remaining,
                           conversion_notes=[policy, 'Raw source header copied byte-exact; no source deletion, fake native alias or true Wheel gate removal.'])
            validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
            candidate = {'registration_key': key, 'candidate_key': spell['identity']['key'], 'candidate_revision': REVISION,
                         'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': [], 'selected_mechanics_registration': selected,
                         'normalization_policy': policy, 'source_sha256': facts[key]['source_sha256'],
                         'selected_mechanics_source_sha256': facts[selected]['source_sha256'],
                         'source_header_sha256': sha((original / 'source-header.json').read_bytes()),
                         'source_fact_sha256': sha(canonical(facts[key])), 'selected_mechanics_fact_sha256': sha(canonical(facts[selected])),
                         'files': {'spell.json': bundle, 'dependencies.json': deps, 'catalog.json': catalog, 'receipt.json': receipt},
                         'source_header_bytes': (original / 'source-header.json').read_bytes()}
            candidates.append(candidate)
            audit.append({'registration_key': key, 'lane': row['baseline_lane'], 'status': 'CANDIDATE_SCHEMA_VALID',
                          'policy': policy, 'selected_mechanics_registration': selected,
                          'true_source_unlock_gate_removed': False, 'runtime_activation': False})
        except (ValueError, base.canary_batch.SpellUnresolved) as error:
            audit.append({'registration_key': key, 'lane': row['baseline_lane'], 'status': 'BLOCKED',
                          'specific_blocker': str(error), 'runtime_activation': False})
    for item in audit:
        fact = facts[item['registration_key']]
        item.update(source_sha256=fact['source_sha256'], source_revision=fact['source_revision'], native_execution_qualified=False)
    return candidates, audit


def write_json(path, value): path.write_text(json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); args = parser.parse_args()
    repo = Path(args.repo).resolve(); candidates, audit = build(repo); out = repo / OUTPUT; out.mkdir(parents=True, exist_ok=True)
    records = []
    proofs = []
    for row in candidates:
        target = out / row['registration_key'].split('/')[0] / row_id(row['registration_key']); target.mkdir(parents=True, exist_ok=True)
        for name, value in row['files'].items(): write_json(target / name, value)
        (target / 'source-header.json').write_bytes(row['source_header_bytes'])
        records.append({key: row[key] for key in ['registration_key', 'candidate_key', 'candidate_revision', 'status', 'blockers']})
        proofs.append({key: value for key, value in row.items() if key not in {'files', 'source_header_bytes'}})
    flags = {'runtime_activation': False, 'native_execution_qualified': False, 'canonical_selection_changed': False,
             'native_identity_allocation': False, 'input_provider_equivalence': False}
    write_json(out / 'receipt.schema.json', base.receipt_schema())
    summary = {'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'records': len(records), 'revision': REVISION,
               'status_counts': {'CANDIDATE_SCHEMA_VALID': len(records)}, 'records_index': records,
               'audited_records': 64, 'blocked_records': 64 - len(records), 'partial_records': 0,
               'native_binding_records': 0, 'full_source_mechanics_1_to_1_complete': False, **flags}
    write_json(out / 'import-summary.json', summary)
    write_json(out / 'lane-audit.json', {'schema': 'OTERYN_STATE_WHEEL_LANE_AUDIT/v1', 'records': 64, 'rows': audit, **flags})
    policies = ['docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md', 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md',
                'docs/architecture/OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md']
    proof = {'schema': 'OTERYN_STATE_WHEEL_CANONICAL_TARGET_PROJECTION/v1', 'records': len(records), 'audited_records': 64,
             'candidate_records': proofs, 'native_binding_records': 0, 'partial_records': 0,
             'policy_proofs': {path: sha((repo / path).read_bytes()) for path in policies},
             'source_pins': source_cast_programs_pins(), 'input_proofs': {str(BASE / 'source-callback-facts.jsonl.gz'): sha((repo / BASE / 'source-callback-facts.jsonl.gz').read_bytes()),
                                                                       str(BASE / 'import-summary.json'): sha((repo / BASE / 'import-summary.json').read_bytes())},
             'cached_declarations_sha256': sha(CACHE.read_bytes()), 'producer_sha256': sha(Path(__file__).read_bytes()),
             'schema_proofs': {name: sha((HERE / name).read_bytes()) for name in ['spell.schema.json', 'spell-dependencies.schema.json']},
             'native_reader_full_profile_guard_sha256': sha((repo / 'apps/game-server/src/spell/native.rs').read_bytes()),
             'true_source_unlock_gate_removed': False, 'existing_reader_executed_on_bundles': False, **flags}
    write_json(out / 'projection-proof.json', proof)
    write_json(out / 'package-manifest.json', {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
               'files': {path.relative_to(out).as_posix(): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'package-manifest.json'}})
    print(json.dumps({'audited': 64, 'candidates': len(records), 'blocked': 64 - len(records)}))


def source_cast_programs_pins(): return {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b', 'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}


if __name__ == '__main__': main()
