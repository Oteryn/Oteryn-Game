# OTV2-20260907-remediation-wp7-ability

```yaml
task_id: OTV2-20260907-remediation-wp7-ability
title: Repair Ability effect magnitude boundary validation
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
pr: 370
admission_main_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
base_sha: b3e637dc43a0a31ff2caf24a6450f7df56b43777
head_sha: 557e42dcddcc75a7e3d203c1d848100211a60984
final_head_sha: 557e42dcddcc75a7e3d203c1d848100211a60984
owner: WP7_ABILITY_REPAIR
created_at: 2026-09-07
updated_at: 2026-09-07
owned_paths: []
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Prevent directly constructed invalid Damage/Heal magnitudes from crossing public plan/commit boundaries or producing atomic/sequential partial mutation. Preserve valid deterministic ordering, helper behavior and replay semantics. Production reachability and G1 remain separate WP8 evidence.

## Architecture and source of truth

PROVEN on admission main: public `Effect::Damage` / `Effect::Heal` variants accept arbitrary `i64`; helper constructors reject `<= 0`, but `EffectPlan::new` validates target/capacity without magnitude validation and `AbilityEngine::commit`/`apply_fixture_effect` apply the direct value. `Damage { magnitude: -7 }` therefore reaches `checked_sub(-7)` and increases fixture health. Exact allocation is #162 comment `5567031784`; fresh open-PR search found no overlapping Ability runtime mutation lineage.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no session authority, PREPARE/COMMIT, durable schema/recovery, production/live state or protected control-plane mutation.

## Acceptance criteria

- [x] Direct zero/negative Damage and Heal variants are rejected at the public plan boundary before a plan is accepted.
- [x] Commit/apply keeps a defense-in-depth fail-closed check before any fixture mutation, including ordered sequential paths.
- [x] Atomic invalid plan/effect causes no partial health mutation; sequential invalid effect causes no newly applied invalid mutation and cannot bypass replay bookkeeping.
- [x] Valid positive Damage/Heal retain existing signed health semantics, ordering and replay behavior.
- [x] Focused integration tests, Rust 1.94 fmt, strict game-server Clippy/tests and local affected tests pass. Exact-head CI remains PR-gate evidence rather than a local claim.

## Excluded scope

No broad Effect API redesign, gameplay activation/registration, AI, Foundation/Durability, Cargo/lock, lib/composition, workflow/protection, production/live data or external-repository changes.

## Validation

Local candidate evidence (all PASS on 2026-09-07):

- `cargo +1.94.0 test -p oteryn-game-server --test ability_engine` — 10 passed, 0 failed.
- `cargo +1.94.0 fmt --all --check` — passed after applying formatter output.
- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` — passed.
- `cargo +1.94.0 test --locked -p oteryn-game-server --all-targets` — passed across all package targets.
- `git diff --check` — passed after the final five-file diff readback.

The focused regression constructs zero and negative `Effect::Damage` and `Effect::Heal` variants directly across `EffectPlan::new` and `EffectPlan::immediate`. A private commit-module harness exercises the pre-mutation apply guard without widening visibility. Existing positive-effect ordering, atomic overflow behavior, sequential retry progress, and occurrence replay tests remain green. This is bounded WP7B kernel repair evidence only; it does not prove production reachability or complete WP7/G1.

- Exact delivery head `557e42dcddcc75a7e3d203c1d848100211a60984`: Merge gate `34102762646`, Architecture semantic audit `34102762682` and Agent governance `34102762609` passed.
- Full Merge Queue run `34103269624` passed; PR #370 squash-merged as `728f25461d5a2b029ed60f7db4b14151d31776d7` and the exact five-file result was read back from protected `main@728f25461d5a2b029ed60f7db4b14151d31776d7`. Durable evidence: PR #370 comment `5568189238`.

## Self-review and closeout

- Exact delivery head: `557e42dcddcc75a7e3d203c1d848100211a60984`.
- Full changed-file and effective-diff review: PASS; zero open material findings, no unresolved review threads and no scope outside the five allocated files.
- Independent review: NOT_REQUIRED under the META-owned policy for this bounded local Ability-kernel repair; exact-head repository gates and Merge Queue passed.
- Ownership: released after protected-main readback; the implementation branch has no continuing provenance role.
- Branch disposition: merged task branch deleted; live matching-ref readback returned no branch.

## Context checkpoint

```yaml
last_progress: PR 370 integrated through successful Merge Queue and read back from protected main; task archived and ownership released
status: completed
branch: null
head_sha: 557e42dcddcc75a7e3d203c1d848100211a60984
final_head_sha: 557e42dcddcc75a7e3d203c1d848100211a60984
pr: 370
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
owner_action_required: null
blocker: null
next_action: null
```
