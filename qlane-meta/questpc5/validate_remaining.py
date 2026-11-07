import json
import os
import subprocess
import time
from pathlib import Path

os.environ['CARGO_BUILD_JOBS'] = '3'
cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
checks = [
    ('server-clippy', [cargo, 'clippy', '--locked', '--offline', '-p', 'oteryn-game-server', '--all-targets', '--all-features', '--quiet', '--', '-D', 'warnings']),
    ('content-native-units', [cargo, 'test', '--locked', '--offline', '-p', 'oteryn-gameplay-content', '--lib', 'content::native_gameplay::tests::', '--', '--test-threads=1']),
    ('server-content', [cargo, 'test', '--locked', '--offline', '-p', 'oteryn-game-server', '--test', 'integration_rest', 'content_', '--', '--test-threads=1']),
    ('world-project', [cargo, 'test', '--locked', '--offline', '-p', 'oteryn-game-server', '--test', 'content_world_project_repository', '--', '--test-threads=1']),
    ('fmt', [cargo, 'fmt', '--all', '--check']),
]
for name, command in checks:
    started = time.monotonic()
    with Path(f'.questpc5/final-{name}.log').open('w') as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    row = {'command': command, 'exit_code': result.returncode, 'duration_seconds': time.monotonic() - started}
    Path(f'.questpc5/final-{name}.result.json').write_text(json.dumps(row, indent=2) + '\n')
    print(name, json.dumps(row), flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
