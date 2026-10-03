# QUEST-STATE-0 Quest progress store

- Decision: `QUEST-STATE0-CHARACTER-TRACK-STORE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and security) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Amended (2026-10-03): §13, reconciled with `main` (the guard now admits eight receipt kinds; the
  Quest family import; CHAR-REV-SEQ-1's writer list), and §14, the QUEST-STATE-1 packet, for
  acceptance (control plane D352).
- Answers: the quest lane's escalation A1, A6 (kv_state) and A7 (random rewards) (#162 5913201286),
  ruled in 5913269950; the owner's direction to finish the playable game
- Builds on: the quest authoring format V1 (D32-D38, CANDIDATE; D34 tracks, D35 transitions, §6.5
  quest-owned cooldown tracks); the account progress decision (D44-D49; §4.1 P2 revision pinning;
  D45 `AccountQuestCompletion`, D46); the reward chest decisions (D39-D42); the composition decision
  §3 (rule 1 and its NPC-0 obligation-row amendment); migrations `0009`, `0012`, `0016`, `0017`,
  `0019`, `0020` (the CharacterRevision receipt chain and its current consistency guard), `0022`
  (which replaced the root revision guard); NPC-0 §5.1; ADR-0010
  (World product profiles); DUR-02 §4.1 and §4.6; FND-02 §13.3
- Amends, pending on acceptance of QUEST-STATE-0: the composition decision rule 1 (obligation rows,
  written in this PR). The quest format §3.2 and §7 gain a pointer with QUEST-STATE-1.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| CHAR-REV-SEQ-1 | hard, persistence and concurrency review | the per-Character revision sequencer of §5.2: one revision-advancing write in flight per Character, the revision cursor, and every `CharacterRevision` writer on `main` moved onto it (§5.2 list: XP, death, Bestiary, charm with its in-transaction fee burn, monk state); STANCE-1 and QUEST-STATE-1 are built on it | this decision |
| QUEST-STATE-1 | hard, persistence review | the track, quest-state, receipt and obligation tables; the ninth receipt kind in the consistency guard (§13.1); a new migration extending the `0012` claim guards for obligation rows; the transition writer (§3-§6); packet §14 | this decision; CHAR-REV-SEQ-1 |
| QUEST-ACCOUNT-1 | hard, persistence review | the account completion table and its write (§3, §13.3) | QUEST-STATE-1; a World product profile family source (ADR-0010) on `main` |
| QUEST-PRED-1 | impl | the read-only predicate API over the session's track copy (§7) | QUEST-STATE-1 |
| QUEST-CONTENT-1 | content lane | `Quest` and `Interaction` as data-only families (ruling A2). **Delivered in part** (§13.2): the 352 Quest definitions (#1596) and 336 RewardClaims (#1489) | this decision |
| QUEST-LOWER-1 | content lane | the rest of QUEST-CONTENT-1 (§13.2): per quest, its tracks (Oteryn key, owner quest, initial value, bounds) and transitions (closed effect kinds, `completes`, `requested_by`) lowered from the source progress data, and the loader into QUEST-STATE-1's catalogue type | QUEST-STATE-1 (the catalogue type) |
| CHEST-RANDOM-1 | CHEST lane, hard | the deterministic `random_one_of` draw and its column on the claim (§8) | this decision |

Callers come with their own decisions: NPC-QUEST-0 (dialogue), QUEST-GATE-0 (doors), the
interaction and creature-event owners. A quest log wire is deferred (A8).

**Amendment (pending on acceptance of QUEST-GATE-0; `OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` §3-§7).** QUEST-GATE-0
decides the gate, trigger and NPC dialogue callers, the quest XP obligation and the quest log wire
(A8), with their children QUEST-GATE-1, QUEST-TRIGGER-1, NPC-QUEST-1, QUEST-XP-1 and
QUEST-LOG-WIRE-1.

## 1. Question

Where does a character's quest progress live, and how does it change safely?

## 2. Facts

**PROVEN**

- D34: a mission is one integer progress track with start and end values; bare-number storages stay
  as `storage/<n>` tracks. Track keys are `…:quest-progress/<track>`, revision-free.
- D35: only the quest domain writes progress, through named transitions that owners request,
  validated against the current value, idempotent and session-generation fenced; progress is
  Character persistence shared across channels. Format §6.5 declares cooldowns as quest-owned
  tracks.
- Account decision §4.1 P2: an active quest is pinned to the content revision it started under.
- Samples (`questlog/`): 1,001 tracks; 1,832 transitions; effects 1,727 set, 63 step, 35 computed
  expression and 7 computed timestamp; `from` operators `==`, `<`, `<=`, `>=`, `~=` or none; 29
  transitions write −1; some values are Unix times (up to 1,522,018,605), about 30 `*_timer` tracks.
- D45 and D46: one insert-only `AccountQuestCompletion` per (Account, profile family, quest) when a
  character completes a quest that grants it; a reader also accepts the character's own completion.
- `0020`: the current consistency guard admits five receipt kinds (XP, death, stance, Bestiary
  kill, charm), exactly one receipt per revision; each receipt carries the original and committed
  revision, level and experience before and after, and the eight interpretation revisions.
- `0012`: a reward claim is keyed by (character, claim), reserved and then minted, item-only, with no
  revision advance (composition rule 1). The format asks for "auditable randomness".

## 3. Store (QUEST-STATE-1)

- **Tracks.** `game_character_quest_tracks`: `(character_id, track_key) -> value BIGINT`, plus
  `last_receipt`. No row reads as the track's declared initial value. QUEST-CONTENT-1 declares, per
  track, its owner quest, initial value (0 by default; −1 where the reference data uses it) and
  `[min, max]`. Rows are never deleted. Character + World scope, the same on every channel.
- **Quest state.** `game_character_quest_states`: `(character_id, quest_key) -> pinned revision,
  definition_hash, completed_receipt (nullable)`. The quest's first transition writes it; a
  `completes` effect sets `completed_receipt` for every quest, whatever its account setting.
- **Account completion** (moved to QUEST-ACCOUNT-1, §13.3). `game_account_quest_completions`: `(account_id, profile_family,
  quest_key)`, insert-only with `ON CONFLICT DO NOTHING` (the first keeps its provenance), written
  only when the pinned revision declares `account_completion: grant` (D46). `profile_family` is the
  World's product profile family (ADR-0010), read from the World registry row.
- **Timestamps.** A track that holds a time is written by the `SET_NOW` effect (§4) and read by the
  `elapsed` comparison. It stays a quest-owned track (format §6.5); it is not a reward-claim
  cooldown.
- **kv_state** (ruling A6) is the same store: a keyed integer is a track.

## 4. Transitions (QUEST-STATE-1)

- A transition is a content record: `transition_key`, its quest, and up to `QUESTSTATE0-RL-02` (8)
  effects on **tracks owned by that quest**. A trigger that moves two quests requests two
  transitions.
- Each effect is `{track_key, from, effect}`:
  - `from`: `any`, `=`, `!=`, `<`, `<=`, `>`, `>=`, `in [a, b]`, or `elapsed >= s` (now minus the
    value, in seconds);
  - `effect`: `SET v`, `ADD n` (checked `i64`), or `SET_NOW` (server Unix time), each within the
    track's `[min, max]`. `SET_NOW` and `elapsed` use the database transaction time (`now()`), never
    a node clock.
- The 35 `computed: expression` effects have no kind yet: each needs a closed kind added by
  amendment, and until then its transition is `NOT_SUPPORTED`.
- A transition may carry `completes: true`, which completes its quest (§3).
- **Request.** An owner calls `request_transition(fence, character, transition_key, cause)`. The
  cause is an occurrence bound 1:1 to its trigger: a CommandRef (NPC-0 §5.1 pattern), a USE
  interaction occurrence, a reward-claim obligation (§5.4) or a creature-death reward occurrence.
- **Validation** in the database transaction: every `from` holds on the locked value; the quest
  state's hash equals the current definition hash (or there is no state yet); results stay in
  bounds. A failure writes nothing and returns `STAGE_MISMATCH`, `REVISION_MISMATCH`,
  `OUT_OF_RANGE` or `NOT_SUPPORTED`.

## 5. Receipt, fence, revision and obligations (QUEST-STATE-1)

### 5.1 Receipt

- `game_character_quest_receipts`: one immutable row per committed transition, keyed by
  **(character_id, cause occurrence, transition_key)**, so one kill or reply may move several
  quests and several characters. It also records the quest's pinned content revision and
  `definition_hash` (ruling A1, #707 P2). It holds a
  SHA-256 binding of the **request only** (character, transition key, cause), and separately the
  effects' before and after values.
- The chain columns are those of the `0020` receipts: original and committed CharacterRevision,
  level and experience before and after (equal), and the eight interpretation revisions.
- The same key and binding replay the first outcome; a changed binding conflicts.

### 5.2 Revision

- Quest progress is Character state, so a transition advances CharacterRevision by one: the writer
  advances `game_character_roots` and `game_character_progression_state` together, as the charm
  writer does. QUEST-STATE-1 replaces the current consistency guard to admit a **ninth** receipt kind (§13.1),
  exactly one receipt per revision.
- **Expected revision, as its siblings.** Like the XP, death, Bestiary, charm and monk writers
  (`character_progression.rs`, `death_reward.rs`), the quest writer checks the fence's
  `expected_character_revision` and returns `CharacterRevisionMismatch` otherwise.
- **One write in flight per Character (new requirement, CHAR-REV-SEQ-1).** Nothing on `main` keeps
  a revision cursor in production today. The owning channel runtime gets one sequencer per
  Character: every revision-advancing write is submitted to it. On `main` these are, each with its
  root advance:
  - XP (`character_progression.rs:293`);
  - death (`character_death.rs:259`);
  - Bestiary (`bestiary_progress.rs:260`);
  - charm (`charm_state.rs:461`), including the gold fee burn it runs in the same transaction
    (`item_fee_burn.rs:6,212`, which checks the source's expected revision and relies on the
    source's advance);
  - monk state save (`monk_state.rs:239`, called from `monk_save.rs`);
  - build change (`character_build.rs`, migration `0030`) and proficiency (`character_proficiency.rs`,
    migration `0032`), both on `main` since this list was written (§13.1); both bind the revision.

  Later writers are built on the sequencer: stance (STANCE-1; `0017` has no writer yet) and quest.
  A writer that advances the revision outside the sequencer is a defect. The sequencer runs them
  one at a time and holds the revision cursor, advanced after each committed receipt. A composition holds the slot for its whole chain: in a creature death, XP,
  then Bestiary, then quest transitions, each taking the revision the previous one committed (the
  `bestiary_expected_revision` pattern). An NPC reply, a USE or an obligation waits for the slot, so
  it can never commit between XP and Bestiary.
- **Mismatch.** With the sequencer, a mismatch means another writer bypassed it. Where a request's
  binding excludes the revision (quest; Bestiary, `bestiary_progress.rs:540-542`), the runtime
  reloads the cursor and retries it once (replay or one commit). Where it includes the revision
  (XP `character_progression.rs:817`, death `character_death.rs:478`, charm `charm_state.rs:720`,
  monk `monk_state.rs:397`), a mismatch is not retried: it fails closed and is reported as a
  defect.

**Amendment (pending on acceptance of PREY-0;
`reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md` §4.2 and §9).** The
death chain becomes XP, then Bestiary or Bosstiary, then task kill credit, then quest transitions.
Task kill credit is written only when the Creature matches an active task. Prey and Task Board
writes are revision-advancing writers on the sequencer; their bindings exclude the revision, so a
mismatch reloads the cursor and retries once.

### 5.3 Fence and locks

- The XP writer's fence (composition decision §2 steps 1-6), with the quest receipt key in place of
  the XP occurrence; the cause's CharacterId equals the fenced Character; a still-pending cause after
  a same-GameSession reconnect commits with the current generation (FND-02 §13.3).
- **Lock order:** the recovery fence and admission relations; the receipt key; the Character's
  session checks; `character_root` FOR UPDATE; the quest state; the tracks by key; the account
  completion row (deferred to QUEST-ACCOUNT-1, §13.3).

### 5.4 Reward claim plus quest: two transactions and an obligation

- A chest that both gives items and advances a quest does not share one transaction: the claim
  stays item-only (composition rule 1; DUR-03 §39.3 unchanged). The claim transaction also writes a
  `game_character_quest_obligations` row (character, claim occurrence, transition key, state
  `PENDING`), an obligation outside the revision chain like NPC-0's pending arrival. The claim does
  not lock quest tracks, so the items may be given while the quest step is later refused: that is
  accepted by design, since the claim is the reward and the step only records progress.
- The runtime then requests the transition with that obligation as its cause. The committing
  transition deletes the row in its own transaction, and a guard allows that delete only together
  with the receipt that names the obligation. A validation refusal sets the row to `REFUSED`
  (terminal, with its result code, kept for audit); nothing is lost and nothing is retried. The
  exception is `REVISION_MISMATCH` (§6): the row becomes `WAITING_MIGRATION`, is not retried, still
  counts toward `QUESTSTATE0-RL-07`, and returns to pending when a DUR-04 migration for that quest
  lands, so that progress is not lost.
- Pending obligations are requested again at admission and after a failed attempt within the
  session (backoff, at most once a minute). A claim that would make more than `QUESTSTATE0-RL-07`
  (64) pending obligations for the character is refused before anything is written
  (`OBLIGATIONS_FULL`).
- The claim guards of `0012` are extended by a **new** QUEST-STATE-1 migration (not by editing
  `0012`), to allow the obligation row as a companion of the claim; composition rule 1 is amended
  to cover obligation rows (written in this PR).

## 6. Revision pinning (QUEST-STATE-1)

- `definition_hash` covers the quest's tracks and transitions only, not its journal text, so text
  edits never block players in progress (architect ruling).
- A different hash is `REVISION_MISMATCH` until a DUR-04 migration for that quest exists.
- A completed quest's state keeps its pin and no longer blocks anything.

## 7. Predicates (QUEST-PRED-1, ruling A3)

- The runtime loads the character's tracks and quest states at admission and applies each
  committed receipt to that copy. Predicates read it and never write: `track op value`,
  `elapsed(track) >= s`, `quest_completed(key)` (the character's own record), `account_completed(key)`
  (D45: only while the ruleset enables account completion and the quest's pinned revision declares
  `grant`), `level >= n`, `holds_item(definition)` (the last two read their own owners).
- A predicate is advisory; the writer re-checks under lock.

## 8. Random rewards (CHEST-RANDOM-1, ruling A7)

- `random_one_of` draws from `HMAC-SHA256(world_draw_key, u16 length || claim_key ||
  character_id (16 bytes) || cycle_ordinal (u64 big-endian))`: the first 8 bytes as an unsigned
  integer, by rejection sampling onto the option count. `world_draw_key` is a per-World server
  secret, so players cannot predict which character would draw which option. It is never in the
  repository; provisioning it needs separate secret authority. It rotates only at a planned world
  reset; a draw not yet claimed then may give another option, which is acceptable because nothing
  shown before the claim (a full-backpack refusal names no option) reveals the earlier one. `cycle_ordinal` is 0
  for a once-only claim (all of `0012`); the D42 cooldown decision owns it for repeatable claims.
- The seed has no command occurrence, so a retry after a refusal (for example a full backpack,
  D41) draws the same option: no re-roll. The claim row records the index. This refines ruling A7's
  "claim occurrence" wording (5913269950); the reference servers draw again on each attempt.

## 9. Rows (registered by QUEST-STATE-1)

| Row | Value |
|---|---|
| `QUESTSTATE0-RL-01` tracks per character | 4,096 |
| `QUESTSTATE0-RL-02` effects per transition | 8 |
| `QUESTSTATE0-RL-03` track value | `i64` within the declared bounds |
| `QUESTSTATE0-RL-04` request binding | 1,024 bytes |
| `QUESTSTATE0-RL-05` quest states per character | 1,024 |
| `QUESTSTATE0-RL-06` key length | 128 bytes per track, quest or transition key |
| `QUESTSTATE0-RL-07` pending obligations per character | 64 |
| `QUESTSTATE0-RL-08` runtime copy | 4,096 tracks and 1,024 states per online character |
| Per transition | 8 track rows, 1 quest state, 1 account completion (QUEST-ACCOUNT-1, §13.3), 1 receipt, 1 revision advance |

- **Retention (DUR-02 §4.6):** receipts are part of the CharacterRevision chain and are kept, as the
  charm receipts are. **Rollback:** a new migration; once receipts exist, the table and its chain arm
  stay and only new writes can be stopped (the `0020` pattern).

## 10. Rejected options

- **A generic key-value store.** It bypasses named transitions and D35 validation.
- **No revision advance.** Quest progress is Character state, recovered and audited through the
  receipt chain like every other non-item Character state.
- **One transaction for claim and quest.** It would amend composition rule 6 and DUR-03 §39.3; an
  obligation row gives the same guarantee without that.
- **Receipts keyed by the cause alone.** One kill can move several quests.
- **Interpreting `computed` expressions.** A closed kind per case is safer than an evaluator.
- **A draw seeded by the command.** A refusal would allow a re-roll.

## 11. Decision test

- **Must decide now:** YES. It is the root blocker for 97 quests (#162 5913201286); gates, NPC
  dialogue hooks and quest content wait on it.
- **Minimum sufficient:** one track table, quest states, receipts, obligations, one account
  completion table, closed comparisons and three effect kinds.
- **Superseding evidence:** a quest needing non-integer state; the 35 `computed` expressions.
- **Deliberately not decided:** the quest log wire, party/guild/world scope, legacy migration,
  DUR-04 migrations of changed quests, the `computed` kinds.

## 12. Before-freeze checklist

1. **Contract amendments:** none now; the format pointer and the `0012` companion come with
   QUEST-STATE-1.
2. **Serialization:** `character_root`, then quest state, then tracks by key.
3. **Restart:** tracks, states, receipts and obligations are durable; the runtime copy is rebuilt,
   and pending obligations re-requested, at admission.
4. **Typed references:** CharacterId, AccountId, World profile family, keys, occurrences, revisions.
5. **Wire:** none.
6. **Split work:** one transition per transaction, at most 8 effects.

## 13. Reconciliation with `main` (2026-10-03)

### 13.1 The revision chain

- The consistency guard on `main` is the `0032` body. It admits **eight** receipt kinds, exactly one
  per revision: XP, death, stance, Bestiary kill, charm, monk state (`0026`), build (`0030`) and
  proficiency (`0032`). The quest receipt is the ninth. QUEST-STATE-1 replaces the then-current
  guard body with every existing arm kept unchanged; if another receipt kind merges first, it
  merges `main` and keeps that arm too.
- The chain columns of §5.1 are the siblings' on `main`: original and committed revision (committed
  = original + 1), level and experience before and after, and the eight interpretation revisions
  (`profile`, `ruleset`, `content`, `simulation`, `evidence`, `declaration`, `policy`, `reward`).
- **CHAR-REV-SEQ-1's writer list** (the D327 batch §1.3) names XP, death, Bestiary, charm and monk.
  Build and proficiency also advance the revision and bind it (`character_build.rs`,
  `character_proficiency.rs`), so they move onto the sequencer too, with fail-closed mismatch
  handling (§5.2). §5.2 is corrected; the control plane extends that packet's owned paths.

### 13.2 Quest content on `main`

- #1596 imported the Quest family: 352 definitions (284 source-derived, 68 Oteryn-authored with
  `runtime_enabled: false`), and #1489 the 336 RewardClaims. Readiness: 42 `reward_only`
  `definition_ready`; 63 `reward_only`, 121 `script_only` and 58 `storyline` `waiting_data`;
  68 `waiting_native_bindings`.
- **No tracks or transitions are lowered.** 247 definitions carry `native_lowering.state:
  WAITING_IMPLEMENTATION` ("QuestState tracks, bounds, transitions and requested-by bindings have
  not been lowered"); their `source_data.progress` names source keys
  (`canary:quest-progress/...`), with missions, gate readers and transitions. So QUEST-CONTENT-1 is
  delivered in part, and its remainder is QUEST-LOWER-1 (brief).
- **Keys.** The store keys tracks, quests and transitions by Oteryn keys only (`oteryn:` namespace,
  at most `QUESTSTATE0-RL-06`). QUEST-LOWER-1 assigns them and keeps each source key as a source
  binding (the ADR-0021 §4.5 pattern); a source key never reaches the store or the wire.
- **`reward_only` quests** need no track: their reward is a RewardClaim on the CHEST-1 path, which
  is already on `main`. Tracks start with the quests that read or write progress.
- **QUEST-STATE-1 does not wait for content.** It defines the catalogue type (§14) and is tested
  with synthetic quests; QUEST-LOWER-1 fills it.

### 13.3 Account completion is deferred

`main` has no World product profile family (ADR-0010) to key `game_account_quest_completions`, and
no lowered quest declares `account_completion: grant`. The table and its write move to
QUEST-ACCOUNT-1 (brief); until it merges, no quest grants account completion and
`account_completed` (§7) is false, failing closed. D45 and D46 are unchanged.

Until QUEST-ACCOUNT-1 merges, this supersedes the account-completion parts of the earlier
sections: §3's account completion table, the account completion row in §5.3's lock order, and the
account completion write in §9's per-transition row. QUEST-STATE-1's transaction has no account
completion step: it locks and writes 8 track rows, 1 quest state, 1 receipt and 1 revision
advance. QUEST-ACCOUNT-1 restores those parts, with the lock taken last as §5.3 orders it.

### 13.4 Unchanged

§3-§12 stand, except the account-completion parts §13.3 supersedes. The composition decision's rule 1 amendment for obligation rows (#1373) is on
`main`. The `0012` claim guard extension stays a new QUEST-STATE-1 migration.

## 14. QUEST-STATE-1 packet (for the control plane)

```yaml
task_id: OTV2-2026100x-quest-state-1
title: "QUEST-STATE-1 quest tracks, transitions, receipts and obligations"
mode: IMPLEMENT
worker: oteryn-hard-worker   # persistence, session-generation fence, CharacterRevision chain
repository: Oteryn/Oteryn-Game
issue: 162
lane_id: quest
depends_on: ["QUEST-STATE-0 accepted", "CHAR-REV-SEQ-1 merged"]
leases: migration 0056 (control plane lease; 0054 and 0055 are TIMED-RT-1's and PROF-SHAPE-1's)
owned_paths:
  - apps/game-server/migrations/0056_character_quest_state.sql
  - apps/game-server/src/durability/quest_state.rs        # new: transition writer and loads
  - apps/game-server/src/durability/mod.rs                # module and re-exports only
  - apps/game-server/src/quest/                           # new: catalogue type, effects, validation
  - apps/game-server/src/lib.rs                           # module wiring only
  - apps/game-server/src/durability/reward_claim_mint.rs  # optional obligation companion of a claim (§5.4)
  - apps/game-server/src/interaction/chest_use.rs         # passes a chest's quest transition to the claim
  - apps/game-server/src/gameplay_transport/mod.rs        # ComposedFreshAdmission admit and resume: quest copy and obligations; its tests
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - apps/game-server/tests/support/quest_state_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs  # one `mod` include
  - docs/agents/tasks/archive/OTV2-2026100x-quest-state-1.md
