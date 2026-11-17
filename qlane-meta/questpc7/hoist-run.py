"""Retain bounded commands, exit codes and elapsed time without environment data."""
import json
from pathlib import Path
import subprocess
import sys
import time

name, *command = sys.argv[1:]
start = time.monotonic()
log = Path(f".questpc7/hoist-{name}.log")
with log.open("w") as output:
    result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT)
row = {"command": command, "exit_code": result.returncode,
       "seconds": round(time.monotonic() - start, 3), "log": str(log)}
Path(f".questpc7/hoist-{name}.result.json").write_text(json.dumps(row, indent=2) + "\n")
print(json.dumps(row), flush=True)
raise SystemExit(result.returncode)
