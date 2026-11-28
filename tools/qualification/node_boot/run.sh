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

readonly PLATFORM_SHA=6d04b52a16145723eedb0c56640ad052dbd26e94
readonly TOPOLOGY_REVISION=wp5-node-boot-v1
readonly ACCOUNT_ID=01934f10-7c00-7000-8000-000000000001
readonly SECOND_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000002
readonly PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000003
readonly SECOND_PACKET_ACCOUNT_ID=01934f10-7c00-7000-8000-000000000004
readonly EXIVA_ACCOUNT_ID=01934f10-7c00-7000-8000-00000000000e
readonly LIGHT_ACCOUNT_ID=01934f10-7c00-7000-8000-00000000000d
readonly EXTRA_WORLD_ACCOUNTS=(01934f10-7c00-7000-8000-000000000007 01934f10-7c00-7000-8000-000000000008 01934f10-7c00-7000-8000-000000000009 01934f10-7c00-7000-8000-00000000000a 01934f10-7c00-7000-8000-00000000000b 01934f10-7c00-7000-8000-00000000000c)
readonly F5_C17A_ACCOUNT=01934f10-7c00-7000-8000-00000000000f
readonly F5_C17C_ACCOUNT=01934f10-7c00-7000-8000-000000000010
readonly F5_C13_ACCOUNT=01934f10-7c00-7000-8000-000000000011
readonly F5_C18A_ACCOUNT=01934f10-7c00-7000-8000-000000000012
readonly F5_C17D_ACCOUNT=01934f10-7c00-7000-8000-000000000013
readonly C11_UNASSIGNED_ACCOUNT=01934f10-7c00-7000-8000-000000000014
readonly C11_KNIGHT_ACCOUNT=01934f10-7c00-7000-8000-000000000015
readonly C12_MONK_ACCOUNT=01934f10-7c00-7000-8000-000000000016
readonly S13_ACCOUNT=01934f10-7c00-7000-8000-000000000017
readonly W19A_ACCOUNT=01934f10-7c00-7000-8000-000000000019
readonly C05_TARGET_ACCOUNT=01934f10-7c00-7000-8000-00000000001a
readonly S13_PEER_ACCOUNT=01934f10-7c00-7000-8000-00000000001b
readonly C06_CASTER_ACCOUNT=01934f10-7c00-7000-8000-00000000001c
readonly C06_PEER_ACCOUNT=01934f10-7c00-7000-8000-00000000001d
readonly W18_ACCOUNT=01934f10-7c00-7000-8000-00000000001e
readonly C01_ACCOUNT=01934f10-7c00-7000-8000-00000000001f
readonly P23C_ACCOUNT=01934f10-7c00-7000-8000-000000000020
readonly FRESH_KEY_ID=node-boot-fresh-key-1
readonly RECOVERY_KEY_ID=node-boot-recovery-key-1
readonly WORLD_ID=01934f10-7c02-7001-805b-3b1122334401
readonly CHANNEL_ID=01934f10-7c03-7001-805b-3b1122334401
readonly INTENT_OPERATIONS=(01934f10-7c04-7001-805b-3b1122334401 01934f10-7c04-7002-805b-3b1122334402 01934f10-7c04-7003-805b-3b1122334403 01934f10-7c04-7004-805b-3b1122334404 01934f10-7c04-7007-805b-3b1122334407 01934f10-7c04-7008-805b-3b1122334408 01934f10-7c04-7009-805b-3b1122334409 01934f10-7c04-700a-805b-3b112233440a 01934f10-7c04-700b-805b-3b112233440b 01934f10-7c04-700c-805b-3b112233440c 01934f10-7c04-700d-805b-3b112233440d 01934f10-7c04-700e-805b-3b112233440e 01934f10-7c04-700f-805b-3b112233440f 01934f10-7c04-7010-805b-3b1122334410 01934f10-7c04-7011-805b-3b1122334411 01934f10-7c04-7012-805b-3b1122334412 01934f10-7c04-7013-805b-3b1122334413 01934f10-7c04-7014-805b-3b1122334414 01934f10-7c04-7015-805b-3b1122334415 01934f10-7c04-7016-805b-3b1122334416 01934f10-7c04-7017-805b-3b1122334417 01934f10-7c04-7019-805b-3b1122334419 01934f10-7c04-701a-805b-3b112233441a 01934f10-7c04-701b-805b-3b112233441b 01934f10-7c04-701c-805b-3b112233441c 01934f10-7c04-701d-805b-3b112233441d 01934f10-7c04-701e-805b-3b112233441e 01934f10-7c04-701f-805b-3b112233441f 01934f10-7c04-7020-805b-3b1122334420)
# The v4 bootstrap intent explicitly binds name, sex and vocation.
readonly INTENT_NAMES=("Qualification One" "Qualification Two" "Packet One" "Packet Two" "Party One" "Party Two" "Party Three" "Party Four" "Sweep One" "Sweep Two" "Light Caster" "Locate Caster" "Haste Caster" "Illusion Caster" "Summon Caster" "Aim Caster" "Challenge Knight" "Oressa Candidate" "Wheel Knight" "Augment Monk" "Party Caster" "Seasonal Bundle Buyer" "Locate Target" "Party Spell Peer" "Rune Caster" "Rune Refusal Target" "House Transfer Player" "Armored Rat Hunter" "Weekly Task Hunter")
# Runtime build writers bind to the shipped training formula's content revision,
# just as the S3-B seam does; progression is initialized from this interpretation.
# The served readiness.content_revision is this same revision: the quest and Task Board
# owners refuse a Character whose stored progression content differs from the served one.
readonly INTERPRETATION=(nb-profile-1 nb-ruleset-1 s3b-content-1 newhaven-starter-r1)
# The second, Open PvP (world_type=2) World of the World pass: its own Accounts, intents and node.
readonly PVP_WORLD_ID=01934f10-7c02-7002-805b-3b1122334402
readonly PVP_CHANNEL_ID=01934f10-7c03-7002-805b-3b1122334402
readonly PVP_ACCOUNT_IDS=(01934f10-7c00-7000-8000-000000000005 01934f10-7c00-7000-8000-000000000006)
readonly PVP_INTENT_OPERATIONS=(01934f10-7c04-7005-805b-3b1122334405 01934f10-7c04-7006-805b-3b1122334406)
readonly PVP_INTENT_NAMES=("Arena One" "Arena Two")
readonly PVP_DATABASE=oteryn_node_boot_pvp
readonly COMPOSE_FILE=tools/qualification/wp5_s3a/compose.yml
readonly COMPOSE_S3B=tools/qualification/wp5_s3b/compose.override.yml
readonly COMPOSE_OVERLAY=tools/qualification/node_boot/compose.override.yml
readonly PG_IMAGE=postgres:18.6-bookworm@sha256:bbf83594798e144bff365a9c191c5d779f181fb3135902064ab8ff11918020fc
# Local parallel stacks override the service user, base directory, Compose project and
# ports; CI keeps the defaults. Everything is checked before any sudo or Docker use:
# cleanup kills the service user's processes and removes BASE as root.
readonly NODE_BOOT_UNPRIVILEGED="${NODE_BOOT_UNPRIVILEGED:-0}"
result_mode=""
as_operator=(sudo)
as_service=()
if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  result_mode=" mode=unprivileged"
  as_operator=()
fi
blocked() { printf 'NODE_BOOT_RESULT=BLOCKED reason=%s%s\n' "$1" "$result_mode"; exit 2; }
if [[ "$NODE_BOOT_UNPRIVILEGED" != 1 ]]; then
  SERVICE_USER="${NODE_BOOT_SERVICE_USER:-oteryn-node-boot}"
  BASE="${NODE_BOOT_BASE:-/srv/oteryn-node-boot}"
  [[ "$SERVICE_USER" =~ ^oteryn-node-boot(-[a-z0-9]{1,16})?$ ]] || blocked invalid_service_user
  node_boot_tmp="$(realpath -m -- "${TMPDIR:-/tmp}")"
  if [[ "$BASE" != "$(realpath -m -- "$BASE")" ]] \
    || ! [[ "$BASE" == /srv/?* || ( "$node_boot_tmp" != / && "$BASE" == "$node_boot_tmp"/?* ) ]]; then
    blocked base_outside_srv_or_tmpdir
  fi
  readonly SERVICE_USER BASE
  as_service=(sudo -u "$SERVICE_USER")
fi
[[ -z "${NODE_BOOT_PROJECT+x}" || "$NODE_BOOT_PROJECT" =~ ^[a-z0-9][a-z0-9_-]{0,47}$ ]] || blocked invalid_project
for node_boot_port in NODE_BOOT_PG_PORT NODE_BOOT_PLATFORM_PORT NODE_BOOT_GAME_PORT NODE_BOOT_PVP_GAME_PORT; do
  [[ -z "${!node_boot_port+x}" || ( "${!node_boot_port}" =~ ^[1-9][0-9]{3,4}$ && "${!node_boot_port}" -le 65535 ) ]] \
    || blocked "invalid_port_${node_boot_port,,}"
done
# Port zero lets the real PvP listener bind atomically and report its assigned port.
PVP_GAME_PORT="${NODE_BOOT_PVP_GAME_PORT:-0}"
# NODE_BOOT_KEEP_STACK=1 leaves the stack up after the seam stage; NODE_BOOT_REUSE_STACK=1
# reruns only the seam stage against it, =teardown removes it. Local development only.
NODE_BOOT_KEEP_STACK="${NODE_BOOT_KEEP_STACK:-0}"
NODE_BOOT_REUSE_STACK="${NODE_BOOT_REUSE_STACK:-0}"
case "$NODE_BOOT_KEEP_STACK/$NODE_BOOT_REUSE_STACK" in
  0/0) ;;
  1/0|0/1|0/teardown)
    if [[ "${CI:-}" == true || -n "${GITHUB_ACTIONS:-}" ]]; then
      blocked stack_reuse_is_local_only
    fi ;;
  *) blocked invalid_stack_reuse_mode ;;
esac


if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 && "$NODE_BOOT_KEEP_STACK/$NODE_BOOT_REUSE_STACK" != 0/0 ]]; then
  blocked unprivileged_requires_disposable_stack
fi

GAME_SOURCE="$(git rev-parse --show-toplevel)"
# The separately pinned portal-rs lane owns only Platform lifecycle and its stages.
# A fixed repository path prevents an environment variable from sourcing arbitrary code.
case "${NODE_BOOT_LIFECYCLE:-}" in
  ''|native_reference)
    NODE_BOOT_LIFECYCLE=native_reference
    source "$GAME_SOURCE/tools/qualification/game_platform/lifecycle.sh"
    ;;
  game_platform)
    [[ "$NODE_BOOT_UNPRIVILEGED" != 1 ]] || blocked unprivileged_requires_qualification_spawn
    # shellcheck source=tools/qualification/game_platform/lifecycle.sh
    source "$GAME_SOURCE/tools/qualification/game_platform/lifecycle.sh"
    game_platform_validate
    ;;
  *) blocked unknown_lifecycle ;;
