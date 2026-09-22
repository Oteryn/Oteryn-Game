> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #675 merged as `c2b755b3ff5996a4f18001bbf9f935c5dd59b3b6`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-626-a-platform-ipv6-loopback-http

```yaml
task_id: OTV2-20260919-626-a-platform-ipv6-loopback-http
title: Repair Platform IPv6 loopback HTTP admission
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/626-a-platform-ipv6-loopback-http
issue: 626
pr: null
base_sha: eb122df0a94e1b461c882ae66a394058c595711c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl client"
created_at: 2026-09-19T17:04:18+02:00
updated_at: 2026-09-19T17:14:11+02:00
execution_policy: continuous_progress
owned_paths:
  - crates/platform-client/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260919-626-a-platform-ipv6-loopback-http.md
public_contracts: []
depends_on:
  - "#162 comment 5742747008"
  - "#626 Finding A"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```
## Outcome

Make the intended local-development HTTP exception admit IPv6 loopback while preserving the exact transport trust boundary: HTTPS remains accepted, and HTTP remains limited to typed host equality with `localhost`, `127.0.0.1`, or `::1`.

## Authority and source of truth

- **FACT** — #162 comment `5742747008` allocates exactly the two owned paths above to `Oteryn: impl client`.
- **FACT** — admission branch/head is `agent/626-a-platform-ipv6-loopback-http@eb122df0a94e1b461c882ae66a394058c595711c`.
- **FACT** — protected `main` advanced to `c7688069bc22ac3cde46e48e6b05d8051418fed1` by one path-disjoint Content/World commit.
- **FACT** — fresh census of all 28 open PRs found zero overlap with either custody path.
- **FACT** — reqwest 0.13.4 re-exports `url::Url`; `Url::host()` provides typed host values without adding a direct dependency.
- **FACT** — the previous `host_str()` comparison observes bracketed serialized IPv6 and rejects `http://[::1]/`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this repair changes local Platform-client URL admission only. It does not perform a production mutation, authorize PREPARE/COMMIT, restore an authority-bearing session, or interpret persisted recovery authority.

## Acceptance criteria

- [x] RED reproduces rejection of `http://[::1]/` through `PlatformClientConfig::new`.
- [x] HTTP `localhost`, `127.0.0.1`, `::1`, and parser-equivalent IPv6 loopback are accepted.
- [x] arbitrary hostname, non-loopback IPv4, and non-loopback IPv6 HTTP remain rejected.
- [x] HTTPS behavior remains accepted unchanged.
- [x] repository-required local validation is green.
- [ ] exact-head repository CI is green.
- [ ] genuinely independent exact-head review has zero material findings.
## Excluded scope

No Cargo/lock mutation, dependency addition, Platform repository mutation, workflow/governance mutation, protocol/session/gameplay routing change, redirect/proxy-policy change, HTTPS relaxation, response/cancellation/directory semantic change, broader loopback range, suffix/subdomain localhost rule, or production authority.

## Implementation

The repair keeps `PlatformClientConfig::new` as the only exercised decision boundary. It replaces serialized-host text matching with typed host equality from the existing `reqwest::Url` parser. The candidate host is compared only against three parser-produced typed hosts for `localhost`, `127.0.0.1`, and `::1`.

No helper-only acceptance path, direct `url` dependency, or IP-range predicate is introduced.

## Validation

### RED

`cargo +1.94.0 test --locked -p oteryn-platform-client transport_scheme_and_loopback_allowlist_are_enforced -- --nocapture`

Result before implementation repair: **FAILED** exactly at `http://[::1]/` with the test message `expected http://[::1]/ to be accepted`.

### Focused GREEN

The same focused command after the repair: **PASS**, 1 passed / 0 failed.

### Repository gates

- `cargo +1.94.0 fmt --all --check` — **PASS**.
- `cargo +1.94.0 test --locked -p oteryn-platform-client` — **PASS**, 3 passed / 0 failed.
- `cargo +1.94.0 clippy --locked -p oteryn-platform-client --all-targets -- -D warnings` — **PASS**.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .` — **PASS**.
- `cargo +1.94.0 build --locked --workspace --all-targets` — **PASS**.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings` — **PASS**.
- `cargo +1.94.0 test --locked --workspace` — **PASS**.
- `cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness` — **PASS**.
- Windows `x86_64-pc-windows-msvc` release client build, strict client Clippy, exact release `--smoke`, synthetic harness and simulation-determinism tests — **PASS**; determinism 7 passed / 0 failed.
- configured PostgreSQL 17.6 evidence — **NOT_RUN locally**; the routing-selected hosted FULL gate remains authoritative for real PG17.6.
- `python tools/agents/validate_governance.py` — **PASS**, 26 required policy documents / 9 project lanes.
- `git diff --check` — **PASS**.
- exact changed-path readback — **PASS**, exactly the two allocated custody paths.
## Self-review

- exact head: pending immutable commit readback; staged/pre-freeze candidate reviewed in full
- method/reviewer: implementing agent, whole-diff adversarial review
- material findings: 0; checked HTTPS preservation, exact typed host equality, non-loopback IPv4/IPv6 rejection, no dependency/Cargo change and no out-of-custody mutation
- verdict: `PASS_ZERO_MATERIAL_FINDINGS`

## Independent review

- required: YES — transport-security allowlist boundary per #162 comment `5742747008`
- exact head: pending
- method/auditor: genuinely independent exact-head reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- protected integration authority: none for this worker
- direct merge: forbidden
- final handoff: `READY_FOR_INTEGRATION` only after exact-head CI and independent review qualify the same head

## Context checkpoint

```yaml
last_progress: RED/GREEN plus component, workspace, Windows, governance and diff validation pass; self-review zero material findings
status: validating
branch: agent/626-a-platform-ipv6-loopback-http
admission_head: eb122df0a94e1b461c882ae66a394058c595711c
current_local_head: eb122df0a94e1b461c882ae66a394058c595711c
pr: null
blocker: null
next_action: freeze and publish immutable candidate, then consume exact-head hosted CI and independent review
```
