# PARTY-XP-1 — party shared experience and party spells (consolidated task card)

```yaml
task_id: PARTY-XP-1
title: "PARTY-XP-1: party shared experience and party spells"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PARTY-XP-1; §7 'PARTY-XP-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
base_branch: main
branch: w5/party-xp-1
pr: null
class: hard
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2172, branch skeleton/party-xp-1

Status note: the draft added a detached shared-XP planner (`gameplay_transport/party_xp.rs`,
501 lines + 801 test lines) and a `plan_shared_xp_in_transaction` read on the pre-ADR-0022
`apps/game-server/src/...` layout; 15 native cases passed (13 rule bodies + 2 PG 17.6
strong reads) through a disclosed out-of-tree bridge, so no in-tree execution qualifies it.
Its World-owner hunks were inverted to PARTY-1 (CP D1101). No kill-settlement caller, PZ
accessor, toggle command or feedback was wired. It is closing; the mechanic is re-implemented
fresh on the current layout. Feeds BOSS-REWARD-1 (MONSTER-META-1).

## Goal

Parties share experience under PARTY_PVP0 §5.1 (level range, activity, leader/PZ) inside the
real kill transaction; party spells affect only party members with feedback through CHAT-2.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| PARTY_PVP0 acceptance | ACCEPTED (plan §7); D118 §2.4 amendment fixes party → Prey → stamina order |
| PARTY-1 (World owner, `shared_xp_enabled` reads) | MERGED (#2201; card still active — toggle command: verify at allocation) |
| D118 XP lane (LOOP-PROGRESS-1) | MERGED (#2192, skeleton) |
| LOOP-DAMAGE-1 (D3-4) | MERGED (#2195, bounded checkpoint) |
| CHAT-2B relay (party spell feedback) | MERGED (#2227) |
| DEATH-3 | waived by D1022 (#2028); verify at allocation |
| DEATH-1, ATTACK-1b, COND-1, SPEED-1, VIS-2, ATTACK-WIRE-1 | MERGED (archive records, #1798, #2122) |
| Party bonus (vocation normalizer) and summon share accounting | BLOCKING (no accepted owner on main) |
| Leader PZ production accessor | BLOCKING (`combat_lock_fact.rs` read is `pub(super)`; verify reuse at allocation) |
| HUNT-1 per-Character Prey (after the split) | BLOCKING — re-implementation pending (draft #2149 closing) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/world_party.rs` (1037 lines) —
  `WorldPartyXp`, `read_world_party_xp_in_transaction`,
  `read_world_party_xp_for_scope_actor_in_transaction` (toggle + strong membership read).
  PARTY-1 owns it; edits here are wiring-only.
- `crates/gameplay/social-rules/src/shared_experience.rs` — 120 s activity window,
  `committed_shared_experience_activity`, `heal_credits_party_member`.
- `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` — existing activity recording
  (`record_committed_shared_experience_batch`); reuse, do not duplicate.
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs` (1213 lines) and
  `crates/gameplay/combat/src/combat/death_reward.rs` (1002 lines) — the kill transaction;
  today single-principal. Over 1000 lines: call into a new settlement file, ≤ ~150 lines growth.
- `crates/gameplay/dispatch/src/gameplay_transport/party_spell_owner.rs` — party spell
  selection (target selection stays intact).
- `crates/gameplay/dispatch/src/gameplay_transport/combat_lock_fact.rs` — leader PZ fact source.
- `crates/gameplay/entities/src/carrier/runtime_actor_party.rs`,
  `crates/gameplay/social-rules/src/party.rs` — party snapshot (unchanged).
- `crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs`,
  `apps/game-server/tests/support/attack_kill_reward_postgres_cases.rs`,
  `apps/game-server/tests/combat_death_reward_postgres.rs` — existing PG patterns.
- `apps/game-server/migrations/0046_world_parties.sql` — `game_parties.shared_xp_enabled`
  already exists; no schema is invented.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/social-rules/src/shared_experience_split.rs` (new) +
  `shared_experience_split_tests.rs` (new) — L3 pure eligibility and equal split.
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp.rs` (new) —
  `plan_shared_xp_in_transaction` (same-transaction read + planner).
- `crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs` (new) + tests
  (new) — atomic multi-principal settlement called from the kill transaction.
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp_postgres_tests.rs` (new) —
  canonical PG cases, mounted once under dispatch `--lib`; no new PostgreSQL target.