esac
for program in docker python3 node openssl cargo "${as_operator[@]}" psql setsid timeout; do
  command -v "$program" >/dev/null 2>&1 || blocked "missing_binary_$program"
done
NODE_BOOT_PLATFORM_PIN="$PLATFORM_SHA"
NODE_BOOT_TOPOLOGY="$TOPOLOGY_REVISION"
if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform || "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
  # portal-rs pin is owned by game_platform/lifecycle.sh (includes Platform #1833).
  NODE_BOOT_PLATFORM_PIN="$GAME_PLATFORM_SHA"
  NODE_BOOT_TOPOLOGY=portal-rs-game-platform-v1
fi
readonly NODE_BOOT_PLATFORM_PIN NODE_BOOT_TOPOLOGY
export NODE_BOOT_PROFILE="${NODE_BOOT_PROFILE:-sorcerer}"
case "$NODE_BOOT_PROFILE" in
  explorer|knight|paladin|sorcerer|druid|monk) ;;
  *) blocked invalid_qualification_profile ;;
esac
# Optional full spell input uses this same disposable topology and admission path.
NODE_BOOT_SPELLS="${NODE_BOOT_SPELLS:-0}"
case "$NODE_BOOT_SPELLS" in
  0|1) ;;
  *) blocked invalid_spell_mode ;;
esac
# Keep every fixture-only transport/door assertion intact in its own disposable pass.
NODE_BOOT_WORLD="${NODE_BOOT_WORLD:-1}"
case "$NODE_BOOT_WORLD" in 0|1) ;; *) blocked invalid_world_mode ;; esac
NODE_BOOT_D150_LEGACY="${NODE_BOOT_D150_LEGACY:-0}"
case "$NODE_BOOT_D150_LEGACY" in
  0) ;;
  1)
    if [[ "$NODE_BOOT_LIFECYCLE/$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/$NODE_BOOT_KEEP_STACK/$NODE_BOOT_REUSE_STACK" != native_reference/1/0/0/0 \
      || -n "${NODE_BOOT_E2E_ONLY:-}" || "${CI:-}" == true || -n "${GITHUB_ACTIONS:-}" ]]; then
      blocked d150_requires_disposable_local_world
    fi
    NODE_BOOT_SKIP_BASIC=1
    ;;
  *) blocked invalid_d150_mode ;;
esac
NODE_BOOT_WORLD_PROFILE="${NODE_BOOT_WORLD_PROFILE:-town}"
case "$NODE_BOOT_WORLD_PROFILE" in
  # Real Thais/town outskirts and the default-start and seller regions.
  town)
    WORLD_REGIONS="$(<"$GAME_SOURCE/tools/qualification/node_boot/town-regions.txt");125,124"
    selected_groups="${NODE_BOOT_E2E_ONLY:-}"
    case ",${selected_groups//[[:space:]]/}," in
      ,,|*,rows,*) WORLD_REGIONS+=";131,122" ;;
    esac
    ;;
  quests) WORLD_REGIONS='125,121;126,124;126,125;126,126;126,127;127,123;127,125;127,126;127,127;128,122;128,125;128,126;128,128;129,123;129,124;129,126;129,127;130,126;132,125;133,123;133,124' ;;
  *) blocked invalid_world_profile ;;
esac
readonly NODE_BOOT_WORLD_PROFILE WORLD_REGIONS
if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then NODE_BOOT_WORLD=0; fi
BOOTSTRAP_ACCOUNTS=("$ACCOUNT_ID" "$SECOND_ACCOUNT_ID")
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  # Packet journeys may move, loot or kill their Characters before the required core runs.
  BOOTSTRAP_ACCOUNTS+=("$PACKET_ACCOUNT_ID" "$SECOND_PACKET_ACCOUNT_ID" "${EXTRA_WORLD_ACCOUNTS[@]}" "$LIGHT_ACCOUNT_ID" "$EXIVA_ACCOUNT_ID" "$F5_C17A_ACCOUNT" "$F5_C17C_ACCOUNT" "$F5_C13_ACCOUNT" "$F5_C18A_ACCOUNT" "$F5_C17D_ACCOUNT" "$C11_UNASSIGNED_ACCOUNT" "$C11_KNIGHT_ACCOUNT" "$C12_MONK_ACCOUNT" "$S13_ACCOUNT" "$W19A_ACCOUNT" "$C05_TARGET_ACCOUNT" "$S13_PEER_ACCOUNT" "$C06_CASTER_ACCOUNT" "$C06_PEER_ACCOUNT" "$W18_ACCOUNT" "$C01_ACCOUNT" "$P23C_ACCOUNT")
fi
readonly BOOTSTRAP_ACCOUNTS
source "$GAME_SOURCE/tools/qualification/node_boot/activation_wait.sh"
NODE_ACTIVATION_WAIT_SECONDS="$(node_activation_wait_seconds "$NODE_BOOT_WORLD" "$NODE_BOOT_SPELLS")"
NODE_GAMEPLAY_ENV=()
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Default: the canonical imported book. The r21 reference artifact stays selectable explicitly.
  NODE_BOOT_SPELL_MANIFEST="${NODE_BOOT_SPELL_MANIFEST:-$GAME_SOURCE/content/spells.manifest.json}"
  NODE_BOOT_SPELL_MANIFEST="$(realpath -e -- "$NODE_BOOT_SPELL_MANIFEST" 2>/dev/null)" || blocked spell_manifest_missing
  [[ -f "$NODE_BOOT_SPELL_MANIFEST" ]] || blocked spell_manifest_missing
  # The staging root is the repository root: the manifest keeps its repository-relative path.
  [[ "$NODE_BOOT_SPELL_MANIFEST" == "$GAME_SOURCE"/* ]] || blocked spell_manifest_outside_repository
  NODE_BOOT_SPELL_MANIFEST_REL="${NODE_BOOT_SPELL_MANIFEST#"$GAME_SOURCE"/}"
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
  blocked exact_platform_checkout_missing
fi
readonly SPAWN_PROOF=node::serve::qualification_fixture::tests::qualification_spawn_transaction_returns_pool_and_bounds_backend
NODE_SPAWN_FEATURES=(--features qualification-spawn)
if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  [[ -z "${NODE_BOOT_PREBUILT_DIR:-}" ]] || blocked unprivileged_requires_local_game_build
  NODE_SPAWN_FEATURES=(--features qualification-unprivileged)
  work_parent="$(realpath -e -- "${RUNNER_TEMP:-/tmp}")" || blocked unprivileged_work_ancestor_not_owned
  [[ -d "$work_parent" ]] || blocked unprivileged_work_ancestor_not_owned
  [[ "$work_parent" != /srv && "$work_parent" != /srv/* ]] || blocked unprivileged_work_under_srv
  # game-ops checks the full state-directory path, including its root.
  ancestor="$work_parent"
  while :; do
    owner="$(stat -c %u -- "$ancestor")" || blocked unprivileged_work_ancestor_not_owned
    [[ "$owner" == 0 || "$owner" == "$EUID" ]] || blocked unprivileged_work_ancestor_not_owned
    [[ "$ancestor" != / ]] || break
    ancestor="${ancestor%/*}"
    ancestor="${ancestor:-/}"
  done
fi

if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  node_boot_ports="$(python3 "$GAME_SOURCE/tools/qualification/game_platform/prepare.py" --select-ports)" \
    || blocked listener_port_allocation_failed
  while IFS= read -r node_boot_port; do
    export "$node_boot_port"
  done <<< "$node_boot_ports"
  printf 'NODE_BOOT_PORTS %s NODE_BOOT_PVP_GAME_PORT=%s\n' "${node_boot_ports//$'\n'/ }" "$PVP_GAME_PORT"
fi

# A kept or reused stack is one topology, so it never runs the separate basic pass.
# The game_platform lifecycle owns one Platform stack, so it never runs the basic pass either.
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_SPELLS" == 0 && "${NODE_BOOT_SKIP_BASIC:-0}" != 1 \
  && "${NODE_BOOT_LIFECYCLE:-}" == native_reference \
  && "$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then
  NODE_BOOT_WORLD=0 bash "$GAME_SOURCE/tools/qualification/node_boot/run.sh"
fi
NODE_BOOT_LIBTEST=""
WORLD_COMPILER=(cargo +1.100.0 run --locked --release -p oteryn-world-bundle-compiler --)
if [[ "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  # The qualification clock is available only in disposable World runs.
  NODE_CLOCK_FEATURES=()
  export OTERYN_NODE_BOOT_REAL_CLOCK="${OTERYN_NODE_BOOT_REAL_CLOCK:-0}"
  case "$OTERYN_NODE_BOOT_REAL_CLOCK" in
    0|1) ;;
    *) blocked invalid_real_clock_mode ;;
  esac
  variant=qualification-spawn
  if [[ "$NODE_BOOT_LIFECYCLE" == game_platform ]]; then
    NODE_SPAWN_FEATURES=()
    variant=default
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 && "$OTERYN_NODE_BOOT_REAL_CLOCK" == 0 ]]; then
    NODE_CLOCK_FEATURES=(--features qualification-clock)
    variant=qualification-clock
  fi
  TARGET="${CARGO_TARGET_DIR:-$GAME_SOURCE/target}/debug"
  if [[ -n "${NODE_BOOT_PREBUILT_DIR:-}" ]]; then
    prebuilt="$GAME_SOURCE/tools/qualification/node_boot/prebuilt.py"
    if ! python3 "$prebuilt" verify "$variant" "$NODE_BOOT_PREBUILT_DIR"; then
      blocked invalid_prebuilt_game_artifact
    fi
    TARGET="$(realpath -e -- "$NODE_BOOT_PREBUILT_DIR/$variant")"
    NODE_BOOT_LIBTEST="$TARGET/node-boot-libtest"
    if [[ "$NODE_BOOT_LIFECYCLE" == native_reference ]] &&
      ! "$NODE_BOOT_LIBTEST" --list --exact "$SPAWN_PROOF" | grep -Fx "$SPAWN_PROOF: test" >/dev/null; then
      blocked prebuilt_qualification_spawn_missing
    fi
    if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
      if ! python3 "$prebuilt" verify world-compiler "$NODE_BOOT_PREBUILT_DIR"; then
        blocked invalid_prebuilt_world_compiler
      fi
      compiler_dir="$(realpath -e -- "$NODE_BOOT_PREBUILT_DIR/world-compiler")"
      WORLD_COMPILER=("$compiler_dir/oteryn-world-bundle-compiler")
    fi
  fi
fi
if [[ "$NODE_BOOT_SPELLS" == 1 && "$NODE_BOOT_REUSE_STACK" == 0 ]]; then
  # Execute the existing compile/decode/stage qualification before creating services.
  RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}" \
    bash "$GAME_SOURCE/tools/qualification/spells/run.sh" map "$NODE_BOOT_SPELL_MANIFEST"
fi

if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  WORK="$(mktemp -d "${work_parent%/}/node-boot.XXXXXX")"
  BASE="$WORK/deploy"
else
  WORK="$(mktemp -d "${RUNNER_TEMP:-/tmp}/node-boot.XXXXXX")"
fi
readonly BASE PVP_BASE="$BASE/pvp"
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  NODE_GAMEPLAY_ENV=("OTERYN_NATIVE_GAMEPLAY_MANIFEST=$BASE/gameplay/$NODE_BOOT_SPELL_MANIFEST_REL")
fi
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
export GAME_SOURCE PLATFORM_SOURCE WP5_PKI WP5_SCRATCH WP5_PORT WP5_PROJECT
export WP5_DB_PASSWORD WP5_DB_ROOT_PASSWORD WP5_APP_KEY WP5_TOPOLOGY_REVISION WP5_FSYNC_FAULT
if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  WP5_PROJECT="${WP5_PROJECT}-${WORK##*.}"
  WP5_PROJECT="${WP5_PROJECT,,}"
fi
PG_CONTAINER="${WP5_PROJECT}-postgres"
PG_PORT="${NODE_BOOT_PG_PORT:-15433}"
PG_ADMIN_PASSWORD="$(openssl rand -hex 24)"
CONTROL_PASSWORD="$(openssl rand -hex 24)"
RUNTIME_PASSWORD="$(openssl rand -hex 24)"
NODE_PID=""
RUNTIME_DIAGNOSTICS_PID=""
PVP_RUNTIME_DIAGNOSTICS_PID=""
ACTIVE_STAGE=setup
# Redacted PostgreSQL samples and stage dumps (pg_observe.py); kept with node.log on failure.
PG_OBS="$WORK/postgres"
PG_SAMPLER_PID=""
STACK_KEPT=0
REUSE_STATE="${TMPDIR:-/tmp}/node-boot-reuse-$WP5_PROJECT.env"
if [[ "$NODE_BOOT_REUSE_STACK" != 0 ]]; then
  rm -rf -- "$WORK"
  # Owner-only state from the NODE_BOOT_KEEP_STACK run; it holds the disposable stack's secrets.
  if [[ ! -f "$REUSE_STATE" || -L "$REUSE_STATE" || ! -O "$REUSE_STATE" || "$(stat -c %a "$REUSE_STATE")" != 600 ]]; then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_state_missing'; exit 2
  fi
  # shellcheck source=/dev/null
  source "$REUSE_STATE"
  kept_mode="${kept_mode:-}"
  export NODE_BOOT_GAME_PORT="${NODE_BOOT_GAME_PORT:-${kept_mode##*/}}"
  if [[ "${kept_mode:-}" != "$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/${NODE_BOOT_GAME_PORT:-17181}" ]]; then
    echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_mode_mismatch'; exit 2
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
    if [[ "$(docker inspect -f '{{.State.Running}}' "$PG_CONTAINER" 2>/dev/null)" != true ]] \
      || ! sudo kill -0 "$NODE_PID" 2>/dev/null || ! sudo test -S "$BASE/run/control.sock"; then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_not_running'; exit 2
    fi
    if [[ -n "${NODE_BOOT_PVP_GAME_PORT+x}" && "${kept_pvp_port:-0}" != "$PVP_GAME_PORT" ]]; then
      echo 'NODE_BOOT_RESULT=BLOCKED reason=kept_stack_mode_mismatch'; exit 2
    fi
    PVP_GAME_PORT="${kept_pvp_port:-$PVP_GAME_PORT}"
    STACK_KEPT=1
  fi
