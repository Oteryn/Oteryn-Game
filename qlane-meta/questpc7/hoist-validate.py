"""Run only the affected crate/content checks and this callback mechanic's PG regression."""
from pathlib import Path
import subprocess
import sys

cargo = "/home/mole/oteryn-cache/codex-bin/cargo"
common = [cargo, "--locked", "--offline"]
commands = [
    ("check", common + ["check", "-p", "oteryn-gameplay-quests", "-p", "oteryn-gameplay-dispatch", "--quiet"]),
    ("clippy", common + ["clippy", "-p", "oteryn-gameplay-quests", "-p", "oteryn-gameplay-dispatch", "--all-targets", "--quiet", "--", "-D", "warnings"]),
    ("unit", common + ["test", "-p", "oteryn-gameplay-quests", "--lib", "world_callbacks", "--quiet"]),
    ("use-pg", common + ["test", "-p", "oteryn-gameplay-dispatch", "--lib", "quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes", "--", "--nocapture"]),
    ("content", common + ["test", "-p", "oteryn-game-server", "--test", "integration_rest", "content_", "--quiet"]),
    ("world", common + ["test", "-p", "oteryn-game-server", "--test", "content_world_project_repository", "--quiet"]),
    ("fmt", [cargo, "fmt", "--all", "--", "--check"]),
    ("mounted-fmt", ["rustfmt", "--edition", "2024", "--check", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_hoist.rs", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs"]),
]
for name, command in commands:
    result = subprocess.run([sys.executable, ".questpc7/hoist-run.py", name, *command])
    if result.returncode:
        raise SystemExit(result.returncode)
