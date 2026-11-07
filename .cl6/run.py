"""Capture local validation commands, elapsed time, and full output."""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time

root = Path(__file__).resolve().parent
label, *command = sys.argv[1:]
started = time.monotonic()
env = dict(os.environ, CARGO_NET_OFFLINE="true")
with (root / f"{label}.log").open("w") as output:
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env, text=True)
    for line in process.stdout:
        output.write(re.sub(r"postgres(?:ql)?://[^\s\"']+", "[REDACTED_POSTGRES_URL]", line))
        output.flush()
    process.wait()
duration = time.monotonic() - started
log_path = root / f"{label}.log"
log = re.sub(r"postgres(?:ql)?://[^\s\"']+", "[REDACTED_POSTGRES_URL]", log_path.read_text())
log_path.write_text(log)
record = {
    "label": label,
    "command": shlex.join(command),
    "exit_code": process.returncode,
    "duration_seconds": round(duration, 3),
    "log": log_path.name,
}
with (root / "commands.jsonl").open("a") as output:
    output.write(json.dumps(record) + "\n")
print(json.dumps(record), flush=True)
print("\n".join(log.splitlines()[-30:]), flush=True)
sys.exit(process.returncode)
