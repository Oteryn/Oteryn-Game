# Collector handoff: freeze this branch after the one merge push

Collector main78294c081 is integrated with both main and questpc8 logic retained.
Own migrations remain1300..1307; all352migrations passed PostgreSQL18.6 in3s.
Own native PG8/8PASS365.597s, including atomic Father, source XP, read-only
Secret Library and Wrath opening. Full Quest units323/323PASS36.05s; remaining
focused merge regressions13/13PASS. Clippy13changed crates PASS. Exact46-output
registration replay PASS294.478s; reviewed source approval snapshots unchanged.
Final workspace check PASS347s; fmt PASS. See collector-main-validation.json.

After the single collector push this branch is FROZEN. No new features or
follow-up work here; continue only on claude/happy-davinci-ijz3wy-questpc8b.
Whole Wrath and Colours qualification is unchanged; do not promote those entries
from the successful opening/shared-mechanic slice. Full remaining suites go to CI
under the latest owner speed rule. Historical checkpoints follow below.

# questpc8 final report

**Quest disposition: 1 QUALIFIED, 32 BLOCKED. All 33 assigned titles have a final status.**

QUALIFIED: Lone Medusa Quest — `questpc8_lone_medusa_claim_use_replay_and_refusal`; positive PostgreSQL marker46.032445154s (native placement5.087234109s). The existing strict cohort proves its complete standalone source contract.

Native source slice qualified: Killing in the Name of... selected task/boss progression — `questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards`; PostgreSQL1/1PASS, marker334.924269088s. It proves2000ordinary native deaths, three source bosses/portals/reports, the native rank, live journal, exact39900taskXP/18points, three achievements and replay/refusal of duplicate rewards. Deaths use the production native owner/queue; NPC and movement use real framed dispatch. This does not claim framed AttackTarget combat coverage.

**The full chosen hunting quest remains BLOCKED.** Its pinned overlay selects10000XP and5000GoldCoin. The source-native path does not implement that authored final payout. The shared source NPC executor refuses general item/money/XP effects and the source Grizzly contract cannot be privately overridden by a lane hook. This discrepancy was caught before any whole-quest qualification.

Complete per-title missing pieces, exact file:line evidence, attempted routes and recheck conditions are in [QUEST-STATUS.json](/home/mole/oteryn-cache/rs-wt/questpc8/.questpc8/QUEST-STATUS.json).

## Changed Rust files

- [crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_boss.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_boss.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_boss/world.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_boss/world.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_npc.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_npc.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_npc/questpc8_campaign.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/grizzly_npc/questpc8_campaign.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs)
- [crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_reward_claims.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_reward_claims.rs)
- [crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs)
- [crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs](/home/mole/oteryn-cache/rs-wt/questpc8/crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs)

Lane records updated: `QUEST-STATUS.json`, `QUEST-STATUS.md`, `RESUME.md`, `SHARED-FIXES.md`, `lane-review.md`, `lane-validation.json`, `disposition.py`, and this report; focused logs are retained under `.questpc8/`.

## Exact test filters and evidence

All PostgreSQL commands used the absolute queued cargo wrapper and `CARGO_BUILD_JOBS=3`; no full PostgreSQL suite ran.

```sh
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc8_lone_medusa_claim_use_replay_and_refusal -- --nocapture
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib questpc8_ -- --nocapture
```

The third filter includes the unit regression `questpc8_authored_selector_cannot_impersonate_a_crystal_reward_source`, which passed. Earlier combined runs had the then-unfinished hunting fixture failure; their selector and Medusa cases passed individually. The final hunting-only run above resolved that failure. Do not interpret an earlier mixed-run exit as whole-suite green.

Logs: `lane-hunting-qualified-pg.log` (new native source slicePASS), `lane-medusa-pg.log` (retained standaloneMedusaPASS), `lane-hunting-promotion-red.log` (selectorunitPASS and positiveMedusa marker, historical hunting fixtureRED).

## Validation

- Dispatch and durability all-target/all-feature Clippy with `-D warnings`: PASS.
- `cargo fmt --all --check`: PASS; explicit mounted-test rustfmt: PASS.
- Complete diff against assignedbase28e4313e1 reviewed against both owner/repository checklists; diff whitespace check: PASS.
- No added production enum variant, struct field, trait method or public API; test-helper caller sites inventoried across `apps/`, `crates/`, `tools/`.
- After the successful PG run, only the test module comment changed to clarify source-slice scope. Hash comparison proves every executable line unchanged; fresh formatting passed.

## Unfinished

| Shared boundary / exclusion | Quests |
| --- | --- |
| W14 owner exclusion | 4 |
| qengine | 7 |
| qengine2 | 15 |
| shared-chosen-NPC-reward-owner / qengine2 | 1 |
| shared-native-placement-owner | 4 |
| shared-world-callback-owner | 1 |

No remaining row is blocked for lacking a test or composition alone. No owner question is pending. These engine/binding boundaries remain unimplemented here; this report does not claim32finished quests.

The agent issued no Git write commands. An external process made WIP snapshots during the run; review/inventory uses assignedbase28e4313e1. No publication, CI-green or mergeability claim.
