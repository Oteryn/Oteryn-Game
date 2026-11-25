#!/usr/bin/env bash
# Stage the pinned NPC project and prepare the node's serve arguments.
node_boot_stage_npc_data() {
  # Bind the NPC catalogue from the same checkout that compiled the World.
  # The node verifies the project digest and independently matches the bundle's catalogue digest.
  (
    # The privileged runner stages as the operator and serves as a separate user.
    umask 022
    PYTHONPATH="$GAME_SOURCE/deploy/synology-game" python3 - "$GAME_SOURCE" "$WORK" <<'PY_NPC_STAGE'
import json
import sys
from pathlib import Path
from stage_npc_data import PIN_SCHEMA, catalogue_digest, collect_catalogue, project_files, stage

repo, work = map(Path, sys.argv[1:])
content = repo / "content"
revision, project = project_files(content / "world")
pin = work / "npc-data.pin.json"
pin.write_text(json.dumps({
    "schema": PIN_SCHEMA,
    "project_revision": revision,
    "project_sha256": catalogue_digest(project.items()),
    "catalogue_sha256": catalogue_digest(collect_catalogue(content)),
}), encoding="utf-8")
stage(pin, repo, work / "npc-data")
PY_NPC_STAGE
  )
  "${as_operator[@]}" mv -- "$WORK/npc-data" "$BASE/npc-data"
  NODE_NPC_ARGS=(--npc-data-project "$BASE/npc-data/content/world"
    --npc-data-sha256 "$(< "$BASE/npc-data/PROJECT_SHA256")")
}
