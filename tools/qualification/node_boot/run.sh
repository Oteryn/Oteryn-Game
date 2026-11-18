#!/usr/bin/env bash
# OPS-NODE-BOOT-01 §4: physical qualification with the shipped binaries.
#
# The real Platform producer (S3-A/S3-B topology, one client identity for
# evidence and intents), a TLS-verified PostgreSQL 18.6, a system service
# user running `oteryn-game-server serve`, and root running
# `oteryn-game-ops` on a separate control-plane login. The node boots,
# waits for the operator's assignment, bootstraps Characters from real
# Platform intents through its control socket, and reproduces every #823
# SEAM stage against its own bound port.
set -Eeuo pipefail
umask 077

readonly PLATFORM_SHA=3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05
readonly TOPOLOGY_REVISION=wp5-node-boot-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000003
readonly SECOND_PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000004
readonly EXTRA_WORLD_ACCOUNTS=(01934f10-7c00-7000-8000-000000000007 01934f10-7c00-7000-8000-000000000008 01934f10-7c00-7000-8000-000000000009 01934f10-7c00-7000-8000-00000000000a 01934f10-7c00-7000-8000-00000000000b 01934f10-7c00-7000-8000-00000000000c)
readonly FRESH_KEY_ID=node-boot-fresh-key-1
readonly RECOVERY_KEY_ID=node-boot-recovery-key-1
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly CHANNEL_ID=01934f10-7c03-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402 01934f10-7c04-7003-805b-3b1122334403 01934f10-7c04-7004-805b-3b1122334404 01934f10-7c04-7007-805b-3b1122334407 01934f10-7c04-7008-805b-3b1122334408 01934f10-7c04-7009-805b-3b1122334409 01934f10-7c04-700a-805b-3b112233440a 01934f10-7c04-700b-805b-3b112233440b 01934f10-7c04-700c-805b-3b112233440c)
# CHAR-NAME-1: bootstrap intent contract version 2 names each Character (naming policy revision 1).
readonly INTENT_NAMES=("Qualification One" "Qualification Two" "Packet One" "Packet Two" "Party One" "Party Two" "Party Three" "Party Four" "Sweep One" "Sweep Two")
# Runtime build writers bind to the shipped training formula's content revision,
# just as the S3-B seam does; progression is initialized from this interpretation.
# The served readiness.content_revision is this same revision: the quest and Task Board
# owners refuse a Character whose stored progression content differs from the served one.
readonly INTERPRETATION=(nb-profile-1 nb-ruleset-1 s3b-content-1 nb-starter-1)
# The second, Open PvP (world_type=2) World of the World pass: its own Accounts, intents and node.
readonly PVP_WORLD_ID=01934f10-7c02-7002-805b-3b1122334402
readonly PVP_CHANNEL_ID=01934f10-7c03-7002-805b-3b1122334402
readonly PVP_ACCOUNT_IDS=(01934f10-7c00-7000-8000-000000000005 01934f10-7c00-7000-8000-000000000006 01934f10-7c00-7000-8000-000000000007 01934f10-7c00-7000-8000-000000000008)
readonly PVP_INTENT_OPERATIONS=(01934f10-7c04-7005-805b-3b1122334405 01934f10-7c04-7006-805b-3b1122334406 01934f10-7c04-7007-805b-3b1122334407 01934f10-7c04-7008-805b-3b1122334408)
readonly PVP_INTENT_NAMES=("Arena One" "Arena Two" "Arena Caster" "Arena Target")
readonly PVP_DATABASE=oteryn_node_boot_pvp
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_S3B=tools/qualification/wp5_s3b/compose.override.yml
readonly COMPOSE_OVERLAY=tools/qualification/node_boot/compose.override.yml
readonly PG_IMAGE=postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc
# Local parallel stacks override the service user, base, Compose project and ports.
# Validate them before provisioning, then record the resources acquired by this run.
SERVICE_USER="${NODE_BOOT_SERVICE_USER:-oteryn-node-boot}"
BASE="${NODE_BOOT_BASE:-/srv/oteryn-node-boot}"
[[ "$SERVICE_USER" =~ ^oteryn-node-boot(-[a-z0-9]{1,16})?$ ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_service_user'; exit 2; }
node_boot_tmp="$(realpath -m -- "${TMPDIR:-/tmp}")"
if [[ "$BASE" != "$(realpath -m -- "$BASE")" ]] \
  || ! [[ "$BASE" == /srv/?* || ( "$node_boot_tmp" != / && "$BASE" == "$node_boot_tmp"/?* ) ]]; then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=base_outside_srv_or_tmpdir'; exit 2
fi
[[ -z "${NODE_BOOT_PROJECT+x}" || "$NODE_BOOT_PROJECT" =~ ^[a-z0-9][a-z0-9_-]{0,47}$ ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_project'; exit 2; }
for node_boot_port in NODE_BOOT_PG_PORT NODE_BOOT_PLATFORM_PORT NODE_BOOT_GAME_PORT NODE_BOOT_PVP_GAME_PORT; do
  [[ -z "${!node_boot_port+x}" || ( "${!node_boot_port}" =~ ^[1-9][0-9]{3,4}$ && "${!node_boot_port}" -le 65535 ) ]] \
    || { echo "NODE_BOOT_RESULT=BLOCKED reason=invalid_port_${node_boot_port,,}"; exit 2; }
done
readonly SERVICE_USER BASE
# The Open PvP node has its own listener. Allow an override for parallel stacks.
readonly PVP_BASE="$BASE/pvp"
node_boot_game_port="${NODE_BOOT_GAME_PORT:-17181}"
# Port zero lets the real listener bind atomically and report its assigned port.
PVP_GAME_PORT="${NODE_BOOT_PVP_GAME_PORT:-0}"
if [[ -n "${NODE_BOOT_GAME_PORT+x}" && -z "${NODE_BOOT_PVP_GAME_PORT+x}" ]]; then
  PVP_GAME_PORT=$((node_boot_game_port + 1))
fi
if [[ -n "${NODE_BOOT_PVP_GAME_PORT+x}" ]] && (( PVP_GAME_PORT == node_boot_game_port
  || PVP_GAME_PORT == ${NODE_BOOT_PG_PORT:-15433}
  || PVP_GAME_PORT == ${NODE_BOOT_PLATFORM_PORT:-18463} )); then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_port_node_boot_pvp_game_port'; exit 2
fi
# NODE_BOOT_KEEP_STACK=1 leaves the stack up after the seam stage; NODE_BOOT_REUSE_STACK=1
# reruns only the seam stage against it, =teardown removes it. Local development only.
NODE_BOOT_KEEP_STACK="${NODE_BOOT_KEEP_STACK:-0}"
NODE_BOOT_REUSE_STACK="${NODE_BOOT_REUSE_STACK:-0}"
case "$NODE_BOOT_KEEP_STACK/$NODE_BOOT_REUSE_STACK" in
  0/0) ;;
  1/0|0/1|0/teardown)
    if [[ "${CI:-}" == true || -n "${GITHUB_ACTIONS:-}" ]]; then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=stack_reuse_is_local_only'; exit 2
    fi ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_stack_reuse_mode'; exit 2 ;;
esac


GAME_SOURCE="$(git rev-parse --show-toplevel)"
# The separately pinned portal-rs lane owns only Platform lifecycle and its stages.
# A fixed repository path prevents an environment variable from sourcing arbitrary code.
case "${NODE_BOOT_LIFECYCLE:-}" in
  '') ;;
  game_platform)
    # shellcheck source=tools/qualification/game_platform/lifecycle.sh
    source "$GAME_SOURCE/tools/qualification/game_platform/lifecycle.sh"
    game_platform_validate
    ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=unknown_lifecycle'; exit 2 ;;
esac
NODE_BOOT_PLATFORM_PIN="$PLATFORM_SHA"
NODE_BOOT_TOPOLOGY="$TOPOLOGY_REVISION"
if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
  # portal-rs pin is owned by game_platform/lifecycle.sh (includes Platform #1833).
  NODE_BOOT_PLATFORM_PIN="$GAME_PLATFORM_SHA"
  NODE_BOOT_TOPOLOGY=portal-rs-game-platform-v1
fi
readonly NODE_BOOT_PLATFORM_PIN NODE_BOOT_TOPOLOGY
export NODE_BOOT_PROFILE="${NODE_BOOT_PROFILE:-sorcerer}"
case "$NODE_BOOT_PROFILE" in
  explorer|knight|paladin|sorcerer|druid|monk) ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_qualification_profile'; exit 2 ;;
esac
# Optional full spell input uses this same disposable topology and admission path.
NODE_BOOT_SPELLS="${NODE_BOOT_SPELLS:-0}"
case "$NODE_BOOT_SPELLS" in
  0|1) ;;
  *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_spell_mode'; exit 2 ;;
esac
# Keep every fixture-only transport/door assertion intact in its own disposable pass.
NODE_BOOT_WORLD="${NODE_BOOT_WORLD:-1}"
case "$NODE_BOOT_WORLD" in 0|1) ;; *) echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_world_mode'; exit 2 ;; esac
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_SPELLS" == 0 && "$NODE_BOOT_REUSE_STACK" == 0 && -z "${NODE_BOOT_LIFECYCLE:-}" ]]; then
  if (( PVP_GAME_PORT > 65535 || PVP_GAME_PORT == ${NODE_BOOT_PG_PORT:-15433} || PVP_GAME_PORT == ${NODE_BOOT_PLATFORM_PORT:-18463} )) \
    || ! python3 - "$PVP_GAME_PORT" <<'PY_PORT'
import socket
import sys

with socket.socket() as listener:
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", int(sys.argv[1])))
PY_PORT
  then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=pvp_port_unavailable'; exit 2
  fi
fi
# A kept or reused stack is one topology, so it never runs the separate basic pass.
# The game_platform lifecycle owns one Platform stack, so it never runs the basic pass either.
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_SPELLS" == 0 && "${NODE_BOOT_SKIP_BASIC:-0}" != 1 \
  && -z "${NODE_BOOT_LIFECYCLE:-}" \
  && "$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then
  stop_basic_pass() {
    trap '' INT TERM
    kill -TERM "$basic_pid" 2>/dev/null || true
    wait "$basic_pid" 2>/dev/null || true
    echo 'NODE_BOOT_RESULT=NODE_BOOT_FAIL reason=interrupted_basic_pass'
    exit "$1"
  }
  set -m
  NODE_BOOT_WORLD=0 bash "$GAME_SOURCE/tools/qualification/node_boot/run.sh" &
  basic_pid=$!
  set +m
  trap 'stop_basic_pass 130' INT
  trap 'stop_basic_pass 143' TERM
  basic_exit=0
  wait "$basic_pid" || basic_exit=$?
  trap - INT TERM
  [[ "$basic_exit" == 0 ]] || exit "$basic_exit"
fi
if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then NODE_BOOT_WORLD=0; fi
BOOTSTRAP_ACCOUNTS=("$ACCOUNT_ID" "$SECOND_ACCOUNT_ID")
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # Packet journeys may move, loot or kill their Characters before the required core runs.
  BOOTSTRAP_ACCOUNTS+=("$PACKET_ACCOUNT_ID" "$SECOND_PACKET_ACCOUNT_ID" "${EXTRA_WORLD_ACCOUNTS[@]}")
