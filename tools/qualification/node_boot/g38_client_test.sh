#!/usr/bin/env bash
# Compile the production client library test target for the selected G38 journey.
set -euo pipefail
[[ $1 == 1 ]] || exit 0

groups=${NODE_BOOT_E2E_ONLY-}
case ",${groups//[[:space:]]/}," in
  ,,|*,rows,*) ;;
  *) exit 0 ;;
esac

python3 - "${OTERYN_E2E_ROW_BATCH-}" <<'PY' || exit 0
import re
import sys

batch = sys.argv[1]
if batch == "all":
    sys.exit(0)
match = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", batch)
if not match:
    sys.exit(1)
start, count = map(int, match.groups())
sys.exit(0 if start <= 305 < start + count and count < 2**64 else 1)
PY

client_test=$(cargo +1.100.0 test --locked -p oteryn-client --lib --no-run --message-format=json |
  python3 -c '
import json
import sys

executables = []
for line in sys.stdin:
    message = json.loads(line)
    if message.get("reason") == "compiler-message":
        sys.stderr.write(message["message"].get("rendered") or "")
    if (message.get("reason") == "compiler-artifact"
            and message["target"]["name"] == "oteryn_client"
            and message["target"]["kind"] == ["lib"]
            and message["profile"]["test"]
            and message.get("executable")):
        executables.append(message["executable"])
if len(executables) != 1:
    sys.exit("G38 requires exactly one client library test executable")
print(executables[0])
'
)
printf '%s\n' "$client_test"
