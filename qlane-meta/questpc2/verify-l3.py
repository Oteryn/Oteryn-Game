"""Focused L3 checks; never runs an unfiltered PostgreSQL target."""
import json
from pathlib import Path
import subprocess
import time

cargo = "/home/mole/oteryn-cache/codex-bin/cargo"
base = [cargo, "test", "--locked", "--offline"]
checks = [
    ("endgame-regression", base + ["-p", "oteryn-gameplay-dispatch", "--lib", "endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02", "--", "--nocapture", "--test-threads=1"], True),
    ("ordinary-admission-regression", base + ["-p", "oteryn-gameplay-dispatch", "--lib", "boss_activity_postgres_real_wire_native_entry_and_scope_barrier", "--", "--nocapture", "--test-threads=1"], True),
    ("endgame-units", base + ["-p", "oteryn-gameplay-encounters", "--lib", "endgame_entry", "--", "--nocapture"], False),
    ("clippy", [cargo, "clippy", "--locked", "--offline", "-p", "oteryn-gameplay-durability", "-p", "oteryn-gameplay-dispatch", "--all-targets", "--all-features", "--quiet", "--", "-D", "warnings"], False),
    ("fmt", [cargo, "fmt", "--all", "--", "--check"], False),
    ("new-module-fmt", ["rustfmt", "--edition", "2024", "--config", "skip_children=true", "--check", "crates/gameplay/durability/src/durability/activity_handoff/boss_room/quest_entry.rs"], False),
    ("whitespace", ["git", "diff", "7aa809818", "--check", "--", "crates", ".questpc2/RESUME.md", ".questpc2/QUEST-STATUS.json"], False),
]
results = []
for name, command, postgres in checks:
    log = Path(f".questpc2/l3-{name}.log")
    started = time.monotonic()
    with log.open("w") as output:
        result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, check=False)
    duration = round(time.monotonic() - started, 3)
    text = log.read_text()
    executed = not postgres or ("POSTGRES_CASE_COMPLETED" in text and "POSTGRES_CASE_SKIPPED" not in text)
    ok = result.returncode == 0 and executed
    results.append(dict(name=name, command=command, exit_code=result.returncode, duration_seconds=duration, postgres_executed=executed if postgres else None, passed=ok, log=str(log)))
    Path(".questpc2/l3-verification.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"{name}: {'PASS' if ok else 'FAIL'} ({duration}s)", flush=True)
    if not ok:
        raise SystemExit(result.returncode or 1)
