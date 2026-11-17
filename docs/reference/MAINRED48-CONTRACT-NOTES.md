# MAINRED48 contract notes

- The default node-boot script runs the fixture pass first, then the world pass;
  both world mode and explicit spell mode stage the full gameplay manifest.
- CI's assignment `NotGranted` and first content-activation rejection are the
  script's expected negative checks. The missing-event assertion occurs later.
- Exact CI node-event tail requested from the coordinator; the job-log storage
  host is unavailable through this environment's network proxy. The host has
  no `sudo`; an isolated Ubuntu 24.04 Docker runner was used for the
  required full-topology attempts.
- Closest local production harness: qualify the committed manifest as the
  operator, then activate it through the node's real activation controller with
  the resulting exact World, sequence, server/client digests and frame binding.
  Idle measurements: operator 31.917s; node activation 48.227s; matching pins pass.
- Under six CPU load workers, valid node activation completed in 122.451s and
  failed the old 60s qualification wait. The same profile now allows 180s; fixture
  activation stays at 60s. Assignment/readiness and authority checks are unchanged.
- GREEN with the same six-worker setup: operator 44.775s; node activation
  97.062s; total 142.06s. Exact issued server digest and sequence remain verified;
  the production activation API also checks client digest and frame binding.

- Whole-script fixture/world and explicit full-spell attempts stop before node
  startup at `tools/qualification/node_boot/run.sh:194`: Compose MariaDB
  initialization reports errno 28 (No space left on device). The same pinned
  MariaDB image initializes successfully standalone. Full-topology validation
  remains incomplete; no Platform-stage success is claimed. The full-spell
  production content preflight passed in 118.79s.
- Workspace check, changed-crate clippy and formatting passed before the final
  main merge (which changed workflow/repository-policy files only). Final
  workspace check and formatting are rerun after that merge. The additional
  game-server library unit-test build was stopped when disk space ran out;
  those unit tests remain pending. The new production integration case ran
  red and green with measured durations as recorded above.
