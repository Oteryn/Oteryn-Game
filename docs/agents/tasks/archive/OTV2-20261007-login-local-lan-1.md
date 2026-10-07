# OTV2-20261007-login-local-lan-1

```yaml
task_id: OTV2-20261007-login-local-lan-1
title: LOGIN-LOCAL-LAN-1 NAS server plus PC client over the LAN
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/login-local-lan-1-20261007
issue: 1622
pr: 1919
base_sha: c627dd43
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: LOGIN-LOCAL-LAN-1 writer (coordination #1622, D721 native login)
created_at: 2026-10-07T00:00:00Z
updated_at: 2026-10-07T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/qualification/login_local/**
  - docs/agents/tasks/archive/OTV2-20261007-login-local-lan-1.md
public_contracts: []
depends_on: [OTV2-20261007-login-local-tls-1 (#1915, not merged)]
blocks: []
cross_repository_coordination_id: 1622
external_repositories:
  - Oteryn/Oteryn-Platform@b18d32d30c4c4e496330077d12b37db3f0f29011 (read only)
```

## Outcome

`LOGIN_LOCAL_HOST=<private IPv4>` serves the Platform and gateway over per-run LAN-CA TLS and the game node on the LAN IP, so the
server side can run on a NAS and the Windows client on another PC. The default `127.0.0.1` is unchanged.

## Architecture and source of truth

- PROVEN: `crates/platform-client/src/lib.rs` `PlatformClientConfig::new` accepts `http` only for localhost/127.0.0.1/[::1]; other hosts need `https`.
- PROVEN: the client OAuth redirect is its own `127.0.0.1` listener (`crates/identity`), reached by the browser on the same PC.
- DERIVED: reqwest `rustls` feature verifies through the OS trust store, so the PC imports the LAN CA. Not run here.
- UNVERIFIED: Platform behaviour behind the TLS listener (own-URL generation, route host); first real run confirms.

## Acceptance criteria

- [x] Private IPv4 only (10/8, 172.16/12, 192.168/16); 0.0.0.0, public and malformed values are BLOCKED.
- [x] Platform HTTP, gateway and game node on the LAN IP; PostgreSQL and the Platform mTLS port stay on 127.0.0.1.
- [x] Gameplay SAN `DNS:localhost,IP:<host>`; route and `world:ensure` host is the LAN IP; `tls_server_name` stays `localhost`.
- [x] `client.env` URLs use the LAN IP over https; LAN CA and gameplay root written next to it.
- [x] README "NAS server + PC client" with LAN-only firewall ports and a no-router-forwarding warning.

## Excluded scope

Client code, Platform, other qualification directories, workflows, gate files.

## Validation

- `bash -n tools/qualification/login_local/run.sh`: pass
- `shellcheck`: NOT_APPLICABLE, not installed
- host validation matrix (loopback, three private ranges, rejects): pass
- `docker compose config` with and without `compose.lan.yml`: pass (default publishes unchanged)
- `openssl verify -verify_ip` on an `IP:` SAN leaf: OK
- full `run.sh`: NOT_APPLICABLE, pinned Platform checkout not available
- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

Whole diff re-read; no keys, certificates or secrets committed; only owned paths changed.

## PR and closeout

- merge commit/result: squash merge of #1919 (pending; base #1915 merges first)
- review: control plane requests review on the frozen head
- decision: `tls_server_name` stays `localhost` (control plane)