fi

# Seed only this run's extra synthetic accounts through the same portal binary
# and disposable environment as the upstream harness and composition.py.
seed_mp_accounts() {
  python3 - "$WORK" "$PVP_WORLD_ID" "${PVP_ACCOUNT_IDS[@]}" "${EXTRA_WORLD_ACCOUNTS[@]}" "$LIGHT_ACCOUNT_ID" "$EXIVA_ACCOUNT_ID" "$F5_C17A_ACCOUNT" "$F5_C17C_ACCOUNT" "$F5_C13_ACCOUNT" "$F5_C18A_ACCOUNT" "$F5_C17D_ACCOUNT" "$C11_UNASSIGNED_ACCOUNT" "$C11_KNIGHT_ACCOUNT" "$C12_MONK_ACCOUNT" "$S13_ACCOUNT" "$W19A_ACCOUNT" "$C05_TARGET_ACCOUNT" "$S13_PEER_ACCOUNT" "$C06_CASTER_ACCOUNT" "$C06_PEER_ACCOUNT" "$W18_ACCOUNT" "$C01_ACCOUNT" "$P23C_ACCOUNT" <<'PY_MP_SEED'
import json
from pathlib import Path
import secrets
import subprocess
import sys

work = Path(sys.argv[1])
world, *accounts = sys.argv[2:]
env = json.loads((work / "portal/secrets/env.json").read_text())
binary = json.loads((work / "identity.json").read_text())["portal_binary"]

def seed(entity, args):
    try:
        subprocess.run([binary, "seed", entity, json.dumps(args)], env=env,
                       capture_output=True, timeout=20, check=True)
    except (OSError, subprocess.SubprocessError):
        raise SystemExit(f"node_boot_mp_seed_failed entity={entity}") from None

seed("world", {"worldId": world, "name": "Open PvP", "region": "local"})
row_accounts = {}
for account in accounts:
    credentials = {"email": f"nb-{account}@example.invalid", "password": secrets.token_urlsafe(48)}
    if account == "01934f10-7c00-7000-8000-000000000020":
        credentials["email"] = "q_p23c@example.invalid"
    seed("account", {"accountId": account, **credentials})
    if account.endswith(("000000000016", "00000000001c", "00000000001d")):
        row_accounts[account] = credentials
(work / "row-accounts.json").write_text(json.dumps(row_accounts))
PY_MP_SEED
}

