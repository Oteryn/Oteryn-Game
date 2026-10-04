#!/usr/bin/env bash
# Reuse the existing engine tests, dev client and native room; no second topology.
set -euo pipefail
readonly SPELL_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$SPELL_REPO_ROOT"
mode="${1:-help}"
if [[ $# -gt 0 ]]; then shift; fi
prepare_current_manifest() {
  SPELL_CURRENT_MANIFEST="$SPELL_REPO_ROOT/content/spells.manifest.json"
  [[ -f "$SPELL_CURRENT_MANIFEST" ]] || { echo 'Imported spell manifest missing' >&2; exit 2; }
}
case "$mode" in
  runtime)
    # Existing composed-content tests require a larger test stack, not a production change.
    export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
    exec cargo test --locked -p oteryn-game-server --lib "${1:-spell::}" -- --nocapture
    ;;
  map)
    export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
    if [[ $# -eq 0 ]]; then prepare_current_manifest; fi
    export OTERYN_FULL_SPELL_TEST_MANIFEST="${1:-${SPELL_CURRENT_MANIFEST:-}}"
    [[ -f "$OTERYN_FULL_SPELL_TEST_MANIFEST" ]] || { echo 'Spell map manifest missing' >&2; exit 2; }
    cargo test --locked -p oteryn-game-server --lib \
      content::native_gameplay::tests::actual_full_manifest_qualifies_source_world_and_all_owner_profiles \
      -- --ignored --exact --nocapture
    ;;
  client)
    exec cargo test --locked -p oteryn-synthetic-client-harness
    ;;
  live)
    exec cargo run --locked -p oteryn-synthetic-client-harness -- --live "$@"
    ;;
  server)
    if [[ $# -gt 1 ]]; then echo 'server accepts at most one manifest path' >&2; exit 2; fi
    if [[ $# -eq 1 ]]; then
      export OTERYN_SEAM_SPELL_MANIFEST="$1"
    else
      prepare_current_manifest
      export OTERYN_SEAM_SPELL_MANIFEST="$SPELL_CURRENT_MANIFEST"
    fi
    export WP5_QUALIFICATION=spell-seam
    bash tools/qualification/wp5_s3b/run.sh
    ;;
  room)
    exec bash tools/qualification/native_entry_room/run.sh "$@"
    ;;
  help|--help|-h)
    cat <<'USAGE'
Usage: bash tools/qualification/spells/run.sh MODE [ARGS]
  runtime [TEST_FILTER]  existing fast spell engine tests (default: spell::)
  map [MANIFEST]         qualify existing Thalom map + full gameplay manifest
  client                 harness parsing/model/TLS-loopback regression tests
  live [LIVE_ARGS]       existing dev client, interactive or --script FILE
  server [MANIFEST]      existing real-owner TCP/TLS server spell qualification
  room                   existing Platform-issued room qualification
Live mode needs the existing server, its trusted CA and an admission grant.
USAGE
    ;;
  *) echo "Unknown spell test mode: $mode" >&2; exit 2 ;;
esac
