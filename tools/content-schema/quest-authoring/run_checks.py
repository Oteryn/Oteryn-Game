"""Run all offline Quest data checks without regenerating committed files."""
import argparse
import concurrent.futures
import json
import os
import subprocess
import sys
import time
from pathlib import Path

DONOR_SOURCE_CHECK = ['quest_donor_source_authoring.py', '--check']
# CI runs each part as its own job; `all` is the local default and runs them in sequence.
PARTS = ('all', 'checks', 'donor', 'unit-tests')
SERIAL_UNIT_TESTS = ['-m', 'unittest', 'discover', '-s', '.', '-p', 'test_*.py']
# Started first so the longest modules do not finish last; the order only affects wall time.
SLOW_TEST_MODULES = ('test_quest_tree_authoring', 'test_quest_donor_refinements', 'test_quest_donor_attachment',
                     'test_bundle_authoring')
# Prints {"discovered": [...ids], "modules": {module: [...ids]}} for the top-level test files.
LIST_TESTS = """
import json, pathlib, unittest
def ids(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from ids(test)
        else:
            yield test.id()
loader = unittest.TestLoader()
names = sorted(path.stem for path in pathlib.Path('.').glob('test_*.py'))
print(json.dumps({'discovered': sorted(ids(loader.discover('.', 'test_*.py'))),
                  'modules': {name: sorted(ids(loader.loadTestsFromName(name))) for name in names}}))
"""


def unit_test_modules(here, env):
    """Top-level test modules whose tests together are exactly what discover finds, else None."""
    listing = subprocess.run([sys.executable, '-c', LIST_TESTS], cwd=here, env=env,
                             capture_output=True, text=True)
    if listing.returncode:
        return None
    try:
        tests = json.loads(listing.stdout)
    except ValueError:
        return None
    combined = sorted(test for ids in tests['modules'].values() for test in ids)
    if combined != tests['discovered'] or any(test.startswith('unittest.') for test in combined):
        return None
    modules = [name for name, ids in tests['modules'].items() if ids]
    rank = {name: index for index, name in enumerate(SLOW_TEST_MODULES)}
    return sorted(modules, key=lambda name: (rank.get(name, len(rank)), name))


