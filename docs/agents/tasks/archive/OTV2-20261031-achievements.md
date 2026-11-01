# OTV2-20261031-achievements

```yaml
task_id: OTV2-20261031-achievements
type: mechanic
title: Achievement granters and Cyclopedia character facts
mode: IMPLEMENT
status: completed
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
updated_at: 2026-11-01T09:31:51Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/progression/src/
  - crates/gameplay/content/src/content/project.rs
  - crates/gameplay/content/src/content/project/bestiary.rs
  - crates/gameplay/foundation/src/foundation/condition.rs
  - crates/gameplay/foundation/src/foundation/condition_tests.rs
  - crates/gameplay/foundation/src/foundation/condition_attribute_slot_tests.rs
  - crates/gameplay/foundation/src/foundation/condition_gate_tests.rs
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
  - docs/agents/tasks/OTV2-20261031-achievements.md
  - docs/agents/tasks/archive/OTV2-20261031-achievements.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The authorized server/protocol slice implements the reachable P11 granters and
P12a/b/c facts through existing production owners. It preserves current
session-generation fencing, transactional receipts, replay behavior, account
ownership and character-info consent.

Ten named achievements are granted by committed fishing, skinning/dust,
arrival, rust and condition-potion outcomes. Connection notices observe the
bounded durable account fact set and suppress history/replay duplicates.

The title catalogue adds stable IDs 44–84 (41 additions): native Charm, Ahau,
receipt-backed cosmetics and actual complete Bestiary/Bosstiary predicates.
All 20 previously sourced badges retain their native owners and rules.
Cyclopedia tabs 8–10 expose 84 ordered effective-combat fields, distinguishing
unavailable values from known zero through a durable current observation.

The four admitted condition potions reuse the existing ConditionStore, spell
lane and item-use authority. Their actual buff, item burn, immutable application
receipt and achievement progress form one fenced outcome. Uncertain commits
reconcile the original prepared application and expiry.

The authoring closeout is based on the executed implementation checkpoint and
the separately recorded repair results below. The final commit identity and
its repeated qualification are recorded on the PR. Missing producers and
separately owned dependencies are not represented as implemented mechanics.

## Architecture and source of truth

- Selected Game base: `ae4188a9c8d461acb152044814ea23075de02ed4`.
- Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Source classification remains `OTS_HYPOTHESIS_ONLY`: observed donor code
  establishes reference behavior, not official Tibia behavior.
- The existing achievement, appearance, Charm, Bestiary, Bosstiary, badge,
  item-use, condition and combat owners remain authoritative.
- [P11 source inventory](../../evidence/OTV2-20261031-achievements/p11-source-inventory.md)
  names all 223 default-pack mutating call sites, 121 dynamic declaration rows and
  47 authored taming pairs, with a separate 17-call alternative-pack appendix.
- [P12 source inventory](../../evidence/OTV2-20261031-achievements/p12-titles-badges.md)
  gives every original title exclusion and every badge disposition.
- [Validation evidence](../../evidence/OTV2-20261031-achievements/validation.md)
  distinguishes historical controls from final candidate qualification.
- A commit cannot record its own SHA. The exact final frozen and published
  head is recorded by the final FREEZE_SHA entry on
  [PR #2782](https://github.com/Oteryn/Oteryn-Game/pull/2782).

## Accepted source and scope decisions

| Question | Accepted decision | Source answer |
|---|---|---|
| Q1 | A: no invented tournament facts; badges 15–21 remain excluded. The source inventory establishes that no separate seven-title tournament catalogue exists in the pinned donor. | [6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543) |
| Q2 | B: actual all-member Bestiary/Bosstiary completion, correcting the donor enum-as-race defect at `src/creatures/players/cyclopedia/player_title.cpp:254–267` / `src/creatures/players/player.cpp:9761–9766`. | [6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543) |
| Q3 | B: canonical outfit identity counts once; the male/female pair is one outfit. | [6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543) |
| Q4 | B: correct the unreachable potion achievement hook; 100 actually applied uses, never replay/refusal. | [6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543) |
| Q5 | A: observations from actual native combat owners with explicit unknowns and disclosed formula differences. | [6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543) |
| Q6 | B: implement all four condition potions here, including source admission, real buffs and atomic achievement receipts; narrow item-use dispatch arm; HP/MP randomization untouched. | [6547466349](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547466349) |
| Q7 | A: complete server/protocol scope for the bundle world; native client UI and broader activation belong to a separate migration. | [6547727362](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547727362) |
| Q8 | A: native fresh-session condition lifecycle remains; CLOSE-2/migrations 0690–0699 owns cross-session save/restore. | [6547749872](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547749872) |
| Q9 | A: existing Attributes/SpellSkills(0) shared mutation/dispel/immunity slot, with three behavioral RED→GREEN controls; typed reads, other subIDs and serialization unchanged. | [6547886810](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547886810) |
| Q10 | A: fourth foundation path allowed only to update the exhaustive inventory expectation: 29 typed inputs retained, 28 physical slots, replacement only for SpellSkills(0), unchanged 32/33 limit. | [6548355109](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6548355109) |

No source, value or scope question remains open after Q1–Q10.
PR comments reread at 2026-11-01T09:34:08Z: 16 comments; latest accepted answer remains Q10=A (6548355109), with no later source/value/scope decision.

## Acceptance criteria

- [x] Exhaustive donor granter and title-condition inventories reviewed against exact sources.
- [x] Reachable rules use production callers and their existing durable owners.
- [x] Unit/refusal/replay tests and mounted real-dispatch PostgreSQL cases completed.
- [x] Formatting, changed-package Clippy/tests, content, migration and privilege checks completed.
- [x] Final parity table, explicit exclusions and authoring/repair evidence are complete. Required exact-head validation and publication readback remain mandatory post-authoring gates recorded on PR #2782; this record does not assert their future outcome.

## Reference parity

| Behavior | Reference | Native implementation | Evidence |
|---|---|---|---|
| P11 successful gameplay granters | Crystal fishing, skinning, rust and NPC travel scripts; exact lines in P11 inventory | Existing craft/transform/arrival receipts and account achievement owner | Repair-14: all 13 real PostgreSQL gathering cases pass after 0704 and refreshed embedded migration artifacts. 28ac858 additionally passed account notices, rust 4 and paid travel 1; the initial mechanical RED and its repair are preserved below. |
| Four condition potions | `data/scripts/actions/items/potions.lua:1–31,52–54,63,115–129`; accepted Q4/Q6 correction | Existing condition/spell lane plus fenced immutable application receipt | 28ac858: 4 real-command PostgreSQL cases passed; historical 4-case RED→GREEN retained. |
| Titles and badges | `src/creatures/players/cyclopedia/player_title.cpp`, `src/creatures/players/cyclopedia/player_badge.cpp`, `src/game/game.cpp`; accepted Q1–Q3 | Native durable predicates and existing title/badge owners | 28ac858: title selection 5 pass, character-info 1 pass, badges 1 pass. |
| Character combat tabs | `src/server/network/protocol/protocolgame.cpp:5016–5325`; accepted Q5 | Actual installed combat/equipment observation and command 388 | 28ac858: combat snapshot/fence 1 pass and character tabs 1 pass. |
| Shared zero attribute slot | `src/creatures/creature.cpp:1409–1416`; `src/creatures/combat/condition.cpp:436–447,552–592` | Existing ConditionStore; approved Q9/Q10 scope | 28ac858: full foundation 347 unit +31 doc/other pass after the prior Q10 failure; Q9 3-case RED→GREEN retained separately. |

## Numbers

Migration 0704 is the narrowly scoped post-28ac858 repair admitting mechanical
rod 9306 through the existing SQL gathering gate. Migrations 0705 (title bounds),
0706 (combat observations) and 0707 (condition-application receipts) were
already present in the executed implementation checkpoint.

No new capability, command, mechanic or offer ordinal; reserved 252–253,
474–477 and 614–617 remain unused. Existing wire identities are retained.

The executed `numbers-resume-11` audit used separately fetched main
`1f7f76c6bc7c5014f9aa553d94a493b1d1544e19` and live open PRs. Its exit 1
reported unrelated migration 620–623 and offer-order 560 overlaps between
#2788/#2764; no collision in this task's reserved block was reported.
This is not a global audit PASS. The publication-time refresh belongs in
the final PR readback. No main merge/rebase is authorized.

## Validation

### Observed implementation checkpoint

The `resume-11` run tested `28ac858ceecaa83cb383f19b584822213682e8bb`
from `2026-11-01T08:41:50Z` to `2026-11-01T09:08:32Z`. The following
commands and results belong to that checkpoint. The later 0704 migration,
Clippy repairs and archive edit are tracked separately below. No result here
claims to qualify a future commit.

- `cargo fmt --all -- --check` — pass, exit 0.
- `cargo clippy --locked --offline --quiet --all-targets --all-features -p oteryn-protocol-oteryn -p oteryn-gameplay-content -p oteryn-gameplay-progression -p oteryn-gameplay-foundation -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server -- -D warnings` — FAIL, exit 101; 9 dispatch test lints. The actual repair result belongs in the next subsection.
- `cargo test --locked --offline --quiet --all-features -p oteryn-gameplay-foundation -- --test-threads=3` — pass, exit 0; 347 unit and 31 doc/other tests, no failures/ignored. This is the complete foundation GREEN after the previously recorded Q10 failure.
- `cargo test --locked --offline --quiet --all-features --no-fail-fast -p oteryn-protocol-oteryn -p oteryn-gameplay-content -p oteryn-gameplay-progression -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -- --test-threads=3` — pass, exit 0; 3280 unit and 10 doc/other tests, no failures, 15 existing ignored.
- Combined foundation/library execution — 3627 unit and 41 doc/other tests passed; ordinary package runs unset both PostgreSQL environment variables and do not count route-less returns as database execution.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib p11_ -- --nocapture --test-threads=1` — FAIL; 10 passed, 1 failed. `p11_mechanical_ice_fishing_grants_exquisite_taste` returned `unexpected end of file`.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib rust_lottery -- --nocapture --test-threads=1` — pass, 4 tests.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals -- --nocapture --test-threads=1` — pass, 1 test.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib condition_potions_postgres_ -- --nocapture --test-threads=1` — pass, 4 tests and four distinct Q6 completion markers.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib character_title -- --nocapture --test-threads=1` — pass, 5 tests.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib character_info_postgres_real_dispatch -- --nocapture --test-threads=1` — pass, 1 test.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib p12c_cyclopedia_combat_postgres_dispatch_snapshot_replay_and_fences -- --nocapture --test-threads=1` — pass, 1 test.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib cyclopedia_badges_postgres_real_dispatch -- --nocapture --test-threads=1` — pass, 1 test.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib cyclopedia_character_tabs_postgres_real_dispatch -- --nocapture --test-threads=1` — pass, 1 test.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib standard_fishing -- --skip p11_ --nocapture --test-threads=1` — pass, 8 tests.
- `cargo test --locked --offline -p oteryn-game-server --test durability_postgres fresh_migration_applies_only_the_embedded_game_ledger -- --exact --nocapture --test-threads=1` — pass, 1 test, before migration 0704.
- `cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres check_function_privileges_postgres_cases::runtime_role_executes_every_check_function -- --exact --nocapture --test-threads=1` — pass, 1 test, before migration 0704.
- `cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres item_use_postgres_cases::the_runtime_role_cannot_change_items_without_a_use_record -- --exact --nocapture --test-threads=1` — pass, 1 test, before migration 0704.
- `cargo test --locked --offline -p oteryn-game-server --test integration_rest content_ --quiet -- --test-threads=1` — pass, exit 0; 123 tests, 152 filtered.
- `cargo run --locked --offline --release -p oteryn-world-bundle-compiler -- pin-check . /home/mole/chatgpt-achievements-logs/resume-11/pins` — pass, exit 0; world digest `3bb1f0f22f3327f95e9e46ca513944c3aeba3d76068e68dc19d5e3b5c5c4997c`, 30,948,410 bytes.
- Complete content validator — NOT RUN: isolated dependency installation exited 1 with `No module named pip`; `content-validate.blocked` records this blocker. The passing content integration selection and pin-check do not replace it.
- `python3 tools/content-migration/test_condition_potion_profiles.py` — pass, exit 0; 6 tests.
- `python3 tools/content-migration/test_gathering_item_profiles.py` — pass, exit 0; 3 tests.
- `python3 tools/agents/validate_governance.py` — pass, exit 0; 22 required policy documents and 9 lanes on 28ac858.
- `python3 -m unittest discover -s tools/agents/tests` — pass, exit 0; 129 tests on 28ac858.
- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py --ref refs/remotes/number-audit/main` — exit 1 on main `1f7f76c6bc7c5014f9aa553d94a493b1d1544e19`; unrelated 620–623/offer560 overlap between #2788/#2764; no observed collision in this task's reserved block.
- `git diff --check ced19ac43f34891b2cddcdeed475e34a297c2082 HEAD` — pass, exit 0 on 28ac858.

