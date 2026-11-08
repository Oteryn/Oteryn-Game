"""Run serial current-tree checks after the filtered reward PG command ends."""
import json
import os
from pathlib import Path
import subprocess
import time

previous = json.loads(Path('.questpc/restart5-reward-pg.result.json').read_text())
if previous['exit'] != 0:
    raise SystemExit('Reward PG command did not pass; inspect before final validation')
cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
commands = [
    ('status', ['python3', '.questpc/restart5-quest-status.py']),
    ('fmt', [cargo, 'fmt', '--all', '--', '--check']),
    ('mounted-fmt', ['rustup', 'run', '1.99.1', 'rustfmt', '--edition', '2024',
                     '--check', '--config', 'skip_children=true',
                     'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs']),
    ('clippy', [cargo, 'clippy', '--locked', '--offline', '-p',
                'oteryn-gameplay-dispatch', '-p', 'oteryn-gameplay-quests',
                '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings']),
    ('storage-units', [cargo, 'test', '--locked', '--offline', '-p',
                       'oteryn-gameplay-quests', '--lib', 'storage_owner_', '--', '--nocapture']),
]
checks = {}
environment = dict(os.environ, CARGO_BUILD_JOBS='4')
for label, command in commands:
    log = Path(f'.questpc/restart5-{label}.log')
    start = time.monotonic()
    with log.open('w') as output:
        completed = subprocess.run(command, env=environment, stdout=output,
                                   stderr=subprocess.STDOUT, timeout=1800)
    result = {'command': command, 'exit': completed.returncode,
              'duration_seconds': round(time.monotonic() - start, 3), 'log': str(log)}
    checks[label] = result
    Path(f'.questpc/restart5-{label}.result.json').write_text(json.dumps(result, indent=2) + '\n')
    Path('.questpc/restart5-final-checks.result.json').write_text(json.dumps(checks, indent=2) + '\n')
    print(label, completed.returncode, result['duration_seconds'], flush=True)
    if completed.returncode:
        raise SystemExit(completed.returncode)
