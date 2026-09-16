# OTV2-20260916-wp3-playable-first-q-triage

```yaml
task_id: OTV2-20260916-wp3-playable-first-q-triage
title: Tighten WP3 playable-first Q and numeric-floor triage
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/wp3-playable-first-q-triage-20260916
pr: null
base_sha: 1995bd97460774ea9fc136959d5548471b81c987
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT GPT-5.6 Sol
created_at: 2026-09-16T15:33:00+02:00
updated_at: 2026-09-16T15:33:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md
  - docs/agents/tasks/active/OTV2-20260916-wp3-playable-first-q-triage.md
public_contracts: []
depends_on:
  - PR #634 protected playable-first policy
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Prevent the WP3-v2 programme coordinator from reintroducing broad #356 obligations into WP3-A by blanket inheritance of historical Q01-Q75 cells or numeric resource floors. Require current protected authority and next-playable-milestone relevance before any historical obligation becomes a WP3-A blocker, while preserving every actually accepted correctness/security/durability invariant.

## Architecture and source of truth

- `PROVEN`: protected `main@1995bd97460774ea9fc136959d5548471b81c987` contains `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` from PR #634.
- `PROVEN`: PR #635 currently uses `ALL_OTHER_Q = CURRENT_EVIDENCE_REQUIRED` and carries the historical 12 MiB equation as an assumed current floor.
- `DERIVED`: that blanket inheritance can recreate broad-fork-era proof scope even when individual cells are mechanism-specific, representative-load work, downstream-composed, superseded, or no longer current protected authority.
- `REQUIRED`: accepted protected safety/correctness limits remain binding until protected authority changes them; this task does not waive any accepted invariant.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: prompt/governance only. No runtime, persistence, authority-bearing session, production, credential or recovery-state mutation.

## Acceptance criteria

- [x] Keep the existing alias `Oteryn: astra wp3-v2 programme coordinator`; create no duplicate coordinator.
- [x] Forbid blanket historical-Q inheritance into WP3-A.
- [x] Require each Q/property to be classified by current protected authority and release relevance.
- [x] Separate current WP3-A invariants, WP3-B representative qualification, downstream triggers, historical mechanism evidence, proven unreachable/superseded paths and unresolved authority/evidence.
- [x] Require the same current-authority proof for numeric resource floors such as the historical 12 MiB equation.
- [x] Explicitly preserve a numeric floor when current protected authority still binds it.
- [x] Prevent mechanism-named #356 tests from forcing retention of that mechanism when a smaller proof satisfies the property.
- [x] Require an explicit Q/numeric-floor disposition summary before protecting a WP3-A acceptance/allocation amendment.

## Excluded scope

- No runtime, Cargo, vendor, workflow, ruleset or production mutation.
- No change to PR #356 history or source.
- No weakening of accepted correctness/security/durability/compatibility requirements.
- No direct amendment of PR #635 from this branch.
- No new prompt alias/control plane.

## Validation

- focused prompt readback and diff review: pending
- lifecycle alias/status/owner/supersession identity: unchanged
- component/E2E: `NOT_APPLICABLE`, prompt/governance only
- exact-head repository checks: pending after PR creation

## Self-review

- exact head: pending
- method: whole prompt delta against protected #634 policy and current #635 prospective allocation
- material findings: pending
- verdict: pending

## Context checkpoint

```yaml
last_progress: canonical WP3 coordinator prompt now requires authority-backed Q/numeric-floor triage
status: validating
branch: governance/wp3-playable-first-q-triage-20260916
head_sha: null
pr: null
```