The selected gameplay/ledger/privilege runs used the isolated PostgreSQL 18.6
instance with required real database execution enabled. Connection details are
not recorded. The checkpoint does not claim full-workspace, Windows/client UI,
E2E or heavy PostgreSQL partition validation.

Historical controls remain in the evidence record: Topmodel 1 behavioral
failure→1 pass; four condition-potion cases 4 failures→4 passes; Q9 shared-slot
regressions 3 failures→3 passes. Compiler errors and zero-selected-test filters
are not behavioral controls. Earlier source-capture and content-generation
evidence stays identified by its original checkpoint.

### Repairs after 28ac858 and archive-authoring checks

The results below were observed on the repaired working tree before the final
authoring commit. Final-head qualification repeats the required gates externally.

- Migration 0704 extends the existing gathering gate for mechanical rod 9306
  on the admitted ice-hole path; the earlier ledger/privilege passes precede
  it. Repair candidate, exact migration filename, mechanical-P11/group and
  migration/runtime-privilege rerun: `repair-14` on base `28ac858ceecaa83cb383f19b584822213682e8bb` plus the captured repair: `0704_mechanical_fishing_craft.sql`; the existing embedded-migration inventory passes before and after refresh of the task-only durability cache. The prior real database diagnostic proved the stale embedded guard lacked 9306. After recompilation, `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib p11_mechanical_ice_fishing_grants_exquisite_taste -- --nocapture --test-threads=1` passes 1/0 (22.83 s). The `standard_fishing` filter with `--skip p11_mechanical_ice_fishing_grants_exquisite_taste` passes the other 12/0 (163.23 s). The three exact schema/privilege commands listed above each pass 1/0 again after 0704 (`repair-14/migration-ledger`, `check-privileges`, `item-use-guard`, all exit 0). All temporary diagnostics were removed.