## Accepted design (PARTY_PVP0 §5.1/§13, D118 §2.4, as built in the draft)

Planner (pure data):

- One activity cell per Character/session on a World/Channel for committed qualified healing
  (of an actual party member) or aggressive-monster attack events; replaced sessions get a
  fresh cell; recovery starts empty; party changes keep recent activity (incl. a solo attack
  before joining).
- 50-principal bound; 120 s inclusive freshness; all participants within 30 tiles Chebyshev
  on any floor; exact two-thirds level test (no float); leader presence, toggle on, PZ
  conditions.
- Equal shares rounded up without `pool + count - 1` overflow (wide-integer oracle up to
  `i64::MAX`). Foreign-Channel members neither participate nor disable sharing. One failing
  participant disables sharing for the whole kill. Missing/mismatched inputs refuse planning.
- Pool input is after admitted party bonus and summon deduction; then the split; then each
  Character's Prey and stamina.

Integration obligations:

1. The real kill owner holds runtime/player locks, current session/lease/node/Channel/World
   fences and the transaction through planning and commit; a stored snapshot is never a grant.
2. Toggle/membership read via the existing `world_party.rs` port in the same transaction;
   predecessors rechecked at the consuming owner.
3. Activity uses the committed event's original semantic timestamp; retries never refresh it.
4. Deterministic CharacterId lock order and native Character sequencers/receipts settle each
   death exactly once; retry, concurrent kills and recovery never duplicate awards.
5. Leader-authorized shared-XP toggle goes through PARTY-1's revision/receipt/event rules.
6. Party buff/Serene behaviour already exists; content and CHAT-2 feedback are its integration,
   not a second spell implementation.

## Review dispositions (from the draft)

- Preliminary review: PartyId reset lost recent solo activity on joining → one cell per
  Character/session + World/Channel.
- D1101: World-owner hunks inverted; PARTY-1 is sole owner of `world_party.rs`.
- Out-of-tree bridge evidence and codegen SIGKILL are not qualification; re-run in-tree.

## Remaining work

The local implementation and qualification checkpoint is below. Integration remains with CP;
content validation and the no-PostgreSQL CI routing decision remain external prerequisites.
Player-owned summon attacks remain absent on this baseline; this lane exposes and tests their
accepted pure split deduction without adding a new summon combat producer.

## Acceptance criteria

- [x] Shared XP split exactly once per kill, only to eligible active members.
- [x] Ineligible members (range/activity/level) disable sharing per §5.1.
- [x] Party spells affect only party members; feedback via CHAT-2.
- [x] Concurrent kills serialize per Character XP lane.
- [x] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.
- [x] Canonical PG cases show real database-start markers.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests in their own child files (not appended to existing files).
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- XP split + kill reward + audit in one transaction
- party spells + bundle content + regeneration + tests

## Validation

Use `/home/mole/oteryn-cache/codex-bin/cargo` with `CARGO_NET_OFFLINE=true` for every
Cargo command. The owner's D1466/2026-10-22 packet supersedes the original workspace-wide
Clippy and unregistered PostgreSQL-target commands: use the seven touched gameplay packages
and game-server for Clippy, and only the filtered dispatch `--lib party_xp` cases for PG.
The final checkpoint records every exact test filter and result. No whole PostgreSQL target
or workspace test run is part of this worker's validation.

```bash
python3 tools/content-migration/regenerate_content.py --check
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
```

## Reference evidence

- PARTY_PVP0 §5.1, §13, PARTYPVP0-RL-06; D118 §2.4
- `docs/reference/spells/r21-local-candidate/bundles/instant-*_party/`; PR #1798
- Completion plan §4, §5 (Wave 3), §7; coordination #2131 (D1101, D1102)

