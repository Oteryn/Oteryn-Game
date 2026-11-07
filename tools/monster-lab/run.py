#!/usr/bin/env python3
"""One entry point for the offline native monster laboratory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from arena_map import prepare_arena
from lab import LabError, inventory, read_json


def prepare_input(config, report, selected, ticks, seed, training_keys=None):
    if not 1 <= ticks <= 600:
        raise LabError('ticks must be between 1 and 600')
    if not 0 <= seed <= 255:
        raise LabError('seed must be between 0 and 255')
    admitted = {row['monster'] for row in report['monsters']
                if row['native_import'] == 'ADMITTED'}
    training_keys = training_keys or {}
    admitted.update(training_keys)
    if set(selected) - admitted:
        raise LabError('Selected monsters must have an admitted native profile: '
                       + ', '.join(sorted(set(selected) - admitted)))
    stage = Path(config['read_only_inputs']['stage']).resolve()
    return {'format_version': 1, 'stage_path': str(stage),
            'stage_sha256': hashlib.sha256(stage.read_bytes()).hexdigest(),
            'map': prepare_arena(config)['map'],
            'selected_creature_keys': sorted(training_keys.get(name, f'oteryn:creature.{name}') for name in set(selected)),
            'seed': seed, 'ticks': ticks}


def output_directory(config, requested):
    output = Path(requested).resolve()
    inputs = [Path(p).resolve() for p in config['read_only_inputs'].values()]
    repository = Path(config['repository']).resolve()
    if any(output == p or p in output.parents for p in [repository, *inputs]):
        raise LabError('Run output must be outside repository and source inputs')
    output.mkdir(parents=True, exist_ok=True)
    return output


def native_environment(config, manifest, result):
    env = dict(os.environ)
    env['OTERYN_MONSTER_LAB_INPUT'] = str(manifest)
    env['OTERYN_MONSTER_LAB_OUTPUT'] = str(result)
    env['CARGO_INCREMENTAL'] = '0'
    if config.get('shared_rust_target'):
        env['CARGO_TARGET_DIR'] = config['shared_rust_target']
    if config.get('cargo_home'):
        env['CARGO_HOME'] = config['cargo_home']
    if config.get('rustup_home'):
        env['RUSTUP_HOME'] = config['rustup_home']
    if config.get('rust_toolchain_bin'):
        env['PATH'] = config['rust_toolchain_bin'] + os.pathsep + env.get('PATH', '')
    return env


def check_result(summary, manifest, expected_keys, expected_health=None):
    outcomes = summary.get('outcomes')
    if not isinstance(outcomes, list) or any(not isinstance(row, dict) for row in outcomes):
        raise LabError('Native result has no outcome list')
    keys = [row.get('key') for row in outcomes]
    if len(keys) != len(set(keys)) or set(keys) != set(expected_keys):
        raise LabError('Native result did not cover exactly the requested Creature profiles')
    passed = sum(row.get('status') == 'PASS_NATIVE_SCHEDULE_PREPARATION' for row in outcomes)
    if any(row.get('status') not in ('PASS_NATIVE_SCHEDULE_PREPARATION', 'COMPONENT_BLOCKED') for row in outcomes):
        raise LabError('Unknown native outcome status')
    if any(type(summary.get(k)) is not int for k in ('creatures_tested', 'passed_profiles', 'blocked')):
        raise LabError('Native result counts must be integers')
    if (summary['creatures_tested'] != len(keys) or summary['passed_profiles'] != passed
            or summary['blocked'] != len(keys) - passed
            or type(summary.get('passed')) is not bool or summary['passed'] != (passed == len(keys))):
        raise LabError('Native result counts or success claim disagree with outcomes')
    if (summary.get('scope') != 'headless_native_component_arena'
            or summary.get('stage_sha256') != manifest['stage_sha256']
            or summary.get('map_source_sha256') != manifest['map']['source_sha256']
            or summary.get('seed') != manifest['seed']
            or summary.get('live_server_started') is not False):
        raise LabError('Native result is not bound to this input and execution scope')
    if summary.get('native_owner_lifecycle', {}).get('status') != 'PASS':
        raise LabError('Native owner lifecycle fixture did not pass')
    for row in outcomes:
        if row['status'] == 'PASS_NATIVE_SCHEDULE_PREPARATION' and (
                row.get('fresh_replay_identical') is not True
                or row.get('schedule', {}).get('retry_idempotent') is not True
                or row.get('schedule', {}).get('ticks') != manifest.get('ticks')):
            raise LabError('Native outcome lacks the requested ticks and deterministic replay checks')
        if expected_health is not None and row['status'] == 'PASS_NATIVE_SCHEDULE_PREPARATION':
            health = expected_health[row['key']]
            lifecycle = row.get('native_owner_lifecycle', {})
            if (row.get('authored_hp_tested') != health
                    or row.get('authored_source_key_admitted') is not True
                    or lifecycle.get('initial_hp') != health
                    or lifecycle.get('status') != 'PASS'
                    or lifecycle.get('damage_commit') is not True
                    or lifecycle.get('lethal_death_projection') is not True
                    or lifecycle.get('retry_idempotent') is not True):
                raise LabError('Native outcome does not prove owner lifecycle with the authored HP')
    return summary['passed']


def merge_training(baseline, extra, report):
    """Add private profiles for testing; the canonical baseline is not rewritten."""
    if extra.get('production_admission') is not False or extra.get('runtime_activated') is not False:
        raise LabError('Training dataset must explicitly exclude production activation')
    merged = dict(baseline)
    for section, identity_field in (('records', 'identity'), ('authoring_profiles', 'target')):
        original = baseline[section]
        existing = {(v[identity_field]['family'], v[identity_field]['key'], v[identity_field]['revision']): v for v in original}
        additions = []
        for value in extra[section]:
            identity = value[identity_field]
            key = (identity['family'], identity['key'], identity['revision'])
            if key in existing:
                if existing[key] != value:
                    raise LabError('Training dataset conflicts with a baseline definition')
                continue
            if identity['family'] != 'Item' and '.lab.' not in identity['key']:
                raise LabError('Training definitions must have private laboratory identities')
            existing[key] = value
            additions.append(value)
        merged[section] = original + additions
    targets = [p['target']['key'] for p in extra['authoring_profiles'] if p['data']['kind'] == 'Creature']
    mapping = {}
    for name in extra['admitted_training_monsters']:
        matching = [key for key in targets if key.startswith(f'oteryn:creature.lab.{name}.')]
        if len(matching) != 1:
            raise LabError('Training roster has an ambiguous or missing private Creature')
        mapping[name] = matching[0]
    if len(targets) != len(mapping):
        raise LabError('Training Creature count differs from roster')
    allowed = {r['monster'] for r in report['monsters'] if r['native_import'] == 'ADMITTED'} | mapping.keys()
    merged['lab_scope'] = {'production_admission': False, 'runtime_activated': False,
                           'unique_test_monsters': len(allowed), 'training_keys': mapping,
                           'baseline_counts': baseline.get('counts', {})}
    if 'counts' in baseline:
        merged['counts'] = dict(baseline['counts'],
                                creatures=sum(p['data']['kind'] == 'Creature' for p in merged['authoring_profiles']),
                                profiles=len(merged['authoring_profiles']), records=len(merged['records']))
    return merged, mapping


def catalog_status(report, native_result, training_report, training_keys):
    outcomes = {row['key']: row for row in native_result['outcomes']}
    approximations = {row['monster']: row for row in (training_report or {}).get('monsters', [])}
    rows = []
    for original in report['monsters']:
        name = original['monster']
        key = training_keys.get(name, f'oteryn:creature.{name}')
        result = outcomes.get(key)
        row = dict(original, tested_profile_key=key,
                   native_component_status=result['status'] if result else 'NOT_SELECTED',
                   live_server_enabled=False)
        if name in approximations:
            row['training_approximation'] = approximations[name]
            row['quality_flags'] = sorted(set(row['quality_flags'] + approximations[name]['flags']))
        rows.append(row)
    return {'scope': 'Native component test and approximation flags; not live gameplay',
            'prepared': len(rows), 'tested': native_result['creatures_tested'],
            'passed': native_result['passed_profiles'], 'blocked': native_result['blocked'],
            'training_profiles': len(training_keys), 'monsters': rows}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('inventory', 'arena'))
    parser.add_argument('--config', required=True)
    parser.add_argument('--output-dir', required=True)
    parser.add_argument('--monster', action='append', default=[], help='Slug; repeat to select several. Default: all native actors.')
    parser.add_argument('--ticks', type=int, default=30)
    parser.add_argument('--seed', type=int, default=9)
    parser.add_argument('--training', action='store_true', help='Include isolated simplified Encounter profiles and unsupported defensive-melee replacements')
    args = parser.parse_args(argv)
    try:
        config = read_json(args.config)
        report = inventory(config)
        output = output_directory(config, args.output_dir)
        (output / 'inventory.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
        if args.command == 'inventory':
            print(json.dumps(report['counts'], sort_keys=True))
            return 0
        manifest = output / 'arena-input.json'
        training_keys = {}
        training_report = None
        input_config = config
        selected = args.monster
        if args.training:
            from training_profiles import generate
            inputs = config['read_only_inputs']
            training_dir = output / 'training-bundles'
            training_report = generate(Path(inputs['index']), Path(inputs['stage']), Path(inputs['bundles']),
                                       Path(inputs['item_map']), training_dir)
            extra = read_json(training_dir / 'training-native-profiles.json')
            baseline_path = Path(inputs['stage'])
            if extra['requires_baseline_stage_sha256'] != hashlib.sha256(baseline_path.read_bytes()).hexdigest():
                raise LabError('Training dataset is bound to another baseline stage')
            merged, training_keys = merge_training(read_json(baseline_path), extra, report)
            merged_path = output / 'isolated-lab-stage.json'
            merged_path.write_text(json.dumps(merged, indent=2) + '\n')
            input_config = dict(config, read_only_inputs=dict(inputs, stage=str(merged_path)))
            if not selected:
                selected = sorted({r['monster'] for r in report['monsters'] if r['native_import'] == 'ADMITTED'} | training_keys.keys())
            (output / 'training-summary.json').write_text(json.dumps(training_report['counts'], indent=2) + '\n')
        prepared = prepare_input(input_config, report, selected, args.ticks, args.seed, training_keys)
        manifest.write_text(json.dumps(prepared, indent=2) + '\n')
        result = output / 'native-arena-result.json'
        # A failed run cannot be confused with an earlier successful result.
        result.unlink(missing_ok=True)
        env = native_environment(config, manifest, result)
        cargo = shutil.which('cargo', path=env.get('PATH'))
        if cargo is None:
            raise LabError('Cargo unavailable; inventory and arena input are prepared, native test did not execute')
        command = [cargo, 'test', '--locked', '-p', 'oteryn-game-server', '--lib',
                   'monster_lab::', '--quiet', '--', '--nocapture']
        with (output / 'native-arena.log').open('w') as log:
            completed = subprocess.run(command, cwd=config['repository'], env=env,
                                       stdout=log, stderr=subprocess.STDOUT, check=False)
        if completed.returncode != 0 or not result.is_file():
            raise LabError('Native arena did not complete; inspect ' + str(output / 'native-arena.log'))
        summary = read_json(result)
        expected = prepared['selected_creature_keys'] or [
            f"oteryn:creature.{row['monster']}" for row in report['monsters'] if row['native_import'] == 'ADMITTED']
        stage_profiles = read_json(prepared['stage_path'])['authoring_profiles']
        health = {p['target']['key']: p['data']['profile']['health']
                  for p in stage_profiles if p['data']['kind'] == 'Creature'}
        passed = check_result(summary, prepared, expected, health)
        (output / 'catalog-status.json').write_text(json.dumps(
            catalog_status(report, summary, training_report, training_keys), indent=2, ensure_ascii=False) + '\n')
        print(json.dumps({'result': str(result), 'scope': summary.get('scope'),
                          'native_component_execution': True, 'live_server_started': False,
                          'passed': passed, 'tested': summary['creatures_tested'],
                          'blocked': summary['blocked']}, sort_keys=True))
        return 0 if passed else 1
    except (LabError, OSError, ValueError) as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