- The nine Clippy findings are addressed without loosening test assertions.
  Repair candidate and actual full seven-package command/result:
  `repair-12/clippy.log` and `.exit`: exit 0 on the working-tree repair based on `28ac858ceecaa83cb383f19b584822213682e8bb`, using `cargo clippy --locked --offline --quiet --all-targets --all-features -p oteryn-protocol-oteryn -p oteryn-gameplay-content -p oteryn-gameplay-progression -p oteryn-gameplay-foundation -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server -- -D warnings`. All nine findings were removed without changing the asserted behavior. This run precedes 0704; final-head qualification repeats the command.
- Complete content validation after restoring the missing isolated Python
  dependency path: `repair-14/python-import` verifies jsonschema 4.26.0, referencing 0.37.0 and rfc3339-validator 0.1.4 in the isolated task prefix (exit 0). `python3 tools/content-migration/regenerate_content.py --check` then completed with exit 0 (`repair-14/content-validate.log` / `.exit`), including its final five passing Rust cases. `repair-14/fmt` and `diff-check` also returned 0. The entire repair runner returned 0.
- `python tools/agents/validate_governance.py` after the repair/archive edits — PASS, exit 0 on the repaired source before archive: 22 required policy documents and 9 project lanes (`authoring-17/governance-before-result-record.log`). The completed record is checked again before commit.
- `python -m unittest discover -s tools/agents/tests` after the repair/archive edits — PASS, exit 0 on the repaired source before archive: 129 tests (`authoring-17/governance-tests-before-result-record.log`). The completed record is checked again before commit.
- `git diff --check` after the repair/archive edits — PASS for the pre-archive repaired source (`repair-14/diff-check.log`); the staged and unstaged archive delta is checked below before commit, followed by the committed source verifier on the final candidate.

