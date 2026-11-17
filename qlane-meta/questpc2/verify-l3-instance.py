"""Serial, filtered L3 encounter-progression qualification; no whole PG target."""
import json
import os
from pathlib import Path
import re
import subprocess
import time

CARGO = '/home/mole/oteryn-cache/codex-bin/cargo'
DISPATCH = 'oteryn-gameplay-dispatch'
DURABILITY = 'oteryn-gameplay-durability'
base = [CARGO, 'test', '--locked', '--offline']
checks = [
    ('scope-units', base + ['-p', DURABILITY, '--lib', 'scoped_death_tests', '--', '--nocapture'], False),
    ('queue-units', base + ['-p', DISPATCH, '--lib', 'kill_reward_tests', '--', '--nocapture'], False),
    ('instance-pg', base + ['-p', DISPATCH, '--lib', 'l3_instance_endgame', '--', '--nocapture', '--test-threads=1'], True),
    ('soul-channel-pg', base + ['-p', DISPATCH, '--lib', 'soul_war_postgres_native_taints_xp_reset_and_replay', '--', '--nocapture', '--test-threads=1'], True),
    ('rotten-channel-pg', base + ['-p', DISPATCH, '--lib', 'rotten_blood_postgres_native_chain_final_reward_replay_and_cleanse', '--', '--nocapture', '--test-threads=1'], True),
    ('primal-channel-pg', base + ['-p', DISPATCH, '--lib', 'primal_hazard_postgres_kill_unlock_selection_replay_cap_and_fences', '--', '--nocapture', '--test-threads=1'], True),
    ('check', [CARGO, 'check', '--locked', '--offline', '-p', DURABILITY, '-p', DISPATCH, '--quiet'], False),
    ('clippy', [CARGO, 'clippy', '--locked', '--offline', '-p', DURABILITY, '-p', DISPATCH, '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings'], False),
    ('fmt', [CARGO, 'fmt', '--all', '--', '--check'], False),
    ('whitespace', ['git', 'diff', '8e24f1d88', '--check', '--', 'apps', 'crates'], False),
]
results = []
os.environ['CARGO_BUILD_JOBS'] = '4'
for name, command, postgres in checks:
    log = Path(f'.questpc2/l3-instance-final-{name}.log')
    started = time.monotonic()
    with log.open('w') as output:
        result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, check=False)
    elapsed = round(time.monotonic() - started, 3)
    output = log.read_text()
    executed = not postgres or ('POSTGRES_CASE_COMPLETED' in output and 'POSTGRES_CASE_SKIPPED' not in output and 'skipped_no_database' not in output)
    counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', output)
    passed = result.returncode == 0 and executed
    if name == 'instance-pg':
        passed = passed and output.count('L3_INSTANCE_ENDGAME POSTGRES_CASE_COMPLETED') == 5
    results.append(dict(name=name, command=command, exit_code=result.returncode, duration_seconds=elapsed,
                        postgres_executed=executed if postgres else None, counts=counts,
                        passed=passed, log=str(log)))
    Path('.questpc2/l3-instance-verification.json').write_text(json.dumps(results, indent=2) + '\n')
    print(f'{name}: {"PASS" if passed else "FAIL"} ({elapsed}s)', flush=True)
    if not passed:
        raise SystemExit(result.returncode or 1)
