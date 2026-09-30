# QUEST-STATE-0 Quest progress store

- Decision: `QUEST-STATE0-CHARACTER-TRACK-STORE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and security) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
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
| QUEST-STATE-1 | hard, persistence review | the track, quest-state, receipt, obligation and account-completion tables; the sixth receipt kind in the consistency guard; a new migration extending the `0012` claim guards for obligation rows; the transition writer (§3-§6) | this decision |
| QUEST-PRED-1 | impl | the read-only predicate API over the session's track copy (§7) | QUEST-STATE-1 |
| QUEST-CONTENT-1 | content lane | `Quest` and `Interaction` as data-only families (ruling A2): track keys with owner quest, initial value and bounds; transitions with closed effect kinds; the 111 gap-free `reward_only` quests first | this decision |
| CHEST-RANDOM-1 | CHEST lane, hard | the deterministic `random_one_of` draw and its column on the claim (§8) | this decision |

Callers come with their own decisions: NPC-QUEST-0 (dialogue), QUEST-GATE-0 (doors), the
interaction and creature-event owners. A quest log wire is deferred (A8).

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
- **Account completion.** `game_account_quest_completions`: `(account_id, profile_family,
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
  writer does. QUEST-STATE-1 replaces the `0020` consistency guard to admit a **sixth** receipt kind,
  exactly one receipt per revision.
- **Expected revision, as its siblings.** Like the XP, death, Bestiary and charm writers
  (`character_progression.rs`, `death_reward.rs`), the quest writer checks the fence's
  `expected_character_revision` and returns `CharacterRevisionMismatch` otherwise. The runtime keeps
  one revision cursor per Character and advances it after every committed receipt of any kind. In a
  creature-death composition, quest transitions run **after** XP and Bestiary and take the revision
  those committed (the `bestiary_expected_revision` pattern). On a mismatch the runtime reloads the
  cursor and retries the same request; the binding excludes the revision, so a retry replays or
  commits once.

### 5.3 Fence and locks

- The XP writer's fence (composition decision §2 steps 1-6), with the quest receipt key in place of
  the XP occurrence; the cause's CharacterId equals the fenced Character; a still-pending cause after
  a same-GameSession reconnect commits with the current generation (FND-02 §13.3).
- **Lock order:** the recovery fence and admission relations; the receipt key; the Character's
  session checks; `character_root` FOR UPDATE; the quest state; the tracks by key; the account
  completion row.

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
  (terminal, with its result code, kept for audit); nothing is lost and nothing is retried.
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
  secret, so players cannot predict which character would draw which option. `cycle_ordinal` is 0
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
| Per transition | 8 track rows, 1 quest state, 1 account completion, 1 receipt, 1 revision advance |

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
