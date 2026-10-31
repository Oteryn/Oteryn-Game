# OTV2-20261031-achievements

```yaml
task_id: OTV2-20261031-achievements
type: mechanic
title: Achievement granters and Cyclopedia character facts
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/achievements
pr: null
base_sha: ae4188a9c8d461acb152044814ea23075de02ed4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: achievements-task-owner
created_at: 2026-10-31T19:00:00Z
updated_at: 2026-10-31T19:00:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/progression/src/
  - crates/gameplay/durability/src/durability/
  - crates/gameplay/dispatch/src/gameplay_transport/
  - crates/protocol-oteryn/src/
  - apps/game-server/migrations/0704_*.sql
  - apps/game-server/migrations/0705_*.sql
  - apps/game-server/migrations/0706_*.sql
  - apps/game-server/migrations/0707_*.sql
public_contracts:
  - docs/contracts/protocol-oteryn/v1/
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Extend the existing P11 and P12a/b/c owners with every source-backed reachable
achievement granter, durable title condition, badge source, and character
offence/defence/misc tab. Every durable transition retains its current session
generation fence, transaction and replay receipt. This is an implementation
draft, not a claim of completed parity.

## Architecture and source of truth

- PROVEN: starting main is `ae4188a9c8d461acb152044814ea23075de02ed4`.
- PROVEN: Crystal donor is `201189f871d23597b82caf88fd611454a2ddefb8`.
- Existing account achievement, character title and Cyclopedia owners remain
  authoritative; no second persistence or grant implementation is introduced.
- UNKNOWN: unreachable donor conditions and missing mechanic owners are being
  inventoried. Material choices go to one `QUESTION:` PR comment each; only
  `ANSWER:` decisions resolve them.

## Acceptance criteria

- [ ] Exhaustive donor granter and title-condition inventory with exact sources.
- [ ] Reachable rules wired through production callers and existing durable owners.
- [ ] Unit/refusal/replay tests and mounted real-dispatch PostgreSQL cases.
- [ ] Formatting, changed-crate Clippy/tests and SQL checks with actual results.
- [ ] Parity table and every excluded source/owner recorded on the draft PR.

## Reference parity

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| Achievement granters | Crystal `player_achievement.cpp`, `data/`, `data-global/` | Existing account achievement owner; audit in progress | Pending |
| Titles and badges | Crystal `player_title.cpp`, `player_badge.cpp` | Existing title and badge owners; audit in progress | Pending |
| Character tabs | Crystal `protocolgame.cpp:5016-5330` | Existing command 388, with production combat facts | Pending |

## Excluded scope

No merge, rebase, force push, review request, heavy-tests label, ledger edit,
protected configuration edit, deployment or unrelated mechanic rewrite.
Reserved new numbers: migrations 0704-0707, capabilities 252-253, commands
474-477, mechanic/offer ordinals 614-617. Reuse existing wire ids where possible.

## Validation

Initial documentation-only checkpoint: `git diff --check` before commit.
Implementation and runtime validation have not run yet.
