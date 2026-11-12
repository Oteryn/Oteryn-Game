#!/usr/bin/env bash
# The fixture and full-manifest activation profiles keep separate bounded waits.
node_activation_wait_seconds() {
  case "${1:?World mode required}:${2:?spell mode required}" in
    0:0) printf '60\n' ;;
    # Canonical full-manifest activation: 48.227s idle, 122.451s under CPU
    # contention (manifest 35de21c6, production qualification/staging path).
    0:1|1:0|1:1) printf '180\n' ;;
    *) return 2 ;;
  esac
}
