#!/usr/bin/env bash
# User-level supervisor for the single preproduction Game service.
# No root, no sudo, no Docker: a pid file under BASE/run and a log under BASE/log.
# usage: supervisor.sh start|stop|status|health
set -Eeuo pipefail
umask 077

BASE="${BASE:-/volume1/oteryn/game-preprod}"
PID_FILE="$BASE/run/node.pid"
LOG_FILE="$BASE/log/node.log"
STATE_FILE="$BASE/run/node-state"
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

start() {
  [[ -n "${OTERYN_NATIVE_GAMEPLAY_MANIFEST:-}" ]] || { echo "OTERYN_NATIVE_GAMEPLAY_MANIFEST unset" >&2; return 1; }
  running && { echo "already running" >&2; return 1; }
  install -d -m 0700 "$BASE/run" "$BASE/log"
  # Previous log is kept for diagnosis; the new incarnation starts a fresh one.
  [[ ! -f "$LOG_FILE" ]] || mv -f "$LOG_FILE" "$LOG_FILE.previous"
  nohup "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" \
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
      node_line="$(grep 'event=registered' "$LOG_FILE" | tail -n 1)"
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

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  status) running && echo running || { echo stopped; exit 3; } ;;
  health) health ;;
  *) echo "usage: $0 start|stop|status|health" >&2; exit 2 ;;
esac