### Final identity is recorded outside this file

This record describes the executed implementation checkpoint and the observed
repair results. Its own final authoring commit does not yet exist while the
record is written. The exact final SHA, FREEZE_SHA timestamp, publication
readback and repeated required validation on that SHA are recorded only on
PR #2782. No future PASS or successful push is asserted here.

## Left out

The P11 inventory names 194 missing producer call sites individually: 84 NPC
quest/dialogue, 59 specific action/quest, 34 quest event/movement, 10 Gnome
rank, 2 other creature/player event, 2 complete-world/area discovery,
2 forced-teleport/item-mint digging actions and 1 snow movement. Source rules
do not create the missing durable gameplay events. Arena completion,
Annihilator's four-choice/container reward and lever, final sculpture branches,
item-target/donkey/Glooth mount producers and ordinary-water mechanical fishing
retain explicit limits.

The P12 inventory leaves 28 of 112 titles excluded: 15 without a pinned
Crystal predicate, 3 hunting-task-point awards, 2 world rankings,
1 complete-map award, 1 Tibiadrome wave award and 6 named quest/event awards.
Existing Reigning Drome Champion ID 4 retains its separate missing rotation
source. Badge IDs 15–21 have no tournament owner; loyalty badges depend on the
Platform projection. Production cosmetic definitions remain
`READY_UNPOPULATED`; synthetic test data is not a production promotion.

