"""Run the final bounded checks serially; retain each complete command result."""
import os
import subprocess
import sys

cargo = '/home/mole/oteryn-cache/codex-bin/cargo'
steps = [
    ('restart3-mounted-fmt-check', ['rustup', 'run', '1.99.1', 'rustfmt', '--edition',
     '2024', '--check', '--config', 'skip_children=true',
     'crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/newhaven_corruptor.rs']),
    ('restart3-fmt-check', [cargo, 'fmt', '--all', '--', '--check']),
    ('restart3-clippy', [cargo, 'clippy', '--locked', '--offline', '-p',
     'oteryn-gameplay-dispatch', '-p', 'oteryn-gameplay-content', '--all-targets',
     '--all-features', '--quiet', '--', '-D', 'warnings']),
    ('restart3-content-unit', [cargo, 'test', '--locked', '--offline', '-p',
     'oteryn-gameplay-content', '--lib', 'equipment_projection_tests']),
    ('restart3-quest-unit', [cargo, 'test', '--locked', '--offline', '-p',
     'oteryn-gameplay-quests', '--lib',
     'tutorial_corruptor_rewards_use_source_items_and_base_vocations']),
    ('restart3-final-pg', [cargo, 'test', '--locked', '--offline', '-p',
     'oteryn-gameplay-dispatch', '--all-features', '--lib', '--',
     'newhaven_corruptor_', 'viola_mission_yes',
     'viola_book_postgres_real_use_repeat_replay_refusal_and_fences',
     '--test-threads=1', '--nocapture']),
]
environment = dict(os.environ, CARGO_BUILD_JOBS='4')
for label, command in steps:
    result = subprocess.run([sys.executable, '.questpc/run_timed.py', label, *command],
                            env=environment, check=False)
    if result.returncode:
        sys.exit(result.returncode)
