#!/usr/bin/env bash
# Stage the pinned NPC project and prepare the node's serve arguments.
node_boot_stage_npc_data() {
  # Bind the NPC catalogue from the same checkout that compiled the World.
  # The node verifies the project digest and independently matches the bundle's catalogue digest.
  (
    # The privileged runner stages as the operator and serves as a separate user.
    umask 022
    python3 "$GAME_SOURCE/deploy/synology-game/stage_npc_data.py" \
      "$GAME_SOURCE/deploy/synology-game/npc-data.pin.json" "$GAME_SOURCE" "$WORK/npc-data" >/dev/null
  )
  "${as_operator[@]}" mv -- "$WORK/npc-data" "$BASE/npc-data"
  NODE_NPC_ARGS=(--npc-data-project "$BASE/npc-data/content/world"
    --npc-data-sha256 "$(< "$BASE/npc-data/PROJECT_SHA256")")
}
