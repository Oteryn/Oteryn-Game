> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #671 is merged on protected main as `ebc860d7cd12bb855228a48759c4cc37b828da63`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw3-b1-minimal-item-semantics-504

```yaml
task_id: OTV2-20260919-content-world-cw3-b1-minimal-item-semantics-504
title: CW3-B1 minimal typed item semantic delta
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-b1-minimal-item-semantics-504
issue: 162
pr: null
base_sha: 715a22f26f6ec5472597f63cf5d6b939d7583cc1
head_sha: pending
final_head_sha: pending
final_head_frozen_at: pending
owner: "Oteryn: content world build"
created_at: 2026-09-19T13:56:00+02:00
updated_at: 2026-09-19T13:56:00+02:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/tests/content_reference_playable.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-b1-minimal-item-semantics-504.md
public_contracts: []
depends_on:
  - "#668 protected bulk catalogue plan"
  - "#670 / CW2-B1 protected integration"
  - "#646 + #648 protected CW3 successor lineage"
blocks:
  - CW2-B2
cross_repository_coordination_id: null
external_repositories: []
```
## Outcome

Add only the smallest typed authored/static Reference item-definition semantics required for the first durable-item compatibility boundary: physical classification, materializable eligibility, stack classification, CharacterInventory destination legality, and existing exact typed definition identity/revision/provenance.

## Architecture and source of truth

- PROVEN: #162 comment 5741368206 is the live bounded allocation and exact custody authority.
- PROVEN: protected R7 evidence requires the minimum static item delta under the Reference playable successor profile; quantity=1 is a later fixture/use-site concern.
- PROVEN: GAME-ITEM-01 owns typed item capability semantics and rejects arbitrary authoritative attribute bags.
- PROVEN: CW2-B1 remains candidate-only and does not authorize any Crystal/display-derived native item identity.
- DERIVED: client projection may expose non-authority classification needed for presentation, but must omit materialization and destination-legality fields.

## High-risk authority/recovery qualification

NOT_APPLICABLE — this child extends an in-memory/static Reference Content semantic API only. It does not mutate production, persistence, session/lease authority, durable transactions, protocol IDs, registries, or recovery evidence.

## Acceptance criteria

- [x] Typed physical/materializable/stack-capable item semantics link deterministically.
- [x] Wrong family and wrong revision remain fail-closed.
- [x] Item definitions without the supported typed capability shape fail closed.
- [x] Client-safe projection omits materialization and destination-legality authority fields.
- [x] First-production profile behavior remains unchanged.
- [x] Tests use only a worker-local probe key unrelated to Crystal/display identity and never promote CW2-B1 UNRESOLVED identities.
- [x] Source enumeration order does not change canonical linked output.

## Excluded scope

No canonical Gold Coin, ItemType or ContentKey minting; no Crystal/display identity promotion; no quantity/current location/ItemInstance state; no transfer/split/merge/equipment/economy/DUR-03; no manifest-v4 dependency; no first-production model/compiler/bundle, protocol/schema/registry, workflow, governance or external-repository mutation.
## Implementation / findings

The Reference successor profile receives a typed item kind with explicit physical class, materializable flag, stack class and legal destination vocabulary. Current fail-closed validation permits CharacterInventory legality only when materializable and rejects Item-family Generic fallback. Client-safe projection intentionally carries only physical and stack classification.

## Validation

### Focused

- `cargo +1.94.0 fmt --all --check`: PASS in clean LF mirror of admission base plus current Rust custody files
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_reference_playable`: PASS, 20/20
- `cargo +1.94.0 test --locked -p oteryn-game-server --test content_first_production`: PASS, 4/4

### Component/integration

- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: PASS
- `python tools/agents/validate_governance.py`: PASS, 26 required policy documents / 9 project lanes
- `git diff --check`: PASS

### E2E

NOT_APPLICABLE — this child adds no runtime item instance/materialization or transaction flow.

### Exact-head CI

pending after durable push/PR.

## Self-review

- exact head: pending
- method/reviewer: Oteryn: content world build, whole-diff adversarial review
- material findings: pending
- verdict: pending

## Independent review

- required: YES — shared Reference Content semantic API extension per #162 allocation
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- protected integration: coordinator-only
- ownership release: pending

## Context checkpoint

```yaml
last_progress: local implementation and required validation passed; candidate ready for final diff freeze
status: validating
branch: agent/content-world-cw3-b1-minimal-item-semantics-504
head_sha: pending
pr: null
final_head_sha: pending
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
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: freeze, commit, push and qualify the exact candidate head
```
