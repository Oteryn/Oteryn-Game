import json, os, subprocess, time
from pathlib import Path
cargo = "/home/mole/oteryn-cache/codex-bin/cargo"
common = [cargo, "--locked", "--offline"]
commands = [
 ("fmt", [cargo, "fmt", "--all", "--", "--check"]),
 ("mounted-fmt", ["rustfmt", "--edition", "2024", "--check", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc7_asymmetric_lever.rs", "crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_world_callbacks/scenario.rs"]),
 ("check", common + ["check", "-p", "oteryn-gameplay-quests", "-p", "oteryn-gameplay-dispatch", "--quiet"]),
 ("clippy", common + ["clippy", "-p", "oteryn-gameplay-quests", "-p", "oteryn-gameplay-dispatch", "--all-targets", "--quiet", "--", "-D", "warnings"]),
 ("units", common + ["test", "-p", "oteryn-gameplay-quests", "--lib", "world_callbacks", "--quiet"]),
 ("pg", common + ["test", "-p", "oteryn-gameplay-dispatch", "--lib", "questpc7_giant_smithhammer", "--", "--nocapture"]),
 ("content", common + ["test", "-p", "oteryn-game-server", "--test", "integration_rest", "content_", "--quiet"]),
 ("world", common + ["test", "-p", "oteryn-game-server", "--test", "content_world_project_repository", "--quiet"]),
 ("tree", ["python3", "tools/content-schema/quest-authoring/quest_tree_authoring.py", "content", "--check", "--source-packet", "tools/content-schema/quest-authoring/samples/migration/quest-source-packet.json"]),
]
env = os.environ.copy()
env["CARGO_BUILD_JOBS"] = "3"
results = []
for name, command in commands:
 start = time.monotonic()
 log = Path(f".questpc7/asymmetric-final-{name}.log")
 with log.open("w") as output:
  result = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
 row = dict(name=name, command=command, exit_code=result.returncode, seconds=time.monotonic()-start, log=str(log))
 results.append(row)
 Path(".questpc7/asymmetric-validation.json").write_text(json.dumps(results, indent=2)+"\n")
 print(f"{name}: rc={row['exit_code']} seconds={row['seconds']:.3f}", flush=True)
 if result.returncode:
  raise SystemExit(result.returncode)