## Prerequisite audit (2026-10-22)

Baseline: `9e5770362a96b2e0e269abf1480f7de3a17163fe` (current worktree HEAD).
Local authoring only; no frozen candidate or PR. The only changed path is this card.

Planned module layout, before implementation: L3 eligibility/equal split in
`social-rules/src/shared_experience_split.rs`; L4 transactional XP persistence in a
dedicated durability mechanic module; L5 planning/composition in `party_xp.rs` and
`party_kill_settlement.rs`; child rule/settlement tests and one `party_xp_postgres.rs`
target. Existing parent modules receive registration only. No production module has
been added: the prerequisite below needs an owner scope decision first.

- **PROVEN:** `durability/world_party.rs` has both same-transaction shared-XP read
  ports, and `PartyAction::SetSharedExperience` has the revisioned durable writer.
  Migration `0046_world_parties.sql` supplies the flag; it needs no migration.
- **PROVEN:** existing `social-rules/shared_experience.rs` and dispatch `attack.rs`
  own committed activity and its original event time. Reuse these owners.
- **PROVEN:** `combat/death_reward.rs` still admits exactly one principal and
  commits loot and XP separately. `character_revision_sequencer.rs` supplies the
  existing Character serialization seam; multi-member atomic settlement is absent.
- **PROVEN:** the durable PZ source exists in `durability/pvp_state.rs`.
  `CombatLockFact` exposes only PvE activity with `PvpLockSource::NotWiredInRuntime`;
  it cannot substitute for the leader's current durable PZ block.
- **PROVEN:** searching all `apps/game-server/migrations/*.sql` finds neither Prey
  nor stamina state. No Prey runtime/durability owner exists in `crates/gameplay/`.
  `characters/stamina.rs` supplies offline regeneration arithmetic, not a durable
  hunting/kill modifier owner. `rulesets/progression/prey/index.json` remains
  `READY_UNPOPULATED`; the HUNT-1 card also records the missing migration/owner.
- **PROVEN:** accepted PREY-0 §§2, 4.1–4.2 and 7 require dedicated typed Prey
  relations, admitted revision receipts, current per-character active bonuses and
  their grade frozen into the XP binding. DUR-02 §4.6 forbids a generic JSON/KV
  substitute. D118 §2.4 requires party → per-character Prey → stamina ordering.

**BLOCKER — owner scope decision:** the explicit lane allocation says
`MIGRATION NUMBER: none (needs_migration: true)`. Implementing the missing durable
prerequisite in this lane requires changing that allocation. The integrator's
option B authorizes prerequisite work but supplies no migration number and does
not override the explicit no-migration rule. Recommended: authorize the minimum
durable Prey/stamina prerequisite scope and allocate a unique migration number;
alternatively supply the merged production owners and resume with no migration.
No migration, invented bonus source, constant no-bonus fallback or test-only
production substitute was introduced. All implementation acceptance remains open.

## Local validation and file-size record (2026-10-22)

