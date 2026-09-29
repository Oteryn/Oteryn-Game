# OTV2-20260929-a11-stance-persistence-decision

```yaml
task_id: OTV2-20260929-a11-stance-persistence-decision
title: "A11 STANCE-0 Character stance persistence decision (D140)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1225
base_sha: d4d5e2c4072c7bdd3f3ef9ecb5c742cd2ae428c3
head_sha: 63d2449be11955c89a1a9cba4960d399bf0b3716
final_head_sha: 63d2449be11955c89a1a9cba4960d399bf0b3716
final_head_frozen_at: "2026-09-29 (#162 5889111016)"
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md
  - docs/agents/tasks/active/OTV2-20260929-a11-stance-persistence-decision.md
  - docs/agents/tasks/active/OTV2-20260929-a9-adr-a10-reconcile.md   # archive move after #1214
  - docs/agents/tasks/archive/OTV2-20260929-a9-adr-a10-reconcile.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records architect item A11 for owner decision D140 (#162 5888104688): the spell stance
slot persists as in Global. It adds a one-row-per-character stance table and a third receipt kind in
the `0009` CharacterRevision chain, with guard changes, a writer that mirrors the XP writer, and
death, login and vocation-change rules.

No migration, runtime or registry change is made; STANCE-0 and STANCE-1 follow.

## Architecture and source of truth

- `PROVEN`: Part C.4 and S27; migration `0009`; `durability/character_progression.rs`; the
  DEATH-0 and composition decisions; the spell cast wire contract.
- `UNKNOWN`: CommandRef UUIDv7; vocation storage; the fit rule; receipt growth; cast latency.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. STANCE-0 changes persistence and needs independent review.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

## Excluded scope

- Migrations, runtime code and content.

## Finding dispositions

Codex review of `4242ea5`: two P1, both ACCEPTED in repair generation 1 of 1. No owner decision
changed.

- 4132744952 (stance row not guarded): the row gets a deferred consistency trigger on insert,
  update and delete, and delete is rejected (§4.3).
- 4132744961 (two revisions for a vocation change): one combined vocation-change receipt carries
  the stance transition at one revision; the stance chain counts it (§4.3, §4.6).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1225 merged through the Merge Queue on 2026-09-29 as `0f3d75aa`.
- Review: two P1 findings, both fixed by `63d2449b`. The owner decided to merge after green CI.
- Protected-main readback: all three owned files are byte-identical to `63d2449b`.
- The owner decision was later renumbered D140 -> D145 (A12 §6), because D4 (#1218) already held
  D140-D144.
- Archived under `OTV2-20260929-a12-item-identity-tibia-id`.

## Context checkpoint

```yaml
last_progress: protected-integrated as 0f3d75aa; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: 63d2449be11955c89a1a9cba4960d399bf0b3716
pr: 1225
owner_action_required: null
blocker: null
next_action: null
```