```

**Scope.**

1. **Migration.** `game_character_quest_tracks`, `game_character_quest_states`,
   `game_character_quest_receipts`, `game_character_quest_obligations` (§3, §5.1, §5.4), with the
   `0019`-`0032` patterns: checks on keys and bounds, immutable receipts, no deletes of tracks,
   states or receipts, the obligation delete allowed only together with the receipt naming it, the
   runtime-role grants. It replaces the progression consistency guard with a ninth arm (§13.1) and
   extends the `0012` claim guards for the obligation companion row by new function bodies, never
   by editing an applied migration.
2. **Catalogue type.** `QuestStateCatalogue`: tracks (Oteryn key, owner quest, initial value,
   `[min, max]`), transitions (key, quest, at most 8 effects on that quest's tracks, `completes`),
   and each quest's `definition_hash` over its tracks and transitions only (§6). Built in code by
   tests; QUEST-LOWER-1 adds the content loader.
3. **Writer.** `request_transition(fence, character, transition_key, cause)` (§4, §5): the closed
   `from` comparisons and `SET`, `ADD` (checked) and `SET_NOW` effects against database time; the
   refusal codes `STAGE_MISMATCH`, `REVISION_MISMATCH`, `OUT_OF_RANGE`, `NOT_SUPPORTED` (computed
   effects) writing nothing; the §5.3 fence and lock order; one revision advance and one receipt
   per transition; replay by (character, cause, transition) with a request-only binding, conflict
   on a changed binding; submission through the CHAR-REV-SEQ-1 sequencer with one reload-and-retry
   on mismatch (§5.2).
4. **Obligations.** The `PENDING`, `REFUSED` and `WAITING_MIGRATION` states and
   `QUESTSTATE0-RL-07`; the claim-side companion write and `OBLIGATIONS_FULL` (§5.4):
   `RewardClaimMintRequest` gains an optional quest transition, and the claim transaction inserts
   the `PENDING` row with the claim or refuses the whole claim before writing anything;
   `interaction::chest_use` passes the chest's transition when its content declares one (none
   until QUEST-LOWER-1 lowers chest bindings, so existing chests are unchanged); the re-request at
   admission and after a failed attempt (once a minute at most).
5. **Admission load.** The production `ComposedFreshAdmission` (`gameplay_transport`) loads
   tracks, states and pending obligations at fresh admission and resume, keeps them as the session
   copy that QUEST-PRED-1 reads, and re-requests the pending obligations; bounded by
   `QUESTSTATE0-RL-01`, `-05` and `-08`, and over a bound fails the load closed. A failed quest
   load fails quest actions closed, not login.

**Required tests:** each refusal code writes nothing; replay and binding conflict; revision chain
continuity with every other receipt kind; a concurrent writer bypassing the sequencer is caught by
the guard; a reconnect in the same GameSession commits a pending cause with the current
generation, and a replaced session's request is refused; obligation delete without its receipt is
rejected; `WAITING_MIGRATION` is not retried; bounds at and over each row; restart reloads the
copy and re-requests obligations, proved through `ComposedFreshAdmission` and not only the loader;
a claim with a quest transition commits the items and the `PENDING` row together, a claim at 64
pending obligations is refused writing nothing, and a claim without one is unchanged.

**Validation.** `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server
--all-targets -- -D warnings`; `cargo test --locked -p oteryn-game-server` with PostgreSQL; the
governance and repository policy validators. Independent persistence review on the frozen head.

**Out of scope.** Callers (QUEST-GATE-1, QUEST-TRIGGER-1, NPC-QUEST-1, QUEST-XP-1), predicates
(QUEST-PRED-1), content lowering (QUEST-LOWER-1), account completion (QUEST-ACCOUNT-1), random
draws (CHEST-RANDOM-1), any wire.
