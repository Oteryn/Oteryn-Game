> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #667 merged as `d251770f6757d1f5e87df9c39c71f91bae8343f9`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260918-fnd02-retained-command-outcome-663

```yaml
task_id: OTV2-20260918-fnd02-retained-command-outcome-663
title: Implement Foundation-owned retained command outcome lifecycle
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/fnd02-retained-command-outcome-663
issue: 663
pr: null
base_sha: c0ab26554669e696ead11526f9977ccfa05a83ec
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn impl foundation
created_at: 2026-09-18
updated_at: 2026-09-18
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/mod.rs
  - docs/agents/tasks/active/OTV2-20260918-fnd02-retained-command-outcome-663.md
public_contracts:
  - FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
depends_on:
  - "#663 comment 5735153528"
  - "#162 runtime allocation / 2026-09-18T20:18:15Z"
  - "PR #666 protected merge/readback c0ab26554669e696ead11526f9977ccfa05a83ec"
blocks:
  - CONTENT_WORLD_CW4_LOCAL_OBJECT_RUNTIME_COMPONENT_504
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Replace the caller-forgeable retained-result boolean seam with one Foundation/GameSession-owned
command lifecycle covering pending, retained-terminal and expired truth.

The first-playable runtime store enforces the protected resource authority:
- exactly one retained terminal semantic record per GameSession;
- at most 3116 charged semantic bytes per GameSession;
- retained binding identity is active Content-generation identity plus stable TransitionKey;
- no full TransitionBinding or policy_guard_refs are retained.

## Architecture and source of truth

FACT

- Protected admission is `main@c0ab26554669e696ead11526f9977ccfa05a83ec`.
- #663 comment `5735153528` is the controlling accepted resource/semantic authority.
- Protected main already contains the count=1 and charged-bytes=3116 registry rows from PR #666.
- No production call sites outside `foundation/mod.rs` use the old CommandIngress duplicate API.

DERIVED

- Charge is exactly four u64 fields plus six two-byte logical length prefixes plus six bounded semantic components.
- The maximum is `4*8 + 6*2 + 6*512 = 3116` charged bytes.
- Move-only terminalization/recovery avoids a second Foundation-owned retained semantic-record copy.

## Runtime behavior

- Reservation stores the bounded normalized semantic identity with the pending CommandId.
- Same pending CommandId plus changed semantic identity classifies as conflict.
- Terminalization preflights order and complete retained charge before modeled gameplay mutation.
- Successful terminalization moves pending semantic identity into the sole retained terminal record.
- A new terminal record deterministically evicts the prior eligible terminal record; pending entries are never eviction candidates.
- Retained duplicate plus identical semantic identity replays the original terminal semantic outcome.
- Retained duplicate plus changed normalized intent or binding conflicts.
- An evicted lower CommandId resolves expired/reconciliation and remains below the non-reusable ingress high-water mark.
- Recovery handoff is move-only; missing required latest terminal truth makes the old GameSession non-resumable.

## Excluded scope

No writes or authority for protocol/schema numeric IDs, Content format, full TransitionBinding,
CW3/CW4 runtime, Durability/persistence topology, Cargo/workspace/lock, workflows/governance,
resource registry, migrations, external repositories or production deployment.

## Acceptance criteria

- [x] Caller-selected `terminal_outcome_retained: bool` removed from duplicate classification.
- [x] Foundation owns pending/retained/expired classification.
- [x] Retained record owns normalized intent, generation+TransitionKey, original outcome and command metadata.
- [x] One retained terminal record maximum is enforced structurally.
- [x] 3116 exact maximum accepted; 3117 semantic candidate rejected.
- [x] Checked charge overflow rejected.
- [x] Preflight failure occurs before modeled gameplay mutation.
- [x] Pending original is never evicted or re-executed.
- [x] Later terminalization cannot pass earlier pending CommandId.
- [x] Eviction preserves high-water and produces expired/reconciliation behavior.
- [x] Recovery preserves required truth or fails closed as non-resumable.

## Validation

### RED

A focused future-API test was added first and compilation failed because the Foundation-owned
semantic identity, retained binding and terminal outcome types did not yet exist.

### GREEN

- `cargo test -p oteryn-game-server foundation::tests --lib`: PASS — 15/15.
- `cargo clippy -p oteryn-game-server --all-targets --all-features -- -D warnings`: PASS.
- `rustfmt --check --config skip_children=true apps/game-server/src/foundation/mod.rs`: PASS.
- `python tools/agents/validate_governance.py`: PASS — 26 policy documents / 9 project lanes.
- `git diff --check`: PASS.

### Full game-server baseline comparison

Candidate run `cargo test -p oteryn-game-server --all-features`:
- 424 passed;
- 3 failed in `durability::schema::contract_tests`.

A clean detached `main@c0ab26554669e696ead11526f9977ccfa05a83ec` rerun of the exact
`durability::schema::contract_tests` family produced the same 3 failures and 4 passes.
Therefore those three failures are pre-existing baseline behavior, not introduced by this child.

The failing baseline tests are:
- `record_derived_current_authority_is_test_only_but_current_facts_remain_public`;
- `record_derived_candidate_binding_is_internal_and_its_convenience_is_test_only`;
- `identity_derived_authority_claim_convenience_is_test_only_across_sibling_family`.

### Formatter environment note

Repo-wide `cargo fmt --all -- --check` on this Windows checkout reports pre-existing newline-style
mismatches across many unchanged files. No out-of-custody formatter mutation is retained.
The custody file itself passes targeted rustfmt check with child-module formatting disabled.

## Self-review

Two material ownership/accounting issues were found before commit and repaired:
1. terminalization initially cloned the full retained record into both the store and return plan;
2. recovery snapshot initially cloned the full retained record.

Final design moves semantic evidence from pending -> retained -> recovery state without cloning the
Foundation-owned retained record. The return plan contains only charged-byte and eviction metadata.

Remaining material findings after repair: NONE after final custody-only diff/readback.

## Independent review

- required: YES — stable high-risk Foundation/GameSession authority state;
- exact head: pending final checkpoint commit;
- reviewer: pending independent exact-head reviewer;
- material findings: pending;
- verdict: pending.

## PR and closeout

- direct merge: FORBIDDEN;
- protected integration: coordinator-owned governed exact-head Merge Queue lifecycle;
- CW4 resume authority: NONE until this child is protected-integrated and read back;
- next after protected runtime readback: #162 may resume the existing CW4 branch only.

## Context checkpoint

```yaml
last_progress: implementation locally committed as f83c2eb95bb4cb960486486c93467ce583e15378 and all local qualification reverified
status: validating
branch: agent/fnd02-retained-command-outcome-663
head_sha: null
pr: null
blocker: independent exact-head review and hosted exact-head qualification pending
next_action: commit this checkpoint, normal push, open PR, then require independent exact-head review
```
