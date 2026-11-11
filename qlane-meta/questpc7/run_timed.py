import json, subprocess, sys, time
from pathlib import Path
name = sys.argv[1]
command = sys.argv[2:]
start = time.monotonic()
with Path(f".questpc/{name}.log").open("w") as log:
    result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
row = {"command": " ".join(command), "exit": result.returncode, "duration_seconds": round(time.monotonic()-start, 3)}
Path(f".questpc/{name}.result.json").write_text(json.dumps(row, indent=2)+"\n")
print(json.dumps(row))
sys.exit(result.returncode)
