import json
import subprocess
from pathlib import Path

cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
checks = [
    ('fmt-final', [cargo, 'fmt', '--all']),
    ('check-final', [cargo, 'check', '--locked', '--workspace', '--all-targets']),
    ('clippy-final', [cargo, 'clippy', '--locked', '-p', 'oteryn-gameplay-dispatch', '-p', 'oteryn-gameplay-quests', '-p', 'oteryn-game-server', '--all-targets', '--', '-D', 'warnings']),
    ('quest-state-final', [cargo, 'test', '--locked', '-p', 'oteryn-gameplay-durability', '--lib', 'durability::quest_state::tests::']),
    ('grizzly-final', [cargo, 'test', '--locked', '-p', 'oteryn-gameplay-dispatch', '--lib', 'gameplay_transport::grizzly_kills::tests::']),
    ('dialogue-final', [cargo, 'test', '--locked', '-p', 'oteryn-gameplay-dispatch', '--lib', 'gameplay_transport::npc_command::quest_dialogue::publication_tests::']),
]
results = []
for name, command in checks:
    log = Path('.fix3042d') / f'fix3042e-{name}.log'
    with log.open('w') as output:
        code = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT).returncode
    results.append({'command': command, 'exit_code': code, 'log': str(log)})
    Path('.fix3042d/fix3042e-final-checks.json').write_text(json.dumps(results, indent=2))
    print(name, code, flush=True)
    if code:
        break