result=NODE_BOOT_FAIL
compose() {
  docker compose --project-name "$WP5_PROJECT" --file "$GAME_SOURCE/$COMPOSE_FILE" \
    --file "$GAME_SOURCE/$COMPOSE_S3B" --file "$GAME_SOURCE/$COMPOSE_OVERLAY" "$@"
}
psql_admin() { docker exec -i "$PG_CONTAINER" psql -v ON_ERROR_STOP=1 -qAt -U oteryn_node_boot_admin -d "$1" "${@:2}"; }
evidence() { printf 'NODE_BOOT_EVIDENCE %s\n' "$*"; }
install_owned() {
  local owner=$1
  shift
  if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
    install "$@"
  elif [[ "$owner" == *:* ]]; then
    sudo install -o "${owner%:*}" -g "${owner#*:}" "$@"
  else
    sudo install -o "$owner" "$@"
  fi
}
node_signal() {
  if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
    python3 "$GAME_SOURCE/tools/qualification/node_boot/signal_node.py" "$WORK/node-pids/$2" "$1"
  else
    sudo kill "$@"
  fi
}
fail() { echo "NODE_BOOT_FAILURE $*"; return 1; }
stop_runtime_diagnostics() {
  local pid
  for pid in "$RUNTIME_DIAGNOSTICS_PID" "${PVP_RUNTIME_DIAGNOSTICS_PID:-}"; do
    [[ -n "$pid" ]] || continue
    kill -TERM "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  RUNTIME_DIAGNOSTICS_PID=""
  PVP_RUNTIME_DIAGNOSTICS_PID=""
}
pg_observe() { python3 "$GAME_SOURCE/tools/qualification/node_boot/pg_observe.py" "$@" --container "$PG_CONTAINER" --user oteryn_node_boot_admin --database oteryn_node_boot; }
start_pg_sampler() {
  mkdir -p "$PG_OBS"
  # Start python3 itself in the background, not a subshell running pg_observe:
  # $! must be the sampler so stop_pg_sampler's SIGTERM reaches it. A shell
  # wrapper died on SIGTERM and left the sampler polling forever with the run's
  # stdout open, so the caller's `run.sh | tee` never saw EOF and the step hung
  # after NODE_BOOT_RESULT until the job timeout. Its stdout is not the run's pipe.
  python3 "$GAME_SOURCE/tools/qualification/node_boot/pg_observe.py" sample --out "$PG_OBS/activity.log" \
    --container "$PG_CONTAINER" --user oteryn_node_boot_admin --database oteryn_node_boot \
    < /dev/null > /dev/null 2>> "$PG_OBS/sampler.err" &
  PG_SAMPLER_PID=$!
}
stop_pg_sampler() {
  [[ -z "${PG_SAMPLER_PID:-}" ]] || {
    kill -TERM "$PG_SAMPLER_PID" 2>/dev/null || true
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
    WP5_APP_KEY PG_PORT PG_ADMIN_PASSWORD CONTROL_PASSWORD RUNTIME_PASSWORD ADMIN_URL FRESH_SEED_HEX
    RECOVERY_SEED_HEX MAP_REVISION NODE_PID NODE_CLOCK_FEATURES NODE_SPAWN_FEATURES NODE_GAMEPLAY_ENV PG_OBS)
  if [[ "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
    names+=(GP_HARNESS_PID GP_PG_CONTAINER WP5_SERVER_NAME WP5_SOURCE_AUTHORITY WP5_ROUTE_REVISION)
  fi
  [[ -z "${OTERYN_QUALIFICATION_CLOCK_FILE:-}" ]] || names+=(OTERYN_QUALIFICATION_CLOCK_FILE)
  ( umask 077
    { declare -p "${names[@]}"
      printf 'kept_pvp_port=%q\n' "$PVP_GAME_PORT"
      printf 'kept_mode=%q\n' "$NODE_BOOT_WORLD/$NODE_BOOT_SPELLS/${NODE_BOOT_GAME_PORT:-17181}"
    } > "$REUSE_STATE" )
  STACK_KEPT=1
}
interrupted() {
  result=NODE_BOOT_FAIL
  evidence "interrupted signal=$1 stage=$ACTIVE_STAGE world_bundle=$NODE_BOOT_WORLD"
  exit "$2"
}
cleanup() {
  local rc=$?
  trap - EXIT INT TERM
  set +e
  # Flush the independent reader before either the server or its private log is removed.
  stop_runtime_diagnostics
  stop_pg_sampler
  if [[ ${STACK_KEPT:-0} == 1 ]]; then
    evidence "stack=kept project=$WP5_PROJECT rerun=NODE_BOOT_REUSE_STACK=1 remove=NODE_BOOT_REUSE_STACK=teardown"
  else
    if [[ "${NODE_BOOT_UNPRIVILEGED:-0}" == 1 ]]; then
      local pid_file
      for pid_file in "$WORK"/node-pids/*; do
        [[ -f "$pid_file" ]] || continue
        node_signal -TERM "${pid_file##*/}" 2>/dev/null || true
      done
    else
      [[ -z "$NODE_PID" ]] || sudo kill -TERM "$NODE_PID" 2>/dev/null || true
      sudo pkill -TERM -u "$SERVICE_USER" 2>/dev/null || true
    fi
    if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform || "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
      game_platform_cleanup
    else
      compose down --volumes --remove-orphans --timeout 15 >/dev/null 2>&1 || true
    fi
    docker rm --force --volumes "$PG_CONTAINER" >/dev/null 2>&1 || true
    if [[ "${NODE_BOOT_UNPRIVILEGED:-0}" != 1 ]]; then
      sudo rm -rf "$BASE" || true
      [[ -z "${REUSE_STATE:-}" ]] || rm -f -- "$REUSE_STATE"
    fi
  fi
  if [[ -n "${OTERYN_P23C_RAW_SERVER_LOG:-}" && ! -f "$OTERYN_P23C_RAW_SERVER_LOG" ]]; then
    local raw_source="$node_log"
    [[ ! -f "$failed_node_log" ]] || raw_source="$failed_node_log"
    if [[ -f "$raw_source" ]]; then
      install -m 0600 -- "$raw_source" "$OTERYN_P23C_RAW_SERVER_LOG"
    fi
  fi
  if [[ $rc != 0 && -n "${NODE_BOOT_FAILURE_LOG_DIR:-}" ]]; then
    mkdir -p "$NODE_BOOT_FAILURE_LOG_DIR"
    if [[ -f "$failed_node_log" ]]; then
      cp "$failed_node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    elif [[ -f "$node_log" ]]; then
      cp "$node_log" "$NODE_BOOT_FAILURE_LOG_DIR/node.log"
    fi
    [[ ! -f "$WORK/pvp-node.log" ]] || cp "$WORK/pvp-node.log" "$NODE_BOOT_FAILURE_LOG_DIR/pvp-node.log"
    # The basic and World passes share the directory; each keeps its own PostgreSQL record.
    [[ ! -d "${PG_OBS:-}" ]] || cp -R "$PG_OBS" "$NODE_BOOT_FAILURE_LOG_DIR/postgres-world$NODE_BOOT_WORLD-spells$NODE_BOOT_SPELLS"
  fi
  if [[ "${NODE_BOOT_UNPRIVILEGED:-0}" == 1 ]]; then
    rm -rf -- "$WORK"
  elif [[ ${STACK_KEPT:-0} != 1 ]]; then
    find "$WORK" -type f -delete 2>/dev/null || true
    find "$WORK" -depth -type d -empty -delete 2>/dev/null || true
  fi
  evidence "cleanup=complete"
  evidence "result_scope world_bundle=$NODE_BOOT_WORLD spells=$NODE_BOOT_SPELLS stage=$ACTIVE_STAGE exit=$rc"
  echo "NODE_BOOT_RESULT=$result${result_mode:-}"
  exit "$rc"
}
trap cleanup EXIT
trap 'interrupted INT 130' INT
trap 'interrupted TERM 143' TERM
if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  evidence "privilege=unprivileged isolation_checks=skipped"
  mkdir "$WORK/node-pids"
fi
if [[ "$NODE_BOOT_REUSE_STACK" == teardown ]]; then
  result=NODE_BOOT_TEARDOWN
  exit 0
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
make_leaf server source.test server-ca serverAuth DNS:source.test
make_leaf client oteryn-game-native-evidence client-ca clientAuth
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
if [[ "$NODE_BOOT_UNPRIVILEGED" != 1 ]]; then
  chmod 644 "$WP5_PKI/db.crt" "$WP5_PKI/db.key"
fi
docker run --detach --name "$PG_CONTAINER" --publish "127.0.0.1:$PG_PORT:5432" \
  --env POSTGRES_USER=oteryn_node_boot_admin --env POSTGRES_PASSWORD="$PG_ADMIN_PASSWORD" \
  --env POSTGRES_DB=postgres --volume "$WP5_PKI/db.crt:/tls/db.crt:ro" \
  --volume "$WP5_PKI/db.key:/tls/db.key:ro" --entrypoint bash "$PG_IMAGE" -c \
  'install -o postgres -m 0600 /tls/db.key /tmp/db.key && install -o postgres -m 0644 /tls/db.crt /tmp/db.crt && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/db.crt -c ssl_key_file=/tmp/db.key -c log_lock_waits=on -c deadlock_timeout=200ms -c log_min_duration_statement=500 -c log_parameter_max_length=0 -c log_parameter_max_length_on_error=0 -c shared_preload_libraries=pg_stat_statements -c "log_line_prefix=%m [%p] %u "' >/dev/null
chmod 600 "$WP5_PKI/db.key"
# Probe over TCP: the entrypoint's temporary init server listens on the unix socket only
# and is shut down after init, so a socket probe can pass before the real server is up.
pg_ready=0
for _ in $(seq 1 60); do
  docker exec "$PG_CONTAINER" pg_isready -h 127.0.0.1 -p 5432 -U oteryn_node_boot_admin -d postgres >/dev/null 2>&1 && { pg_ready=1; break; }
  sleep 1
done
[[ $pg_ready == 1 ]] || fail "postgres_ready"
echo "CREATE DATABASE oteryn_node_boot" | psql_admin postgres
# The view lives in the maintenance database: the Game schema stays exactly as migrated.
echo "CREATE EXTENSION pg_stat_statements" | psql_admin postgres
[[ "$(echo 'SHOW server_version_num' | psql_admin oteryn_node_boot)" == 180006 ]] || fail "postgres_version"
[[ "$(echo 'SHOW ssl' | psql_admin oteryn_node_boot)" == on ]] || fail "postgres_tls"
ADMIN_URL="postgresql://oteryn_node_boot_admin:$PG_ADMIN_PASSWORD@127.0.0.1:$PG_PORT/oteryn_node_boot"

# Use the merged native producer for every shard; the PHP producer is retired.
game_platform_start
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_LIFECYCLE" == native_reference && "$NODE_BOOT_D150_LEGACY" == 0 ]]; then
  seed_mp_accounts
fi

# Shipped binaries and the SEAM client harness.
if [[ -z "$NODE_BOOT_LIBTEST" ]]; then
  cargo +1.100.0 build --locked -p oteryn-game-server --bins "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}"
  cargo +1.100.0 test --locked -p oteryn-game-server --lib --no-run "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}"
fi

# Deployment administration: service user, directories, roles, migration.
if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
  SERVICE_UID="$(id -u)"
else
  sudo useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER" 2>/dev/null || true
  SERVICE_UID="$(id -u "$SERVICE_USER")"
fi
install_owned root:root -d -m 0755 "$BASE" "$BASE/bin" "$BASE/state" "$BASE/fence-parent" "$BASE/node"
install_owned root:root -d -m 0700 "$BASE/ops"
if [[ "$NODE_BOOT_SPELLS" == 1 || "$NODE_BOOT_WORLD" == 1 ]]; then
  # Stage only declared, hash-bound inputs; never publish unrelated siblings.
  python3 - "$NODE_BOOT_SPELL_MANIFEST" "$GAME_SOURCE" "$WORK/gameplay" <<'PY_STAGE'
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
  install_owned root:root -d -m 0755 "$BASE/gameplay"
  "${as_operator[@]}" cp -R "$WORK/gameplay/." "$BASE/gameplay/"
  "${as_operator[@]}" chmod -R a+rX "$BASE/gameplay"
  evidence "spell_input=full_manifest manifest_sha256=$(sha256sum "$NODE_BOOT_SPELL_MANIFEST" | cut -d ' ' -f 1) activation_wait_seconds=$NODE_ACTIVATION_WAIT_SECONDS"
fi
MAP_REVISION=map-s3b-1
NODE_NPC_ARGS=()
if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
  evidence "world_profile=$NODE_BOOT_WORLD_PROFILE regions=$WORLD_REGIONS"
  compile_world=compile-regions
  if [[ "$NODE_BOOT_WORLD_PROFILE" == town ]]; then compile_world=compile-town-qualification; fi
  "${WORLD_COMPILER[@]}" "$compile_world" "$GAME_SOURCE" "$GAME_SOURCE/content/world/pins/oteryn.identity.json" "$WORK/world.otwb" "$WORLD_REGIONS" > "$WORK/world-compile.json"
  PIN="$WORK/world.json"
  python3 - "$GAME_SOURCE/content/world/pins/oteryn.json" "$WORK/world-compile.json" "$PIN" <<'PY_REGION_PIN'
import json, sys
pin = json.load(open(sys.argv[1]))
pin["schema"] = "OTERYN_QUALIFICATION_WORLD/v1"
pin["digest"] = json.load(open(sys.argv[2]))["digest"]
pin["entry_start"] = {"x": 32369, "y": 32238, "floor": -7}
with open(sys.argv[3], "w") as output:
    json.dump(pin, output, sort_keys=True)
PY_REGION_PIN
  WORLD_DIGEST="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["digest"])' "$PIN")"
  install_owned root:root -d -m 0755 "$BASE/world"
  install_owned root:root -m 0644 "$WORK/world.otwb" "$BASE/world/world.otbundle"
  install_owned root:root -m 0644 "$PIN" "$BASE/world/world.json"
  MAP_REVISION="$WORLD_DIGEST"
  source "$GAME_SOURCE/tools/qualification/node_boot/npc_data.sh"
  node_boot_stage_npc_data
fi
install_owned "$SERVICE_UID" -d -m 0700 "$BASE/fence-parent/fence" "$BASE/run"
install_owned "$SERVICE_UID" -d -m 0755 "$BASE/node/secrets"
install_owned root -m 0755 "$TARGET/oteryn-game-server" "$TARGET/oteryn-game-ops" "$TARGET/oteryn-game-migrate" "$BASE/bin/"
OTERYN_GAME_MIGRATION_DATABASE_URL="$ADMIN_URL" "$BASE/bin/oteryn-game-migrate"
psql_admin oteryn_node_boot <<SQL
CREATE ROLE nb_control LOGIN PASSWORD '$CONTROL_PASSWORD' IN ROLE oteryn_game_control;
CREATE ROLE nb_runtime LOGIN PASSWORD '$RUNTIME_PASSWORD' IN ROLE oteryn_game_runtime;
GRANT CONNECT ON DATABASE oteryn_node_boot TO nb_control, nb_runtime;
SQL
secret_to() { # owner source destination
  install_owned "$1" -m 0600 "$2" "$3"
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
install_owned root -m 0600 "$WORK/ops.toml" "$BASE/ops/ops.toml"
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
# The qualification Platform PKI does not issue a T94 producer identity.
[platform.world_event_outcomes]
enabled = false
[launch]
authorization_file = "$BASE/state/$1"
s2_authorization_file = "$BASE/state/s2-fresh-store.json"
TOML
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    python3 - "$PIN" "$BASE/world/world.otbundle" >> "$WORK/node.toml" <<'PY_WORLD'
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
start_floor = 7''')
PY_WORLD
    cat >> "$WORK/node.toml" <<'TOML'
[premium_activation]
activation_id = "node-boot-fixture-premium"
switch_over_us = 1
TOML
  fi
  if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform || "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
    game_platform_config "$WORK/node.toml"
  fi
  install_owned root -m 0644 "$WORK/node.toml" "$BASE/node/node.toml"
}
# Root/client writes an atomic offset; the service can only read it. Start at
# zero on every disposable run, and keep it across the shutdown/restart probes.
if (( ${#NODE_CLOCK_FEATURES[@]} )); then
  install_owned "$(id -u)" -d -m 0755 "$BASE/qualification-clock"
  export OTERYN_QUALIFICATION_CLOCK_FILE="$BASE/qualification-clock/offset-ms"
  printf '0' > "$OTERYN_QUALIFICATION_CLOCK_FILE"
  chmod 0644 "$OTERYN_QUALIFICATION_CLOCK_FILE"
  NODE_GAMEPLAY_ENV+=("OTERYN_QUALIFICATION_CLOCK_FILE=$OTERYN_QUALIFICATION_CLOCK_FILE")
fi
if [[ "$NODE_BOOT_WORLD" == 1 && -f "$WORK/portal-report.json" ]]; then
  # Use the Platform's disposable Premium-purpose client and trust root.
  local_premium_dir="$WORK/portal/certs"
  cat "$local_premium_dir/clients/premium.pem" "$local_premium_dir/clients/premium.key" > "$WORK/profile-b-client.pem"
  secret_to "$SERVICE_UID" "$WORK/profile-b-client.pem" "$BASE/node/secrets/profile-b-client.pem"
  NODE_GAMEPLAY_ENV+=("OTERYN_PLATFORM_PROFILE_B_AUTHORITY_ENABLED=true"
    "OTERYN_PLATFORM_PROFILE_B_AUTHORITY_CLIENT_IDENTITY_PEM=$BASE/node/secrets/profile-b-client.pem"
    "OTERYN_PLATFORM_STREAMS_URL=https://localhost:$WP5_PORT"
    "OTERYN_PLATFORM_STREAMS_PLATFORM_CA_PEM=$BASE/node/secrets/platform-roots.pem")
fi
fi # NODE_BOOT_REUSE_STACK == 0
start_pg_sampler

ops() { "${as_operator[@]}" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-ops" --config "$BASE/ops/ops.toml" "$@"; }
launch_node() { # PID variable, config, log
  # Detached from the stage's output pipe, so the stage ends while it serves.
  (
    if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
      # Record birth time before exec so later stages can detect PID reuse.
      local pid=$BASHPID proc_stat
      local -a fields
      IFS= read -r proc_stat < "/proc/$pid/stat"
      read -ra fields <<< "${proc_stat##*) }"
      printf '%s\n' "${fields[19]}" > "$WORK/node-pids/$pid"
    fi
    exec "${as_service[@]}" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$2" "${NODE_NPC_ARGS[@]}"
  ) < /dev/null > /dev/null 2> "$3" &
  printf -v "$1" '%s' "$!"
}
start_node() { launch_node NODE_PID "$BASE/node/node.toml" "$node_log"; }
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
  local offset=0 log="${2:-$node_log}"
  [[ ! -f "$log" ]] || offset=$(stat -c '%s' "$log")
  python3 -u - "$log" "$1" "$NODE_BOOT_WORLD" "$offset" <<'PY_RUNTIME_DIAGNOSTICS' &
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
  if [[ $# == 2 ]]; then
    PVP_RUNTIME_DIAGNOSTICS_PID=$!
  else
    RUNTIME_DIAGNOSTICS_PID=$!
  fi
}
# Scenario ids of the e2e rows with the given status in a stage log, one per line.
stage_rows() { grep -oE "e2e_all scenario=[^ ]+ status=$2( |$)" "$1" | sed -E 's/e2e_all scenario=([^ ]+) .*/\1/' | sort -u || true; }
# Row ids listed in the known-failure file (first field, comments skipped); a missing file lists none.
known_rows() { [[ -f $1 ]] && grep -vE '^[[:space:]]*(#|$)' "$1" | awk '{print $1}' || true; }

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
  case "$name" in
    seam_stages|spell_scenarios) start_runtime_diagnostics "$name" ;;
  esac
  if [[ "$name" == seam_stages && ${STATUS[pvp_world_ready]:-} == PASS ]]; then
    start_runtime_diagnostics "${name}_pvp" "$WORK/pvp-node.log"
  fi
  set +e
  ( set -Eeuo pipefail; source "$VARS"; "$@" ) 2>&1 | tee "$WORK/stage-$name.log"
  local rc=${PIPESTATUS[0]}
  if [[ "$name" == seam_stages && -n "${OTERYN_P23C_RAW_SERVER_LOG:-}" ]]; then
    if ! install -m 0600 -- "$node_log" "$OTERYN_P23C_RAW_SERVER_LOG"; then
      rc=1
    fi
  fi
  stop_runtime_diagnostics
  pg_observability_dump "$name" "$rc"
  set -e
  evidence "stage=$name state=finish world_bundle=$NODE_BOOT_WORLD exit=$rc"
  echo "::endgroup::"
  if [[ $rc == 0 ]]; then
    STATUS[$name]=PASS
    # A FAIL row fails its stage even when the stage process exits 0, unless the row is
    # listed in known_fail_rows.txt (debt that every run prints).
    local known_fail="$GAME_SOURCE/tools/qualification/node_boot/known_fail_rows.txt" new_rows known_hit stale_rows
    new_rows=$(stage_rows "$WORK/stage-$name.log" FAIL | grep -vxFf <(known_rows "$known_fail") | paste -sd, - || true)
    known_hit=$(stage_rows "$WORK/stage-$name.log" FAIL | grep -xFf <(known_rows "$known_fail") | paste -sd, - || true)
    stale_rows=$(stage_rows "$WORK/stage-$name.log" PASS | grep -xFf <(known_rows "$known_fail") | paste -sd, - || true)
    [[ -z $known_hit ]] || echo "NODE_BOOT_KNOWN_FAIL $name rows=$known_hit"
    [[ -z $stale_rows ]] || echo "NODE_BOOT_KNOWN_FAIL_STALE $name rows=$stale_rows"
    if [[ -n $new_rows ]]; then
      STATUS[$name]="FAIL(rows)"
      # Keep the failing incarnation before later probes truncate the live log.
      [[ -f "$failed_node_log" || ! -f "$node_log" ]] || cp "$node_log" "$failed_node_log"
      echo "NODE_BOOT_STAGE_FAILED $name rows=$new_rows"
    fi
    if grep -q 'e2e_all scenario=.* status=FAIL' "$WORK/stage-$name.log"; then
      echo "--- session close events behind failed e2e rows"
      grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed|connection_[a-z_]+)( |$)' "$node_log" 2>/dev/null | tail -n 120 || true
    fi
  else
    STATUS[$name]="FAIL(exit $rc)"
    # Later probes start new server incarnations and truncate node_log. Preserve
    # the first failed incarnation now so cleanup uploads the log that explains
    # the qualification failure rather than a subsequent successful shutdown.
    [[ -f "$failed_node_log" || ! -f "$node_log" ]] || cp "$node_log" "$failed_node_log"
    echo "NODE_BOOT_STAGE_FAILED $name exit=$rc"
    report_character_authority_failures "$name"
    echo "--- session close events"
    grep -E 'event=(serve_end|serve_command_end|command_guard_failed|join_prepare_failed)( |$)' "$node_log" 2>/dev/null | tail -n 40 || true
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
    for _ in $(seq 1 200); do
      echo "SELECT state || ' ' || left(regexp_replace(query, '\\s+', ' ', 'g'), 200) FROM pg_stat_activity WHERE datname = 'oteryn_node_boot' AND usename <> 'oteryn_node_boot_admin'" \
        | psql_admin oteryn_node_boot 2>/dev/null || true
      sleep 0.02
    done | sort | uniq -c | sort -rn | head -n 25
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
    echo "--- postgres"; docker logs --tail 40 "$PG_CONTAINER" 2>&1 | grep -E 'ERROR|FATAL|STATEMENT' || true
    echo "--- platform"; compose logs --no-color --tail 40 platform nginx 2>&1 || true
  fi
}
node_field() { # log field
  sed -n "s/.*awaiting_assignment.*$2=\([^ \"]*\).*/\1/p" "$1" | tail -n 1
}

operator_setup() { # §4.1 operator setup before the node starts
  local starter="${INTERPRETATION[3]}"
  if [[ "$NODE_BOOT_D150_LEGACY" == 1 ]]; then
    starter=nb-starter-1
  fi
  ops authorization issue --file launch-a.json --binding node-boot-a
  write_node_config launch-a.json
  ops s2 issue --node-config "$BASE/node/node.toml" --file s2-fresh-store.json --namespace node-boot --authorization disposable-qualification
  if ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "$starter"; then
    fail "interpretation configured before the fresh store"
  fi
  ops character fresh-store --request fresh-store.json
  ops character fresh-store --request fresh-store.json
  ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "$starter"
  evidence "operator=authorization,s2_issuance,fresh_store,interpretation interpretation_before_store=refused fresh_store_rerun=idempotent"
}

configuration_negatives() { # exit before binding, no secret in output
  "${as_operator[@]}" cp "$BASE/node/node.toml" "$BASE/node/insecure.toml"; "${as_operator[@]}" chmod 0666 "$BASE/node/insecure.toml"
  "${as_operator[@]}" ln -sf "$BASE/node/node.toml" "$BASE/node/link.toml"
  "${as_operator[@]}" sed '/^map_revision/d' "$BASE/node/node.toml" | "${as_operator[@]}" tee "$BASE/node/missing.toml" >/dev/null
  "${as_operator[@]}" sed 's/^max_connections = 256/max_connections = 257/' "$BASE/node/node.toml" | "${as_operator[@]}" tee "$BASE/node/over.toml" >/dev/null
  "${as_operator[@]}" chmod 0644 "$BASE/node/missing.toml" "$BASE/node/over.toml"
  local config code failed=0
  for config in insecure link missing over; do
    set +e
    "${as_service[@]}" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/$config.toml" 2>> "$WORK/negative.log"
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
assignment_report_args() { # fills the caller's report_args on the Platform lifecycles
  if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform || "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
    report_args=(--report-config "$BASE/ops/assignment-report.toml" --node-config "$BASE/node/node.toml"
      --node-identity "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["clients"]["runtime-status"]["san"])' "$WORK/portal-report.json")")
  fi
}
node_deployment_grants() {
  [[ "$(echo "SELECT current_user = pg_get_userbyid(datdba) FROM pg_database WHERE datname=current_database()" | psql_admin oteryn_node_boot)" == t ]] || fail "deployment issuer is not the database owner"
  local operations=1,2,3,4
  [[ "${NODE_BOOT_LIFECYCLE:-}" != game_platform ]] || operations=1,4
  # The reference run also proves the superseding replacement (operation 2).
  [[ "${NODE_BOOT_LIFECYCLE:-}" != native_reference ]] || operations=1,2,4
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
  ops content activate --request content-1.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --sequence 1 --previous empty
  ops content activate --request content-1.json
  local -a report_args=()
  assignment_report_args
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
  if [[ "$NODE_BOOT_WORLD" == 1 || "$NODE_BOOT_SPELLS" == 1 ]]; then
    # Full-manifest admission initializes Prey against committed reset history.
    # Use the Control initializer for both the bundle and the native entry fixture.
    local epoch_result epoch_bundle=none epoch_digest_sql=NULL
    if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
      epoch_bundle=compiled
      epoch_digest_sql="'$WORLD_DIGEST'"
    fi
    epoch_result="$({
      echo 'PREPARE node_boot_world_epoch(UUID, UUID, TEXT, TEXT) AS'
      cat "$GAME_SOURCE/tools/qualification/node_boot/world_epoch.sql"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', %s, '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$epoch_digest_sql" "$MAP_REVISION"
      printf "EXECUTE node_boot_world_epoch('%s', '%s', %s, '%s');\n" "$WORLD_ID" "$CHANNEL_ID" "$epoch_digest_sql" "$MAP_REVISION"
    } | psql_admin oteryn_node_boot)"
    [[ "$epoch_result" == $'t\nf' ]] || fail "World epoch initialization/replay"
    evidence "world_epoch=0 source=control_initializer bundle=$epoch_bundle content=issued replay=idempotent"
  fi
  local control_socket_mode="uid${SERVICE_UID}_0600"
  if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
    [[ "$(stat -c %a "$BASE/run/control.sock")" == 600 ]] || fail "control socket mode"
    control_socket_mode=0600
  else
    [[ "$(sudo stat -c '%u %a' "$BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "control socket mode"
  fi
  [[ "$(echo 'SELECT count(*) FROM game_node_registrations' | psql_admin oteryn_node_boot)" == 1 ]] || fail "operator registered an incarnation"
  evidence "world_boot_seconds=$((SECONDS - boot_started)) world_bundle=$NODE_BOOT_WORLD"
  evidence "node registered=1 assignment=operator ungranted=refused content_activation=sequence_1 content_ungranted=refused content_replay=exact content_stale=refused readiness=true control_socket=$control_socket_mode"
}

control_socket_peer() {
  if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
    if ! timeout 30s python3 - "$BASE/run/control.sock" <<'PY'
import json
import socket
import sys
import time

deadline = time.monotonic() + 25
while True:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
        control.settimeout(3)
        control.connect(sys.argv[1])
        control.sendall(b"health\n")
        control.shutdown(socket.SHUT_WR)
        with control.makefile("rb") as response:
            line = response.readline(65537)
    if len(line) > 65536 or not line.endswith(b"\n"):
        sys.exit("Qualification peer received an incomplete health response")
    health = json.loads(line)
    if not isinstance(health, dict) or type(health.get("readiness")) is not bool:
        sys.exit("Qualification peer received invalid readiness")
    if health["readiness"]:
        break
    # Health reflects the pool's current idle count, which can dip during work.
    if time.monotonic() >= deadline:
        sys.exit("Qualification peer did not receive ready health")
    time.sleep(0.05)
PY
    then
      fail "qualification control socket peer health"
      return 1
    fi
    evidence "control_socket qualification_peer=ready"
    return 0
  fi
  # With distinct operator and service identities, the service peer must be closed.
  sudo -u "$SERVICE_USER" python3 - "$BASE/run/control.sock" "${INTENT_OPERATIONS[0]}" <<'PY'
import socket, sys
s = socket.socket(socket.AF_UNIX)
s.connect(sys.argv[1])
try:
    s.sendall(sys.argv[2].encode() + b"\n")
    s.shutdown(socket.SHUT_WR)
    answer = s.recv(64)
except OSError:
    answer = b""
sys.exit(0 if answer == b"" else 1)
PY
  evidence "control_socket service_user_peer=closed"
}

character_bootstrap() { # §4.5 Characters from real Platform intents
  local index=0 account operation name requested_vocation failed=0 vocation="${NODE_BOOT_PROFILE^}"
  [[ "$NODE_BOOT_PROFILE" != explorer ]] || vocation=Knight
  for account in "${BOOTSTRAP_ACCOUNTS[@]}"; do
    operation="${INTENT_OPERATIONS[$index]}"
    name="${INTENT_NAMES[$index]}"
    index=$((index + 1))
    requested_vocation="$vocation"
    case "$account" in
      "$S13_ACCOUNT"|"$LIGHT_ACCOUNT_ID"|"$EXIVA_ACCOUNT_ID"|"$F5_C17A_ACCOUNT"|"$F5_C17C_ACCOUNT"|"$F5_C13_ACCOUNT"|"$F5_C18A_ACCOUNT") requested_vocation=Sorcerer ;;
      "$F5_C17D_ACCOUNT"|"$C11_UNASSIGNED_ACCOUNT"|"$C11_KNIGHT_ACCOUNT") requested_vocation=Knight ;;
      "$C12_MONK_ACCOUNT") requested_vocation=Monk ;;
      "$C06_CASTER_ACCOUNT") requested_vocation=Sorcerer ;;
      "$P23C_ACCOUNT") requested_vocation=Knight ;;
    esac
    if ! python3 "$GAME_SOURCE/tools/qualification/game_platform/composition.py" bootstrap \
      "$WORK" "$account" "$WORLD_ID" "$operation" "${INTERPRETATION[@]}" "$name" "$requested_vocation"; then
      echo "platform intent issue failed for operation $operation"
      failed=1
      continue
    fi
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || { failed=1; continue; }
    # An exact retry of the same operation id is idempotent.
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]] || fail "Character bootstrap"
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    local p23c_client_operation=01934f10-7c04-7021-805b-3b1122334421
    python3 "$GAME_SOURCE/tools/qualification/game_platform/composition.py" bootstrap \
      "$WORK" "$P23C_ACCOUNT" "$WORLD_ID" "$p23c_client_operation" "${INTERPRETATION[@]}" "Bounty Task Hunter" Knight
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$p23c_client_operation"
    # A separate character uses the existing disposable tag-3 birth gate for this account.
    local s16_operation=01934f10-7c04-7018-805b-3b1122334418
    python3 "$GAME_SOURCE/tools/qualification/game_platform/composition.py" bootstrap \
      "$WORK" "$C11_UNASSIGNED_ACCOUNT" "$WORLD_ID" "$s16_operation" "${INTERPRETATION[@]}" "First Vocation Candidate" Knight
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$s16_operation"
    ops character bootstrap --socket "$BASE/run/control.sock" --operation-id "$s16_operation"
  fi
  evidence "characters=${#BOOTSTRAP_ACCOUNTS[@]} source=platform_intents path=control_socket retry=idempotent"
}

legacy_d150_seed() {
  psql_admin oteryn_node_boot -v account="$ACCOUNT_ID" -v world="$WORLD_ID" \
    -v profile="${INTERPRETATION[0]}" -v ruleset="${INTERPRETATION[1]}" -v content="${INTERPRETATION[2]}" \
    < "$GAME_SOURCE/tools/qualification/node_boot/legacy_d150.sql"
  evidence "d150_fixture=historical_tag3 database=disposable birth=null phase=before_node_launch"
}

legacy_d150_register_starter() {
  # Migration 0982 registers the preceding Control interpretation for assigned Worlds.
  ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" \
    --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"
  local registered
  registered="$(psql_admin oteryn_node_boot -v account="$ACCOUNT_ID" -v world="$WORLD_ID" \
    -v starter="${INTERPRETATION[3]}" <<'SQL'
SELECT EXISTS (
    SELECT 1
      FROM game_character_roots r
      JOIN game_world_registered_starter_revisions s USING (world_id, starter_template_revision)
      CROSS JOIN (
          SELECT interpretation_revision, starter_template_revision
            FROM game_character_interpretations
           ORDER BY interpretation_revision DESC LIMIT 1
      ) current_interpretation
     WHERE r.account_id = :'account'::uuid AND r.world_id = :'world'::uuid
       AND r.creation_vocation IS NULL AND r.starter_template_revision = 'nb-starter-1'
       AND s.interpretation_revision < current_interpretation.interpretation_revision
       AND current_interpretation.starter_template_revision = :'starter'
);
SQL
)"
  [[ "$registered" == t ]] || { fail "D150 historical starter registration"; return 1; }
  evidence "d150_starter_history=registered historical=nb-starter-1 current=${INTERPRETATION[3]}"
}

seam_stages() { # §4.6 every #823 stage against the node's own port
  if [[ "${NODE_BOOT_D150_LEGACY:-0}" == 0 ]] &&
    [[ -z "${NODE_BOOT_LIBTEST:-}" || "${NODE_BOOT_LIFECYCLE:-}" == native_reference ]]; then
    local spawn_proof spawn_test=node::serve::qualification_fixture::tests::qualification_spawn_transaction_returns_pool_and_bounds_backend
    local spawn_command
    if [[ -n "${NODE_BOOT_LIBTEST:-}" ]]; then
      spawn_command=("$NODE_BOOT_LIBTEST" "$spawn_test" --exact)
    else
      spawn_command=(cargo +1.100.0 test --locked -p oteryn-game-server --lib
        "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}" "$spawn_test" -- --exact)
    fi
    spawn_proof="$(OTERYN_TEST_POSTGRES_ADMIN_URL="$ADMIN_URL" "${spawn_command[@]}")" || {
      printf '%s\n' "$spawn_proof"
      return 1
    }
    printf '%s\n' "$spawn_proof"
    grep -q '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<< "$spawn_proof" || {
      printf '%s\n' 'Spawn PostgreSQL proof did not execute exactly one passing test' >&2
      return 1
    }
  fi
  # The bootstrap evidence ages past five seconds, so each admission fetches its own.
  sleep 6
  local qualification=node_boot_seam_against_running_node
  local test_env=()
  if [[ "${NODE_BOOT_D150_LEGACY:-0}" == 1 ]]; then
    test_env+=("NODE_BOOT_D150_LEGACY=1")
  fi
  if [[ "$NODE_BOOT_SPELLS" == 1 ]]; then
    qualification=node_boot_spells_against_running_node
    test_env=("RUST_MIN_STACK=${RUST_MIN_STACK:-16777216}")
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    test_env+=("NODE_BOOT_CAST_EVIDENCE_LOG=$node_log" "NODE_BOOT_WORLD_BUNDLE=$BASE/world/world.otbundle" "NODE_BOOT_WORLD_PIN=$BASE/world/world.json" "NODE_BOOT_MAP_REVISION=$MAP_REVISION" "RUST_MIN_STACK=${RUST_MIN_STACK:-67108864}")
    local extra_accounts
    extra_accounts="$(IFS=,; echo "${EXTRA_WORLD_ACCOUNTS[*]}")"
    test_env+=("NODE_BOOT_EXTRA_WORLD_ACCOUNTS=$extra_accounts" "NODE_BOOT_EXIVA_ACCOUNT_ID=$EXIVA_ACCOUNT_ID")
    test_env+=("NODE_BOOT_F5_C17A_ACCOUNT=$F5_C17A_ACCOUNT" "NODE_BOOT_F5_C17C_ACCOUNT=$F5_C17C_ACCOUNT" "NODE_BOOT_F5_C13_ACCOUNT=$F5_C13_ACCOUNT" "NODE_BOOT_F5_C18A_ACCOUNT=$F5_C18A_ACCOUNT" "NODE_BOOT_F5_C17D_ACCOUNT=$F5_C17D_ACCOUNT")
    test_env+=("NODE_BOOT_C11_UNASSIGNED_ACCOUNT=$C11_UNASSIGNED_ACCOUNT" "NODE_BOOT_C11_KNIGHT_ACCOUNT=$C11_KNIGHT_ACCOUNT" "NODE_BOOT_C12_MONK_ACCOUNT=$C12_MONK_ACCOUNT" "NODE_BOOT_S13_ACCOUNT=$S13_ACCOUNT")
    test_env+=("NODE_BOOT_W19A_ACCOUNT=$W19A_ACCOUNT")
    test_env+=("NODE_BOOT_C05_TARGET_ACCOUNT=$C05_TARGET_ACCOUNT")
    test_env+=("NODE_BOOT_S13_PEER_ACCOUNT=$S13_PEER_ACCOUNT")
    test_env+=("NODE_BOOT_C06_CASTER_ACCOUNT=$C06_CASTER_ACCOUNT")
    test_env+=("NODE_BOOT_C06_PEER_ACCOUNT=$C06_PEER_ACCOUNT" "NODE_BOOT_PLATFORM_GRANT_DIR=$WORK"
      "NODE_BOOT_PLATFORM_GRANT_ACCOUNTS=$C12_MONK_ACCOUNT,$C06_CASTER_ACCOUNT,$C06_PEER_ACCOUNT")
    test_env+=("NODE_BOOT_P23C_ACCOUNT=$P23C_ACCOUNT")
    test_env+=("NODE_BOOT_W18_ACCOUNT=$W18_ACCOUNT")
    test_env+=("NODE_BOOT_C01_ACCOUNT=$C01_ACCOUNT")
    test_env+=("NODE_BOOT_PACKET_ACCOUNT_ID=$PACKET_ACCOUNT_ID" "NODE_BOOT_PACKET_SECOND_ACCOUNT_ID=$SECOND_PACKET_ACCOUNT_ID" "NODE_BOOT_LIGHT_ACCOUNT_ID=$LIGHT_ACCOUNT_ID")
  fi
  if [[ "$NODE_BOOT_WORLD" == 1 && ${STATUS[pvp_character_bootstrap]:-} == PASS ]]; then
    # MP01's PvP rows run against the Open PvP World's own node and database.
    test_env+=("NODE_BOOT_PVP_ADDRESS=127.0.0.1:$PVP_GAME_PORT" "NODE_BOOT_PVP_CONTROL_SOCKET=$PVP_BASE/run/control.sock"
      "NODE_BOOT_PVP_DATABASE_URL=${ADMIN_URL%/*}/$PVP_DATABASE" "NODE_BOOT_PVP_WORLD_ID=$PVP_WORLD_ID"
      "NODE_BOOT_PVP_CHANNEL_ID=$PVP_CHANNEL_ID" "NODE_BOOT_PVP_ACCOUNT_ID=${PVP_ACCOUNT_IDS[0]}"
      "NODE_BOOT_PVP_SECOND_ACCOUNT_ID=${PVP_ACCOUNT_IDS[1]}")
  fi
  # Row selection, fail-fast and the no-progress watchdog belong to the seam harness.
  local knob
  for knob in NODE_BOOT_E2E_ONLY NODE_BOOT_NO_FAILFAST NODE_BOOT_NO_PROGRESS_S; do
    [[ -z "${!knob+x}" ]] || test_env+=("$knob=${!knob}")
  done
  local staged_manifest="${NODE_GAMEPLAY_ENV[0]:-}"
  staged_manifest="${staged_manifest#OTERYN_NATIVE_GAMEPLAY_MANIFEST=}"
  local test_dir="$PWD"
  local test_command=(cargo +1.100.0 test --locked -p oteryn-game-server --lib "${NODE_CLOCK_FEATURES[@]}" "${NODE_SPAWN_FEATURES[@]}"
    "qualification::$qualification" -- --ignored --exact --nocapture)
  if [[ -n "${NODE_BOOT_LIBTEST:-}" ]]; then
    # Cargo runs each test with its package directory as the working directory.
    test_dir="$GAME_SOURCE/apps/game-server"
    test_command=("$NODE_BOOT_LIBTEST" "qualification::$qualification" --ignored --exact --nocapture)
  fi
  (
    cd "$test_dir"
    env "${NODE_GAMEPLAY_ENV[@]}" "${test_env[@]}" \
      NODE_BOOT_SPELL_MANIFEST="$staged_manifest" \
      NODE_BOOT_CONTROL_SOCKET="$BASE/run/control.sock" \
      NODE_BOOT_ADDRESS="127.0.0.1:${NODE_BOOT_GAME_PORT:-17181}" \
      NODE_BOOT_GAMEPLAY_CERT="$WP5_PKI/gameplay.crt" \
      NODE_BOOT_DATABASE_URL="$ADMIN_URL" \
      NODE_BOOT_WORLD_ID="$WORLD_ID" NODE_BOOT_CHANNEL_ID="$CHANNEL_ID" \
      WP5_S3A_PORT="$WP5_PORT" WP5_S3A_SERVER_NAME="${WP5_SERVER_NAME:-source.test}" \
      WP5_S3A_SOURCE_AUTHORITY="${WP5_SOURCE_AUTHORITY:-platform}" WP5_S3A_ROUTE_REVISION="${WP5_ROUTE_REVISION:-route-s3b-1}" \
      WP5_S3A_CA_CERT="$WP5_PKI/server-ca.crt" \
      WP5_S3A_CLIENT_CERT="$WP5_PKI/client.crt" WP5_S3A_CLIENT_KEY="$WP5_PKI/client.key" \
      WP5_S3A_ACCOUNT_ID="$ACCOUNT_ID" WP5_S3B_SECOND_ACCOUNT_ID="$SECOND_ACCOUNT_ID" \
      WP5_S3B_FRESH_KEY_ID="$FRESH_KEY_ID" WP5_S3B_FRESH_KEY_SEED="$FRESH_SEED_HEX" \
      WP5_S3B_RECOVERY_KEY_ID="$RECOVERY_KEY_ID" WP5_S3B_RECOVERY_KEY_SEED="$RECOVERY_SEED_HEX" \
      "${test_command[@]}"
  )
}

graceful_shutdown() { # ready=false first, socket removed
  node_signal -TERM "$NODE_PID"
  local code=0
  wait "$NODE_PID" 2>/dev/null || code=$?
  # A process started by another stage's subshell is not our child; wait for it.
  for _ in $(seq 1 30); do node_signal -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "event=readiness .*ready=false" "$node_log" || fail "no ready=false on shutdown"
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "no clean shutdown"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after graceful shutdown"
  [[ "$(echo 'SELECT ready FROM game_durability_admission_runtime_guards' | psql_admin oteryn_node_boot)" == f ]] || fail "guard still ready"
  evidence "shutdown ready=false socket=removed"
}

restart_without_supersession() { # fails at S2 custody, never ready
  ops authorization issue --file launch-b.json --binding node-boot-b
  write_node_config launch-b.json
  local code=0
  "${as_service[@]}" env "${NODE_GAMEPLAY_ENV[@]}" timeout 120 "$BASE/bin/oteryn-game-server" serve --config "$BASE/node/node.toml" "${NODE_NPC_ARGS[@]}" 2> "$WORK/restart.log" || code=$?
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
  local -a report_args=()
  assignment_report_args
  ops assignment replace --request replace-c.json --world "$WORLD_ID" --channel "$CHANNEL_ID" --node-id "$node" --revision "$revision" "${report_args[@]}"
  # A new incarnation starts with no active Content and reactivates only the current issuance.
  await_log "event=content_activated .*activation_sequence=1 " "$NODE_ACTIVATION_WAIT_SECONDS"
  await_log "event=readiness .*ready=true" 60
  node_signal -TERM "$NODE_PID"
  for _ in $(seq 1 30); do node_signal -0 "$NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "event=shutdown .*state=complete" "$node_log" || fail "replacement did not shut down cleanly"
  evidence "replacement=superseding custody=claimed content=reactivated_current_issuance readiness=cas"
}

signal_before_ready() { # SIGTERM in the assignment wait: clean exit, never ready
  ops authorization issue --file launch-e.json --binding node-boot-e --supersedes "$node_c"
  write_node_config launch-e.json
  : > "$node_log"
  start_node
  await_log awaiting_assignment 120
  node_signal -TERM "$NODE_PID"
  local code=0
  wait "$NODE_PID" 2>/dev/null || code=$?
  cat "$node_log"
  [[ $code == 0 ]] || fail "signal in the assignment wait exit=$code"
  grep -q 'event=shutdown .*signal before ready' "$node_log" || fail "no pre-ready shutdown event"
  ! grep -q "event=readiness .*ready=true" "$node_log" || fail "ready published after the signal"
  [[ ! -e "$BASE/run/control.sock" ]] || fail "socket left after a pre-ready signal"
  [[ "$(echo 'SELECT ready FROM game_durability_admission_runtime_guards' | psql_admin oteryn_node_boot)" == f ]] || fail "guard became ready"
  evidence "signal_in_assignment_wait exit=0 ready=never socket=absent"
}

# The Open PvP World (world_type=2) of the World pass, beside the unchanged no-PvP World.
# One `serve` process serves one World/Channel scope (node/config.rs ScopeConfig), and one
# database holds one live node's S2 custody (native_admission_source.rs, custody claim),
# so this World gets its own database in the same PostgreSQL container and its own node.
# Platform, PKI, secrets, binaries, bundle and gameplay inputs are shared; the steps mirror
# operator_setup, node_deployment_grants, node_complete_assignment and character_bootstrap.
pvp_ops() { "${as_operator[@]}" env "${NODE_GAMEPLAY_ENV[@]}" "$BASE/bin/oteryn-game-ops" --config "$PVP_BASE/ops/ops.toml" "$@"; }
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
  install_owned root:root -d -m 0755 "$PVP_BASE" "$PVP_BASE/state" "$PVP_BASE/fence-parent" "$PVP_BASE/node"
  install_owned root:root -d -m 0700 "$PVP_BASE/ops"
  install_owned "$SERVICE_UID" -d -m 0700 "$PVP_BASE/fence-parent/fence" "$PVP_BASE/run"
  sed -e "s|^state_directory = .*|state_directory = \"$PVP_BASE/state\"|" \
    -e "s|^database = .*|database = \"$PVP_DATABASE\"|" \
    -e "s|^fence_directory = .*|fence_directory = \"$PVP_BASE/fence-parent/fence\"|" \
    "$WORK/ops.toml" > "$WORK/pvp-ops.toml"
  install_owned root -m 0600 "$WORK/pvp-ops.toml" "$PVP_BASE/ops/ops.toml"
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
    -e "s|^epoch_fence_file = \"$BASE/run/lcfa-epoch-fence\"$|epoch_fence_file = \"$PVP_BASE/run/lcfa-epoch-fence\"|" \
    -e '/^\[platform\.runtime_status\]$/,/^$/{ /^$/!d; }' \
    "$WORK/node.toml" > "$WORK/pvp-node.toml"
  # Platform's testing assignment reporter is authorized only for the baseline
  # scope. PvP uses Game's scoped assignment and its own LCFA epoch fence.
  printf '0\n' | install_owned "$SERVICE_UID" -m 0600 /dev/stdin "$PVP_BASE/run/lcfa-epoch-fence"
  local line
  for line in "address = \"127.0.0.1:$PVP_GAME_PORT\"" "world_id = \"$PVP_WORLD_ID\"" "channel_id = \"$PVP_CHANNEL_ID\"" \
    "database = \"$PVP_DATABASE\"" "socket_path = \"$PVP_BASE/run/control.sock\"" "[world_bundle]"; do
    grep -qxF "$line" "$WORK/pvp-node.toml" || fail "PvP node configuration lacks: $line"
  done
  install_owned root -m 0644 "$WORK/pvp-node.toml" "$PVP_BASE/node/node.toml"
  pvp_ops s2 issue --node-config "$PVP_BASE/node/node.toml" --file s2-fresh-store.json --namespace node-boot-pvp --authorization disposable-qualification
  pvp_ops character fresh-store --request fresh-store.json
  pvp_ops character interpretation --profile "${INTERPRETATION[0]}" --ruleset "${INTERPRETATION[1]}" --content "${INTERPRETATION[2]}" --starter "${INTERPRETATION[3]}"
  launch_node PVP_NODE_PID "$PVP_BASE/node/node.toml" "$WORK/pvp-node.log"
  remember PVP_NODE_PID "$PVP_NODE_PID"
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
    echo 'PREPARE node_boot_world_epoch(UUID, UUID, TEXT, TEXT) AS'
    cat "$GAME_SOURCE/tools/qualification/node_boot/world_epoch.sql"
    printf "EXECUTE node_boot_world_epoch('%s', '%s', '%s', '%s');\n" "$PVP_WORLD_ID" "$PVP_CHANNEL_ID" "$WORLD_DIGEST" "$WORLD_DIGEST"
  } | psql_admin "$PVP_DATABASE")"
  [[ "$epoch_result" == t ]] || fail "PvP World epoch initialization"
  if [[ "$NODE_BOOT_UNPRIVILEGED" == 1 ]]; then
    [[ "$(stat -c %a "$PVP_BASE/run/control.sock")" == 600 ]] || fail "PvP control socket mode"
  else
    [[ "$(sudo stat -c '%u %a' "$PVP_BASE/run/control.sock")" == "$SERVICE_UID 600" ]] || fail "PvP control socket mode"
  fi
  [[ "$PVP_GAME_PORT" != "$NODE_BOOT_GAME_PORT" && "$PVP_NODE_PID" != "$NODE_PID" ]] || fail "PvP listener or process reused the baseline node"
  [[ "$(echo "SELECT world_type FROM game_spell_field_world_policies WHERE world_id='$WORLD_ID'" | psql_admin oteryn_node_boot)" == 1 ]] || fail "Baseline World policy is not no-PvP"
  evidence "pvp_world world_type=2 database=$PVP_DATABASE port=$PVP_GAME_PORT node=separate_process readiness=true world_epoch=0 boot_seconds=$((SECONDS - boot_started)) world=$PVP_WORLD_ID channel=$PVP_CHANNEL_ID pid=$PVP_NODE_PID baseline_port=$NODE_BOOT_GAME_PORT baseline_pid=$NODE_PID baseline_world_type=1"
}

pvp_character_bootstrap() { # two level-qualifiable Characters from real Platform intents
  local index account operation name failed=0
  local vocations=(Knight Sorcerer)
  for index in 0 1; do
    account="${PVP_ACCOUNT_IDS[$index]}"; operation="${PVP_INTENT_OPERATIONS[$index]}"; name="${PVP_INTENT_NAMES[$index]}"
    if ! python3 "$GAME_SOURCE/tools/qualification/game_platform/composition.py" bootstrap \
      "$WORK" "$account" "$PVP_WORLD_ID" "$operation" "${INTERPRETATION[@]}" "$name" "${vocations[$index]}"; then
      echo "platform intent issue failed for PvP operation $operation"
      failed=1
      continue
    fi
    pvp_ops character bootstrap --socket "$PVP_BASE/run/control.sock" --operation-id "$operation" || { failed=1; continue; }
    pvp_ops character bootstrap --socket "$PVP_BASE/run/control.sock" --operation-id "$operation" || failed=1
  done
  [[ $failed == 0 ]] || pvp_fail "PvP Character bootstrap"
  [[ "$(echo "SELECT count(*) FROM game_character_roots WHERE world_id='$PVP_WORLD_ID'" | psql_admin "$PVP_DATABASE")" == 2 ]] || fail "PvP World Characters"
  local births
  births="$(psql_admin "$PVP_DATABASE" <<SQL
SELECT r.creation_vocation, r.starter_template_revision, p.projection_revision
FROM game_character_roots r
JOIN game_character_account_projections p USING (account_id)
WHERE r.world_id = '$PVP_WORLD_ID' AND r.account_id IN ('${PVP_ACCOUNT_IDS[0]}', '${PVP_ACCOUNT_IDS[1]}')
ORDER BY r.account_id;
SQL
)"
  [[ "$births" == "knight|${INTERPRETATION[3]}|1"$'\n'"sorcerer|${INTERPRETATION[3]}|1" ]] || pvp_fail "PvP bootstrap vocation, starter or account projection readback"
  evidence "pvp_characters=2 world=$PVP_WORLD_ID source=platform_intents path=pvp_control_socket retry=idempotent contract=4 starter=${INTERPRETATION[3]} births=knight,sorcerer projection_revision=1"
}

pvp_world_shutdown() { # the Open PvP World's node ends cleanly before the lifecycle probes
  node_signal -TERM "$PVP_NODE_PID"
  for _ in $(seq 1 30); do node_signal -0 "$PVP_NODE_PID" 2>/dev/null || break; sleep 1; done
  grep -q "event=shutdown .*state=complete" "$WORK/pvp-node.log" || pvp_fail "PvP node did not shut down cleanly"
  [[ ! -e "$PVP_BASE/run/control.sock" ]] || fail "PvP socket left after graceful shutdown"
  evidence "pvp_world shutdown=complete socket=removed"
}

if [[ "${NODE_BOOT_LIFECYCLE:-}" == game_platform ]]; then
  game_platform_identity
  exit "$?"
fi

seam_stage_name=seam_stages
[[ "$NODE_BOOT_SPELLS" != 1 ]] || seam_stage_name=spell_scenarios
if [[ "$NODE_BOOT_REUSE_STACK" == 1 ]]; then
  stage "$seam_stage_name" -- seam_stages
else
stage operator_setup -- operator_setup
stage configuration_negatives operator_setup -- configuration_negatives
assignment_needs=operator_setup
if [[ "$NODE_BOOT_D150_LEGACY" == 1 ]]; then
  stage legacy_d150_seed operator_setup -- legacy_d150_seed
  assignment_needs=legacy_d150_seed
fi
stage node_assigned_ready "$assignment_needs" -- node_assigned_ready
stage control_socket_peer node_assigned_ready -- control_socket_peer
if [[ "$NODE_BOOT_D150_LEGACY" == 1 ]]; then
  stage legacy_d150_register_starter node_assigned_ready -- legacy_d150_register_starter
  stage "$seam_stage_name" legacy_d150_register_starter -- seam_stages
else
  stage character_bootstrap node_assigned_ready -- character_bootstrap
  if [[ "$NODE_BOOT_WORLD" == 1 ]]; then
    # MP01's PvP rows report this World's absence themselves, so the seam stage does not need it.
    stage pvp_world_ready node_assigned_ready -- pvp_world_ready
    stage pvp_character_bootstrap pvp_world_ready -- pvp_character_bootstrap
  fi
  stage "$seam_stage_name" character_bootstrap -- seam_stages
fi
fi
if [[ "$NODE_BOOT_KEEP_STACK$NODE_BOOT_REUSE_STACK" == 00 ]]; then
if [[ "$NODE_BOOT_WORLD" == 1 && "$NODE_BOOT_D150_LEGACY" == 0 ]]; then
  stage pvp_world_shutdown pvp_world_ready -- pvp_world_shutdown
fi
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
    NODE_PID="$(source "$VARS"; printf '%s' "${NODE_PID:-}")"
    PVP_GAME_PORT="$(source "$VARS"; printf '%s' "$PVP_GAME_PORT")"
    [[ -z "$NODE_PID" ]] || save_reuse_state
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
  if [[ "$NODE_BOOT_D150_LEGACY" == 1 ]]; then
    result=NODE_BOOT_D150_PASS
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
