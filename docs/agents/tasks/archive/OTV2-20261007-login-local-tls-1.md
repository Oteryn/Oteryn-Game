# OTV2-20261007-login-local-tls-1

```yaml
task_id: OTV2-20261007-login-local-tls-1
title: LOGIN-LOCAL-TLS-1 gateway reaches Platform over per-run test TLS
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/login-local-tls-1-20261007
pr: 1915
base_sha: 66f0d433
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: LOGIN-LOCAL-TLS-1 writer (control plane D607, coordination #1622, issue #1914)
created_at: 2026-10-07T00:00:00Z
updated_at: 2026-10-07T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/qualification/login_local/**
  - docs/agents/tasks/archive/OTV2-20261007-login-local-tls-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: 1622
external_repositories:
  - Oteryn/Oteryn-Platform@b18d32d30c4c4e496330077d12b37db3f0f29011 (read only)
```

## Outcome

The `login_local` gateway no longer exits `configuration_invalid`: it reaches Platform at `https://nginx:8444` and trusts only a per-run test CA.

## Architecture and source of truth

- PROVEN (CP report): the Platform gateway config validator accepts `http` only for loopback hosts.
- DERIVED: Go on Linux loads the default certificate directories alongside `SSL_CERT_FILE`, so `SSL_CERT_DIR` must point at an empty directory for the per-run CA to be the only root (Codex P2 4204518930).

## Acceptance criteria

- [x] `run.sh` generates a test CA and an nginx leaf (SAN `nginx`) in its 0700 PKI directory; nothing is committed.
- [x] nginx serves the native-admissions route on an unpublished TLSv1.3 listener.
- [x] Gateway trusts the CA via a read-only `SSL_CERT_FILE` with an empty read-only `SSL_CERT_DIR`; `run.sh` checks both before starting the gateway.
- [x] No `GAME_SESSION_SERVICE_*` variables; `GATEWAY_NATIVE_LOGIN_ENABLED` is `true`.
- [x] Platform pin bumped to the #1472 merge commit.

## Excluded scope

`wp5_s3a`, `wp5_s3b`, `node_boot`, workflows, gate files, Platform writes.

## Validation

- `bash -n tools/qualification/login_local/run.sh`: pass
- `shellcheck -x tools/qualification/login_local/run.sh`: pass
- overlay YAML parse: pass
- `docker compose config gateway` with the four compose files and dummy env: pass (`SSL_CERT_DIR`/`SSL_CERT_FILE` set, both read-only mounts merged; matches the run.sh check)
- `openssl verify -purpose sslserver -verify_hostname nginx` on the generated chain: OK
- full `run.sh`: NOT_APPLICABLE, Docker daemon unavailable
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

Whole diff re-read; no keys, certificates or secrets committed; only owned paths changed.

## PR and closeout

- merge commit/result: squash merge of #1915 (pending)
- review: Codex P2 4204518930 addressed; control plane requests re-review and auto-merge
