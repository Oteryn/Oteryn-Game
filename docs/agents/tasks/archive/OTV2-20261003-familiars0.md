# OTV2-20261003-familiars0: FAMILIARS-0 Familiars decision

```yaml
task_id: OTV2-20261003-familiars0
title: FAMILIARS-0 Familiars decision
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-decision-20261003
pr: 1644
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_sha: exact frozen head in the #1622 FREEZE_SHA entry
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0.md
public_contracts: []
depends_on: [D296, D299]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Candidate decision `FAMILIARS0-FAMILIARS-V1`: answers §C.1 Q1-Q6 of the spell native behaviours
candidate. Five familiar spells and creatures; cast refused with any summon; a durable
per-character row of familiar time left and cooldown left, both frozen offline, written at cast
and clean session end with session-generation fencing; the familiar returns at login; a crash
loses it and keeps the cooldown. Children FAMILIAR-CONTENT-1 and FAMILIAR-1.

## Architecture and source of truth

- PROVEN: spell native behaviours candidate §C.1; CREATURE-AI-0 §8; TRAVEL-0 R3; PARTY-PVP-0
  §5.1; BOSS-RAID-0 §6.
- CIPSOFT_OFFICIAL: none used.
- TIBIAWIKI_STRUCTURED: F1108992, F1177634, F1182913-F1182917.
- OTS_HYPOTHESIS_ONLY: Canary cast refusal, warnings, return distance, hit points
  (`PARITY_PENDING`).

## High-risk authority/recovery qualification

The decision defines a durable Character write path, so FAMILIAR-1 must prove these cases (§5):
- **Stale generation.** A write from an older `session_generation` (a late clean-end save or a
  removal after a takeover) is refused and changes no column.
- **Restart after a crash.** A row left `open = true` by an older generation loads as lost:
  `familiar_remaining_ms = 0`, `open = false`, cooldown unchanged.
- **Restart after a clean end.** A row with `open = false` and time left returns the familiar
  with that time; the cooldown keeps its saved value.
- **Replay.** A retried cast command returns its first outcome and writes the row once; a retried
  session-end save with the same revision is a no-op.
- **Fenced write.** Every write is a compare-and-set on `revision` inside the writer's fenced
  Character transaction; the cast write commits only with the mana and the admission.

The full negative-case sweep (identity and binding mismatch, provenance substitution, direct
versus reconciled paths, concurrency and PostgreSQL reload, one invariant per case) is the
decision's §5.1, added by the D309 P2 bundle (review finding 4173381895).

## Acceptance criteria

- [ ] Exact-head validation and independent review of the frozen head.
- Review round (Codex on 6a78bcd2): P1 4173319549 (mana and the support group cooldown in the cast
  order), P1 4173319551 (clean-end discriminator `open`), P2 4173319554 (this qualification);
  all fixed in one push.
- [ ] Protected integration through Merge Queue.
