"""Run remaining NPCFIX qualification sequentially; record restart position."""
from pathlib import Path
import subprocess
import argparse

steps = [('default-state-red', ['python3', '.npcfix/prove_default_state_red.py'])]
steps += [(name, ['python3', '.npcfix/run_checks.py', '--only', name]) for name in [
    'regeneration', 'fmt', 'fmt-mounted', 'check', 'clippy',
    'content-unit-npc', 'native-content-unit', 'npc-item-provider', 'npc-unit',
    'durability-npc-unit', 'npc-materializer', 'content-npc-catalogue',
    'content-npc-service-model', 'content-world-repository', 'content-world-v2',
    'integration-content', 'dispatch-service-unit', 'dispatch-npc-unit',
    'dispatch-guardian-chat', 'placements', 'pin-check',
    'world-npc-admission', 'fmt-check',
    'fmt-mounted-check', 'diff-check',
]]
parser = argparse.ArgumentParser()
parser.add_argument('--from', dest='start')
args = parser.parse_args()
active = args.start is None
for name, command in steps:
    if name == args.start:
        active = True
    if not active:
        continue
    Path('.npcfix/final-check-position.txt').write_text(name + '\n')
    print('FINAL PHASE ' + name, flush=True)
    subprocess.run(command, check=True)
Path('.npcfix/final-check-position.txt').write_text('finished\n')
