# OTV2-20260904-rested-death-architecture-continuation

```yaml
task_id: OTV2-20260904-rested-death-architecture-continuation
title: Persist owner-selected Rested XP and death-recovery direction
mode: CONTRACT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/rested-death-owner-checkpoint-20260904
issue: 220
pr: pending
admission_main_sha: 68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705
head_sha: pending
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn architecture continuation
worker_alias: "Oteryn: architektura"
created_at: 2026-09-04T19:46:00+02:00
updated_at: 2026-09-04T19:46:00+02:00
execution_budget_minutes: 240
large_budget_reason: owner-driven continuation spans Rested progression semantics, raw-XP accounting, death progression, corpse recovery, anti-abuse and explicit deferred balance boundaries
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_RESTED_DEATH_OWNER_DECISION_CHECKPOINT_2026-09-04.md
  - docs/agents/tasks/active/OTV2-20260904-rested-death-architecture-continuation.md
  - docs/agents/tasks/active/OTV2-20260827-multichannel-world-architecture-continuation.md
public_contracts: []
depends_on:
  - OTV2-20260827-multichannel-world-architecture-continuation
blocks: []
cross_repository_coordination_id: OTV2-MULTICHANNEL-WORLD-OWNER-DECISIONS
external_repositories: []
```

## Outcome

Persist the owner-selected Rested progression and death-recovery semantic direction reached after the 2026-08-27 multichannel checkpoint, while keeping exact balance values and deferred housing/food details explicitly open and without silently rewriting accepted canonical contracts.

## Architecture and source of truth

- `PROVEN`: admission protected `main` is `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`.
- `PROVEN`: Issue #220 remains open and continues to own the owner-driven architecture sequence.
- `PROVEN`: PR #221 is merged at `4b6656f688868aa2fb59c18392c2f859f1c5a1c7` and is historical checkpoint provenance, not a live draft.
- `PROVEN`: accepted GAME-CHAR architecture leaves exact death arithmetic/profile-specific death behavior to later ruleset/world-profile gates.
- `PROVEN`: GAME-ITEM-01 and DUR-03 require one authoritative item location and typed transaction/custody semantics for loot/corpse movement.
- `OWNER_SELECTED_DIRECTION`: Rested uses bonus-XP-denominated per-Character state, raw-XP accounting and positive recovery semantics for Oteryn; Tryb zgodności z Tibią remains Reference-governed.
- `OWNER_SELECTED_DIRECTION`: ordinary Oteryn death does not delevel an achieved level; current-level progress is lost first and overflow becomes `DeathDebt`.
- `OWNER_SELECTED_DIRECTION`: recent unsecured expedition value may participate in a corpse/death-recovery loop while long-term secured/equipped value is protected from random ordinary-death loss.
- `DEFERRED`: final housing topology/ownership, food/cooking/fishing details and exact numeric balance remain open.

## Acceptance criteria

- [x] New checkpoint distinguishes owner-selected semantic direction from illustrative/unfrozen numeric candidates.
- [x] Rested decisions preserve `EligibleRawXP` ordering and downstream-modifier independence.
- [x] Rested is per Character; multi-character and multiaccount behavior is explicit.
- [x] Tryb zgodności z Tibią remains Reference-governed rather than silently inheriting Oteryn Rested/death differences.
- [x] Oteryn achieved-level milestone and `DeathDebt` direction are recorded without freezing final penalty arithmetic.
- [x] Corpse/death-recovery direction preserves one-authoritative-item-location and anti-duplication constraints.
- [x] Food/cooking/fishing and housing remain explicitly deferred rather than accidentally accepted.
- [ ] Current balance candidates receive later explicit owner disposition before any canonical formula/constant is frozen.
- [ ] Exact current-contract clauses requiring eventual supersession/amendment are identified before canonical gameplay contract merge.
- [ ] Final coherent amendment package receives repository-required validation/review before canonical merge.

## Excluded scope

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas implementation. No final housing model. No exact final Rested percentage, pool curve, recharge multiplier, death percentage, Debt cap, exhaustion timing, corpse expiry/salvage policy or PvP corpse policy.

## Implementation / findings

This task is documentation/architecture continuation only.

Selected semantic direction is recorded in:

`docs/architecture/reviews/OTERYN_GAME_RESTED_DEATH_OWNER_DECISION_CHECKPOINT_2026-09-04.md`.

Important distinction:

```text
selected semantic invariants
!= final balance constants
!= runtime implementation authority
!= canonical supersession of accepted historical contracts
```

## Validation

### Focused

- documentation consistency review against Issue #220, root/docs agent instructions, GAME-CHAR, GAME-ITEM and DUR-03 authority boundaries;
- exact changed-file/diff review on the PR head after publication.

### Component/integration

- `NOT_APPLICABLE` — documentation/architecture checkpoint only.

### E2E

- `NOT_APPLICABLE` — no executable behavior changes.

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted repository policy
- classification: architecture/documentation checkpoint
- result: pending

## Self-review

- exact head: pending
- method/reviewer: architecture continuation agent
- material findings: pending
- verdict: pending

## Independent review

- required for checkpoint-only persistence: not automatically required by META AI-review policy; repository checks remain authoritative
- required before future canonical supersession of material accepted architecture: YES where current policy requires it
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- previous checkpoint PR: #221 merged; not reopened
- current PR: pending
- merge authority: normal protected repository path; no direct `main` mutation
- ownership release: pending

## Context checkpoint

```yaml
last_progress: owner-selected Rested XP semantics and current death/corpse-recovery direction persisted on a fresh continuation branch; housing and food/cooking/fishing held for later
status: investigating
branch: arch/rested-death-owner-checkpoint-20260904
head_sha: pending
pr: pending
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: none for checkpoint persistence; exact balance constants remain intentionally open
next_action: publish/open the checkpoint PR, verify exact changed files/diff and then continue the next genuinely open owner decision without freezing deferred balance values
```