fi
readonly BOOTSTRAP_ACCOUNTS
source "$GAME_SOURCE/tools/qualification/node_boot/activation_wait.sh"
NODE_ACTIVATION_WAIT_SECONDS="$(node_activation_wait_seconds "$NODE_BOOT_WORLD" "$NODE_BOOT_SPELLS")"
NODE_GAMEPLAY_ENV=()
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Default: the canonical imported book. The r21 reference artifact stays selectable explicitly.
  NODE_BOOT_SPELL_MANIFEST="${NODE_BOOT_SPELL_MANIFEST:-$GAME_SOURCE/content/spells.manifest.json}"
  NODE_BOOT_SPELL_MANIFEST="$(realpath -e -- "$NODE_BOOT_SPELL_MANIFEST" 2>/dev/null)" || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  [[ -f "$NODE_BOOT_SPELL_MANIFEST" ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_missing'; exit 2; }
  # The staging root is the repository root: the manifest keeps its repository-relative path.
  [[ "$NODE_BOOT_SPELL_MANIFEST" == "$GAME_SOURCE"/* ]] || { echo 'NODE_BOOT_RESULT=BLOCKED reason=spell_manifest_outside_repository'; exit 2; }
  NODE_BOOT_SPELL_MANIFEST_REL="${NODE_BOOT_SPELL_MANIFEST#"$GAME_SOURCE"/}"
  NODE_GAMEPLAY_ENV=("OTERYN_NATIVE_GAMEPLAY_MANIFEST=$BASE/gameplay/$NODE_BOOT_SPELL_MANIFEST_REL")
fi
# ADMIT-CAP-1: the entry-room seam stages run against a Channel that requires LOGOUT_V1 (21)
# and prove the owner's capability refusals; the spell scenarios declare no required set.
NODE_REQUIRED_CAPABILITIES='[21]'
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # The real bundle offers map interaction, map view and spatial fields, whose
  # mandatory floor must be declared as well as the existing logout requirement.
  NODE_REQUIRED_CAPABILITIES='[11,18,21,30]'
elif [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
  NODE_REQUIRED_CAPABILITIES='[]'
fi
PLATFORM_SOURCE="${PLATFORM_SOURCE:-$GAME_SOURCE/_platform}"
if [[ "$(git -C "$PLATFORM_SOURCE" rev-parse HEAD 2>/dev/null)" != "$NODE_BOOT_PLATFORM_PIN" ]]; then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing'
  exit 2
fi
stage_timeout_default=10800
if [[ "$NODE_BOOT_WORLD" == 1 && -n "${E2E_ALL_CAPTURE_DIR:-}" ]]; then
  # Capture mode selects the whole row registry instead of a hosted shard.
  stage_timeout_default=43200
fi
NODE_BOOT_STAGE_TIMEOUT_SECONDS="${NODE_BOOT_STAGE_TIMEOUT_SECONDS:-$stage_timeout_default}"
if [[ ! "$NODE_BOOT_STAGE_TIMEOUT_SECONDS" =~ ^[1-9][0-9]{0,4}$ ]]; then
  echo 'NODE_BOOT_RESULT=BLOCKED reason=invalid_stage_timeout'; exit 2
fi
WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/node-boot.XXXXXX")"
node_log="$WORK/node.log"
failed_node_log="$WORK/failed-node.log"
WP5_PKI="$WORK/pki"
WP5_SCRATCH="$WORK/scratch"
mkdir -p "$WP5_PKI" "$WP5_SCRATCH"
WP5_PORT="${NODE_BOOT_PLATFORM_PORT:-18463}"
WP5_PROJECT="${NODE_BOOT_PROJECT:-nodeboot${GITHUB_RUN_ID:-local}${GITHUB_RUN_ATTEMPT:-1}}"
WP5_DB_PASSWORD="$(openssl rand -hex 24)"
WP5_DB_ROOT_PASSWORD="$(openssl rand -hex 24)"
WP5_APP_KEY="base64:$(openssl rand -base64 32 | tr -d '\n')"
WP5_TOPOLOGY_REVISION="$TOPOLOGY_REVISION"
WP5_FSYNC_FAULT=none
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT NODE_BOOT_PLATFORM_PIN
export WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT
PG_CONTAINER="${WP5_PROJECT}-postgres"
PG_PORTAL_CONTAINER="${WP5_PROJECT}-portal-postgres"
PG_PORT="${NODE_BOOT_PG_PORT:-15433}"
PG_ADMIN_PASSWORD="$(openssl rand -hex 24)"
CONTROL_PASSWORD="$(openssl rand -hex 24)"
RUNTIME_PASSWORD="$(openssl rand -hex 24)"
NODE_PID=""
RUNTIME_DIAGNOSTICS_PID=""
STAGE_PID=""
STAGE_TIMER_PID=""
ACTIVE_STAGE=setup
# Redacted PostgreSQL samples and stage dumps (pg_observe.py); kept with node.log on failure.
PG_OBS="$WORK/postgres"
PG_SAMPLER_PID=""
WORLD_CAPTURE_DIR=""
STACK_KEPT=0
BASE_PENDING=""
BASE_IDENTITY=""
COMPOSE_OWNED=0
COMPOSE_OWNER="$(openssl rand -hex 16)"
export COMPOSE_OWNER
if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  # Fresh databases own their Docker names independently of legacy journals.
  PG_CONTAINER="$WP5_PROJECT-$COMPOSE_OWNER-postgres"
  PG_PORTAL_CONTAINER="$WP5_PROJECT-$COMPOSE_OWNER-portal-postgres"
fi
REUSE_STATE_OWNED=0
PG_CONTAINER_ID=""
POSTGRES_CREATE_PENDING=""
POSTGRES_CLEANUP_FAILED=0
REUSE_STATE="${TMPDIR:-/tmp}/node-boot-reuse-$WP5_PROJECT.env"
# Keep reuse and teardown mutually exclusive. The lock and recovery journal
# belong to the host and UID, independently of the invocation's scratch path.
state_directory="/tmp/node-boot-state-$(id -u)"
mkdir -m 700 -- "$state_directory" 2>/dev/null || true
if [[ ! -d "$state_directory" || -L "$state_directory" || ! -O "$state_directory" \
  || "$(stat -c %a "$state_directory")" != 700 ]]; then
  rm -rf -- "$WORK"
  echo 'NODE_BOOT_RESULT=BLOCKED reason=unsafe_project_state'; exit 2
fi
lock_directory="$state_directory/locks"
POSTGRES_JOURNALS="$state_directory/postgres"
POSTGRES_JOURNAL="$POSTGRES_JOURNALS/$WP5_PROJECT"
mkdir -p -- "$lock_directory"
if ! { exec 9>>"$lock_directory/$WP5_PROJECT"; flock -n 9; }; then
  rm -rf -- "$WORK"
  echo 'NODE_BOOT_RESULT=BLOCKED reason=project_in_use'; exit 2
fi
if [[ "$NODE_BOOT_REUSE_STACK" == 0 && -e "$REUSE_STATE" ]]; then
  rm -rf -- "$WORK"
  echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_exists'; exit 2
fi
docker() { timeout --foreground --kill-after=5s 600s docker "$@"; }
signal_node_process() { # signal, supervisor directory
  local wanted=$1 directory=$2 pid started
  [[ "${kept_boot_id:-$(</proc/sys/kernel/random/boot_id)}" == "$(</proc/sys/kernel/random/boot_id)" ]] || return 1
  IFS=$' \t\r' read -r pid started < "$directory/process" || return 1
  [[ "$pid" =~ ^[1-9][0-9]*$ && "$started" =~ ^[0-9]+$ ]] || return 1
  timeout --foreground --kill-after=2s 5s sudo python3 - "$pid" "$started" "$wanted" <<'PY_SIGNAL'
import os
from pathlib import Path
import signal
import sys

pid = int(sys.argv[1])
try:
    # The descriptor holds the process identity across the /proc check and signal.
    process = os.pidfd_open(pid)
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(") ", 1)[1].split()
    if fields[19] != sys.argv[2] or fields[0] == "Z":
        sys.exit(1)
    wanted = 0 if sys.argv[3] == "0" else getattr(signal, f"SIG{sys.argv[3]}")
    signal.pidfd_send_signal(process, wanted)
except (OSError, IndexError):
    sys.exit(1)
PY_SIGNAL
}
if [[ "$NODE_BOOT_REUSE_STACK" != 0 ]]; then
  rm -rf -- "$WORK"
  # Owner-only state from the NODE_BOOT_KEEP_STACK run; it holds the disposable stack's secrets.
  if [[ ! -f "$REUSE_STATE" || -L "$REUSE_STATE" || ! -O "$REUSE_STATE" || "$(stat -c %a "$REUSE_STATE")" != 600 ]]; then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_state_missing'; exit 2
  fi
  # shellcheck source=/dev/null
  COMPOSE_OWNER=""
  source "$REUSE_STATE"
  if [[ "${kept_mode:-}" != "$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/${NODE_BOOT_GAME_PORT:-17181}" ]]; then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_mode_mismatch'; exit 2
  fi
  if [[ ! "$COMPOSE_OWNER" =~ ^[a-f0-9]{32}$ || "${kept_base:-}" != "$BASE" || "${kept_service_user:-}" != "$SERVICE_USER" \
    || "${kept_project:-}" != "$WP5_PROJECT" || "${kept_lifecycle:-}" != "${NODE_BOOT_LIFECYCLE:-}" \
    || -z "${kept_base_identity:-}" || "$(stat -c '%d:%i' -- "$BASE" 2>/dev/null)" != "$kept_base_identity" \
    || ( "$PG_CONTAINER" != "$WP5_PROJECT-postgres" && "$PG_CONTAINER" != "$WP5_PROJECT-$COMPOSE_OWNER-postgres" ) \
    || -z "${PG_CONTAINER_ID:-}" || "$(docker inspect -f '{{.Id}}' "$PG_CONTAINER" 2>/dev/null)" != "$PG_CONTAINER_ID" ]]; then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_identity_mismatch'; exit 2
  fi
  if [[ "$NODE_BOOT_REUSE_STACK" == 1 ]]; then
    if [[ "$NODE_BOOT_WORLD" == 1 ]] && ! python3 - "${NODE_BOOT_E2E_ONLY:-}" <<'PY_REUSED_GROUPS'
import os
import sys

try:
    selection = os.fsencode(sys.argv[1]).decode("utf-8").strip()
except UnicodeDecodeError:
    sys.exit(1)
sys.exit(0 if selection and "mp" not in (group.strip() for group in selection.split(",")) else 1)
PY_REUSED_GROUPS
    then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=mp_requires_fresh_stack'; exit 2
    fi
    if [[ "$(docker inspect -f '{{.State.Running}}' "$PG_CONTAINER_ID" 2>/dev/null)" != true ]] \
      || [[ "$(</proc/sys/kernel/random/boot_id)" != "$kept_boot_id" ]] \
      || ! signal_node_process 0 "$NODE_PROCESS_DIR" 2>/dev/null || ! sudo test -S "$BASE/run/control.sock"; then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_not_running'; exit 2
    fi
    if [[ -n "${NODE_BOOT_PVP_GAME_PORT+x}" && "${kept_pvp_port:-0}" != "$PVP_GAME_PORT" ]]; then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_mode_mismatch'; exit 2
    fi
    PVP_GAME_PORT="${kept_pvp_port:-$PVP_GAME_PORT}"
    STACK_KEPT=1
  fi
  BASE_IDENTITY=$kept_base_identity
  COMPOSE_OWNED=1
  REUSE_STATE_OWNED=1
fi

result=NODE_BOOT_FAIL
compose() {
  local seconds=600
  case "$1" in
    logs) seconds=20 ;;
  esac
  timeout --foreground --kill-after=5s "$seconds" docker compose --project-name "$WP5_PROJECT-$COMPOSE_OWNER" --file "$GAME_SOURCE/$COMPOSE_FILE" \
    --file "$GAME_SOURCE/$COMPOSE_S3B" --file "$GAME_SOURCE/$COMPOSE_OVERLAY" "$@"
}
psql_admin() { timeout --foreground --kill-after=2s 20s docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U oteryn_node_boot_admin -d "$1"; }
evidence() { printf 'NODE_BOOT_EVIDENCE %s\n' "$*"; }
fail() { echo "NODE_BOOT_FAILURE $*"; return 1; }
stop_runtime_diagnostics() {
  [[ -z "$RUNTIME_DIAGNOSTICS_PID" ]] || {
    kill -TERM "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null || true
    local limit=$((SECONDS + 5))
    while kill -0 "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null && (( SECONDS < limit )); do sleep 0.1; done
    kill -KILL "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null || true
    wait "$RUNTIME_DIAGNOSTICS_PID" 2>/dev/null || true
    RUNTIME_DIAGNOSTICS_PID=""
  }
}
pg_observe() { python3 "$GAME_SOURCE/tools/qualification/node_boot/pg_observe.py" "$@" --container "$PG_CONTAINER" --user oteryn_node_boot_admin --database oteryn_node_boot --other-database "${PVP_DATABASE:-}"; }
start_pg_sampler() {
  mkdir -p "$PG_OBS"
  # Start python3 itself in the background, not a subshell running pg_observe:
  # $! must be the sampler so stop_pg_sampler's SIGTERM reaches it. A shell
  # wrapper died on SIGTERM and left the sampler polling forever with the run's
  # stdout open, so the caller's `run.sh | tee` never saw EOF and the step hung
  # after NODE_BOOT_RESULT until the job timeout. Its stdout is not the run's pipe.
  python3 "$GAME_SOURCE/tools/qualification/node_boot/pg_observe.py" sample --out "$PG_OBS/activity.log" \
    --container "$PG_CONTAINER" --user oteryn_node_boot_admin --database oteryn_node_boot --other-database "${PVP_DATABASE:-}" \
    < /dev/null > /dev/null 2>> "$PG_OBS/sampler.err" &
  PG_SAMPLER_PID=$!
}
stop_pg_sampler() {
  [[ -z "${PG_SAMPLER_PID:-}" ]] || {
    kill -TERM "$PG_SAMPLER_PID" 2>/dev/null || true
    local limit=$((SECONDS + 12))
    while kill -0 "$PG_SAMPLER_PID" 2>/dev/null && (( SECONDS < limit )); do sleep 0.1; done
    kill -KILL "$PG_SAMPLER_PID" 2>/dev/null || true
    wait "$PG_SAMPLER_PID" 2>/dev/null || true
    PG_SAMPLER_PID=""
  }
}
pg_observability_dump() { # stage exit: statements, lock waits and slow statements of the stage
  [[ -n "${PG_SAMPLER_PID:-}" ]] || return 0
  local verbose=()
  [[ $2 == 0 ]] || verbose=(--verbose)
  pg_observe dump --stage "$1" --dir "$PG_OBS" --activity "$PG_OBS/activity.log" "${verbose[@]}" || true
}
save_reuse_state() { # the seam stage's inputs, for NODE_BOOT_REUSE_STACK=1
  local names=(WORK node_log failed_node_log WP5_PKI WP5_SCRATCH WP5_PORT WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD
    WP5_APP_KEY COMPOSE_OWNER PG_PORT PG_ADMIN_PASSWORD CONTROL_PASSWORD RUNTIME_PASSWORD ADMIN_URL FRESH_SEED_HEX
    RECOVERY_SEED_HEX MAP_REVISION NODE_PID NODE_PROCESS_DIR PG_CONTAINER PG_CONTAINER_ID NODE_CLOCK_FEATURES NODE_SPAWN_FEATURES NODE_GAMEPLAY_ENV PG_OBS)
  [[ -z "${OTERYN_QUALIFICATION_CLOCK_FILE:-}" ]] || names+=(OTERYN_QUALIFICATION_CLOCK_FILE)
  local pending identity
  identity=$(stat -c '%d:%i' -- "$BASE")
  pending=$(mktemp "$REUSE_STATE.XXXXXX")
  if ! {
    declare -p "${names[@]}" &&
    printf 'kept_base=%q\nkept_service_user=%q\nkept_project=%q\nkept_lifecycle=%q\n' \
      "$BASE" "$SERVICE_USER" "$WP5_PROJECT" "${NODE_BOOT_LIFECYCLE:-}" &&
    printf 'kept_base_identity=%q\nkept_boot_id=%q\n' "$identity" "$(</proc/sys/kernel/random/boot_id)" &&
    printf 'kept_pvp_port=%q\n' "$PVP_GAME_PORT" &&
    printf 'kept_mode=%q\n' "$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/${NODE_BOOT_GAME_PORT:-17181}"
  } > "$pending"; then
    rm -f -- "$pending"
    return 1
  fi
  REUSE_STATE_OWNED=1
  if ! mv -f -- "$pending" "$REUSE_STATE"; then
    rm -f -- "$pending"
    return 1
  fi
  STACK_KEPT=1
}
interrupted() {
  trap '' INT TERM
  local worker=${STAGE_PID:-} timer=${STAGE_TIMER_PID:-}
  # A child can receive work before Bash assigns the PID on the next line.
  if [[ -n "${launching:-}" && "${!:-}" != "$previous_pid" ]]; then
    case "$launching" in
      worker) worker=$! ;;
      timer) timer=$! ;;
      portal) GP_HARNESS_PID=$! ;;
    esac
  fi
  [[ -z "$timer" ]] || kill -KILL -- "-$timer" 2>/dev/null || true
  if [[ -n "$worker" ]]; then
    if [[ ${drain_postgres_create:-0} == 1 ]]; then
      # The daemon can register the container after the client disconnects.
      # Drain the client; cleanup reconciles a create that may still be pending.
      wait "$worker" || true
    else
      kill -TERM -- "-$worker" 2>/dev/null || true
      sleep 0.2
      kill -KILL -- "-$worker" 2>/dev/null || true
    fi
  fi
  result=NODE_BOOT_FAIL
  evidence "interrupted signal=$1 stage=$ACTIVE_STAGE world_bundle=$NODE_BOOT_WORLD"
  exit "$2"
}
run_interruptible() {
  if [[ ${IN_STAGE_WORKER:-0} == 1 ]]; then
    "$@"
    return
  fi
  local launching=worker previous_pid=${!:-}
  set -m
  (
    set +m
    # GNU timeout must lead the same process group the entry shell tracks.
    if [[ "$1" == timeout && $(type -t timeout) == file ]]; then
      exec "$@"
    fi
    "$@"
  ) <&0 &
  STAGE_PID=$!
  launching=""
  set +m
  local rc=0
  wait "$STAGE_PID" || rc=$?
  if [[ $rc == 124 || $rc == 137 ]]; then
    kill -KILL -- "-$STAGE_PID" 2>/dev/null || true
  fi
  STAGE_PID=""
  return "$rc"
}
create_owned_postgres() {
  local drain_postgres_create=1 name=$1 rc=0
  shift
  # Record intent before submitting the request. Any nonzero client exit can
  # leave an accepted request in flight, even without a container ID.
  local record="$POSTGRES_JOURNAL/$name.$COMPOSE_OWNER"
  mkdir -p -- "$POSTGRES_JOURNAL" && : > "$record" || return
  POSTGRES_CREATE_PENDING=$name
  run_interruptible docker create --name "$name" "$@" || rc=$?
  if [[ $rc == 0 ]]; then
    rm -f -- "$record" || return
    POSTGRES_CREATE_PENDING=""
  fi
  return "$rc"
}
acquire_base() {
  timeout --foreground --kill-after=2s 10s sudo mkdir -- "$BASE_PENDING" || return
  stat -c '%d:%i' -- "$BASE_PENDING" > "$WORK/base.identity" || return
  # The same-filesystem rename publishes the already recorded inode.
  timeout --foreground --kill-after=2s 10s sudo mv -nT -- "$BASE_PENDING" "$BASE" || return
  [[ "$(stat -c '%d:%i' -- "$BASE")" == "$(<"$WORK/base.identity")" ]]
}
cleanup_owned_postgres() {
  local name=$1 id=$2 record="" lookup_failed=0
  if [[ -n "${POSTGRES_JOURNAL:-}" && "${COMPOSE_OWNER:-}" =~ ^[a-f0-9]{32}$ ]]; then
    record="$POSTGRES_JOURNAL/$name.$COMPOSE_OWNER"
  fi
  if [[ -z "$id" && "${COMPOSE_OWNER:-}" =~ ^[a-f0-9]{32}$ ]]; then
    local limit=5 deadline=$((SECONDS + 30))
    [[ -z "${POSTGRES_RECOVERY_DEADLINE:-}" ]] || deadline=$POSTGRES_RECOVERY_DEADLINE
    while :; do
      if [[ "${POSTGRES_CREATE_PENDING:-}" == "$name" || -n "${POSTGRES_RECOVERY_DEADLINE:-}" ]]; then
        limit=$((deadline - SECONDS))
        (( limit > 0 )) || break
        (( limit <= 5 )) || limit=5
      fi
      lookup_failed=0
      id=$(timeout --foreground --signal=KILL "${limit}s" docker container ls --all --no-trunc -q \
        --filter "name=^/$name$" --filter "label=org.oteryn.node-boot.owner=$COMPOSE_OWNER") || {
          id=""
          lookup_failed=1
        }
      [[ -z "$id" && "${POSTGRES_CREATE_PENDING:-}" == "$name" ]] || break
      (( SECONDS < deadline )) || break
      sleep 0.25
    done
  fi
  if [[ -n "$id" ]]; then
    local remove_seconds=45
    if [[ -n "${POSTGRES_RECOVERY_DEADLINE:-}" ]]; then
      remove_seconds=$((POSTGRES_RECOVERY_DEADLINE - SECONDS))
      (( remove_seconds <= 45 )) || remove_seconds=45
    fi
    if (( remove_seconds > 0 )) &&
      timeout --foreground --signal=KILL "${remove_seconds}s" docker rm --force --volumes "$id" >/dev/null 2>&1; then
      [[ -z "$record" ]] || rm -f -- "$record" || POSTGRES_CLEANUP_FAILED=1
      return 0
    fi
    if [[ -n "$record" ]]; then
      mkdir -p -- "$POSTGRES_JOURNAL" && : > "$record" || \
        evidence "postgres_create_cleanup=journal_failed container=$name"
    fi
    evidence "postgres_create_cleanup=remove_failed container=$name"
  elif [[ -n "$record" && -f "$record" ]]; then
    evidence "postgres_create_cleanup=pending container=$name lookup_failed=$lookup_failed"
    if [[ -n "${POSTGRES_RECOVERY_DEADLINE:-}" && $lookup_failed == 0 ]] &&
      (( SECONDS < POSTGRES_RECOVERY_DEADLINE )); then
      return 0
    fi
  else
    return 0
  fi
  POSTGRES_CLEANUP_FAILED=1
}
recover_project_postgres() {
  local project=$1 POSTGRES_JOURNAL=$2 COMPOSE_OWNER name owner inventory limit
  limit=$((POSTGRES_RECOVERY_DEADLINE - SECONDS))
  if (( limit <= 0 )); then
    evidence "postgres_recovery=deadline operation=inventory project=$project"
    return 1
  fi
  (( limit <= 5 )) || limit=5
  # Take the snapshot under the project lock, after any earlier project work.
  inventory=$(timeout --foreground --signal=KILL "${limit}s" docker container ls --all \
    --filter label=org.oteryn.node-boot.owner \
    --filter "name=^/($project-postgres|$project-portal-postgres|$project-[a-f0-9]{32}-(postgres|portal-postgres))$" \
    --format '{{.Names}} {{.Label "org.oteryn.node-boot.owner"}}') || {
      evidence "postgres_recovery=inventory_failed project=$project"
      return 1
    }
  while read -r name owner; do
    [[ "$owner" =~ ^[a-f0-9]{32}$ ]] || continue
    case "$name" in
      "$project-postgres"|"$project-portal-postgres"|"$project-$owner-postgres"|"$project-$owner-portal-postgres") ;;
      *) continue ;;
    esac
    [[ -f "$POSTGRES_JOURNAL/$name.$owner" ]] || continue
    COMPOSE_OWNER=$owner
    cleanup_owned_postgres "$name" ""
    [[ $POSTGRES_CLEANUP_FAILED == 0 ]] || return 1
  done <<< "$inventory"
  rmdir -- "$POSTGRES_JOURNAL" 2>/dev/null || true
}
import_legacy_postgres_project() {
  local project=$1 journal=$2 record marker records limit scan_rc=0
  limit=$((POSTGRES_RECOVERY_DEADLINE - SECONDS))
  if (( limit <= 0 )); then
    evidence "postgres_recovery=deadline operation=legacy_scan project=$project"
    return 1
  fi
  records=$(mktemp "$WORK/legacy-postgres.XXXXXX") || return 1
  # Collect a complete scan before importing markers. The parent holds the
  # project locks, so the bounded scanner does not need to inherit them.
  timeout --foreground --signal=KILL "${limit}s" find "$journal" -mindepth 1 -maxdepth 1 -type f \
    \( -name "$project-postgres.*" -o -name "$project-portal-postgres.*" \) -print0 \
    > "$records" 7>&- 8>&- 9>&- || scan_rc=$?
  if (( scan_rc != 0 )); then
    rm -f -- "$records"
    evidence "postgres_recovery=legacy_scan_failed project=$project exit=$scan_rc"
    return 1
  fi
  if (( SECONDS >= POSTGRES_RECOVERY_DEADLINE )); then
    rm -f -- "$records"
    evidence "postgres_recovery=deadline operation=legacy_scan project=$project"
    return 1
  fi
  while IFS= read -r -d '' record && (( SECONDS < POSTGRES_RECOVERY_DEADLINE )); do
    marker=${record##*/}
    [[ "${marker##*.}" =~ ^[a-f0-9]{32}$ ]] || continue
    # Publish in the new journal before removing the old marker.
    mkdir -p -- "$POSTGRES_JOURNALS/$project" &&
      : > "$POSTGRES_JOURNALS/$project/$marker" &&
      rm -f -- "$record" || {
        evidence "postgres_recovery=legacy_import_failed project=$project"
        return 1
      }
  done < "$records"
  rm -f -- "$records"
  if (( SECONDS >= POSTGRES_RECOVERY_DEADLINE )); then
    evidence "postgres_recovery=deadline operation=legacy_import project=$project"
    return 1
  fi
  rmdir -- "$journal" 2>/dev/null || true
}
recover_legacy_postgres() {
  local temp legacy journal project
  # For an upgrade that also changes a custom scratch location, point
  # NODE_BOOT_LEGACY_TMPDIR at that old TMPDIR for the first recovery.
  for temp in "${TMPDIR:-/tmp}" /tmp "${NODE_BOOT_LEGACY_TMPDIR:-}"; do
    [[ -n "$temp" ]] || continue
    legacy="$temp/node-boot-locks-$(id -u)"
    [[ -d "$legacy" && ! -L "$legacy" && -O "$legacy" &&
       "$(stat -c %a -- "$legacy")" == 700 ]] || continue
    for journal in "$legacy/postgres/"*; do
      [[ -d "$journal" && ! -L "$journal" ]] || continue
      project=${journal##*/}
      # The old layout reserved postgres for the journal and could not
      # acquire a project lock with that name.
      [[ "$project" =~ ^[a-z0-9][a-z0-9_-]*$ && "$project" != postgres ]] || continue
      if (( SECONDS >= POSTGRES_RECOVERY_DEADLINE )); then
        evidence "postgres_recovery=deadline operation=legacy_scan project=$project"
        return 1
      fi
      if [[ "$project" == "$WP5_PROJECT" ]]; then
        (
          if ! flock -n 7; then
            evidence "postgres_recovery=legacy_project_in_use project=$project"
            return 1
          fi
          import_legacy_postgres_project "$project" "$journal"
        ) 7>>"$legacy/$project" || return 1
      else
        (
          if flock -n 7 && flock -n 8; then
            import_legacy_postgres_project "$project" "$journal"
          fi
        ) 7>>"$legacy/$project" 8>>"$lock_directory/$project" || return 1
      fi
    done
  done
}
recover_owned_postgres() {
  local journal project name inventory limit probe probe_rc POSTGRES_CLEANUP_FAILED=0
  local POSTGRES_RECOVERY_DEADLINE=$((SECONDS + 60))
  recover_legacy_postgres || return 1
  # Reconcile the requested names before attempting optional foreign discovery.
  journal="$POSTGRES_JOURNALS/$WP5_PROJECT"
  if [[ -d "$journal" ]]; then
    recover_project_postgres "$WP5_PROJECT" "$journal" || return 1
  fi
  local -A candidates=()
  for journal in "$POSTGRES_JOURNALS/"*; do
    [[ -d "$journal" && "${journal##*/}" != "$WP5_PROJECT" ]] || continue
    limit=$((POSTGRES_RECOVERY_DEADLINE - SECONDS))
    if (( limit <= 0 )); then
      evidence "postgres_recovery=deferred operation=foreign_discovery reason=deadline"
      return 0
    fi
    (( limit <= 5 )) || limit=5
    # Discover foreign candidates in one call. Their actual ownership is
    # checked against a freshly queried inventory under each project lock.
    inventory=$(timeout --foreground --signal=KILL "${limit}s" docker container ls --all \
      --filter label=org.oteryn.node-boot.owner \
      --format '{{.Names}}') || {
        evidence "postgres_recovery=deferred operation=foreign_discovery reason=inventory_failed"
        return 0
      }
    while IFS= read -r name; do
      [[ "$name" =~ ^[a-z0-9][a-z0-9_-]*-postgres$ ]] || continue
      candidates["$name"]=1
      if [[ "$name" =~ ^(.+)-[a-f0-9]{32}-(portal-postgres|postgres)$ ]]; then
        candidates["${BASH_REMATCH[1]}-${BASH_REMATCH[2]}"]=1
      fi
    done <<< "$inventory"
    break
  done
  local -a discovered=() misses=()
  for journal in "$POSTGRES_JOURNALS/"*; do
    [[ -d "$journal" ]] || continue
    project=${journal##*/}
    [[ "$project" =~ ^[a-z0-9][a-z0-9_-]*$ && "$project" != "$WP5_PROJECT" ]] || continue
    if [[ -n "${candidates["$project-postgres"]:-}" || -n "${candidates["$project-portal-postgres"]:-}" ]]; then
      discovered+=("$journal")
    else
      misses+=("$journal")
    fi
  done
  # Spend the shared budget on discovered containers before probing misses.
  local owner_glob='????????????????????????????????'
  for journal in "${discovered[@]}" "${misses[@]}"; do
    project=${journal##*/}
    if (( POSTGRES_RECOVERY_DEADLINE - SECONDS < 5 )); then
      evidence "postgres_recovery=deferred operation=foreign_project_scan project=$project"
      break
    fi
    (
      if flock -n 8; then
        # Run-specific journal names require a scoped list. Legacy snapshot
        # misses can use an exact-name probe before that list.
        if [[ -z "${candidates["$project-postgres"]:-}" && -z "${candidates["$project-portal-postgres"]:-}" ]] &&
           ! compgen -G "$journal/$project-$owner_glob-postgres.*" >/dev/null &&
           ! compgen -G "$journal/$project-$owner_glob-portal-postgres.*" >/dev/null; then
          probe_rc=0
          probe=$(timeout --foreground --signal=KILL 2s docker container inspect --format '{{.Name}}' \
            "$project-postgres" "$project-portal-postgres" 2>/dev/null) || probe_rc=$?
          if (( probe_rc == 124 || probe_rc == 137 )); then
            evidence "postgres_recovery=probe_timeout project=$project exit=$probe_rc"
          elif [[ -z "$probe" ]] && (( probe_rc != 0 )); then
            evidence "postgres_recovery=probe_no_result project=$project exit=$probe_rc"
          fi
          [[ -n "$probe" ]] || exit 0
        fi
        recover_project_postgres "$project" "$journal"
      fi
    ) 8>>"$lock_directory/$project" || return 1
  done
}
cleanup_owned_compose() {
  [[ "${COMPOSE_OWNER:-}" =~ ^[a-f0-9]{32}$ ]] || return 0
  local resource id ids
  for resource in container network volume; do
    local args=()
    case "$resource" in
      container) args=(--all --no-trunc) ;;
      network) args=(--no-trunc) ;;
    esac
    ids=$(timeout --foreground --kill-after=2s 10s docker "$resource" ls "${args[@]}" -q \
      --filter "label=com.docker.compose.project=$WP5_PROJECT-$COMPOSE_OWNER" \
      --filter "label=org.oteryn.node-boot.owner=$COMPOSE_OWNER") || continue
    for id in $ids; do
      local rm_args=()
      [[ "$resource" != container ]] || rm_args=(--force --volumes)
      if [[ "$resource" == volume ]]; then
        [[ "$(timeout 5s docker volume inspect --format '{{index .Labels "org.oteryn.node-boot.owner"}}' "$id")" == "$COMPOSE_OWNER" ]] || continue
      fi
      timeout --foreground --kill-after=2s 15s docker "$resource" rm "${rm_args[@]}" "$id" >/dev/null 2>&1 || true
    done
  done
}
cleanup() {
  local rc=$?
  trap - EXIT INT TERM
  set +e
  # Flush the independent reader before either the server or its private log is removed.
  stop_runtime_diagnostics
  stop_pg_sampler
  [[ -z "${WORLD_CAPTURE_DIR:-}" ]] || rm -rf -- "$WORLD_CAPTURE_DIR"
  if [[ ${STACK_KEPT:-0} == 1 ]]; then
    evidence "stack=kept project=$WP5_PROJECT rerun=NODE_BOOT_REUSE_STACK=1 remove=NODE_BOOT_REUSE_STACK=teardown"
  else
    local process attempt
    for process in "$WORK"/node-process.*; do
      [[ -f "$process/process" && ! -f "$process/exit" ]] || continue
      signal_node_process TERM "$process" 2>/dev/null || continue
      for attempt in {1..20}; do
        [[ ! -f "$process/exit" ]] || break
        sleep 0.1
      done
      [[ -f "$process/exit" ]] || signal_node_process KILL "$process" 2>/dev/null || true
    done
    if [[ ${COMPOSE_OWNED:-0} == 1 ]]; then
      if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
        game_platform_cleanup
      else
        cleanup_owned_compose
      fi
    fi
    if [[ -z "${PG_CONTAINER_ID:-}" && -s "$WORK/game-postgres.cid" ]]; then
      PG_CONTAINER_ID="$(<"$WORK/game-postgres.cid")"
    fi
    if [[ -n "${PG_CONTAINER_ID:-}" || -n "${COMPOSE_OWNER:-}" ]]; then
      cleanup_owned_postgres "$PG_CONTAINER" "${PG_CONTAINER_ID:-}"
    fi
    if [[ -n "${BASE_PENDING:-}" ]]; then
      timeout --foreground --kill-after=2s 10s sudo rmdir -- "$BASE_PENDING" 2>/dev/null || true
    fi
    if [[ -z "${BASE_IDENTITY:-}" && -s "$WORK/base.identity" ]]; then
      BASE_IDENTITY="$(<"$WORK/base.identity")"
    fi
    if [[ -n "${BASE_IDENTITY:-}" && "$(stat -c '%d:%i' -- "$BASE" 2>/dev/null)" == "$BASE_IDENTITY" ]]; then
      timeout --foreground --kill-after=5s 60s sudo rm -rf -- "$BASE" || true
    fi
    [[ ${REUSE_STATE_OWNED:-0} != 1 ]] || rm -f -- "$REUSE_STATE"
  fi
  if [[ ${POSTGRES_CLEANUP_FAILED:-0} == 1 ]]; then
    result=NODE_BOOT_FAIL
    (( rc != 0 )) || rc=1
  fi
  if [[ $rc != 0 && -n "${NODE_BOOT_FAILURE_LOG_DIR:-}" ]]; then
    mkdir -p "$NODE_BOOT_FAILURE_LOG_DIR"
    if [[ -f "$failed_node_log" ]]; then
      cp "$failed_node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    elif [[ "$ACTIVE_STAGE" == pvp_* && -f "$WORK/pvp-node.log" ]]; then
      cp "$WORK/pvp-node.log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    elif [[ -f "$node_log" ]]; then
      cp "$node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    fi
    local snapshot
    for snapshot in "$WORK"/failed-stage-*.log; do
      [[ -f "$snapshot" ]] || continue
      cp "$snapshot" "$NODE_BOOT_FAILURE_LOG_DIR/"
      # The upload caller retains node.log. Include every failed incarnation there.
      if ! cmp -s "$snapshot" "$failed_node_log"; then
        printf '\n--- %s ---\n' "${snapshot##*/}" >> "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
        cat "$snapshot" >> "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
      fi
    done
    if [[ -f "$WORK/pvp-node.log" ]]; then
      cp "$WORK/pvp-node.log" "$NODE_BOOT_FAILURE_LOG_DIR/"
    fi
    # The basic and World passes share the directory; each keeps its own PostgreSQL record.
    [[ ! -d "${PG_OBS:-}" ]] || cp -R "$PG_OBS" "$NODE_BOOT_FAILURE_LOG_DIR/postgres-world$NODE_BOOT_WORLD-spells$NODE_BOOT_SPELLS"
  fi
  if [[ ${STACK_KEPT:-0} != 1 ]]; then
    find "$WORK" -type f -delete 2>/dev/null || true
    find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  fi
  [[ -z "${POSTGRES_JOURNAL:-}" ]] || rmdir -- "$POSTGRES_JOURNAL" 2>/dev/null || true
  if [[ ${POSTGRES_CLEANUP_FAILED:-0} == 1 ]]; then
    evidence "cleanup=incomplete"
  else
    evidence "cleanup=complete"
  fi
  evidence "result_scope world_bundle=$NODE_BOOT_WORLD spells=$NODE_BOOT_SPELLS stage=$ACTIVE_STAGE exit=$rc"
  echo "NODE_BOOT_RESULT=$result"
  exit "$rc"
}
trap cleanup EXIT
trap 'interrupted INT 130' INT
trap 'interrupted TERM 143' TERM
if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  if ! run_interruptible recover_owned_postgres; then
    POSTGRES_CLEANUP_FAILED=1
    exit 1
  fi
fi
if [[ "$NODE_BOOT_REUSE_STACK" == teardown ]]; then
  result=NODE_BOOT_TEARDOWN
  exit 0
fi
if [[ "$NODE_BOOT_SPELLS" == 1 && "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  # Execute the existing compile/decode/stage qualification before creating services.
  RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}" \
    run_interruptible timeout --kill-after=5s 3600s bash "$GAME_SOURCE/tools/qualification/spells/run.sh" map "$NODE_BOOT_SPELL_MANIFEST"
fi

# A reused stack keeps its provisioning, characters and node: only the seam stage runs again.
if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then

make_ca() {
  openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
    -subj "/CN=$2" -keyout "$WP5_PKI/$1.key" -out "$WP5_PKI/$1.crt" >/dev/null 2>&1
}
make_leaf() {
  local prefix=$1 subject=$2 ca=$3 usage=$4 san=${5:-}
  openssl req -new -newkey rsa:2048 -nodes -sha256 -subj "/CN=$subject" \
    -keyout "$WP5_PKI/$prefix.key" -out "$WP5_PKI/$prefix.csr" >/dev/null 2>&1
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=%s\n' "$usage" > "$WP5_PKI/$prefix.ext"
  [[ -z "$san" ]] || printf 'subjectAltName=%s\n' "$san" >> "$WP5_PKI/$prefix.ext"
  openssl x509 -req -sha256 -days 1 -in "$WP5_PKI/$prefix.csr" -CA "$WP5_PKI/$ca.crt" \
    -CAkey "$WP5_PKI/$ca.key" -CAcreateserial -extfile "$WP5_PKI/$prefix.ext" \
    -out "$WP5_PKI/$prefix.crt" >/dev/null 2>&1
}
make_ca server-ca node-boot-server-ca
make_ca client-ca node-boot-client-ca
make_ca db-ca node-boot-db-ca
make_leaf server source.test server-ca serverAuth DNS:source.test,IP:127.0.0.1
make_leaf client oteryn-game-native-evidence client-ca clientAuth
make_leaf premium oteryn-game-premium-fixture client-ca clientAuth
make_leaf db db.node-boot.test db-ca serverAuth DNS:db.node-boot.test
# The nginx topology expects the S3-B intent client files; the node uses the
# single evidence identity for both routes.
cp "$WP5_PKI/client.crt" "$WP5_PKI/intent-client.crt"
cp "$WP5_PKI/client.key" "$WP5_PKI/intent-client.key"
# An end-entity certificate: TLS rejects a CA certificate used as a server leaf.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 1 \
  -subj "/CN=localhost" -addext "subjectAltName=DNS:localhost" \
  -addext "basicConstraints=critical,CA:FALSE" -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=serverAuth" \
  -keyout "$WP5_PKI/gameplay.key" -out "$WP5_PKI/gameplay.crt" >/dev/null 2>&1
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/fresh-signing.pem" >/dev/null 2>&1
raw_hex() { tail -c 32 | od -An -v -tx1 | tr -d ' \n'; }
FRESH_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -pubout -outform DER | raw_hex)"
FRESH_SEED_HEX="$(openssl pkey -in "$WP5_PKI/fresh-signing.pem" -outform DER | raw_hex)"
# Separate ephemeral Recovery key (#822 resume); fixed issuer/profile/purpose is not Fresh.
openssl genpkey -algorithm ed25519 -out "$WP5_PKI/recovery-signing.pem" >/dev/null 2>&1
RECOVERY_PUBLIC_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -pubout -outform DER | raw_hex)"
RECOVERY_SEED_HEX="$(openssl pkey -in "$WP5_PKI/recovery-signing.pem" -outform DER | raw_hex)"
[[ ${#RECOVERY_PUBLIC_HEX} == 64 && ${#RECOVERY_SEED_HEX} == 64 ]]
evidence "pins platform=$NODE_BOOT_PLATFORM_PIN topology=$NODE_BOOT_TOPOLOGY game=$(git rev-parse HEAD)"

# TLS-verified PostgreSQL 18.6 for the Game durability root. It always logs lock waits
# (after 200 ms), deadlocks and statements of 500 ms or more without parameter values,
# and loads pg_stat_statements, so every run carries its own database evidence.
chmod 644 "$WP5_PKI/db.crt" "$WP5_PKI/db.key"
create_owned_postgres "$PG_CONTAINER" --label "org.oteryn.node-boot.owner=$COMPOSE_OWNER" --cidfile "$WORK/game-postgres.cid" --publish "127.0.0.1:$PG_PORT:5432" \
  --env POSTGRES_USER=oteryn_node_boot_admin --env POSTGRES_PASSWORD="$PG_ADMIN_PASSWORD" \
  --env POSTGRES_DB=postgres --volume "$WP5_PKI/db.crt:/tls/db.crt:ro" \
  --volume "$WP5_PKI/db.key:/tls/db.key:ro" --entrypoint bash "$PG_IMAGE" -c \
  'install -o postgres -m 0600 /tls/db.key /tmp/db.key && install -o postgres -m 0644 /tls/db.crt /tmp/db.crt && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/db.crt -c ssl_key_file=/tmp/db.key -c log_lock_waits=on -c deadlock_timeout=200ms -c log_min_duration_statement=500 -c log_parameter_max_length=0 -c log_parameter_max_length_on_error=0 -c shared_preload_libraries=pg_stat_statements -c "log_line_prefix=%m [%p] %u "'
PG_CONTAINER_ID="$(<"$WORK/game-postgres.cid")"
BASE_PENDING="$(dirname -- "$BASE")/.oteryn-node-boot-$COMPOSE_OWNER"
if ! run_interruptible acquire_base; then
  fail "base acquisition failed: $BASE"
fi
BASE_PENDING=""
run_interruptible docker start "$PG_CONTAINER_ID" >/dev/null
chmod 600 "$WP5_PKI/db.key"
# Probe over TCP: the entrypoint's temporary init server listens on the unix socket only
# and is shut down after init, so a socket probe can pass before the real server is up.
pg_ready=0
for _ in $(seq 1 60); do
  run_interruptible timeout --foreground --kill-after=1s 4s docker exec "$PG_CONTAINER" pg_isready -h 127.0.0.1 -p 5432 -U oteryn_node_boot_admin -d postgres >/dev/null 2>&1 && { pg_ready=1; break; }
  sleep 1
done
[[ $pg_ready == 1 ]] || fail "postgres_ready"
run_interruptible psql_admin postgres <<< "CREATE DATABASE oteryn_node_boot"
# The view lives in the maintenance database: the Game schema stays exactly as migrated.
run_interruptible psql_admin postgres <<< "CREATE EXTENSION pg_stat_statements"
run_interruptible psql_admin oteryn_node_boot <<< "SHOW server_version_num" > "$WORK/pg-version"
[[ "$(<"$WORK/pg-version")" == 180006 ]] || fail "postgres_version"
run_interruptible psql_admin oteryn_node_boot <<< "SHOW ssl" > "$WORK/pg-ssl"
[[ "$(<"$WORK/pg-ssl")" == on ]] || fail "postgres_tls"
ADMIN_URL="postgresql://oteryn_node_boot_admin:$PG_ADMIN_PASSWORD@127.0.0.1:$PG_PORT/oteryn_node_boot"

# Each Compose object carries this run's owner token from creation.
COMPOSE_OWNED=1
if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
  game_platform_start
else
run_interruptible compose config --quiet
run_interruptible compose build --pull platform
run_interruptible compose up --detach --wait db platform nginx
# Platform output is kept: a failing operator command must be diagnosable.
php_exec() { compose exec --no-TTY --user www-data platform php -r "$1"; }
run_interruptible php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"$ACCOUNT_ID"'","nb-one@example.invalid"],["'"$SECOND_ACCOUNT_ID"'","nb-two@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); } app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey(App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER,App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE,"fresh_admission","'"$FRESH_KEY_ID"'",hex2bin("'"$FRESH_PUBLIC_HEX"'")); app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class)->publishTrustedKey("urn:oteryn:platform:game-recovery","oteryn-reauth-recovery-v1","existing_actor_recovery","'"$RECOVERY_KEY_ID"'",hex2bin("'"$RECOVERY_PUBLIC_HEX"'"));'
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  for account in "${BOOTSTRAP_ACCOUNTS[@]:2}"; do
    run_interruptible php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $accountId="'"$account"'"; Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>"nb-".$accountId."@example.invalid","password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]);'
  done
  # Only the disposable gameplay packet accounts receive Premium. Use the
  # same audited Platform owner as the operator's Premium grant route.
  run_interruptible php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $operator=App\Identity\Models\Identity::where("account_id","'"$ACCOUNT_ID"'")->firstOrFail(); $ledger=app(App\ProductsEntitlements\Premium\PremiumTimeLedger::class); foreach (["'"$PACKET_ACCOUNT_ID"'","'"$SECOND_PACKET_ACCOUNT_ID"'"] as $account) { $target=App\Identity\Models\Identity::where("account_id",$account)->firstOrFail(); $ledger->grant($operator,$target,1,"Disposable node-boot gameplay qualification",(string) Illuminate\Support\Str::uuid()); }'
fi
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  for account in "${EXTRA_WORLD_ACCOUNTS[@]}"; do
    php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>"nb-'"$account"'@example.invalid","password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>"'"$account"'","native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]);'
  done
fi
evidence "platform_seed=synthetic_accounts=${#BOOTSTRAP_ACCOUNTS[@]} fresh_trust=published_public_key_only client_identities=1"
fi # Platform lifecycle

# The accelerated build is disposable: production binaries omit its file input.
# Real-clock qualification deliberately builds the ordinary shipping binary.
NODE_CLOCK_FEATURES=()
# The root-only `spawn` control command needs the disposable qualification-spawn build.
NODE_SPAWN_FEATURES=(--features qualification-spawn)
export OTERYN_NODE_BOOT_REAL_CLOCK="${OTERYN_NODE_BOOT_REAL_CLOCK:-0}"
case "$OTERYN_NODE_BOOT_REAL_CLOCK" in 0|1) ;; *) fail invalid_real_clock_mode ;; esac
if [[ "$NODE_BOOT_WORLD" == 1 && "$OTERYN_NODE_BOOT_REAL_CLOCK" == 0 ]]; then
  NODE_CLOCK_FEATURES=(--features qualification-clock)
fi
# Shipped binaries and the SEAM client harness.
run_interruptible timeout --kill-after=5s 3600s cargo +1.100.0 build --locked -p oteryn-game-server --bins "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}"
run_interruptible timeout --kill-after=5s 3600s cargo +1.100.0 test --locked -p oteryn-game-server --lib --no-run "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}"
run_interruptible timeout --kill-after=2s 120s cargo +1.100.0 metadata --locked --no-deps --format-version 1 > "$WORK/cargo-metadata.json"
TARGET="$(python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])' < "$WORK/cargo-metadata.json")/debug"

# Deployment administration: service user, directories, roles, migration.
run_interruptible timeout --foreground --kill-after=2s 120s sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null || true
SERVICE_UID="$(id -u "$SERVICE_USER")"
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o root -g root -m 0755 "$BASE" "$BASE/bin" "$BASE/state" "$BASE/fence-parent" "$BASE/node"
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o root -g root -m 0700 "$BASE/ops"
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Stage only declared, hash-bound inputs; never publish unrelated siblings.
  run_interruptible timeout --foreground --kill-after=2s 300s python3 - "$NODE_BOOT_SPELL_MANIFEST" "$GAME_SOURCE" "$WORK/gameplay" <<'PY_STAGE'
import hashlib
import json
import pathlib
import sys

manifest = pathlib.Path(sys.argv[1]).resolve(strict=True)
repo_root = pathlib.Path(sys.argv[2]).resolve(strict=True)
staged = pathlib.Path(sys.argv[3])
fields = {
    "catalog", "source_selection", "creature_profiles", "presentation_profiles",
    "item_profiles", "spell_appearances", "build_training", "familiar_config",
    "familiar_defenses", "wheel_profile", "source_world", "progression", "item_keys",
    "loot_tables",
    "cosmetics", "starting_loadout_item_profiles", "item_profile_updates",
}
manifest_bytes = manifest.read_bytes()
document = json.loads(manifest_bytes)
# Staged paths are repository-relative, so a locator that leaves the manifest directory
# (the canonical manifest's source_world and wheel_profile) lands where it resolves.
selected = {manifest.relative_to(repo_root).as_posix(): manifest_bytes}
for field in fields:
    pin = document.get(field)
    if pin is None:
        continue
    raw = pin["path"]
    locator = pathlib.PurePosixPath(raw)
    if locator.is_absolute() or "\\" in raw:
        raise ValueError("unsafe manifest locator")
    source = manifest.parent.joinpath(*locator.parts).resolve(strict=True)
    if not source.is_relative_to(repo_root):
        raise ValueError("manifest input escapes repository root")
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != pin["sha256"]:
        raise ValueError("manifest input digest mismatch")
    name = source.relative_to(repo_root).as_posix()
    if name in selected and selected[name] != data:
        raise ValueError("conflicting manifest input")
    selected[name] = data
sys.path.insert(0, str(repo_root / "tools/qualification/spells"))
from provisioning_inputs import select_cosmetics_inputs
select_cosmetics_inputs(document, manifest, repo_root, selected)
staged.mkdir(mode=0o700)
for name, data in selected.items():
    destination = staged / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
PY_STAGE
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o root -g root -m 0755 "$BASE/gameplay"
  run_interruptible timeout --foreground --kill-after=2s 120s sudo cp -R "$WORK/gameplay/." "$BASE/gameplay/"
  run_interruptible timeout --foreground --kill-after=2s 120s sudo chmod -R a+rX "$BASE/gameplay"
  evidence "spell_input=full_manifest manifest_sha256=$(sha256sum "$NODE_BOOT_SPELL_MANIFEST" | cut -d ' ' -f 1) activation_wait_seconds=$NODE_ACTIVATION_WAIT_SECONDS"
fi
MAP_REVISION=map-s3b-1
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # Real Thais/town outskirts and the default-start region for ordinary home respawns.
  # Match the first-entry fixture below to its authored Thais hometown (8).
  # The committed full-world pin defaults to Newhaven (33), on another island.
  run_interruptible timeout --kill-after=5s 3600s cargo +1.100.0 run --locked --release -p oteryn-world-bundle-compiler -- compile-regions "$GAME_SOURCE" "$GAME_SOURCE/content/world/pins/oteryn.identity.json" "$WORK/world.otwb" '126,125;126,126;127,127' 8 > "$WORK/world-compile.json"
  PIN="$WORK/world.json"
  run_interruptible timeout --foreground --kill-after=2s 300s python3 - "$GAME_SOURCE/content/world/pins/oteryn.json" "$WORK/world-compile.json" "$PIN" <<'PY_REGION_PIN'
import json, sys
pin = json.load(open(sys.argv[1]))
pin["schema"] = "OTERYN_QUALIFICATION_WORLD/v1"
pin["digest"] = json.load(open(sys.argv[2]))["digest"]
pin["entry_start"] = {"x": 32369, "y": 32238, "floor": -7}
with open(sys.argv[3], "w") as output:
    json.dump(pin, output, sort_keys=True)
PY_REGION_PIN
  WORLD_DIGEST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["digest"])' "$PIN")"
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o root -g root -m 0755 "$BASE/world"
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o root -g root -m 0644 "$WORK/world.otwb" "$BASE/world/world.otbundle"
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o root -g root -m 0644 "$PIN" "$BASE/world/world.json"
  MAP_REVISION="$WORLD_DIGEST"
