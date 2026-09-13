#!/usr/bin/env bash
set -euo pipefail

upload_blob() {
  local path="$1"
  local payload
  payload="$(mktemp)"
  python - "$path" "$payload" <<'PY'
import base64, json, sys
src, dst = sys.argv[1], sys.argv[2]
with open(src, 'rb') as f:
    content = base64.b64encode(f.read()).decode('ascii')
with open(dst, 'w', encoding='utf-8') as f:
    json.dump({'content': content, 'encoding': 'base64'}, f)
PY
  gh api --method POST "repos/${GITHUB_REPOSITORY}/git/blobs" --input "$payload" --jq .sha
  rm -f "$payload"
}

worker_sha="$(upload_blob .github/workflows/oteryn-agentic-pilot-worker.lock.yml)"
router_sha="$(upload_blob .github/workflows/oteryn-agentic-pilot-router.lock.yml)"
printf 'PILOT_WORKER_BLOB_SHA:%s\n' "$worker_sha"
printf 'PILOT_ROUTER_BLOB_SHA:%s\n' "$router_sha"
