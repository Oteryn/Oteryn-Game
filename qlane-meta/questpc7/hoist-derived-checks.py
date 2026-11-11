import json, subprocess, time
from pathlib import Path
base = Path("tools/content-schema/quest-authoring")
names = ["quest_completion_import", "quest_native_binding_preflight", "quest_reward_only_binding_candidates"]
results = []
for name in names:
 command = ["python3", str(base / (name + ".py")), "--check"]
 start = time.monotonic()
 result = subprocess.run(command, capture_output=True, text=True)
 Path(f".questpc7/hoist-derived-{name}.log").write_text(result.stdout + result.stderr)
 results.append(dict(name=name, command=command, exit_code=result.returncode, seconds=time.monotonic()-start))
 Path(".questpc7/hoist-derived-validation.json").write_text(json.dumps(results, indent=2)+"\n")
 print(f"{name}: {result.returncode}", flush=True)
 if result.returncode:
  print(result.stdout + result.stderr, flush=True)
  if not any(word in (result.stdout + result.stderr).lower() for word in ("stale", " differs", " differences")):
   raise SystemExit(result.returncode)
  generation = subprocess.run(command[:-1], capture_output=True, text=True)
  Path(f".questpc7/hoist-derived-{name}-regenerate.log").write_text(generation.stdout + generation.stderr)
  if generation.returncode:
   print(generation.stdout + generation.stderr, flush=True)
   raise SystemExit(generation.returncode)
  repaired = subprocess.run(command, capture_output=True, text=True)
  Path(f".questpc7/hoist-derived-{name}-verified.log").write_text(repaired.stdout + repaired.stderr)
  results[-1]["regenerated"] = True
  results[-1]["verified_exit_code"] = repaired.returncode
  Path(".questpc7/hoist-derived-validation.json").write_text(json.dumps(results, indent=2)+"\n")
  print(f"{name}: repaired check={repaired.returncode}", flush=True)
  if repaired.returncode:
   print(repaired.stdout + repaired.stderr, flush=True)
   raise SystemExit(repaired.returncode)
