# OTV2-20260929-a13-character-build-state

```yaml
task_id: OTV2-20260929-a13-character-build-state
title: "A13 Character build state: vocation and magic level (D150-D151)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/laughing-goldberg-4gwjfq
issue: 162
pr: "successor of #1265; exact PR in the #162 FREEZE_SHA entry"
base_sha: 48de3868
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (successor Sol Supervising Architect; the first owner, session 01XdHJyZNPJMcmMnmSDgwQvZ, is archived)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260929-a13-character-build-state.md
  - docs/agents/tasks/archive/OTV2-20260929-a13-character-build-state.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records architect item A13 (SPELL-CASTER-FACTS, #162 5896414182), with owner decisions
D150 and D151. The ruling is 5896480875.

- **Storage:** Game-owned build state (vocation, magic level, mana spent), one row per Character.
- **Receipts:** a new build receipt kind in the `0009` chain, joining the DEATH-0, STANCE-0 and
  H-1 guard chain.
- **Acquisition:** the vocation is chosen on Dawnport, as in Global. Magic-level training from mana
  spent ships in V1, with a bounded checkpoint.
- **Spell contract:** §10 now points to A13 for magic-level training.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: #162 5896414182; migrations `0001`-`0017` (`0017` merged in #1270 as `878f6fad`; next
  free migration `0018`); DUR-02 rule 2; `CasterState`; the spell cast contract §10.
- `UNKNOWN`: the Dawnport choice details, the magic-level formula and multipliers, and the death
  loss amounts. These are for the implementation lanes.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. CHAR-BUILD-1 changes persistence and needs its own
persistence review.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Migrations, runtime code and content.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the successor of #1265 named in the #162 FREEZE_SHA entry; #1265 is superseded. Merge
  commit/result: its squash merge.
- Review: Codex could not review (quota exhausted, 5896518704). The control plane routed a separate
  non-authoring agent, which reviewed `d4e97ce` (5897183488) with disposition FIX. The repair
  commit carries these dispositions:
  - Findings 1-3 (material, A13-RECEIPT-CHAIN, #162 5897202372):
    1. The vocation choice is one combined build receipt carrying `stance_before` and
       `stance_after` (STANCE-0 §4.6).
    2. A death is three single-receipt revisions in one transaction (flush, death, loss), so
       DEATH-0 is unchanged.
    3. An idempotent D88-style initializer creates the row at creation and backfills it at the
       first admission.
  - Finding 4: the receipt shape now lists the stance, level and XP fields, the cause, the UUIDv7
    key, `command_binding`, `policy_digest`, replay-or-conflict and the shared state-guard branch.
  - Findings 5-7: the live magic level changes only after its receipt commits; unchanged
    checkpoints are skipped; `mana_spent` is progress toward the next magic level; and W2b follows
    CHAR-BUILD-1.
  - Successor architect repair of `ee21804d` (the frozen head was not rewritten; the successor
    branch continues its history):
    1. §4.1: no initializer receipt. At revision 1 the chain admits no receipt, and an
       initializer that advanced the revision would move it under the admission fence. No row
       means (`none`, 0, 0), as in STANCE-0 §4.1; no creation insert and no backfill.
    2. §4.2: stance fields are NULL unless a `vocation_choice` or `promotion` receipt prunes the
       stance; only those join the stance chain and update the stance row.
    3. §4.2: the writer, fence, `character_root` serialization, occurrence key and binding,
       reconcile, the typed death reference and the row guard are stated.
    4. §4.6: the death composite runs under one fence and is retried and reconciled by the death
       occurrence; an empty loss is skipped.
    5. An implementation brief and the before-freeze checklist (§9) are added.
    6. Read-only `oteryn-hard-worker` self-review of the complete draft: its material finding (a
       flush found by revision could be an earlier checkpoint) is fixed by the `death_flush` cause
       and typed reference; per-cause direction CHECKs, the advisory lock, the stance prune
       shape, the guard order, growth measurement and the retry binding are added.
  - Independent review of `ef18a7ca` (#1271 5898945224), disposition FIX:
    1. Material (F2): the death composite advanced up to three revisions in one transaction,
       against DUR-02 rule 2. Fixed in §4.6: the flush is its own earlier `training` transaction,
       and the loss is carried by the death receipt's new nullable build fields. That is a DEATH-0
       §3.1 amendment, applied in the DEATH-0 document. DEATH-0 §3.5 is unchanged, and the
       `death_flush` and `death_loss` causes are removed.
    2. The clauses of ruling 5896480875 that this document supersedes are listed in its header.
    3. W2b measures guard latency against chain length (§4.5 "Growth").
    4. One session orders its own Character commits (§4.2 "Writer").
    5. `0017` is merged (`878f6fad`); CHAR-BUILD-1 is `0018`.
    6. Read-only `oteryn-hard-worker` self-review of this repair: its material finding (no
       before/after chain across build-carrying receipts) is fixed by §4.2 "Build chain". Also
       added: the row-guard placement and the revision-1 list; the flush failure paths; the death
       binding version; the death build-field CHECKs; and "training is not enabled in production
       before the DEATH ML loss child".
  - Re-review of the successor head goes through the control plane.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

## Context checkpoint

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/laughing-goldberg-4gwjfq
pr: "successor of #1265"
owner_action_required: null
blocker: null
next_action: null
```