All cargo commands used `/home/mole/oteryn-cache/codex-bin/cargo` with
`CARGO_NET_OFFLINE=true`; no dependencies were downloaded.

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked -p oteryn-gameplay-social-rules shared_experience -- --nocapture`:
  1 passed (existing activity rule).
- `cargo test --locked -p oteryn-gameplay-dispatch world_party_runtime_postgres_cases::durable_world_read_observes_remote_channel_members_and_new_revision -- --exact --nocapture --test-threads=1`:
  1 passed, 915 filtered out; database test body ran for 10.48 s, with no
  `PRE-ROUTING / NONCANONICAL` skip. The existing canonical harness creates a real
  database, asserts PostgreSQL `170006` and applies `GAME_MIGRATOR` before the case.
  This is baseline PARTY-1 evidence, not the new PARTY-XP settlement qualification.
- `python3 tools/agents/validate_governance.py`: passed (22 documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: 124 passed.
- No Rust/content changes; crate Clippy and content regeneration were not run.
  The required new `party_xp`/`party_xp_postgres` suites remain unimplemented.
- Reused existing code: `shared_experience::tests` and the dispatch
  `world_party_runtime_postgres_cases` case with its existing canonical
  `bestiary_postgres_harness`; no helper, fixture, wrapper or registration copied.
- Only this card grew (under 100 added lines); no file grew by more than 200 lines.
  No new enum variants, fields, trait methods or public functions require caller
  updates. Production code remains unchanged pending the owner decision.


## CP continuation allocation — 2026-10-22

The current owner packet supersedes the prerequisite audit's migration blocker: option (a),
minimum durable Prey/stamina ownership needed by shared XP, provisional migration **0141**
(CP renumbers at integration). Work remains local AUTHORING on `w5/party-xp-1`; no candidate
freeze, commit, push or PR is authorized in this worker session.

Prerequisite implementation scope:

- Dedicated L4 `hunting_xp.rs`, `party_kill_xp.rs` and `party_pz_fact.rs`; pure modifier
  arithmetic in L3 `characters/hunting_experience.rs`. Prey slot, permanent slot unlock and
  XP binding relations are typed. No JSON/KV state, rolls, wildcard economy, Task
  Board, Store delivery, offline training or full offline settlement was added.
- OFFLINE-0 §3's canonical stamina value is `game_character_build_state.stamina_minutes`,
  with the accepted absent-row seed of 2,520. No second stamina relation or speculative
  hunting-clock/checkpoint implementation is introduced. OFFLINE-0/STAMINA-1 keeps the
  accepted damage/taken-damage clock, build checkpoint, death and logout lifecycle.
- A committed kill stores the per-Character raw share, active Prey slot/epoch/grade/percent,
  stamina and resulting award. The hunting binding joins the native XP command binding;
  all positive XP receipts and the kill audit commit together. This consumer reads stored
  hunting state and does not spend or initialize Prey time.
- PREY-0 §6.5 absent slots remain uninitialized; no synthetic active/no-bonus producer was
  substituted. This prerequisite grants the runtime read-only access to the Prey relations;
  activation, expiry and consumption remain with their separately allocated owners. Slot 2 checks current Premium or its permanent
  unlock; slot 3 requires its permanent unlock.
- The existing PvP writer and leader PZ read share one per-Character transaction lock,
  protecting both an existing deadline and the no-row case.

Original scope implemented so far:

- Eligibility, exact level/range/activity boundaries, promoted-vocation normalization,
  party bonus and equal rounding live in their own L3/L5 modules. Per-death activity is
  captured from committed activity cells, so a later attack cannot rewrite a queued kill.
- The native scalar XP writer is reused by an in-transaction port. The real kill drain
  acquires Character sequencers in CharacterId order and consumes a fresh same-transaction
  World membership/toggle read under the runtime/player/attack and SQL fences.
- The carrier gains a separate 50-principal occurrence port; the scalar compatibility
  port keeps its original conflict behavior and tests.
- CHAT command 13 carries its genuine CommandId through dispatch. `/sharedxp on` and
  `/sharedxp off` call PARTY-1's existing leader-authorized revision/receipt/event writer;
  they allocate no protocol code. Party spell outcomes use the bounded private CHAT egress.
- Existing production party spell bundles and target selection are reused. The card's
  remaining validation will distinguish existing behavior from newly qualified evidence.

Validation is still in progress. The existing solo real-dispatch PostgreSQL kill test passed
before the later prerequisite refinements (1 actual PG case, 53.88 s); that is not final-change
qualification. The new canonical two-player tests print `POSTGRES_DATABASE_STARTED` after the
real 17.6 harness/migrator succeeds. Fixture/implementation failures are being repaired rather
than classified as passing evidence. No acceptance checkbox has been closed on that basis.

Review corrections during local qualification:

- A kill with several contributing parties plans a separate damage pool per strongly read
  party, plus solo contributors, before merging the bounded recipient set. A foreign
  contributor's damage is never absorbed into the principal party's pool.
- A lethal first hit qualifies activity using the retained dead-creature policy. Spell
  deaths use the precise original committed batch clock, including receipt replay; the
  later drain clock cannot refresh activity or change the death binding.
- The PZ reader takes the same named per-Character lock as the PvP writer using a try-lock.
  Contention returns the typed unavailable/retry path, avoiding a wait cycle between
  multiple party leaders and the writer's sorted target order.
- Loot and Bestiary keep their established independent descendants even when the atomic
  XP transaction fails. All XP members/audit still roll back together; retry reconciles
  each existing descendant through its native receipt.

Qualification boundary: production damage inputs currently contain physical Character
contributors; no player-owned summon attack producer exists on this baseline. The L3
split accepts and tests the accepted summon deduction, while this reachable production
path supplies zero. Future summon combat must supply its authoritative share at that
owner seam. No fabricated summon grant or new attack producer was added to this lane.

The content regeneration check reached item-authoring validation, then stopped because
this environment lacks `rfc3339-validator`. Dependencies were not downloaded or installed;
this remains an external validation prerequisite. No content artifact was changed.

Pending routing decision sent to the owner: the standard Rust fast library jobs have no
PostgreSQL and the dedicated PostgreSQL job reruns the same library with its database.
Existing tests return NONCANONICAL in the no-DB jobs; this worker's new cases currently
require the canonical URL, honoring the instruction not to skip tests. CP must select the
existing routing guard or arrange database-only routing before integration. The actual local
qualification always uses the wrapper-provided canonical database and asserts both markers.

## Local changed-file inventory

- `crates/gameplay/characters/src/lib.rs`
- `crates/gameplay/combat/src/combat/death_reward.rs`
- `crates/gameplay/combat/src/combat/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/attack.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/capabilities_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/spell_timer_callbacks_tests.rs`
- `crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs`
- `crates/gameplay/durability/src/durability/character_progression.rs`
- `crates/gameplay/durability/src/durability/character_revision_sequencer.rs`
- `crates/gameplay/durability/src/durability/core_modules.rs`
- `crates/gameplay/durability/src/durability/mod.rs`
- `crates/gameplay/durability/src/durability/pvp_state.rs`
- `crates/gameplay/entities/src/carrier.rs`
- `crates/gameplay/entities/src/carrier/runtime_actor_spell.rs`
- `crates/gameplay/entities/src/lib.rs`
- `crates/gameplay/social-rules/src/lib.rs`
- `crates/gameplay/spell/src/spell/cast.rs`
- `docs/agents/tasks/PARTY-XP-1.md`
- `docs/contracts/resource-limits/entries/0368-COMBAT01-XP-DESCENDANTS-PER-DEATH.json`
- `docs/contracts/resource-limits/entries/0369-COMBAT01-REWARD-PRINCIPALS.json`
- `docs/contracts/resource-limits/entries/0609-PARTYPVP0-RL-04.json` through `0612-PARTYPVP0-RL-07.json`
- `.party-xp-validation-results.json` (local evidence only, not committed)
- `apps/game-server/migrations/0141_party_xp_hunting.sql`
- `crates/gameplay/characters/src/hunting_experience.rs`
- `crates/gameplay/characters/src/hunting_experience_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_spell_feedback.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_spell_kill_time_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp_command.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp_postgres_support.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp_postgres_tests.rs`
- `crates/gameplay/durability/src/durability/character_experience_transaction.rs`
- `crates/gameplay/durability/src/durability/hunting_xp.rs`
- `crates/gameplay/durability/src/durability/party_kill_xp.rs`
- `crates/gameplay/durability/src/durability/party_pz_fact.rs`
- `crates/gameplay/entities/src/carrier/party_reward_occurrence_tests.rs`
- `crates/gameplay/social-rules/src/shared_experience_split.rs`
- `crates/gameplay/social-rules/src/shared_experience_split_tests.rs`


## Resumed qualification and self-review — 2026-10-22

Same local AUTHORING task and allocation; no frozen SHA, PR, Git mutation or publication.
The META policy was refreshed at the exact bound immutable revision. Review used the whole
uncommitted diff and `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` (D1462), plus the
repository checklist. The production/public function and enum/field/trait changes were
searched throughout `apps/`, `crates/` and `tools/`: all new ports have production consumers;
all constructors, matches and chat-intent implementations were reconciled. The new
`NoExperienceAward` variant is a runtime disposition, not a persisted SQL cause code.

Additional corrections in this resumed session:

- The concurrent-kill fixture originally refreshed only the designated second killer's
  weapon facts. The other attacker could stop swinging and become inactive, producing the
  correctly unshared second kill. Both connection-owned weapon facts are now refreshed.
  The original `(11, 11)` XP, four-receipt and two-kill assertions remain intact; the
  focused real PostgreSQL rerun passed with both database-start and completion markers.
- The game-server all-target compile exposed test aggregates that include durability's
  `core_modules.rs` directly. The native XP transaction helper and PZ lock reader are now
  registered in that shared core; the native-world-only modules remain in `mod.rs`.
  This is module wiring, with no copied implementation or new PostgreSQL target.
- Registered `PARTYPVP0-RL-04` through `PARTYPVP0-RL-07` for the existing accepted range,
  exact level ratio, activity age and vocation-bonus schedule. The two Combat principal/XP
  descendant limits are raised to 50 only for the new shared-XP owner; scalar compatibility
  tests keep their original one-principal contract.
- Standalone entities tests need their existing `--features test-support` to expose the
  Content fixtures. With that feature, both occurrence/idempotency/eviction cases passed;
  no test or fixture gate was removed. The combined gameplay Clippy run also passed.

The first filtered `party_xp` run executed 12 real PostgreSQL cases and the precise spell
clock regression: 12 passed, with only the concurrent fixture failing. Its focused rerun
passed after the correction above. Across that run and repair rerun, all 12 unique new
PostgreSQL cases have passed, without skips, plus the spell-clock regression. PostgreSQL
markers are emitted only after the canonical harness has asserted `170006` and completed
`GAME_MIGRATOR`; the cases also print `POSTGRES_ASSERTIONS_COMPLETED` after their assertions.

Content regeneration was retried: spell-family import checks and world-project checks
passed, but `tools/content-schema/item-authoring/test_engine_items.py` stopped because
`rfc3339-validator` is absent. No dependency was downloaded, installed, or bypassed. No
content artifact changed. This remains an external validation prerequisite.

Final focused validation results follow. All queued checks have ended; the implementation remains
local AUTHORING for CP, with no claim of a published or frozen candidate.


### Final focused validation

All Cargo invocations used `/home/mole/oteryn-cache/codex-bin/cargo`, `--locked` on
Clippy/tests and `CARGO_NET_OFFLINE=true`. Every test command below ended with
`-- --nocapture --test-threads=1`; feature flags expose the existing fixture owners.
The complete argv/results are in `.party-xp-validation-results.json`.

| Package | Exact test arguments following `test --locked` | Passed |
|---|---|---:|
| social-rules | `-p oteryn-gameplay-social-rules shared_experience` | 5 |
| characters | `-p oteryn-gameplay-characters hunting_experience` | 1 |
| entities | `-p oteryn-gameplay-entities --features test-support --lib reward_occurrence` | 2 |
| combat | `-p oteryn-gameplay-combat --features test-support --lib death_reward` | 5 |
| spell | `-p oteryn-gameplay-spell --features test-support --lib spell::party` | 11 |
| durability | `-p oteryn-gameplay-durability --features test-support --lib character_revision_sequencer` | 11 |
| dispatch | `-p oteryn-gameplay-dispatch --lib kill_reward::tests` | 35 |
| dispatch | `-p oteryn-gameplay-dispatch --lib spell_timer_callbacks::tests` | 3 |
| dispatch | `-p oteryn-gameplay-dispatch --lib chat_wire` | 2 |
| dispatch PG | `-p oteryn-gameplay-dispatch --lib party_xp` | 11 PG + clock unit initially; concurrent case repaired below |
| dispatch PG repair | `-p oteryn-gameplay-dispatch --lib party_xp_real_dispatch_concurrent_kills_serialize_character_awards` | 1 PG |
| dispatch PG regression | `-p oteryn-gameplay-dispatch --lib auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres` | 1 PG |

Total: **75 unique focused unit tests and 13 unique PostgreSQL cases passed**, with no
ignored/skipped tests. The new twelve-case PG suite printed 12 database-start markers;
the focused concurrency repair printed its own start/completion markers. The existing solo
PG case ran for 117.85 s through the canonical 17.6 harness, without a NONCANONICAL return.

- Clippy `--all-targets --quiet -- -D warnings`: passed for all seven changed gameplay
  packages together, and separately for `oteryn-game-server` after the aggregate-mount fix.
  Vendored Tokio emits its existing capped dependency `missing_docs` warning; no changed
  crate warning or Clippy failure remains.
- `cargo fmt --all -- --check`: passed after formatting the concurrent fixture.
- Governance validation: passed; governance Python tests: 124 passed.
- `git diff --check`: passed. No manifest, lockfile, workflow, repository-policy or
  `tools/repository/` file changed. No whole PG suite ran.
- The largest new module is the PG child at 995 lines. Every previously >1,000-line source
  module grew by fewer than 150 lines. `mod.rs`/`lib.rs`/`connection.rs`/`core_modules.rs`
  changes are registration or signature forwarding only.

### Unfinished / integration handoff

1. Content regeneration remains blocked by missing `rfc3339-validator` in the existing
   item-authoring check. Dependencies were not downloaded or installed.
2. CP must resolve the previously recorded fast-job/no-PostgreSQL routing decision. The
   new canonical library cases require the database and have no absent-database skip;
   this worker did not edit protected CI routing files or weaken tests.
3. CP retains migration **0141** renumbering, independent material review, freeze,
   publication and integration. No PR exists in this local worker scope. The task record
   remains active for CP to archive in the completing authoring commit.
4. Prey activation/expiry/consumption, the full stamina clock/offline lifecycle and a future
   player-owned summon attack producer remain separately allocated owners, as bounded in
   the prerequisite scope above. This change supplies only the typed state/reader and
   shared-XP consumer required by this packet.

There are no unresolved implementation, compilation, Clippy or relevant local test failures.
No unrelated load-contention failure needed a CI-only deferral.

## Port onto DISPATCH-WIRING-1 (2026-10-23)

Ported onto the DISPATCH-WIRING-1 head `de3fd2190` (with REGISTRY-SPLIT-1, DAILY-1a and
QUEST-RT-1). Adaptations, no behavior change:

- Channel session state now lives in `ChannelSessionStates`; the party XP owners and the PG
  suite read `states.spell_states`, `states.attack` and `states.chat`.
- `party_xp_postgres_tests.rs` builds its owner through `ComposedTestOwner::build` and then
  overrides `active_generation`, `rooms` and `clock_origin`, as the kill-reward e2e does.
- The native XP transaction keeps main's obligation read without `FOR UPDATE` (the held
  `character_root` lock serializes it; the runtime role lacks UPDATE there).
- TUTORIAL-1's `Vocation::None` maps to vocation `0`, which the pure split already excludes
  from the vocation bonus.
- Resource limits are per-entry files (REGISTRY-SPLIT-1): two edited Combat entries and four
  new `PARTYPVP0-RL-04..07` entries at ordinals 0609-0612 (0592-0608 are allocated to
  SPECTATE-1b and HOUSE-1a).
- Migration number 0141 is unchanged and does not collide with main.
- The inactive-member PG case selects Offensive fight mode for its lone level-one fist
  attacker: main's content changes moved the content-seeded damage draws, and Balanced-mode
  chip damage no longer killed the rat within the unchanged 64-pass bound. Every assertion
  is unchanged.