fi
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o "$SERVICE_UID" -m 0700 "$BASE/fence-parent/fence" "$BASE/run"
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o "$SERVICE_UID" -m 0755 "$BASE/node/secrets"
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o root -m 0755 "$TARGET/oteryn-game-server" "$TARGET/oteryn-game-ops" "$TARGET/oteryn-game-migrate" "$BASE/bin/"
run_interruptible env OTERYN_GAME_MIGRATION_DATABASE_URL="$ADMIN_URL" timeout --foreground --kill-after=5s 600s "$BASE/bin/oteryn-game-migrate"
run_interruptible psql_admin oteryn_node_boot <<SQL
CREATE ROLE nb_control LOGIN PASSWORD '$CONTROL_PASSWORD' IN ROLE oteryn_game_control;
CREATE ROLE nb_runtime LOGIN PASSWORD '$RUNTIME_PASSWORD' IN ROLE oteryn_game_runtime;
GRANT CONNECT ON DATABASE oteryn_node_boot TO nb_control, nb_runtime;
SQL
secret_to() { # owner source destination
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o "$1" -m 0600 "$2" "$3"
}
printf '%s\n' "$CONTROL_PASSWORD" > "$WORK/control-password"
printf '%s\n' "$RUNTIME_PASSWORD" > "$WORK/runtime-password"
secret_to root "$WORK/control-password" "$BASE/ops/pg-password"
secret_to root "$WP5_PKI/db-ca.crt" "$BASE/ops/db-ca.pem"
secret_to "$SERVICE_UID" "$WORK/runtime-password" "$BASE/node/secrets/pg-password"
secret_to "$SERVICE_UID" "$WP5_PKI/db-ca.crt" "$BASE/node/secrets/db-ca.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/server-ca.crt" "$BASE/node/secrets/platform-roots.pem"
secret_to "$SERVICE_UID" "$WP5_PKI/client.crt" "$BASE/node/secrets/platform-client.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/client.key" "$BASE/node/secrets/platform-client.key"
if [[ "$NODE_BOOT_WORLD" == 1 && -z "${NODE_BOOT_LIFECYCLE:-}" ]]; then
  cat "$WP5_PKI/premium.crt" "$WP5_PKI/premium.key" > "$WORK/premium-identity.pem"
  secret_to "$SERVICE_UID" "$WORK/premium-identity.pem" "$BASE/node/secrets/premium-identity.pem"
  rm -f "$WORK/premium-identity.pem"
  NODE_GAMEPLAY_ENV+=(
    "OTERYN_PREMIUM_SNAPSHOT_URL=https://127.0.0.1:$WP5_PORT"
    "OTERYN_PREMIUM_CLIENT_IDENTITY_PEM=$BASE/node/secrets/premium-identity.pem"
    "OTERYN_PREMIUM_PLATFORM_CA_PEM=$BASE/node/secrets/platform-roots.pem"
  )