def run_unit_tests(here, env, jobs):
    """Run each test module in its own process, ``jobs`` at a time, and report every failure."""
    modules = unit_test_modules(here, env) if jobs > 1 else None
    if modules is None:
        if jobs > 1:
            print('Quest unit tests: module split does not match discovery, running serially', flush=True)
        print('Quest check: ' + ' '.join(SERIAL_UNIT_TESTS), flush=True)
        return subprocess.run([sys.executable, *SERIAL_UNIT_TESTS], cwd=here, env=env).returncode
    print(f'Quest check: unit tests, {len(modules)} modules on {jobs} workers', flush=True)

    def run(name):
        start = time.monotonic()
        result = subprocess.run([sys.executable, '-m', 'unittest', name], cwd=here, env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        return name, result, time.monotonic() - start

    failed = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        for future in concurrent.futures.as_completed([pool.submit(run, name) for name in modules]):
            name, result, seconds = future.result()
            lines = result.stdout.strip().splitlines()
            ran = [line for line in lines if line.startswith('Ran ')]
            summary = ', '.join(ran[-1:] + lines[-1:]) or 'no output'
            print(f'  {seconds:6.1f}s rc={result.returncode} {name}: {summary}', flush=True)
            if result.returncode:
                failed.append(name)
                print(result.stdout, flush=True)
    if failed:
        print('Quest unit tests failed: ' + ', '.join(sorted(failed)), flush=True)
        return 1
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    # CI passes this only when the protected-base classifier found no donor input in the diff.
    parser.add_argument('--skip-donor-source', action='store_true',
                        help='leave out quest_donor_source_authoring.py --check')
    parser.add_argument('--jobs', type=int, default=os.cpu_count() or 1,
                        help='parallel unit-test processes; 1 runs plain discover')
    parser.add_argument('--part', choices=PARTS, default='all',
                        help='checks: every check but the donor source; donor: only the donor source; '
                             'unit-tests: only the unit tests')
    args = parser.parse_args()
    root = args.root.resolve()
    here = root / 'tools/content-schema/quest-authoring'
    packet = here / 'samples/migration/quest-source-packet.json'
    scripts = ['verify_quest_schema.py', 'validate_quest_content.py',
               'refresh_quest_source_checks.py', 'quest_catalogue_authoring.py',
               'quest_completion_matrix.py', 'collect_claim_candidates.py', 'use_trigger_candidates.py', 'quest_tree_authoring.py', 'bundle_authoring.py', 'source_text_authoring.py',
               'build_wiki_source_schema.py', 'wiki_source_inventory.py', 'wiki_source_supplements.py', 'wiki_all_source_inventory.py', 'quest_rollout_authoring.py', 'wiki_requirement_interpretations.py', 'source_facts_authoring.py', 'authored_quest_authoring.py', 'quest_completion_authoring.py', 'quest_binding_authoring.py', 'quest_donor_source_authoring.py', 'quest_state_lowering.py', 'source_lowered_binding_plan.py']
    for name in ('source_text_capture.json', 'samples/source_texts/source_texts.json'):
        if not (here / name).is_file():
            parser.error('required source prose check input missing: ' + str(here / name))
    missing = [str(here / name) for name in scripts if not (here / name).is_file()]
    bundle = here / 'samples/source_migration/bundle.json'
    if not bundle.is_file():
        missing.append(str(bundle))
    if not packet.is_file():
        missing.append(str(packet))
    if missing:
        parser.error('required Quest check inputs missing: ' + ', '.join(missing))
    commands = [
        ['verify_quest_schema.py'],
        ['../reward-claim-authoring/test_reward_claim_variant_authoring.py'],
        ['../reward-claim-authoring/test_variant_stack_serialization.py'],
        ['../reward-claim-authoring/reward_claim_authoring.py', 'content', '--check'],
        ['../reward-claim-authoring/reward_claim_variant_migration.py', '--check'],
        ['validate_quest_content.py', 'samples/chests/claims.json', 'samples/questlog/quests.json',
         '--catalog', 'samples/chests/catalog.json', '--manifest', 'samples/chests/manifest.json',
         '--gates', 'samples/doors/gates.json', '--gates-manifest', 'samples/doors/manifest.json',
         '--progress', 'samples/questlog/progress.json', '--interactions', 'samples/interactions/interactions.json',
         '--interactions-manifest', 'samples/interactions/manifest.json'],
        ['refresh_quest_source_checks.py', '--check'],
        ['quest_catalogue_authoring.py', '--check'],
        ['quest_completion_matrix.py', '--check'],
        ['collect_claim_candidates.py', '--check'],
        ['use_trigger_candidates.py', '--check'],
        ['source_text_authoring.py', '--check'],
        ['build_wiki_source_schema.py', '--check'],
        ['wiki_source_inventory.py', '--check'],
        ['wiki_source_supplements.py', '--check'],
        ['wiki_all_source_inventory.py', '--check'],
        ['wiki_all_source_inventory.py', '--schema-out', 'wiki_all_source_specs.schema.json', '--check'],
        ['wiki_requirement_interpretations.py', '--check'],
        ['source_facts_authoring.py', '--specifications', 'samples/wiki-source-all373/source-specs-373.json', '--samples', 'samples/unbound-source-facts', '--check'],
        ['authored_quest_authoring.py', '--check'],
        ['quest_completion_authoring.py', '--check'],
        ['quest_rollout_authoring.py', '--check'],
        ['quest_binding_authoring.py', '--check'],
        ['quest_v2_export.py', '--check'],
        ['quest_state_lowering.py', '--check'],
        ['source_lowered_binding_plan.py', '--check'],
        ['quest_completion_import.py', '--check'],
        DONOR_SOURCE_CHECK,
        ['bundle_authoring.py', '--check'],
        ['quest_tree_authoring.py', 'content', '--check', '--source-packet', str(packet)],
    ]
    if args.part == 'checks':
        commands.remove(DONOR_SOURCE_CHECK)
    elif args.part == 'donor':
        commands = [DONOR_SOURCE_CHECK]
    elif args.part == 'unit-tests':
        commands = []
    if args.skip_donor_source and DONOR_SOURCE_CHECK in commands:
        commands.remove(DONOR_SOURCE_CHECK)
        print('Quest check skipped: quest_donor_source_authoring.py --check (no donor input changed)', flush=True)
    env = {**os.environ, 'PYTHONDONTWRITEBYTECODE': '1'}
    for arguments in commands:
        print('Quest check: ' + ' '.join(arguments), flush=True)
        result = subprocess.run([sys.executable, *arguments], cwd=here, env=env)
        if result.returncode:
            return result.returncode
    if args.part not in ('all', 'unit-tests'):
        return 0
    # Slowest step last: the cheap evidence-consistency checks above fail first.
    return run_unit_tests(here, env, max(1, args.jobs))


if __name__ == '__main__':
    sys.exit(main())
