import hashlib
import json
from pathlib import Path
import subprocess
import time

cargo = "/home/mole/oteryn-cache/codex-bin/cargo"
mounted = ["crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/pressure_plate_sewer.rs", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks.rs", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs"]
commands = [
    ("fmt", [cargo, "fmt", "-p", "oteryn-gameplay-world", "-p", "oteryn-gameplay-dispatch"]),
    ("mounted-fmt", ["rustfmt", "--edition", "2024", "--config", "skip_children=true", *mounted]),
    ("check", [cargo, "check", "--locked", "--offline", "-p", "oteryn-gameplay-world", "-p", "oteryn-gameplay-dispatch", "--all-targets"]),
    ("clippy", [cargo, "clippy", "--locked", "--offline", "-p", "oteryn-gameplay-world", "-p", "oteryn-gameplay-dispatch", "--all-targets", "--", "-D", "warnings"]),
    ("world-unit", [cargo, "test", "--locked", "--offline", "-p", "oteryn-gameplay-world", "--lib", "movement_item_", "--", "--nocapture"]),
    ("movement-unit", [cargo, "test", "--locked", "--offline", "-p", "oteryn-gameplay-dispatch", "--lib", "paced_movement::special_movement::tests", "--", "--nocapture"]),
    ("pg", [cargo, "test", "--locked", "--offline", "-p", "oteryn-gameplay-dispatch", "--lib", "devil_helmet_pressure_plate", "--", "--nocapture", "--test-threads=1"]),
    ("fmt-check", [cargo, "fmt", "--all", "--check"]),
    ("mounted-fmt-check", ["rustfmt", "--edition", "2024", "--config", "skip_children=true", "--check", *mounted]),
]
results = []
for name, command in commands:
    started = time.monotonic()
    log = Path(f".questpc6/pressure-final-{name}.log")
    with log.open("w") as output:
        result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, check=False)
    entry = {"name": name, "command": command, "exit_code": result.returncode, "wall_seconds": time.monotonic() - started, "log": str(log)}
    results.append(entry)
    Path(".questpc6/pressure-final-validation.json").write_text(json.dumps(results, indent=2) + "\n")
    print(json.dumps(entry), flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
paths = Path(".questpc6/CHANGED-FILES.txt").read_text().splitlines()
Path(".questpc6/pressure-final-source-hashes.json").write_text(json.dumps({p: hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in paths if p.startswith("crates/")}, indent=2) + "\n")
