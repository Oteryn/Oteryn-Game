# OTV2-20261006-runbook-1-fu

```yaml
task_id: OTV2-20261006-runbook-1-fu
title: RUNBOOK-1-FU login_local readiness and F7
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/runbook-1-fu-20261006
pr: 1884
base_sha: 6560803c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: RUNBOOK-1-FU writer (control plane D844, coordination #1622)
created_at: 2026-10-06T00:00:00Z
updated_at: 2026-10-06T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/qualification/login_local/**
  - docs/agents/tasks/archive/OTV2-20261006-runbook-1-fu.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: 1622
external_repositories:
  - Oteryn/Oteryn-Platform@3896bcdf75a511f1e386ac645303eaf8f234ffcf (read only)
```

## Outcome

`login_local` survives slow MariaDB first initialisation and the preproduction topology step no longer relies on MySQL.

## Architecture and source of truth

- PROVEN: Platform `NativeTopologyRegistry::isolatedConnection()` at the pinned SHA allows `mysql` only when `APP_ENV=testing` and database `oteryn_concurrency`; otherwise only SQLite, as a regular file `<tmp>/oteryn-native-topology-<hex>/oteryn-native-topology.sqlite`. F7 confirmed.
- DERIVED: the fixture is a separate database, so issued world/channel rows are mirrored into the Platform database after issuance.

## Acceptance criteria

- [x] `db` healthcheck `start_period` 300s in `compose.override.yml` only, configurable via `LOGIN_LOCAL_DB_START_PERIOD`; `wp5_s3a` unchanged.
- [x] F7 confirmed; SQLite fixture inside the 0700 work directory.
- [x] README notes (same-host Windows client, WSL2, NAS stops at READY, slow hosts).

## Excluded scope

`wp5_s3a`, workflows, gate files, Platform writes.

## Implementation / findings

Fixture bind-mounted into `platform` at the required `/tmp/oteryn-native-topology-<hex>/`; migrate, world ensure, issue and route publish run with `DB_CONNECTION=sqlite`; ids and route columns are mirrored into MariaDB. Not executed: Docker daemon unreachable here, so the first operator run is the first evidence (SQLite migration compatibility and mirror are the parts to confirm).

## Validation

- `bash -n tools/qualification/login_local/run.sh`: pass
- `shellcheck -x tools/qualification/login_local/run.sh`: pass
- `docker compose config` with the four compose files and dummy env: pass (db `start_period: 5m0s`, platform bind mount merged)
- full `run.sh`: NOT_APPLICABLE, Docker daemon unavailable
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

Whole diff re-read; no secrets; only owned paths changed.

## PR and closeout

- merge commit/result: squash merge of #1884 (pending)
- review: control plane requests Codex review and auto-merge