fi
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.crt" "$BASE/node/secrets/gameplay.crt"
secret_to "$SERVICE_UID" "$WP5_PKI/gameplay.key" "$BASE/node/secrets/gameplay.key"

cat > "$WORK/ops.toml" <<TOML
[operator]
state_directory = "$BASE/state"
service_uid = $SERVICE_UID
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.node-boot.test"
database = "oteryn_node_boot"
username = "nb_control"
password_file = "$BASE/ops/pg-password"
root_ca_file = "$BASE/ops/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
TOML
run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o root -m 0600 "$WORK/ops.toml" "$BASE/ops/ops.toml"
# One descriptor installation time: changed facts under an unchanged
# revision are refused at boot, as D3 requires.
DESCRIPTOR_INSTALLED_AT="$(date +%s)"
write_node_config() { # launch-file
  cat > "$WORK/node.toml" <<TOML
qualification_fixture = true
[listener]
address = "127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}"
entry_deadline_ms = 20000
max_connections = 256
max_handshake_units = 64
certificate_chain_file = "$BASE/node/secrets/gameplay.crt"
private_key_file = "$BASE/node/secrets/gameplay.key"
[scope]
world_id = "$WORLD_ID"
channel_id = "$CHANNEL_ID"
assignment_wait_ms = 120000
preproduction_actor_capacity = 131072
[database]
transport_ip = "127.0.0.1"
port = $PG_PORT
tls_server_name = "db.node-boot.test"
database = "oteryn_node_boot"
username = "nb_runtime"
password_file = "$BASE/node/secrets/pg-password"
root_ca_file = "$BASE/node/secrets/db-ca.pem"
[character]
fence_directory = "$BASE/fence-parent/fence"
authority_scope_id = "character-primary"
issuer_identity = "game-ops"
[control]
socket_path = "$BASE/run/control.sock"
[readiness]
source_authority = "oteryn:runtime:node-boot"
route_revision = "route-s3b-1"
runtime_observation_revision = "runtime-1"
ruleset_revision = "rules-s3b-1"
content_revision = "s3b-content-1"
map_revision = "$MAP_REVISION"
world_policy_revision = "policy-s3b-1"
required_gameplay_capabilities = $NODE_REQUIRED_CAPABILITIES
offer_revision = "offer-s3b-1"
[platform]
source_authority = "platform"
endpoint = "127.0.0.1:$WP5_PORT"
peer_name = "source.test"
trust_roots_file = "$BASE/node/secrets/platform-roots.pem"
client_certificate_file = "$BASE/node/secrets/platform-client.crt"
client_key_file = "$BASE/node/secrets/platform-client.key"
descriptor_revision = 1
installed_at = $DESCRIPTOR_INSTALLED_AT
[platform.public_projections]
enabled = false
character_online_sessions_enabled = false
world_daily_kills_enabled = false
source_authority = "oteryn:game:primary"
client_certificate_file = "$BASE/node/secrets/public-projections.crt"
client_key_file = "$BASE/node/secrets/public-projections.key"
epoch_fence_file = "$BASE/public-projections/epoch-fence"
[launch]
authorization_file = "$BASE/state/$1"
s2_authorization_file = "$BASE/state/s2-fresh-store.json"
TOML
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    run_interruptible timeout --foreground --kill-after=2s 300s python3 - "$PIN" "$BASE/world/world.otbundle" >> "$WORK/node.toml" <<'PY_WORLD'
import json, sys
pin = json.load(open(sys.argv[1]))
print(f'''[world_bundle]
path = "{sys.argv[2]}"
digest = "{pin['digest']}"
project_format_version = "{pin['project_format_version']}"
world_schema_version = "{pin['world_schema_version']}"
content_revision = "{pin['content_revision']}"
production = false
start_x = 32369
start_y = 32238
start_floor = -7''')
PY_WORLD
  fi
  if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
    game_platform_config "$WORK/node.toml"
  fi
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -o root -m 0644 "$WORK/node.toml" "$BASE/node/node.toml"
}
# Root/client writes an atomic offset; the service can only read it. Start at
# zero on every disposable run, and keep it across the shutdown/restart probes.
if (( ${#NODE_CLOCK_FEATURES[@]} )); then
  run_interruptible timeout --foreground --kill-after=2s 120s sudo install -d -o "$(id -u)" -m 0755 "$BASE/qualification-clock"
  export OTERYN_QUALIFICATION_CLOCK_FILE="$BASE/qualification-clock/offset-ms"
  printf '0' > "$OTERYN_QUALIFICATION_CLOCK_FILE"
  chmod 0644 "$OTERYN_QUALIFICATION_CLOCK_FILE"
  NODE_GAMEPLAY_ENV+=("OTERYN_QUALIFICATION_CLOCK_FILE=$OTERYN_QUALIFICATION_CLOCK_FILE")
fi
fi # NODE_BOOT_REUSE_STACK == 0
start_pg_sampler

ops() { sudo timeout --foreground --kill-after=5s 600s env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-ops" --config "$BASE/ops/ops.toml" "$@"; }
start_node_process() { # log, configuration, then hand the supervisor directory back to the caller
  local log=$1 config=$2
  NODE_PROCESS_DIR=$(mktemp -d "$WORK/node-process.XXXXXX")
  # The supervisor owns the wait across stage subshells. The service account
  # reports the PID that execs serve, so neither TERM nor KILL depends on sudo
  # forwarding signals or on polling a sudo monitor's lifetime.
  setsid bash -c '
    directory=$1; shift
    "$@" > "$directory/process"
    printf "%s\n" "$?" > "$directory/exit.tmp"
    mv -- "$directory/exit.tmp" "$directory/exit"
  ' node-supervisor "$NODE_PROCESS_DIR" sudo -u "$SERVICE_USER" env "${NODE_GAMEPLAY_ENV[@]}" OTERYN_NODE_BOOT_WITNESSES=1 bash -c '
    IFS= read -r stat < "/proc/$$/stat" || exit 1
    read -r -a fields <<< "${stat##*) }"
    printf "%s %s\n" "$$" "${fields[19]}"
    exec "$@"
  ' node-exec "$BASE/bin/oteryn-game-server" serve --config "$config" \
    < /dev/null > /dev/null 2> "$log" 9>&- &
  local attempt started
  for attempt in {1..100}; do
    if [[ -s "$NODE_PROCESS_DIR/process" ]] && IFS=$' \t\r' read -r NODE_PID started < "$NODE_PROCESS_DIR/process" \
      && [[ "$NODE_PID" =~ ^[1-9][0-9]*$ && "$started" =~ ^[0-9]+$ ]]; then
      return 0
    fi
    [[ ! -s "$NODE_PROCESS_DIR/exit" ]] || break
    sleep 0.05
  done
  fail "node process identity unavailable"
}
start_node() {
  start_node_process "$node_log" "$BASE/node/node.toml"
  remember NODE_PROCESS_DIR "$NODE_PROCESS_DIR"
}
stop_node() { # supervisor directory
  local directory=$1 code attempt
  if [[ ! -s "$directory/exit" ]]; then
    signal_node_process TERM "$directory" || true
  fi
  for attempt in {1..30}; do
    if [[ -s "$directory/exit" ]]; then
      read -r code < "$directory/exit"
      [[ "$code" == 0 ]] || { fail "node shutdown exit=$code"; return 1; }
      return
    fi
    sleep 1
  done
  signal_node_process KILL "$directory" || true
  fail "node shutdown deadline exceeded"
}
await_log() { # pattern seconds
  for _ in $(seq 1 "$2"); do grep -q "$1" "$node_log" && return 0; sleep 1; done
  fail "missing node event: $1"
}

# Every stage runs to a recorded outcome, so one run reports every failing
# stage instead of stopping at the first one. A stage whose prerequisite did
# not pass is recorded as SKIP. State shared between stages lives in $VARS.
VARS="$WORK/vars"
: > "$VARS"
declare -A STATUS=()
ORDER=()
remember() { printf '%s=%q\n' "$1" "$2" >> "$VARS"; }
durability_report_lines() {
  # Keep the report's legacy selectors while preserving the raw node log.
  sed -E \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=holder_window /DURABILITY_HOLDER_WINDOW /' \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=holder_usage /DURABILITY_HOLDER_USAGE /' \
    -e 's/^oteryn-game-server ts=[0-9]+ level=(error|warn|info|debug) module=durability event=pass_dropped /DURABILITY_PASS_DROPPED /'
}
report_character_authority_failures() {
  # Startup runs outside the seam monitor. Export only the closed check names
  # from the current log snapshot, never identifiers, SQL, or stored payloads.
  python3 - "$node_log" "$1" "$NODE_BOOT_WORLD" <<'PY_CHARACTER_AUTHORITY'
import os, re, sys

path, stage, world = sys.argv[1:]
checks = {'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
          'bazaar', 'revision_chain', 'appearance', 'build_chain',
          'proficiency_history', 'familiar_chain', 'spell_familiar_group',
          'spell_item_chain', 'audit_payload'}
reported = set()
try:
    with open(path, 'rb') as stream:
        remaining = os.fstat(stream.fileno()).st_size
        oversized = False
        while remaining > 0 and len(reported) < len(checks):
            piece = stream.readline(min(4097, remaining))
            if not piece:
                break
            remaining -= len(piece)
            complete = piece.endswith(b'\n')
            if oversized or len(piece) > 4096 or not complete:
                oversized = not complete
                continue
            line = piece.decode('utf-8', errors='replace')
            if not line.startswith('oteryn-game-server '):
                continue
            fields = dict(re.findall(r'\b([a-z_]+)=([^\s]+)', line))
            check = fields.get('stage')
            if (fields.get('module') == 'durability'
                    and fields.get('event') == 'character_authority_check_failed'
                    and check in checks and check not in reported):
                print(f'NODE_BOOT_CHARACTER_AUTHORITY_CHECK stage={stage} '
                      f'world_bundle={world} check={check}')
                reported.add(check)
except FileNotFoundError:
    pass
PY_CHARACTER_AUTHORITY
}
start_runtime_diagnostics() {
  # Read the regular file independently: neither a slow log consumer nor a stalled
  # qualification pipeline can hold up the server's stderr. No raw error text is copied.
  local offset=0
  [[ ! -f "$node_log" ]] || offset=$(stat -c '%s' "$node_log")
  python3 -u - "$node_log" "$1" "$NODE_BOOT_WORLD" "$offset" <<'PY_RUNTIME_DIAGNOSTICS' &
import collections, os, re, signal, sys, time

path, stage, world, initial_offset = sys.argv[1:]
stopping = False
def stop(_signal, _frame):
    global stopping
    stopping = True
signal.signal(signal.SIGINT, stop)
signal.signal(signal.SIGTERM, stop)
events = {'serve_end', 'serve_command_end', 'command_guard_failed',
          'join_prepare_failed', 'listener_poll_slow', 'listener_connection_poll_slow',
          'pass_dropped', 'step_rejected_dawnport', 'step_rejected_source_map',
          'step_rejected_dead', 'step_rejected_lane', 'step_rejected_pvp_locks',
          'step_rejected_stale_position', 'step_rejected_pending_spell',
          'step_rejected_drome', 'step_rejected_movement', 'step_rejected_holder_busy',
          'session_durability_failed', 'logout_unknown', 'logout_busy',
          'control_loss_reconcile_pending', 'control_loss_result',
          'control_loss_publication', 'control_loss_grace', 'character_authority_check_failed',
          'appearance_initial_failed', 'appearance_observation_failed'}
legacy = {'DURABILITY_PASS_SLOW', 'DURABILITY_PASS_RETIRED',
          'DURABILITY_HOLDER_STUCK', 'DURABILITY_ROOT_REESTABLISHED'}
labels = {
    'kind': {'Actor', 'SnapshotMismatch', 'ContextMismatch', 'ScopeMismatch',
             'CoordinateOverflow', 'Cell', 'Blocked', 'CapacityExceeded', 'NotQualified',
             'holder_busy', 'storage', 'deadline', 'commit_unknown', 'authority',
             'invalid_state', 'unavailable', 'unknown', 'recorded', 'resumed_history',
             'terminal', 'not_applicable', 'refused', 'pending'},
    'class': {'observation', 'mappings', 'attach', 'encode', 'content', 'fence',
              'revision', 'state', 'visibility', 'conditions'},
    'failure_stage': {'account_closure', 'activity_destination', 'activity_loss_session',
                      'activity_publish', 'activity_return', 'activity_scope',
                      'activity_successor', 'activity_view', 'actor_unreserved',
                      'admitted_binding', 'appearance_observation', 'familiar_cast',
                      'combat_hold', 'condition_save', 'death_save', 'entry', 'familiar_save',
                      'foreground_demand', 'gourmet_checkpoint', 'hunting_stamina_readback',
                      'item_effects', 'kill_rewards', 'logout_settle', 'loss_activity_origin',
                      'loss_readback', 'monk_save', 'native_publication',
                      'loss_resumed_history', 'loss_session', 'loss_session_time',
                      'loss_transferred_history', 'native_cast', 'newhaven_travel',
                      'parameters', 'pending_spell_commit', 'prey_checkpoint_fence',
                      'prey_checkpoint_session', 'proficiency_character', 'proficiency_fence',
                      'proficiency_readback', 'proficiency_snapshot', 'pvp_refresh',
                      'release_attempt', 'release_commit', 'release_fence',
                      'release_not_applicable', 'release_not_expired', 'release_retry_lifted',
                      'release_settle',
                      'resting_stamina_readback', 'retire', 'retire_session',
                      'soul_war_source_use', 'spell_reconcile', 'staff_release',
                      'stamina_character', 'stamina_session',
                      'terminal_session', 'training_character', 'training_readback',
                      'training_save', 'training_session', 'wheel_gift', 'world_items',
                      'zones_after_reconcile', 'zones_before_reconcile',
                      'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
                      'bazaar', 'revision_chain', 'appearance', 'build_chain',
                      'proficiency_history', 'familiar_chain', 'spell_familiar_group',
                      'spell_item_chain', 'audit_payload'},
    'reason': {'transferred', 'disconnected', 'control_lost', 'logged_out',
               'protocol_error', 'commit_outcome_unknown', 'closed', 'no_evidence'},
    'path': {'server_message_timeout', 'liveness_missed', 'probe_write_timeout',
             'updates_timeout', 'activity_transfer', 'house_transfer'},
    'guard': {'house_transfer_input', 'activity_transfer_input'},
    'phase': {'activity_scope', 'initial', 'items', 'sessions', 'world',
              'trade_open', 'trade_observation', 'inventory', 'visibility', 'appearances',
              'frame', 'frame_error', 'held', 'step_due', 'probe', 'server_message',
              'serene', 'quest_log', 'chat', 'combat', 'chase', 'target_move', 'npc'},
    'deadline_exceeded': {'true', 'false'},
    'ready': {'true', 'false'},
}
authority_stages = {'recovery_fence', 'offline_training', 'npc_purchase', 'creation',
                    'bazaar', 'revision_chain', 'appearance', 'build_chain',
                    'proficiency_history', 'familiar_chain', 'spell_familiar_group',
                    'spell_item_chain', 'audit_payload'}
numbers = {'ts', 'next_command_id', 'server_sequence', 'command_type', 'command_id',
           'ms', 'connections', 'lifecycles', 'held_ms'}
counts = collections.Counter()
event_counts = collections.Counter()
emitted = 0
limited = set()
started = time.monotonic()

def report(line):
    global emitted
    fields = dict(re.findall(r'\b([a-z_]+)=([^\s]+)', line))
    # Keep the qualification stage and the session failure stage distinct.
    fields['failure_stage'] = fields.get('stage', '')
    event = fields.get('event') if line.startswith('oteryn-game-server ') else None
    marker = line.split(' ', 1)[0]
    if marker in legacy:
        event = marker.lower()
    signature = ('permission_denied' if 'permission denied' in line else
                 'root_not_ready' if 'root is not ready' in line else None)
    if event not in events and marker not in legacy:
        if not signature:
            return
        event = 'runtime_failure'
    safe = [('event', event)]
    for key, allowed in labels.items():
        if fields.get(key) in allowed:
            safe.append((key, fields[key]))
    if event == 'character_authority_check_failed' and fields.get('stage') in authority_stages:
        safe.append(('check_stage', fields['stage']))
    for key in ('source', 'caller'):
        value = fields.get(key, '')
        if value in {'maintenance', 'none'} or re.fullmatch(
                r'(?:crates|apps)/[A-Za-z0-9_./-]{1,220}\.rs:[0-9]{1,6}:[0-9]{1,6}', value):
            safe.append((key, value))
    if signature:
        safe.append(('signature', signature))
    identity = tuple(safe)
    if counts[identity] >= 2:
        return
    # Reserve room for terminal failures even if many different slow callers recur.
    limit = {'serve_end': 32, 'durability_pass_slow': 16,
             'logout_busy': 16, 'control_loss_reconcile_pending': 16,
             'control_loss_grace': 16,
             'character_authority_check_failed': 16}.get(event, 8)
    if event_counts[event] >= limit or emitted >= 128:
        if event not in limited:
            print(f'NODE_BOOT_RUNTIME_LIMIT stage={stage} world_bundle={world} '
                  f'event={event} records={event_counts[event]}')
            limited.add(event)
        return
    counts[identity] += 1
    event_counts[event] += 1
    for key in sorted(numbers):
        if re.fullmatch(r'[0-9]{1,20}', fields.get(key, '')):
            safe.append((key, fields[key]))
    print(f'NODE_BOOT_RUNTIME stage={stage} world_bundle={world} '
          + ' '.join(f'{key}={value}' for key, value in safe)
          + f' observed_ms={int((time.monotonic() - started) * 1000)}')
    emitted += 1

print(f'NODE_BOOT_RUNTIME_MONITOR state=watching stage={stage} world_bundle={world}')
stream = None
pending = b''
oversized = False
try:
    while True:
        final_pass = stopping
        try:
            if stream is None:
                stream = open(path, 'rb')
                stream.seek(int(initial_offset))
            if os.fstat(stream.fileno()).st_size < stream.tell():
                stream.seek(0)
                pending, oversized = b'', False
            # A final flush is also bounded; it never waits for PostgreSQL or Docker.
            for _ in range(1024):
                piece = stream.readline(4097)
                if not piece:
                    break
                if not oversized:
                    pending += piece
                    oversized = len(pending) > 4096
                if piece.endswith(b'\n'):
                    if not oversized:
                        report(pending.decode('utf-8', errors='replace').rstrip('\n'))
                    pending, oversized = b'', False
        except FileNotFoundError:
            pass
        if final_pass:
            break
        if not stopping:
            time.sleep(0.25)
finally:
    pending_bytes = max(0, os.fstat(stream.fileno()).st_size - stream.tell()) if stream else 0
    if stream:
        stream.close()
    print(f'NODE_BOOT_RUNTIME_MONITOR state=stopped stage={stage} world_bundle={world} '
          f'records={emitted} pending_bytes={pending_bytes}')
PY_RUNTIME_DIAGNOSTICS
  RUNTIME_DIAGNOSTICS_PID=$!
}
# Classify FAIL records by the typed cause and root captured in the debt list.
classify_stage_rows() {
  local log=$1 known=$2
  [[ -e "$known" ]] || known=/dev/null
  awk '
    FILENAME == ARGV[1] {
      if ($1 !~ /^#/ && $3 == "at" && $4 != "") {
        if ($4 ~ /;$/) {
          reason = $0
          sub(/^[^;]*;[[:space:]]*/, "", reason)
          prefixes[$1] = reason
        }
        sub(/;$/, "", $4)
        expected[$1] = $2 " " $4
      }
      next
    }
    $1 == "SEAM_EVIDENCE" && $2 == "e2e_all" {
      row = $3; sub(/^scenario=/, "", row)
      if ($4 != "status=FAIL") {
        if (row in expected) print "stale", row
        next
      }
      cause = root = detail = ""
      for (i = 6; i <= NF; i++) {
        if ($i ~ /^cause=/) cause = substr($i, 7)
        if ($i ~ /^root=/) root = substr($i, 6)
        if ($i ~ /^evidence=/) detail = substr($i, 10)
      }
      if ((row in expected) && expected[row] == cause " " root &&
          (!(row in prefixes) || index(detail, prefixes[row]) == 1)) print "known", row
      else {
        print "new", row
        if (row in expected) print "stale", row
      }
    }
  ' "$known" "$log"
}

stage() { # name prerequisite... -- function
  local name=$1; shift
  local prerequisites=()
  while [[ $1 != -- ]]; do prerequisites+=("$1"); shift; done
  shift
  ORDER+=("$name")
  local missing
  for missing in "${prerequisites[@]}"; do
    if [[ ${STATUS[$missing]:-} != PASS ]]; then
      STATUS[$name]="SKIP(needs $missing)"
      return 0
    fi
  done
  ACTIVE_STAGE=$name
  echo "::group::stage $name"
  evidence "stage=$name state=start world_bundle=$NODE_BOOT_WORLD spells=$NODE_BOOT_SPELLS"
  case "$name" in seam_stages|spell_scenarios) start_runtime_diagnostics "$name" ;; esac
  set +e
  local stage_log="$WORK/stage-$name.log"
  # Failure reports and snapshots follow the node used by the stage.
  local node_log="$node_log"
  [[ "$name" != pvp_* ]] || node_log="$WORK/pvp-node.log"
  local attempt
  if ! attempt=$(mktemp -d "$WORK/stage-$name.XXXXXX"); then
    STATUS[$name]="FAIL(evidence)"
    stop_runtime_diagnostics
    set -e
    evidence "stage=$name state=finish world_bundle=$NODE_BOOT_WORLD exit=1"
    echo "::endgroup::"
    return 0
  fi
  local deadline_marker="$attempt/deadline"
  local status_file="$attempt/status"
  local launching=worker previous_pid=${!:-}
  set -m
  (
    set +m
    local rc=0 IN_STAGE_WORKER=1
    trap 'printf "%s\n%s\n" "$rc" "${STATUS[$name]:-FAIL(evidence)}" > "$status_file"' EXIT
    ( set -Eeuo pipefail; source "$VARS"; "$@" ) 2>&1 | tee "$stage_log"
    pipeline_status=("${PIPESTATUS[@]}")
    if [[ ${pipeline_status[1]} != 0 ]]; then
      echo "NODE_BOOT_FAILURE stage=$name evidence_log=unavailable"
      rc=1
    else
      rc=${pipeline_status[0]}
    fi
    if [[ $rc == 0 ]] && ! cat -- "$stage_log" >/dev/null; then
      echo "NODE_BOOT_FAILURE stage=$name evidence_log=unavailable"
      rc=1
    fi
    case "$name" in
      seam_stages|spell_scenarios)
        if [[ $rc == 0 ]] && ! grep -Eq '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$stage_log"; then
          echo "NODE_BOOT_FAILURE stage=$name qualification_test_count=unproven"
          rc=1
        fi
        if [[ $rc == 0 ]]; then
          local qualification_case=basic
          if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
            qualification_case=spells
          elif [[ "$NODE_BOOT_WORLD" == 1 ]]; then
            qualification_case=world
            local selection=("--groups=${NODE_BOOT_E2E_ONLY:-}")
            if [[ -n "${E2E_ALL_CAPTURE_DIR:-}" ]]; then
              selection+=(--batch=all)
            elif [[ -v OTERYN_E2E_ROW_BATCH ]]; then
              selection+=("--batch=$OTERYN_E2E_ROW_BATCH")
            fi
            python3 "$GAME_SOURCE/tools/qualification/node_boot/matrix.py" verify-world "$stage_log" "${selection[@]}" || rc=$?
          fi
          [[ $rc != 0 ]] || evidence "qualification_case=$qualification_case"
        fi ;;
    esac
    pg_observability_dump "$name" "$rc"
    set -e
    if [[ $rc == 0 ]]; then
      # A FAIL row fails its stage even when the stage process exits 0, unless the row is
      # listed in known_fail_rows.txt (debt that every run prints).
      local known_fail="$GAME_SOURCE/tools/qualification/node_boot/known_fail_rows.txt" classified new_rows known_hit stale_rows
      if ! classified=$(classify_stage_rows "$stage_log" "$known_fail"); then
        STATUS[$name]="FAIL(evidence)"
        return 0
      fi
      new_rows=$(awk '$1 == "new" {print $2}' <<< "$classified" | sort -u | paste -sd, -)
      known_hit=$(awk '$1 == "known" {print $2}' <<< "$classified" | sort -u | paste -sd, -)
      stale_rows=$(awk '$1 == "stale" {print $2}' <<< "$classified" | sort -u | paste -sd, -)
      STATUS[$name]=PASS
      [[ -z $known_hit ]] || echo "NODE_BOOT_KNOWN_FAIL $name rows=$known_hit"
      [[ -z $stale_rows ]] || echo "NODE_BOOT_KNOWN_FAIL_STALE $name rows=$stale_rows"
      if [[ -n $new_rows ]]; then
        STATUS[$name]="FAIL(rows)"
        echo "NODE_BOOT_STAGE_FAILED $name rows=$new_rows"
        if [[ -f "$node_log" ]]; then
          cp "$node_log" "$WORK/failed-stage-$name.log" || true
          [[ -f "$failed_node_log" ]] || cp "$node_log" "$failed_node_log" || true
        fi
        if [[ "$name" == seam_stages && -f "$WORK/pvp-node.log" ]]; then
          cp "$WORK/pvp-node.log" "$WORK/failed-stage-$name-pvp.log" || true
        fi
      fi
      if grep -q 'e2e_all scenario=.* status=FAIL' "$WORK/stage-$name.log"; then
        echo "--- session close events behind failed e2e rows"
        grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed|transport_close|connection_[a-z_]+)( |$)' "$node_log" 2>/dev/null | tail -n 120 || true
      fi
    else
      STATUS[$name]="FAIL(exit $rc)"
      # Later probes start new server incarnations and truncate node_log. Preserve
      # the first failed incarnation now so cleanup uploads the log that explains
      # the qualification failure rather than a subsequent successful shutdown.
      if [[ -f "$node_log" ]]; then
        cp "$node_log" "$WORK/failed-stage-$name.log" || true
        [[ -f "$failed_node_log" ]] || cp "$node_log" "$failed_node_log" || true
      fi
      if [[ "$name" == seam_stages && -f "$WORK/pvp-node.log" ]]; then
        cp "$WORK/pvp-node.log" "$WORK/failed-stage-$name-pvp.log" || true
      fi
      echo "NODE_BOOT_STAGE_FAILED $name exit=$rc"
      report_character_authority_failures "$name"
      echo "--- session close events"
      grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed|transport_close)( |$)' "$node_log" 2>/dev/null | tail -n 40 || true
      echo "--- last node events"; durability_report_lines < "$node_log" 2>/dev/null | grep -v 'DURABILITY_' | tail -n 40 || true
      # Repeated background-pass retries hide the event that took the root down.
      echo "--- node events without background retries"
      durability_report_lines < "$node_log" 2>/dev/null | grep -vE 'statistics_retention|audit_collector|corpse_decay_read_deferred|server_save_countdown_read_deferred|_world_pass state=retry|admission_refused|DURABILITY_' | tail -n 150 || true
      echo "--- first root-not-ready"
      grep -n -m 1 -B 40 'root is not ready' "$node_log" 2>/dev/null | tail -n 41 || true
      # Which node events dominate, and what admission reported, beyond the tail above.
      echo "--- node event counts"
      grep -oE 'event=[a-z_]+' "$node_log" 2>/dev/null | sort | uniq -c | sort -rn | head -n 40 || true
      # Which semantic passes kept the single holder long or retired it (NBDIAG-3).
      echo "--- durability holder reports"
      durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_[A-Z_]+( caller=[^ ]+)?( reason=[a-z_]+)?' | sort | uniq -c | sort -rn | head -n 40 || true
      echo "--- slowest durability passes"
      grep -oE 'DURABILITY_PASS_SLOW caller=[^ ]+ held_ms=[0-9]+ deadline_exceeded=[a-z]+' "$node_log" 2>/dev/null | sort -t= -k3 -n -r | head -n 25 || true
      echo "--- holder kept or dropped by a pass"
      durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_(HOLDER_STUCK|PASS_DROPPED) caller=[^ ]+( held_ms=[0-9]+)?' | head -n 25 || true
      # How busy the single holder was and who kept it (NBDIAG-5).
      echo "--- holder usage windows"
      durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_HOLDER_WINDOW .*' | head -n 60 || true
      echo "--- holder usage by caller over the run"
      durability_report_lines < "$node_log" 2>/dev/null | grep -oE 'DURABILITY_HOLDER_USAGE caller=[^ ]+ passes=[0-9]+ held_ms=[0-9]+ refused=[0-9]+' \
          | awk '{split($2,c,"=");split($3,p,"=");split($4,h,"=");split($5,r,"=");P[c[2]]+=p[2];H[c[2]]+=h[2];R[c[2]]+=r[2]} END {for (k in H) print H[k], P[k], R[k], k}' \
          | sort -rn | head -n 30 || true
      echo "--- node message kinds"
      sed -E 's/ts=[0-9]+ //; s/trace=[0-9a-f-]+ //; s/[0-9]+/N/g' "$node_log" 2>/dev/null | cut -c1-140 | sort | uniq -c | sort -rn | head -n 40 || true
      echo "--- node admission and session events"
      durability_report_lines < "$node_log" 2>/dev/null | grep -iE 'admission|admit|refus|serve_|session_' | grep -vE 'quest_log_excluded|DURABILITY_' | tail -n 60 || true
      # How each gameplay connection entered and ended, and whether the shared listener task
      # was kept by one poll (NBDIAG-6).
      echo "--- connection entry and admission outcomes"
      grep -oE 'event=(connection_[a-z_]+|listener_[a-z_]+)( [a-z_]+=[a-z_]+)*' "$node_log" 2>/dev/null | sort | uniq -c | sort -rn | head -n 30 || true
      echo "--- connection and listener events"
      grep -E 'event=(connection_|listener_)' "$node_log" 2>/dev/null | sed -E 's/^.*(event=)/\1/' | head -n 80 || true
      echo "--- postgres sessions"
      echo "SELECT pid, usename, application_name, state, wait_event_type, wait_event, now() - backend_start, now() - xact_start, now() - state_change, left(regexp_replace(query, '\\s+', ' ', 'g'), 160) FROM pg_stat_activity WHERE datname = 'oteryn_node_boot' AND pid <> pg_backend_pid() ORDER BY backend_start" \
        | psql_admin oteryn_node_boot 2>&1 | tail -n 30 || true
      # The holder is alive but always busy: show which statements and tables keep it so.
      echo "--- postgres statements on the holder (200 samples)"
      local pg_sample_deadline=$((SECONDS + 20))
      for _ in $(seq 1 200); do
        (( SECONDS < pg_sample_deadline )) || break
        echo "SELECT state || ' ' || left(regexp_replace(query, '\\s+', ' ', 'g'), 200) FROM pg_stat_activity WHERE datname = 'oteryn_node_boot' AND usename <> 'oteryn_node_boot_admin'" \
          | psql_admin oteryn_node_boot 2>/dev/null || true
        sleep 0.02
      done | sort | uniq -c | sort -rn | head -n 25 || true
      echo "--- postgres table writes and scans over 10 s"
      psql_admin oteryn_node_boot 2>&1 <<'SQL' | tail -n 30 || true
CREATE TEMP TABLE node_boot_stat_before AS
  SELECT relid, n_tup_ins + n_tup_upd + n_tup_del AS writes, coalesce(seq_scan, 0) + coalesce(idx_scan, 0) AS scans
  FROM pg_stat_user_tables;
SELECT pg_sleep(10);
SELECT pg_stat_clear_snapshot();
SELECT s.relname, (s.n_tup_ins + s.n_tup_upd + s.n_tup_del) - b.writes AS writes, (coalesce(s.seq_scan, 0) + coalesce(s.idx_scan, 0)) - b.scans AS scans
  FROM pg_stat_user_tables s JOIN node_boot_stat_before b USING (relid)
  WHERE (s.n_tup_ins + s.n_tup_upd + s.n_tup_del) - b.writes + (coalesce(s.seq_scan, 0) + coalesce(s.idx_scan, 0)) - b.scans > 0
  ORDER BY 2 DESC, 3 DESC LIMIT 25;
SQL
      echo "--- postgres"; timeout --foreground --kill-after=2s 15s docker logs --tail 40 "$PG_CONTAINER" 2>&1 | grep -E 'ERROR|FATAL|STATEMENT' || true
      echo "--- platform"; compose logs --no-color --tail 40 platform nginx 2>&1 || true
    fi
  ) &
  STAGE_PID=$!
  previous_pid=$! launching=timer
  (
    set +m
    command sleep "${NODE_BOOT_STAGE_TIMEOUT_SECONDS:-10800}" || exit
    : > "$deadline_marker"
    kill -TERM -- "-$STAGE_PID" 2>/dev/null || true
    command sleep 0.2
    kill -KILL -- "-$STAGE_PID" 2>/dev/null || true
  ) &
  STAGE_TIMER_PID=$!
  launching=""
  set +m
  local worker_status=0
  wait "$STAGE_PID" 2>/dev/null || worker_status=$?
  kill -KILL -- "-$STAGE_TIMER_PID" 2>/dev/null || true
  wait "$STAGE_TIMER_PID" 2>/dev/null || true
  STAGE_TIMER_PID=""
  local rc=1
  if [[ -f "$deadline_marker" ]]; then
    kill -KILL -- "-$STAGE_PID" 2>/dev/null || true
    echo "NODE_BOOT_FAILURE stage=$name deadline=exceeded"
    rc=124
    STATUS[$name]="FAIL(exit 124)"
    if [[ -f "$node_log" ]]; then
      cp "$node_log" "$WORK/failed-stage-$name.log" || true
      [[ -f "$failed_node_log" ]] || cp "$node_log" "$failed_node_log" || true
    fi
    if [[ "$name" == seam_stages && -f "$WORK/pvp-node.log" ]]; then
      cp "$WORK/pvp-node.log" "$WORK/failed-stage-$name-pvp.log" || true
    fi
  elif [[ $worker_status == 0 && -s "$status_file" ]]; then
    { read -r rc; read -r "STATUS[$name]"; } < "$status_file"
  else
    STATUS[$name]="FAIL(evidence)"
  fi
  STAGE_PID=""
  stop_runtime_diagnostics
  set -e
  evidence "stage=$name state=finish world_bundle=$NODE_BOOT_WORLD exit=$rc"
  echo "::endgroup::"
}