Combat fields without a real consumer remain unavailable. Existing native
equipment aggregation, level curve, additive protection, mantra slot policy
and shield/spellbook rules retain their accepted semantics; the inventory
discloses Crystal differences, including +30%/+60% shield/spellbook bonuses.

Q7 excludes native client UI for caps 97/199 and broader activation as a
separate migration. Q8/Q9 exclude cross-session condition persistence:
CLOSE-2 and reserved migrations 0690–0699 own the shared implementation.

No protected configuration/workflow/tools-repository or audit-ledger change,
main integration, rebase, force push, review request, heavy-tests label,
deployment or unrelated mechanic rewrite is included.

## Closeout and review

- Authorized handoff: implemented, validated code on draft PR #2782.
- Final archive move: this record moves in the final authoring commit, before
  the exact frozen SHA is recorded externally on the PR.
- Review state: draft; no external review requested, as the task requires.
- Merge commit/result: not merged; merge is explicitly outside this task.
  If a separately authorized coordinator later merges it, resolve the
  `squash merge of #2782` from Git history.
- Ownership remains with achievements-task-owner until every required final-12 stage succeeds on the exact candidate, the normal push is verified by live branch-head readback, and the final PR comments are reread. Ownership release is recorded on PR #2782 after those gates; archiving this record does not itself release ownership.
