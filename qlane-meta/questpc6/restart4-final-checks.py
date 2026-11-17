"""Run focused exact-tree checks serially without publication or index writes."""
import os
import subprocess
import sys

cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
steps = [
    ('restart4-coverage-check', ['python3',
     'tools/content-schema/quest-authoring/quest_callback_coverage.py',
     '--canary', '.questpc/sources/canary', '--crystal', '.questpc/sources/crystal', '--check']),
    ('restart4-ledger-check', ['python3',
     'tools/content-schema/quest-authoring/check_quest_claim_qualification.py', '--check']),
    ('restart4-qualification-units', ['python3', '-m', 'unittest',
     'discover', '-s', 'tools/content-schema/quest-authoring',
     '-p', 'test_quest_claim_qualification.py']),
    ('restart4-governance', ['python3', 'tools/agents/validate_governance.py']),
    ('restart4-governance-units', ['python3', '-m', 'unittest', 'discover',
     '-s', 'tools/agents/tests']),
    ('restart4-fmt-check', [cargo, 'fmt', '--all', '--', '--check']),
    ('restart4-mounted-fmt-check', ['rustup', 'run', '1.99.1', 'rustfmt',
     '--edition', '2024', '--check', '--config', 'skip_children=true',
     'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs']),
    ('restart4-clippy', [cargo, 'clippy', '--locked', '--offline', '-p',
     'oteryn-gameplay-dispatch', '--all-targets', '--all-features', '--quiet',
     '--', '-D', 'warnings']),
]
for label, command in steps:
    result = subprocess.run([sys.executable, '.questpc/run_timed.py', label, *command],
                            env=dict(os.environ, CARGO_BUILD_JOBS='4'), check=False)
    if result.returncode:
        sys.exit(result.returncode)
result = subprocess.run([sys.executable, '.questpc/restart4-story-reward-run.py',
                         '.questpc/restart4-story-reward-cohorts.json'],
                        env=dict(os.environ, CARGO_BUILD_JOBS='4'), check=False)
sys.exit(result.returncode)