node_field() { # log field
  sed -n "s/.*awaiting_assignment.*$2=\([^ \"]*\).*/\1/p" "$1" | tail -n 1
}

assert_operator_replay() { # database, receipt query, first outcome, replay command ...
  local database=$1 query=$2 first=$3 before after replay
  shift 3
  before=$(psql_admin "$database" <<< "$query") || return
  [[ -n "$before" ]] || { fail "operator receipt missing before replay"; return 1; }
  replay=$("$@") || return
  after=$(psql_admin "$database" <<< "$query") || return
  [[ "$replay" == "$first" && "$after" == "$before" ]] || {
    fail "operator replay changed the outcome or durable receipt"
    return 1
  }
  printf '%s\n' "$replay"
}

operator_setup() { # §4.1 operator setup before the node starts
  ops authorization issue --file launch-a.json --binding node-boot-a
  write_node_config launch-a.json
  ops s2 issue --node-config "$BASE/node/node.toml" --file s2-fresh-store.json --namespace node-boot --authorization disposable-qualification
  local refusal code=0 first
  refusal=$(ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}" 2>&1) || code=$?
  printf '%s\n' "$refusal"
  [[ $code == 6 && "$refusal" == *'interpretation refused until generation 1 is admitted (Rejected)'* ]] \
    || fail "expected interpretation refusal before fresh-store admission (exit=$code)"
  first=$(ops character fresh-store --request fresh-store.json)
  [[ "$first" == 'character_recovery=admitted generation=1' ]] || fail "unexpected fresh-store outcome"
  printf '%s\n' "$first"
  assert_operator_replay oteryn_node_boot 'SELECT row_to_json(r) FROM game_character_recovery_admissions r ORDER BY recovery_generation' \
    "$first" ops character fresh-store --request fresh-store.json
  ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"
  evidence "operator=authorization,s2_issuance,fresh_store,interpretation interpretation_before_store=refused fresh_store_rerun=idempotent"
}

