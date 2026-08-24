# OTV2-20260822-impl-domain-core

```yaml
task_id: OTV2-20260822-impl-domain-core
title: Implement Character and Item semantic domain core
mode: IMPLEMENT
status: review_pending
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-domain-core-01
issue: 55
pr: 56
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
head_sha: null
final_head_sha: null
final_head_frozen_at: null
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
integration_base_sha: 55e30e23c3d5775ce760c6b210ea77f152b359ae
owner: chat-github-20260818-implementation-coordinator
previous_owner: chat-github-20260822-domain-core
ownership_transfer_reason: coordinator autonomous continuation after shared lease activation
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-24T10:35:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
shared_lease_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
public_contracts:
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
  - docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
  - OTV2-20260818-impl-simulation
blocks:
  - OTV2-IMPL-DURABILITY
  - OTV2-IMPL-ABILITY
  - OTV2-IMPL-INTERACTION
  - OTV2-IMPL-AI
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver protocol- and persistence-neutral Character/Item semantic primitives required by the first VSL, with deterministic legality/transitions and explicit revision/fixture context. The active coordinator lease now composes the module through the production game-server crate while executable gameplay remains fail-closed.

## Architecture and source of truth

- `PROVEN`: PR #46 merged as `fd39c6aa026e82062a8b29af24811d467c115f19`, activating the Wave 1 DOMAIN allocation.
- `PROVEN`: this lane owns only `apps/game-server/src/domain/**` plus this task record until the coordinator advances the shared lease.
- `PROVEN`: Character owns durable lifecycle/build/progression semantics; exact formulas remain ruleset/SIM-owned.
- `PROVEN`: GAME-ITEM owns semantic item/equipment/container legality while DUR-03 owns authoritative location transitions, conservation and transaction mechanics.
- `PROVEN`: DUR-03 requires typed inventory/equipment/container/ground immediate-location vocabulary and forbids generic owner/location JSON/EAV authority.
- `PROVEN`: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` currently contains no product item/container ceilings; this delivery therefore embeds no product/Reference limit values.
- `UNKNOWN`: unresolved Reference formula/rate/limit values remain unavailable to product activation and are represented only by explicitly fixture-only structural test inputs where needed.

## Acceptance criteria

- [x] Typed Character UUIDv7 identity, lifecycle and revision-fenced transition semantics reject invalid/stale/terminal transitions.
- [x] Character build primitives support explicit unselected state plus stable versioned profession/promotion definitions without a universal vocation enum.
- [x] Typed progression facts bind stable definition identity/revision without implementing Reference arithmetic.
- [x] Explicit profile/ruleset/content/starter context and item-definition revision compatibility fail closed.
- [x] Typed ItemDefinition/ItemInstance identities plus inventory/equipment/container/ground location vocabulary and definition-level placement legality exist.
- [x] Equipment multi-slot and container self-cycle/cycle/direct/depth/reachable legality are deterministic; detached-subtree and ancestor-limit cases are covered.
- [x] Versioned structural fixture profiles cannot activate as ordinary product policy.
- [x] Stable typed errors contain no wire IDs, SQL, UI or generic untyped misc state.
- [x] Standalone domain production compile proves no transport/persistence/UI/external crate dependency.
- [x] Shared game-server composition is integrated after the coordinator lawfully transferred the serialized shared-path lease to DOMAIN.
- [ ] Exact-head PR CI and terminal closeout complete after composition integration.

## Excluded scope

No PostgreSQL/schema/transactions, protocol wire IDs, UI, entitlement authority, exact Reference XP/skill/stat/death formulas, permanent PvP profile, production item/container ceilings, content bundle format or shared composition mutation without coordinator lease.

## Implementation / findings

- Extended the existing domain kernel with typed Character build/progression primitives and explicit context compatibility.
- Added typed immediate item-location surfaces (`CharacterInventory`, `CharacterEquipment`, `Container`, `Ground`) and placement/world-scope validation while leaving all durable transitions to DUR-03.
- Added item-definition revision rejection and a versioned structural fixture profile whose product activation path fails closed.
- Hardened containment legality: an already-parented child fails closed, detached subtree depth is included, and reachable-item limits are checked for every affected ancestor.
- Repaired lifecycle revision exhaustion so a failed successor allocation cannot leave a partially mutated lifecycle state.
- Coordinator integration merged current main, composed `pub mod domain` through `apps/game-server/src/lib.rs`, and kept gameplay fail-closed.
- Composition exposed `clippy::double_must_use` on `EquipPattern::claims`; the targeted repair removes only the redundant attribute.
- No contract, registry, workflow, persistence, protocol, UI, new crate topology or external repository was modified.

## Validation

### Focused
- `cargo +1.94.0 test --locked -p oteryn-game-server --lib` — PASS, 114/114 including all 10 DOMAIN tests.
- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` — PASS after the one-line `double_must_use` repair.

### Component/integration
- `cargo +1.94.0 test --locked --workspace --all-targets` — PASS.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings` — PASS.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .` — PASS (`workspace-boundaries: PASS`).
- `python tools/agents/validate_governance.py` — PASS (25 policy documents / 9 lanes).
- `git diff --check` — PASS.
- Windows `cargo fmt --all --check` is not used as authority because this checkout materializes repository-wide CRLF and reports unchanged main files; exact-head Linux CI remains authoritative for format.

### E2E
- `NOT_EVALUATED`: DOMAIN is semantic/composition scope only; no production gameplay listener/client journey is introduced by this PR.

### Exact-head CI
- pending final pushed head and repository-required exact-head workflows.

## Self-review

- exact head: recorded in immutable PR review evidence after the final content head is frozen; do not self-reference a commit from this file
- method/reviewer: implementation coordinator takeover; complete three-file diff plus accepted GAME-CHAR/GAME-ITEM/FND-ID/SIM/DUR-03/GAME-CHANNEL boundaries
- prior repaired findings: lifecycle revision-exhaustion partial mutation, two containment bound gaps, and integration-time `clippy::double_must_use`
- current material findings: pending final exact-head whole-diff self-review
- verdict: pending final exact-head whole-diff self-review

## Independent review

- required: YES — coordinator policy requires genuinely independent exact-head review for item semantic changes.
- exact head: pending final pushed head
- method/auditor: pending qualifying independent reviewer/audit mechanism
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file scope: exactly `apps/game-server/src/domain/mod.rs`, `apps/game-server/src/lib.rs`, and this task record
- unresolved review threads: must be zero on the final exact head
- related coordinator lineage: stale PR #50 closed unmerged; PR #76 transferred the shared lease; PR #78 recorded DOMAIN lease active
- protected merge: not eligible until final exact-head self-review, genuinely independent exact-head review and required CI are all clean
- merge commit/result: pending
- ownership release: pending post-merge archive/closeout

## Context checkpoint

```yaml
last_progress: Current main 55e30e23c3d5775ce760c6b210ea77f152b359ae was merged into the worker branch; coordinator-owned game-server composition now exposes DOMAIN through pub mod domain while gameplay remains fail-closed. Integration found one Clippy double_must_use defect in EquipPattern::claims and repaired it minimally. Game-server 114/114, full workspace tests, package/full Clippy, architecture-check, governance and diff checks pass locally.
status: review_pending
branch: agent/otv2-impl-domain-core-01
head_sha: null
pr: 56
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: final exact-head self-review, genuinely independent exact-head review, and repository CI remain before readiness/merge
next_action: push this final lifecycle-metadata correction, freeze the resulting exact head in PR evidence, self-review the full three-file diff, obtain a qualifying independent exact-head review, and require exact-head repository gates before merge.
```
