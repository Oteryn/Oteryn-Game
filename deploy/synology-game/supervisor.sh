#!/usr/bin/env bash
# User-level supervisor for the single preproduction Game service.
# No root, no sudo, no Docker: a pid file under BASE/run and a log under BASE/log.
# usage: supervisor.sh start|stop|status|health|registration-gap
set -Eeuo pipefail
umask 077

BASE="${BASE:-/volume1/oteryn/game-preprod}"
PID_FILE="$BASE/run/node.pid"
LOG_FILE="$BASE/log/node.log"
STATE_FILE="$BASE/run/node-state"
# The pinned NPC catalogue the deploy installs (stage_npc_data.py); present only when the deploy staged it.
NPC_DATA_DIR="$BASE/run/npc-data"
HEALTH_SECONDS="${HEALTH_SECONDS:-120}"

running() {
  [[ -s "$PID_FILE" ]] || return 1
  local pid
  pid="$(cat "$PID_FILE")"
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  # The pid must be our server, not a recycled number.
  [[ "$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null)" == "$BASE/bin/oteryn-game-server serve "* ]]
}

stop() {
  if running; then
    local pid
    pid="$(cat "$PID_FILE")"
    kill -TERM "$pid"
    for _ in $(seq 1 60); do
      running || break
      sleep 1
    done
    if running; then
      echo "service did not stop within 60s" >&2
      return 1
    fi
  fi
  rm -f "$PID_FILE" "$STATE_FILE"
}

# npc_data_args: prints, one per line, the node's --npc-data-project/--npc-data-sha256 flags when the deploy
# staged the catalogue. Fail closed: a directory that is not a plain directory, a pin that is not 64
# lowercase hex digits or a missing project root is an error, never a start without NPC data. No directory
# at all means no NPC staging (the deploy removes it when the stage_npc_data input is off): no flags.
npc_data_args() {
  [[ -e "$NPC_DATA_DIR" || -L "$NPC_DATA_DIR" ]] || return 0
  local digest
  if [[ -L "$NPC_DATA_DIR" || ! -d "$NPC_DATA_DIR" ]]; then echo "npc-data is not a plain directory: $NPC_DATA_DIR" >&2; return 1; fi
  if [[ -L "$NPC_DATA_DIR/PROJECT_SHA256" || ! -f "$NPC_DATA_DIR/PROJECT_SHA256" ]]; then echo "npc-data pin missing: $NPC_DATA_DIR/PROJECT_SHA256" >&2; return 1; fi
  digest="$(tr -d '\n' < "$NPC_DATA_DIR/PROJECT_SHA256")"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || { echo "npc-data pin is not a lowercase sha256" >&2; return 1; }
  if [[ -L "$NPC_DATA_DIR/content/world" || ! -d "$NPC_DATA_DIR/content/world" ]]; then echo "npc-data project root missing: $NPC_DATA_DIR/content/world" >&2; return 1; fi
  printf '%s\n' --npc-data-project "$NPC_DATA_DIR/content/world" --npc-data-sha256 "$digest"
}

start() {
  [[ -n "${OTERYN_NATIVE_GAMEPLAY_MANIFEST:-}" ]] || { echo "OTERYN_NATIVE_GAMEPLAY_MANIFEST unset" >&2; return 1; }
  running && { echo "already running" >&2; return 1; }
  local npc_out
  local -a npc_args=()
  npc_out="$(npc_data_args)" || return 1
  [[ -z "$npc_out" ]] || mapfile -t npc_args <<< "$npc_out"
  install -d -m 0700 "$BASE/run" "$BASE/log"
  # Previous log is kept for diagnosis; the new incarnation starts a fresh one.
  [[ ! -f "$LOG_FILE" ]] || mv -f "$LOG_FILE" "$LOG_FILE.previous"
  nohup "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" ${npc_args[@]+"${npc_args[@]}"} \
    < /dev/null > /dev/null 2> "$LOG_FILE" &
  echo $! > "$PID_FILE"
}

# Healthy: the process stays alive and the log shows registration, then either
# awaiting_assignment (operator assigns the scope) or readiness ready=true.
health() {
  local node_line
  for _ in $(seq 1 "$HEALTH_SECONDS"); do
    running || { echo "service exited" >&2; tail -n 40 "$LOG_FILE" >&2 || true; return 1; }
    if grep -q 'readiness ready=true' "$LOG_FILE" 2>/dev/null; then
      echo ready > "$STATE_FILE"
    elif grep -q 'event=awaiting_assignment' "$LOG_FILE" 2>/dev/null; then
      echo awaiting_assignment > "$STATE_FILE"
    fi
    if [[ -s "$STATE_FILE" ]]; then
      node_line="$({ grep 'event=registered' "$LOG_FILE" || true; } | tail -n 1)"
      sed -n 's/.*node_id=\([^ ]*\).*/\1/p' <<<"$node_line" > "$BASE/run/current-node-id"
      [[ -s "$BASE/run/current-node-id" ]] || { echo "no node id in log" >&2; return 1; }
      echo "health=ok state=$(cat "$STATE_FILE")"
      return 0
    fi
    sleep 1
  done
  echo "health timeout" >&2
  tail -n 40 "$LOG_FILE" >&2 || true
  return 1
}

# The node logs event=registering before it commits its registration and event=registered after.
# A registering line without the matching registered line (a crash, kill or cancel in that gap) means
# the registration may be durable although the log does not show it: print the candidate id, exit 1.
# The log is the newest incarnation's (start moves the previous one aside), and it is final once the
# process has exited. No registering line means the process never reached the commit.
registration_gap() {
  [[ -f "$LOG_FILE" ]] || return 0
  local id
  id="$({ grep 'event=registering' "$LOG_FILE" || true; } | tail -n 1 | sed -n 's/.*node_id=\([^ ]*\).*/\1/p')"
  [[ -n "$id" ]] || return 0
  if { grep 'event=registered' "$LOG_FILE" || true; } | grep -qF "node_id=$id "; then return 0; fi
  echo "$id"
  return 1
}

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  status) running && echo running || { echo stopped; exit 3; } ;;
  health) health ;;
  registration-gap) registration_gap ;;
  *) echo "usage: $0 start|stop|status|health|registration-gap" >&2; exit 2 ;;
esac
