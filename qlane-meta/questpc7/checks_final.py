import subprocess
commands = [
 ["python3", "tools/content-schema/quest-authoring/run_checks.py", "--part", "checks"],
 ["python3", "tools/content-schema/quest-authoring/run_checks.py", "--part", "unit-tests", "--jobs", "2"],
]
results=[]
for command in commands:
 print("RUN " + " ".join(command), flush=True)
 results.append(subprocess.run(command).returncode)
raise SystemExit(any(results))
