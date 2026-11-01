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
pr: 2782
base_sha: ae4188a9c8d461acb152044814ea23075de02ed4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: achievements-task-owner
created_at: 2026-10-31T18:11:39Z
updated_at: 2026-10-31T19:09:15Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/progression/src/
  - crates/gameplay/content/src/content/project.rs
  - crates/gameplay/content/src/content/project/bestiary.rs
  - crates/gameplay/foundation/src/foundation/condition.rs
  - crates/gameplay/foundation/src/foundation/condition_tests.rs
  - crates/gameplay/foundation/src/foundation/condition_gate_tests.rs
  - crates/gameplay/foundation/src/foundation/condition_attribute_slot_tests.rs
  - crates/gameplay/durability/src/durability/
  - crates/gameplay/dispatch/src/gameplay_transport/
  - crates/protocol-oteryn/src/
  - apps/game-server/migrations/0704_*.sql
  - apps/game-server/migrations/0705_*.sql
  - apps/game-server/migrations/0706_*.sql
  - apps/game-server/migrations/0707_*.sql
  - tools/content-migration/condition_potion_profiles.py
  - tools/content-migration/test_condition_potion_profiles.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/gathering_item_profiles.py
  - tools/content-migration/test_gathering_item_profiles.py
  - tools/content-schema/native-gameplay/build_gathering_item_profiles.py
  - tools/content-schema/native-gameplay/gathering-item-profiles.json
  - tools/content-schema/native-gameplay/condition-potion-profiles.json
  - content/
  - docs/agents/evidence/OTV2-20261031-achievements/
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
- The exhaustive call-site inventory is in
  `docs/agents/evidence/OTV2-20261031-achievements/p11-source-inventory.md`;
  title/badge disposition is in the adjacent `p12-titles-badges.md`.
- PR answers 6547429543 and 6547466349 resolve Q1-Q6: exclude unsupported
  tournament facts; count real Bestiary/Bosstiary completion and canonical
  outfit identities; repair the four unreachable potion granters and supply
  their condition owner; publish actual combat facts with explicit unknowns.
  The latter answer explicitly authorizes the added condition-potion content
  and production consumer. No HP/MP randomization changes are authorized.
- Q7=A (6547727362) retains bundle-world server/protocol scope; native client UI
  and broader activation remain a separate client migration. Q8=A (6547749872)
  retains the current fresh-session condition lifetime. CLOSE-2, reserved
  migrations 0690-0699, owns shared timed-condition save/restore at logout.
- Q9=A (6547886810) authorizes only the existing ConditionStore slot equivalence
  of Attributes and SpellSkills(0), including dispel/immunity identity and three
  regression tests. Typed reads, nonzero subIDs and serialization are unchanged.

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

Validation is in progress on an isolated Molehill checkout and PostgreSQL 18.6
cluster. The cloud checkout has insufficient build storage and lacks the content
validator dependency; it is used for source work and small checks. Commands and
actual outcomes are recorded in the PR. Current evidence:

- `git diff --check`: passed on the evolving tree.
- `python3 tools/agents/validate_governance.py`: passed (22 policy documents,
  9 lanes); governance unit discovery: 129 passed.
- `python3 tools/content-migration/test_condition_potion_profiles.py`: 3 passed.
- Condition profile capture `--check` against the exact pinned inputs: passed.
- `python3 tools/content-migration/register_spell_families.py`: passed on
  Molehill, 38 generated files checked/written. Semantic comparison against
  HEAD confirms exactly four added profiles and zero changed/removed profiles.
- Early remote compilation found integration errors in new fixtures/borrows;
  those findings were repaired. No final build or PostgreSQL pass is claimed
  at this checkpoint.
- Number audit against separately fetched main e95fd08a2c and open PRs found
  unrelated reservations 624/625 shared by #2771/#2779 (audit exit 1). This
  task's entire reserved block was free; no main merge or rebase occurred.

## Recovery checkpoint (2026-11-01)

The previous cloud workspace expired before publication. The task-owned
Molehill checkout and verified delta12 preserve the complete implementation.
The prior unpublished local commit identity is not being claimed or recreated.
This ordinary Git successor starts from the unchanged published predecessor
ced19ac43f34891b2cddcdeed475e34a297c2082; final validation will name its actual SHA.
The recovered pre-application tree, binary diff and index are retained outside
the repository. One writer owns this task branch; no merge, rebase or force is used.

Q10=A (6548355109) permits the existing closed inventory fixture correction
in condition_gate_tests.rs: all 29 typed inputs and the 32/33 cap remain;
SpellSkills(0) alone replaces Attributes and leaves 28 physical slots.
The full foundation GREEN and remaining final gates are pending the resumed
run. Earlier results in this record are historical checkpoints, not results
for this successor. The Remote Desktop execution interruption has recovered.
