"""Regenerate the existing dependent model after the Corruptor loot projection."""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path('tools/content-migration').resolve()))
import prey_difficulty as prey

root = Path.cwd()
profiles = prey.fresh_profiles(root)
model = prey.model_input(root, profiles)
inputs = root / '.questpc/r8-prey-input.json'
creatures = root / '.questpc/r8-prey-creatures.json'
inputs.write_bytes(prey.encoded(model))
creatures.write_bytes(prey.encoded(profiles))
subprocess.run(['/home/mole/oteryn-cache/codex-bin/cargo', 'run', '--locked', '--offline',
    '-p', 'oteryn-gameplay-dispatch', '--example', 'prey_time_sim', '--',
    '--input', str(inputs), '--output', str(root / prey.RESULTS),
    '--creature-profiles', str(creatures)], check=True)
files, _ = prey.outputs(root, profiles)
prey.write_files(root, files)
print(json.dumps({'generated_files': sorted(files), 'input_sha256': model['input_sha256']}))
