# OTV2-20260826-durability-cdla-license-lease

```yaml
task_id: OTV2-20260826-durability-cdla-license-lease
title: Admit the SQLx webpki-roots license through a serialized policy lease
mode: COORDINATE
status: allocation_pending
repository: Oteryn/Oteryn-Game
base_branch: main
allocation_branch: coord/durability-cdla-license-lease
implementation_branch: coord/durability-shared-cargo-ci
issue: 162
blocked_issue: 167
base_sha: 54623daef2c1b22ed1f463604940c33f5773e8a6
owner: Oteryn: work coordinator
owned_paths:
  - deny.toml
public_contracts: []
depends_on:
  - PR #181 shared Cargo/CI lease
  - PR #182 exact-head supply-chain failure on webpki-roots 1.0.9
blocks:
  - PR #182 merge gate
  - Issue #167 resume from WAITING_EXTERNAL
external_repositories: []
```

## Purpose

Acquire one serialized coordinator policy path after PR #182 proved that the accepted SQLx `=0.9.0` dependency introduces `webpki-roots 1.0.9`, whose SPDX license `CDLA-Permissive-2.0` is rejected solely because it is absent from the current `deny.toml` allow list. This is supply-chain policy integration for an already accepted dependency, not a persistence or product architecture decision.

## Exact implementation authority after allocation merge

- add exactly `CDLA-Permissive-2.0` to `[licenses].allow` in `deny.toml`;
- change no other license/advisory/ban/source policy;
- do not change SQLx version/features or any runtime code;
- integrate the change only into the existing PR #182 branch after refreshing it to current protected `main`.

## Excluded scope

No wildcard/unknown license admission, no advisory/bans/source weakening, no runtime/schema/persistence semantic change, no production database/config/secrets, no registry/stable-ID, no Platform/Atlas/META/external-repository write, and no reopening or modification of terminal #115/#131.

## Required validation

- exact-head `cargo-deny check --all-features` through the protected merge gate;
- all existing PR #182 Rust/governance/security gates;
- genuinely independent exact-head review after the final head moves;
- expected-head merge only after zero material findings and all gates PASS.

## Context checkpoint

```yaml
last_progress: PR #182 exact head passed build/test/governance but failed only Rust supply chain because CDLA-Permissive-2.0 is not allowed
status: allocation_pending
blocker: deny.toml is outside PR #181 shared lease
next_action: merge this one-path allocation, then add the exact SPDX license to PR #182 and rerun final review/CI
```