configuration_negatives() { # exit before binding, no secret in output
  sudo cp "$BASE/node/node.toml" "$BASE/node/insecure.toml"; sudo chmod 0666 "$BASE/node/insecure.toml"
  sudo ln -sf "$BASE/node/node.toml" "$BASE/node/link.toml"
  sudo sed '/^map_revision/d' "$BASE/node/node.toml" | sudo tee "$BASE/node/missing.toml" >/dev/null
  sudo sed 's/^max_connections = 256/max_connections = 257/' "$BASE/node/node.toml" | sudo tee "$BASE/node/over.toml" >/dev/null
  sudo chmod 0644 "$BASE/node/missing.toml" "$BASE/node/over.toml"
  local config code failed=0
  for config in insecure link missing over; do
    set +e
    sudo -u "$SERVICE_USER" timeout --foreground --kill-after=5s 120s env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/$config.toml" 2>> "$WORK/negative.log"
    code=$?
    set -e
    echo "configuration $config exit=$code"
    [[ $code == 10 ]] || failed=1
  done
  ! grep -q "$RUNTIME_PASSWORD" "$WORK/negative.log" || { echo "secret in output"; failed=1; }
  [[ $failed == 0 ]]
  evidence "configuration group_writable=exit10 symlink=exit10 missing_key=exit10 over_maximum=exit10 secrets_in_output=0"
}

