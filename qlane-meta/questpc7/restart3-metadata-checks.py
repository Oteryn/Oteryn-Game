"""Validate the one-claim promotion and re-run its unchanged dispatch scenario."""
import os
import subprocess
import sys

cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
steps = [
    ('restart3-key-coverage-check', ['python3',
     'tools/content-schema/quest-authoring/quest_callback_coverage.py',
     '--canary', '.questpc/sources/canary', '--crystal', '.questpc/sources/crystal', '--check']),
    ('restart3-key-ledger-check', ['python3',
     'tools/content-schema/quest-authoring/check_quest_claim_qualification.py', '--check']),
    ('restart3-key-qualification-units', ['python3', '-m', 'unittest',
     'discover', '-s', 'tools/content-schema/quest-authoring',
     '-p', 'test_quest_claim_qualification.py']),
    ('restart3-governance', ['python3', 'tools/agents/validate_governance.py']),
    ('restart3-governance-units', ['python3', '-m', 'unittest', 'discover',
     '-s', 'tools/agents/tests']),
    ('restart3-key-fmt-check', [cargo, 'fmt', '--all', '--', '--check']),
    ('restart3-key-clippy', [cargo, 'clippy', '--locked', '--offline', '-p',
     'oteryn-gameplay-dispatch', '-p', 'oteryn-gameplay-content', '--all-targets',
     '--all-features', '--quiet', '--', '-D', 'warnings']),
    ('restart3-key-final-pg', [cargo, 'test', '--locked', '--offline', '-p',
     'oteryn-gameplay-dispatch', '--all-features', '--lib',
     'quest_catalogue_chests_mintwallin_prison_key3620_use_replay_and_refusal',
     '--', '--test-threads=1', '--nocapture']),
]
for label, command in steps:
    result = subprocess.run([sys.executable, '.questpc/run_timed.py', label, *command],
                            env=dict(os.environ, CARGO_BUILD_JOBS='4'), check=False)
    if result.returncode:
        sys.exit(result.returncode)
