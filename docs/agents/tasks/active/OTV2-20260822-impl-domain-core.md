# OTV2-20260822-impl-domain-core

```yaml
task_id: OTV2-20260822-impl-domain-core
title: Implement Character and Item semantic domain core
mode: IMPLEMENT
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-domain-core-01
issue: 55
pr: null
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
head_sha: null
final_head_sha: null
final_head_frozen_at: null
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
owner: chat-github-20260822-domain-core
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-22T20:08:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
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

Deliver protocol- and persistence-neutral Character/Item semantic primitives required by the first VSL, with deterministic legality/transitions and explicit revision/fixture context. Shared composition remains blocked until the coordinator transfers the serialized lease to DOMAIN.

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
- [ ] Shared game-server composition is integrated after the coordinator lawfully transfers the serialized shared-path lease to DOMAIN.
- [ ] Exact-head PR CI and terminal closeout complete after composition integration.

## Excluded scope

No PostgreSQL/schema/transactions, protocol wire IDs, UI, entitlement authority, exact Reference XP/skill/stat/death formulas, permanent PvP profile, production item/container ceilings, content bundle format or shared composition mutation without coordinator lease.

## Implementation / findings

- Extended the existing domain kernel with typed Character build/progression primitives and explicit context compatibility.
- Added typed immediate item-location surfaces (`CharacterInventory`, `CharacterEquipment`, `Container`, `Ground`) and placement/world-scope validation while leaving all durable transitions to DUR-03.
- Added item-definition revision rejection and a versioned structural fixture profile whose product activation path fails closed.
- Hardened containment legality: an already-parented child fails closed, detached subtree depth is included, and reachable-item limits are checked for every affected ancestor.
- Repaired lifecycle revision exhaustion so a failed successor allocation cannot leave a partially mutated lifecycle state.
- No shared composition path, contract, registry, workflow, persistence, protocol, UI or external repository was modified.

## Validation

### Focused

- command/run: `rustc --edition 2024 --test apps/game-server/src/domain/mod.rs -D warnings -o target/domain-tests.exe && target/domain-tests.exe`
- result: PASS ? 10 tests; 0 failed.
- command/run: `rustc --edition 2024 --crate-type lib apps/game-server/src/domain/mod.rs -D warnings -o target/domain.rlib`
- result: PASS ? standalone production compile with warnings denied; only `std` imports are present.

### Component/integration

- command/run: `cargo test --workspace`
- result: PASS on the current branch workspace. The domain module is not yet composed into `apps/game-server/src/lib.rs`, so its owned-path code is additionally proven by the focused standalone compile/test above.
- command/run: `cargo clippy --workspace --all-targets -- -D warnings`
- result: PASS. Same shared-composition limitation applies until lease transfer.

### E2E

- scenario: `NOT_EVALUATED` ? consuming VSL/runtime seams are not composed and DOMAIN does not own the QA path.
- result: blocked by shared composition lease and downstream seams.

### Exact-head CI

- final head: pending PR head after commit/push
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: `d09086b80077caa0c0c829496e4c36a3ab311fa2` (reviewed code head before PR-metadata-only follow-up)
- method/reviewer: implementing agent; full owned-path diff plus accepted GAME-CHAR/GAME-ITEM/FND-ID/SIM/DUR-03/GAME-CHANNEL contracts
- material findings: fixed revision-exhaustion partial mutation and two container graph bound gaps before commit; no remaining primary-path material finding identified
- verdict: PASS for allocated primary-path implementation; terminal delivery remains BLOCKED on shared composition lease

## Independent review

- required: NO ? this change is semantic-model-only and does not alter durable-value ownership, persistence, security/session authority or protocol authority
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: PR #56 contains exactly the two allocated files; primary-path diff reviewed
- unresolved review threads: pending PR
- related/superseded PRs: coordinator PR #50 is read-only to this lane and preserves FOUNDATION-first shared lease
- protected auto-merge: not eligible while shared composition lease is blocked
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Primary-path DOMAIN scope is implemented and locally green with 10/10 focused tests, standalone production `-D warnings` compile, full workspace tests and full workspace clippy; lifecycle and containment review findings were repaired.
status: blocked
branch: agent/otv2-impl-domain-core-01
head_sha: null
pr: null
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: Serialized shared composition lease is still held by FOUNDATION; DOMAIN may not mutate apps/game-server/src/lib.rs or other shared paths until the coordinator transfers it.
next_action: Obtain a coordinator transfer of the serialized shared composition lease to DOMAIN, then add the minimal game-server composition in this same task/branch/PR and rerun exact-head validation.
```