# Deployment administration: only the database owner writes these grant tables.
node_deployment_grants() {
  [[ "$(echo "SELECT current_user = pg_get_userbyid(datdba) FROM pg_database WHERE datname=current_database()" | psql_admin oteryn_node_boot)" == t ]] || fail "deployment issuer is not the database owner"
  local operations=1,2,3,4
  [[ "${NODE_BOOT_LIFECYCLE:-}" != game_platform ]] || operations=1,4
  psql_admin oteryn_node_boot <<SQL
INSERT INTO game_control_scope_grants SELECT 'nb_control', '$WORLD_ID', '$CHANNEL_ID', op FROM unnest(ARRAY[$operations]::smallint[]) op;
INSERT INTO game_control_field_policy_grants VALUES ('nb_control', '$WORLD_ID', 'policy-s3b-1');
SQL
  # Match the readiness revision with explicit control-owned non-PvP facts.
  # Authenticate as the granted login: SET ROLE cannot satisfy the insert guard.
  docker exec -i -e PGPASSWORD="$CONTROL_PASSWORD" "$PG_CONTAINER" \
    psql -v ON_ERROR_STOP=1 -qAt -h 127.0.0.1 -U nb_control -d oteryn_node_boot <<SQL
INSERT INTO game_spell_field_world_policies (world_id, world_policy_revision, world_type, protection_level, in_fight_ms, source_pin, control_role, decision_identity)
VALUES ('$WORLD_ID', 'policy-s3b-1', 1, 8, 60000, '99902524e052f37574194466c2949c576e4ab269', session_user, 'node-boot-qualification');
SQL
  [[ "$(echo "SELECT has_table_privilege('nb_control','game_control_scope_grants','INSERT,UPDATE,DELETE,TRUNCATE') OR has_table_privilege('nb_runtime','game_control_scope_grants','INSERT,UPDATE,DELETE,TRUNCATE')" | psql_admin oteryn_node_boot)" == f ]] || fail "Game role can issue control-scope grants"
  [[ "$(echo "SELECT string_agg(operation::text, ',' ORDER BY operation) FROM game_control_scope_grants WHERE control_role='nb_control' AND world_id='$WORLD_ID' AND channel_id='$CHANNEL_ID'" | psql_admin oteryn_node_boot)" == "$operations" ]] || fail "deployment grant scope or operations mismatch"
  evidence "flow=deployment_administration issuer=database_owner scope=seeded_world_channel operations=$operations game_roles_can_issue=0"
}

node_assigned_ready() { # §4.2–§4.4
  local boot_started=$SECONDS
  start_node
  remember NODE_PID "$NODE_PID"
  await_log awaiting_assignment 120
  node_complete_assignment "$boot_started"
}

# Shared continuation: the lane keeps this node alive during ungranted proofs.
node_complete_assignment() {
  local boot_started=$1
  local node revision
  node="$(node_field "$node_log" node_id)"; revision="$(node_field "$node_log" registration_revision)"
  remember node_a "$node"
  local code=0
  ops assignment assign --request assign-ungranted.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision" || code=$?
  [[ $code == 6 ]] || fail "assignment without an exact-scope grant exit=$code"
  # #935: the node activates the scope's current native entry Content issuance before its
  # Channel runtime exists. Issuance requires an exact-scope operation-4 grant.
  code=0
  ops content activate --request content-ungranted.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty || code=$?
  [[ $code == 6 ]] || fail "content activation without an exact-scope grant exit=$code"
  node_deployment_grants
  local activation
  activation=$(ops content activate --request content-1.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty)
  [[ "$activation" == 'content_activation=recorded activation_sequence=1 '* ]] || fail "unexpected content activation outcome"
  printf '%s\n' "$activation"
  assert_operator_replay oteryn_node_boot 'SELECT row_to_json(r) FROM game_content_activations r ORDER BY world_id, channel_id, activation_sequence' \
    "$activation" ops content activate --request content-1.json
  local -a report_args=()
  if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
    report_args=(--report-config "$BASE/ops/assignment-report.toml" --node-config "$BASE/node/node.toml"
      --node-identity "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["clients"]["runtime-status"]["san"])' "$WORK/portal-report.json")")
  fi
  ops assignment assign --request assign-a.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision" "${report_args[@]}"
  await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  await_log "event=readiness .*ready=true" 60
  # The canonical manifest makes each validation substantial. Prove the same
  # stale-predecessor rejection against the now-active sequence 1, without
  # spending the unassigned node's unchanged wait on this independent negative.
  code=0
  ops content activate --request content-stale.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 2 --previous empty || code=$?
  [[ $code == 6 ]] || fail "stale content activation predecessor exit=$code"
  [[ "$(echo 'SELECT count(*) FROM game_content_activations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "content activation rows"
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    # World admission prepares Cyclopedia/boost state against the admitted epoch.
    # A fresh database has none: initialize through the existing Control owner,
    # with the actual bundle and issued content, before any Character admission.
    local epoch_result
    epoch_result="$({
      echo 'PREPARE node_boot_world_epoch(UUID, UUID, TEXT) AS'
      cat "$GAME_SOURCE/tools/qualification/node_boot/world_epoch.sql"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$WORLD_DIGEST"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$WORLD_DIGEST"
    } | psql_admin oteryn_node_boot)"
    [[ "$epoch_result" == $'t\nf' ]] || fail "World epoch initialization/replay"
    evidence "world_epoch=0 source=control_initializer bundle=compiled content=issued replay=idempotent"
  fi
  [[ "$(sudo stat -c '%u %a' "$BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "control socket mode"
  [[ "$(echo 'SELECT count(*) FROM game_node_registrations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "operator registered an incarnation"
  evidence "world_boot_seconds=$((SECONDS - boot_started)) world_bundle=$NODE_BOOT_WORLD"
  evidence "node registered=1 assignment=operator ungranted=refused content_activation=sequence_1 content_ungranted=refused content_replay=exact content_stale=refused readiness=true control_socket=uid${SERVICE_UID}_0600"
}

control_socket_peer() { # require a UID refusal for this exact connection
  local peer_pid offset
  offset=$(stat -c %s "$node_log")
  peer_pid=$(sudo -u "$SERVICE_USER" python3 - "$BASE/run/control.sock" <<'PY_PROBE'
import os
import socket
import sys

with socket.socket(socket.AF_UNIX) as stream:
    stream.settimeout(2)
    stream.connect(sys.argv[1])
    # The UID is checked before the server reads any request bytes.
    if stream.recv(1) != b"":
        sys.exit("unexpected reply to non-root control peer")
print(os.getpid())
PY_PROBE
  ) || fail "non-root control peer did not receive bounded EOF"
  [[ "$peer_pid" =~ ^[1-9][0-9]*$ ]] || fail "non-root control peer PID unavailable"
  if ! tail -c "+$((offset + 1))" "$node_log" \
    | grep -F 'event=control_peer_rejected ' \
    | grep -F 'detail="peer_pid='"$peer_pid"' peer_uid='"$SERVICE_UID"'"' >/dev/null; then
    fail "control EOF was not attributed to the service-user UID"
  fi
  evidence "control_socket service_user_peer=closed"
}

character_bootstrap() { # §4.5 Characters from real Platform intents
  local index=0 account operation name first failed=0
  for account in "${BOOTSTRAP_ACCOUNTS[@]}"; do
    operation="${INTENT_OPERATIONS[$index]}"
    name="${INTENT_NAMES[$index]}"
    index=$((index + 1))
    if ! php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$account"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$operation"'","--target-world-id"=>"'"$WORLD_ID"'","--requested-name"=>"'"$name"'","--requested-sex"=>"Female","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); fwrite(STDERR, Illuminate\Support\Facades\Artisan::output()); exit($code);'; then
      echo "platform intent issue failed for operation $operation"
      failed=1
      continue
    fi
    first=$(ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation") || { failed=1; continue; }
    [[ "$first" == 'character_bootstrap=committed' ]] || { failed=1; continue; }
    printf '%s\n' "$first"
    assert_operator_replay oteryn_node_boot "SELECT row_to_json(r) FROM game_character_operation_receipts r WHERE operation_id='$operation'" \
      "$first" ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]]
  evidence "characters=${#BOOTSTRAP_ACCOUNTS[@]} source=platform_intents path=control_socket retry=idempotent"
}

seam_stages() { # §4.6 every #823 stage against the node's own port
  local spawn_proof
  spawn_proof="$(OTERYN_TEST_POSTGRES_ADMIN_URL="$ADMIN_URL" \
    cargo +1.100.0 test --locked -p oteryn-game-server --lib "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}" \
    node::serve::qualification_fixture::tests::qualification_spawn_transaction_returns_pool_and_bounds_backend -- --exact)" || {
    printf '%s\n' "$spawn_proof"
    return 1
  }
  printf '%s\n' "$spawn_proof" >&2
  grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<< "$spawn_proof" || {
    printf '%s\n' 'Spawn PostgreSQL proof did not execute exactly one passing test' >&2
    return 1
  }
  # The bootstrap evidence ages past five seconds, so each admission fetches its own.
  sleep 6
  local qualification=node_boot_seam_against_running_node
  local test_env=()
  if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
    qualification=node_boot_spells_against_running_node
    test_env=("RUST_MIN_STACK=${RUST_MIN_STACK:-16777216}")
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    test_env+=("NODE_BOOT_PRIMARY_LOG=$node_log" "NODE_BOOT_WORLD_BUNDLE=$BASE/world/world.otbundle" "NODE_BOOT_WORLD_PIN=$BASE/world/world.json" "NODE_BOOT_MAP_REVISION=$MAP_REVISION" "RUST_MIN_STACK=${RUST_MIN_STACK:-67108864}")
    local extra_accounts
    extra_accounts="$(IFS=,; echo "${EXTRA_WORLD_ACCOUNTS[*]}")"
    test_env+=("NODE_BOOT_EXTRA_WORLD_ACCOUNTS=$extra_accounts")
    test_env+=("NODE_BOOT_PACKET_ACCOUNT_ID=$PACKET_ACCOUNT_ID" "NODE_BOOT_PACKET_SECOND_ACCOUNT_ID=$SECOND_PACKET_ACCOUNT_ID")
    if [[ -n "${E2E_ALL_CAPTURE_DIR:-}" ]]; then
      test_env+=("OTERYN_E2E_ROW_BATCH=all")
    else
      test_env+=("E2E_ALL_CAPTURE_DIR=$WORLD_CAPTURE_DIR")
    fi
    local extra_index
    for extra_index in "${!EXTRA_WORLD_ACCOUNTS[@]}"; do
      test_env+=("NODE_BOOT_EXTRA_ACCOUNT_$extra_index=${EXTRA_WORLD_ACCOUNTS[$extra_index]}")
    done
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 && -z "${NODE_BOOT_LIFECYCLE:-}" ]]; then
    # MP01's PvP rows run against the Open PvP World's own node and database.
    test_env+=("NODE_BOOT_PVP_ADDRESS=127.0.0.1:$PVP_GAME_PORT" "NODE_BOOT_PVP_CONTROL_SOCKET=$PVP_BASE/run/control.sock"
      "NODE_BOOT_PVP_DATABASE_URL=${ADMIN_URL%/*}/$PVP_DATABASE" "NODE_BOOT_PVP_WORLD_ID=$PVP_WORLD_ID"
      "NODE_BOOT_PVP_CHANNEL_ID=$PVP_CHANNEL_ID" "NODE_BOOT_PVP_ACCOUNT_ID=${PVP_ACCOUNT_IDS[0]}"
      "NODE_BOOT_PVP_SECOND_ACCOUNT_ID=${PVP_ACCOUNT_IDS[1]}"
      "NODE_BOOT_PVP_RUNE_CASTER_ACCOUNT_ID=${PVP_ACCOUNT_IDS[2]}"
      "NODE_BOOT_PVP_RUNE_TARGET_ACCOUNT_ID=${PVP_ACCOUNT_IDS[3]}")
  fi
  # Row selection, fail-fast and the no-progress watchdog belong to the seam harness.
  local knob
  for knob in NODE_BOOT_E2E_ONLY NODE_BOOT_NO_FAILFAST NODE_BOOT_NO_PROGRESS_S; do
    [[ -z "${!knob+x}" ]] || test_env+=("$knob=${!knob}")
  done
  local staged_manifest="${NODE_GAMEPLAY_ENV[0]:-}"
  staged_manifest="${staged_manifest#OTERYN_NATIVE_GAMEPLAY_MANIFEST=}"
  env "${NODE_GAMEPLAY_ENV[@]}" "${test_env[@]}" \
  NODE_BOOT_SPELL_MANIFEST="$staged_manifest" \
  NODE_BOOT_CONTROL_SOCKET="$BASE/run/control.sock" \
  NODE_BOOT_ADDRESS="127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}" \
  NODE_BOOT_GAMEPLAY_CERT="$WP5_PKI/gameplay.crt" \
  NODE_BOOT_DATABASE_URL="$ADMIN_URL" \
  NODE_BOOT_WORLD_ID="$WORLD_ID" NODE_BOOT_CHANNEL_ID="$CHANNEL_ID" \
  WP5_S3A_PORT="$WP5_PORT" WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt" \
  WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt" WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key" \
  WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID" WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID" \
  WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID" WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
  WP5_S3B_RECOVERY_KEY_ID="$RECOVERY_KEY_ID" WP5_S3B_RECOVERY_KEY_SEED="$RECOVERY_SEED_HEX" \
    cargo +1.100.0 test --locked -p oteryn-game-server --lib "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}" \
    "qualification::$qualification" -- --ignored --exact --nocapture
}

assert_shutdown_guards() {
  local database=$1
  [[ "$(echo 'SELECT count(*) > 0 AND count(*) FILTER (WHERE ready) = 0 FROM game_durability_admission_runtime_guards' | psql_admin "$database")" == t ]] || {
    echo 'NODE_BOOT_GUARD_DIAGNOSTIC ready_scopes:' >&2
    echo "SELECT encode(scope_key,'hex') AS scope_key,ownership_generation,publication_revision FROM game_durability_admission_runtime_guards WHERE ready ORDER BY scope_key LIMIT 16" | psql_admin "$database" >&2 || true
    fail "guard still ready or absent"
  }
}
graceful_shutdown() { # ready=false first, socket removed
  stop_node "$NODE_PROCESS_DIR"
  grep -q "event=readiness .*ready=false" "$node_log" || fail "no ready=false on shutdown"
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "no clean shutdown"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after graceful shutdown"
  assert_shutdown_guards oteryn_node_boot
  evidence "shutdown ready=false socket=removed"
}

restart_without_supersession() { # fails at S2 custody, never ready
  ops authorization issue --file launch-b.json --binding node-boot-b
  write_node_config launch-b.json
  local code=0
  sudo -u "$SERVICE_USER" timeout --foreground --kill-after=5s 120s env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" 2> "$WORK/restart.log" || code=$?
  cat "$WORK/restart.log"
  [[ $code == 13 ]] || fail "restart without supersession exit=$code"
  local line node revision
  line="$(grep 'event=registered' "$WORK/restart.log")"
  node="$(sed -n 's/.*node_id=\([^ ]*\).*/\1/p' <<<"$line")"
  revision="$(sed -n 's/.*registration_revision=\([0-9]*\).*/\1/p' <<<"$line")"
  ops registration revoke --node-id "$node" --revision "$revision"
  evidence "restart_without_supersession=refused_at_custody exit=13"
}

superseding_replacement() { # claims custody, ready by CAS after replace
  ops authorization issue --file launch-c.json --binding node-boot-c --supersedes "$node_a"
  write_node_config launch-c.json
  : > "$node_log"
  start_node
  await_log awaiting_assignment 120
  local node revision
  node="$(node_field "$node_log" node_id)"; revision="$(node_field "$node_log" registration_revision)"
  remember node_c "$node"
  ops assignment replace --request replace-c.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision"
  # A new incarnation starts with no active Content and reactivates only the current issuance.
  await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  await_log "event=readiness .*ready=true" 60
  stop_node "$NODE_PROCESS_DIR"
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "replacement did not shut down cleanly"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after replacement shutdown"
  assert_shutdown_guards oteryn_node_boot
  evidence "replacement=superseding custody=claimed content=reactivated_current_issuance readiness=cas"
}

signal_before_ready() { # SIGTERM in the assignment wait: clean exit, never ready
  ops authorization issue --file launch-e.json --binding node-boot-e --supersedes "$node_c"
  write_node_config launch-e.json
  : > "$node_log"
  start_node
  await_log awaiting_assignment 120
  stop_node "$NODE_PROCESS_DIR"
  cat "$node_log"
  grep -q 'event=shutdown .*signal before ready' "$node_log" || fail "no pre-ready shutdown event"
  ! grep -q "event=readiness .*ready=true" "$node_log" || fail "ready published after the signal"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after a pre-ready signal"
  assert_shutdown_guards oteryn_node_boot
  evidence "signal_in_assignment_wait exit=0 ready=never socket=absent"
}

# The Open PvP World (world_type=2) of the World pass, beside the unchanged no-PvP World.
# One `serve` process serves one World/Channel scope (node/config.rs ScopeConfig), and one
# database holds one live node's S2 custody (native_admission_source.rs, custody claim),
# so this World gets its own database in the same PostgreSQL container and its own node.
# Platform, PKI, secrets, binaries, bundle and gameplay inputs are shared; the steps mirror
# operator_setup, node_deployment_grants, node_complete_assignment and character_bootstrap.
pvp_ops() { sudo timeout --foreground --kill-after=5s 600s env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-ops" --config "$PVP_BASE/ops/ops.toml" "$@"; }
pvp_fail() { echo "--- PvP node log"; tail -n 120 "$WORK/pvp-node.log" 2>/dev/null || true; fail "$@"; }
pvp_await_log() { # pattern seconds
  for _ in $(seq 1 "$2"); do grep -q "$1" "$WORK/pvp-node.log" && return 0; sleep 1; done
  pvp_fail "missing PvP node event: $1"
}

pvp_world_ready() {
  local boot_started=$SECONDS pvp_url="${ADMIN_URL%/*}/$PVP_DATABASE"
  echo "CREATE DATABASE $PVP_DATABASE" | psql_admin postgres
  OTERYN_GAME_MIGRATION_DATABASE_URL="$pvp_url" "$BASE/bin/oteryn-game-migrate"
  echo "GRANT CONNECT ON DATABASE $PVP_DATABASE TO nb_control, nb_runtime" | psql_admin "$PVP_DATABASE"
  sudo install -d -o root -g root -m 0755 "$PVP_BASE" "$PVP_BASE/state" "$PVP_BASE/fence-parent" "$PVP_BASE/node"
  sudo install -d -o root -g root -m 0700 "$PVP_BASE/ops"
  sudo install -d -o "$SERVICE_UID" -m 0700 "$PVP_BASE/fence-parent/fence" "$PVP_BASE/run"
  sed -e "s|^state_directory = .*|state_directory = \"$PVP_BASE/state\"|" \
    -e "s|^database = .*|database = \"$PVP_DATABASE\"|" \
    -e "s|^fence_directory = .*|fence_directory = \"$PVP_BASE/fence-parent/fence\"|" \
    "$WORK/ops.toml" > "$WORK/pvp-ops.toml"
  sudo install -o root -m 0600 "$WORK/pvp-ops.toml" "$PVP_BASE/ops/ops.toml"
  pvp_ops authorization issue --file launch-pvp.json --binding node-boot-pvp
  # The existing World's launch-a configuration with this World's scope, port, store and paths.
  sed -e "s|^address = .*|address = \"127.0.0.1:$PVP_GAME_PORT\"|" \
    -e "s|^world_id = .*|world_id = \"$PVP_WORLD_ID\"|" \
    -e "s|^channel_id = .*|channel_id = \"$PVP_CHANNEL_ID\"|" \
    -e "s|^database = .*|database = \"$PVP_DATABASE\"|" \
    -e "s|^fence_directory = .*|fence_directory = \"$PVP_BASE/fence-parent/fence\"|" \
    -e "s|^socket_path = .*|socket_path = \"$PVP_BASE/run/control.sock\"|" \
    -e "s|^authorization_file = .*|authorization_file = \"$PVP_BASE/state/launch-pvp.json\"|" \
    -e "s|^s2_authorization_file = .*|s2_authorization_file = \"$PVP_BASE/state/s2-fresh-store.json\"|" \
    "$WORK/node.toml" > "$WORK/pvp-node.toml"
  local line
  for line in "address = \"127.0.0.1:$PVP_GAME_PORT\"" "world_id = \"$PVP_WORLD_ID\"" "channel_id = \"$PVP_CHANNEL_ID\"" \
    "database = \"$PVP_DATABASE\"" "socket_path = \"$PVP_BASE/run/control.sock\"" "[world_bundle]"; do
    grep -qxF "$line" "$WORK/pvp-node.toml" || fail "PvP node configuration lacks: $line"
  done
  sudo install -o root -m 0644 "$WORK/pvp-node.toml" "$PVP_BASE/node/node.toml"
  pvp_ops s2 issue --node-config "$PVP_BASE/node/node.toml" --file s2-fresh-store.json --namespace node-boot-pvp --authorization disposable-qualification
  pvp_ops character fresh-store --request fresh-store.json
  pvp_ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"
  start_node_process "$WORK/pvp-node.log" "$PVP_BASE/node/node.toml"
  remember PVP_PROCESS_DIR "$NODE_PROCESS_DIR"
  pvp_await_log awaiting_assignment 120
  local node revision
  node="$(node_field "$WORK/pvp-node.log" node_id)"; revision="$(node_field "$WORK/pvp-node.log" registration_revision)"
  psql_admin "$PVP_DATABASE" <<SQL
INSERT INTO game_control_scope_grants SELECT 'nb_control', '$PVP_WORLD_ID', '$PVP_CHANNEL_ID', op FROM unnest(ARRAY[1,2,3,4]::smallint[]) op;
INSERT INTO game_control_field_policy_grants VALUES ('nb_control', '$PVP_WORLD_ID', 'policy-s3b-1');
SQL
  # Open PvP (world_type=2) under the same readiness revision; the granted login inserts it.
  docker exec -i -e PGPASSWORD="$CONTROL_PASSWORD" "$PG_CONTAINER" \
    psql -v ON_ERROR_STOP=1 -qAt -h 127.0.0.1 -U nb_control -d "$PVP_DATABASE" <<SQL
INSERT INTO game_spell_field_world_policies (world_id, world_policy_revision, world_type, protection_level, in_fight_ms, source_pin, control_role, decision_identity)
VALUES ('$PVP_WORLD_ID', 'policy-s3b-1', 2, 8, 60000, '99902524e052f37574194466c2949c576e4ab269', session_user, 'node-boot-qualification');
SQL
  [[ "$(echo "SELECT world_type FROM game_spell_field_world_policies WHERE world_id='$PVP_WORLD_ID'" | psql_admin "$PVP_DATABASE")" == 2 ]] || fail "PvP World policy is not Open PvP"
  pvp_ops content activate --request content-1.json --world "$PVP_WORLD_ID" --channel "$PVP_CHANNEL_ID" --sequence 1 --previous empty
  pvp_ops assignment assign --request assign-pvp.json --world "$PVP_WORLD_ID" --channel "$PVP_CHANNEL_ID" --node-id "$node" --revision "$revision"
  pvp_await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  pvp_await_log "event=readiness .*ready=true" 60
  local bound_address
  bound_address="$(sed -nE 's/.* event=listener_bound (trace=[^ ]+ )?detail="address=([^"]+)"$/\2/p' "$WORK/pvp-node.log" | tail -n 1)"
  if [[ ! "$bound_address" =~ ^127\.0\.0\.1:([1-9][0-9]{3,4})$ ]] || (( BASH_REMATCH[1] > 65535 )); then
    fail "PvP node did not report a valid bound listener"; return 1
  fi
  PVP_GAME_PORT="${BASH_REMATCH[1]}"
  remember PVP_GAME_PORT "$PVP_GAME_PORT"
  local epoch_result
  epoch_result="$({
    echo 'PREPARE node_boot_world_epoch(UUID, UUID, TEXT) AS'
    cat "$GAME_SOURCE/tools/qualification/node_boot/world_epoch.sql"
    printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s');\n" "$PVP_WORLD_ID" "$PVP_CHANNEL_ID" "$WORLD_DIGEST"
  } | psql_admin "$PVP_DATABASE")"
  [[ "$epoch_result" == t ]] || fail "PvP World epoch initialization"
  [[ "$(sudo stat -c '%u %a' "$PVP_BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "PvP control socket mode"
  evidence "pvp_world world_type=2 database=$PVP_DATABASE port=$PVP_GAME_PORT node=separate_process readiness=true world_epoch=0 boot_seconds=$((SECONDS - boot_started))"
}

pvp_character_bootstrap() { # level-qualifiable Characters from real Platform intents
  local index account operation name first failed=0
  php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); foreach ([["'"${PVP_ACCOUNT_IDS[0]}"'","nb-five@example.invalid"],["'"${PVP_ACCOUNT_IDS[1]}"'","nb-six@example.invalid"],["'"${PVP_ACCOUNT_IDS[2]}"'","nb-rune-caster@example.invalid"],["'"${PVP_ACCOUNT_IDS[3]}"'","nb-rune-target@example.invalid"]] as [$accountId,$email]) { Illuminate\Support\Facades\DB::table("identities")->insert(["email"=>$email,"password"=>password_hash(bin2hex(random_bytes(24)),PASSWORD_BCRYPT),"account_id"=>$accountId,"native_security_generation"=>1,"created_at"=>now(),"updated_at"=>now()]); }'
  for index in "${!PVP_ACCOUNT_IDS[@]}"; do
    account="${PVP_ACCOUNT_IDS[$index]}"; operation="${PVP_INTENT_OPERATIONS[$index]}"; name="${PVP_INTENT_NAMES[$index]}"
    if ! php_exec 'require "vendor/autoload.php"; $app=require "bootstrap/app.php"; $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap(); $id=Illuminate\Support\Facades\DB::table("identities")->where("account_id","'"$account"'")->value("id"); $code=Illuminate\Support\Facades\Artisan::call("game-auth:character-bootstrap-intent:issue",["--identity-id"=>(string)$id,"--operation-id"=>"'"$operation"'","--target-world-id"=>"'"$PVP_WORLD_ID"'","--requested-name"=>"'"$name"'","--requested-sex"=>"Female","--profile-revision"=>"'"${INTERPRETATION[0]}"'","--ruleset-revision"=>"'"${INTERPRETATION[1]}"'","--content-revision"=>"'"${INTERPRETATION[2]}"'","--starter-template-revision"=>"'"${INTERPRETATION[3]}"'"]); fwrite(STDERR, Illuminate\Support\Facades\Artisan::output()); exit($code);'; then
      echo "platform intent issue failed for PvP operation $operation"
      failed=1
      continue
    fi
    first=$(pvp_ops character bootstrap --socket "$PVP_BASE/run/control.sock" --operation-id "$operation") || { failed=1; continue; }
    [[ "$first" == 'character_bootstrap=committed' ]] || { failed=1; continue; }
    printf '%s\n' "$first"
    assert_operator_replay "$PVP_DATABASE" "SELECT row_to_json(r) FROM game_character_operation_receipts r WHERE operation_id='$operation'" \
      "$first" pvp_ops character bootstrap --socket "$PVP_BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]] || pvp_fail "PvP Character bootstrap"
  [[ "$(echo "SELECT count(*) FROM game_character_roots WHERE world_id='$PVP_WORLD_ID'" | psql_admin "$PVP_DATABASE")" == "${#PVP_ACCOUNT_IDS[@]}" ]] || fail "PvP World Characters"
  evidence "pvp_characters=${#PVP_ACCOUNT_IDS[@]} world=$PVP_WORLD_ID source=platform_intents path=pvp_control_socket retry=idempotent"
}

pvp_world_shutdown() { # the Open PvP World's node ends cleanly before the lifecycle probes
  stop_node "$PVP_PROCESS_DIR"
  grep -q "event=shutdown .*state=complete" "$WORK/pvp-node.log" || pvp_fail "PvP node did not shut down cleanly"
  [[ ! -e "$PVP_BASE/run/control.sock" ]] || fail "PvP socket left after graceful shutdown"
  assert_shutdown_guards "$PVP_DATABASE"
  evidence "pvp_world shutdown=complete socket=removed"
}

if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
  game_platform_identity
  exit "$?"
fi

# Some client journeys are capture-gated, and the quest reporter requires
# its capture directory to be inside the Game worktree.
if [[ "$NODE_BOOT_WORLD" == 1 && -z "${E2E_ALL_CAPTURE_DIR:-}" ]]; then
  WORLD_CAPTURE_DIR="$(mktemp -d "$GAME_SOURCE/.node-boot-captures.XXXXXX")"
fi

seam_stage_name=seam_stages
[[ "$NODE_BOOT_SPELLS" != 1 ]] || seam_stage_name=spell_scenarios
if [[ "$NODE_BOOT_REUSE_STACK" == 1 ]]; then
  stage "$seam_stage_name" -- seam_stages
else
stage operator_setup -- operator_setup
stage configuration_negatives operator_setup -- configuration_negatives
stage node_assigned_ready operator_setup -- node_assigned_ready
stage control_socket_peer node_assigned_ready -- control_socket_peer
stage character_bootstrap node_assigned_ready -- character_bootstrap
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # MP01's PvP rows report this World's absence themselves, so the seam stage does not need it.
  stage pvp_world_ready node_assigned_ready -- pvp_world_ready
  stage pvp_character_bootstrap pvp_world_ready -- pvp_character_bootstrap
fi
stage "$seam_stage_name" character_bootstrap -- seam_stages
fi
if [[ "$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then
[[ "$NODE_BOOT_WORLD" != 1 ]] || stage pvp_world_shutdown pvp_world_ready -- pvp_world_shutdown
stage graceful_shutdown node_assigned_ready -- graceful_shutdown
stage restart_without_supersession graceful_shutdown -- restart_without_supersession
stage superseding_replacement graceful_shutdown -- superseding_replacement
stage signal_before_ready superseding_replacement -- signal_before_ready
NODE_PID=""
elif [[ ${STATUS[node_assigned_ready]:-PASS} == PASS ]]; then
  # The shutdown and restart stages did not run, so this is never a qualification pass.
  if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
    # The node was started inside a stage subshell; its pid is in $VARS.
    # shellcheck source=/dev/null
    source "$VARS"
    [[ -z "${NODE_PROCESS_DIR:-}" ]] || save_reuse_state
  fi
  ORDER+=(stack_kept)
  STATUS[stack_kept]="SKIP(shutdown and restart stages not run)"
fi

# The failed rows of this pass, grouped by their first cause, ahead of the stage table.
python3 "$GAME_SOURCE/tools/qualification/node_boot/failure_summary.py" "$WORK"/stage-*.log \
  ${GITHUB_STEP_SUMMARY:+--summary "$GITHUB_STEP_SUMMARY"} || true
echo "NODE_BOOT_STAGES"
failures=0
for name in "${ORDER[@]}"; do
  printf 'NODE_BOOT_STAGE %-30s %s\n' "$name" "${STATUS[$name]}"
  [[ ${STATUS[$name]} == PASS ]] || failures=$((failures + 1))
done
unset FRESH_SEED_HEX RECOVERY_SEED_HEX
if [[ $failures == 0 ]]; then
  selected_groups="${NODE_BOOT_E2E_ONLY:-}"
  if [[ "$NODE_BOOT_WORLD" == 1 && ( -n "${selected_groups//[[:space:]]/}" || -v OTERYN_E2E_ROW_BATCH || -n "${E2E_ALL_CAPTURE_DIR:-}" ) ]]; then
    result=NODE_BOOT_SELECTION_PASS
  else
    result=NODE_BOOT_PASS
  fi
elif [[ $failures == 1 && ${STATUS[stack_kept]:-} == SKIP* ]]; then
  # Every stage that ran passed and the stack stays up; still not a qualification pass.
  result=NODE_BOOT_PARTIAL
  exit 3
else
  exit 1
fi
