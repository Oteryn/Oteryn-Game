# ACHIEVEMENT-0 Achievement grant runtime

- Decision: `ACHIEVEMENT0-GRANT-RUNTIME-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence and
  protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control-plane allocation D292 (#1622) and the base-mechanics close-out ruling. That
  ruling calls for "an ACHIEVEMENT-0 architecture decision (the Achievement owner contract, its
  grant runtime and durable state), with the Character progression lane as runtime owner. The typed
  grant requests already authored stay as they are."
- Builds on:
  - `OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1` §1-§5 (catalogue, grant path, points);
  - `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1` (the panel query);
  - D48 and D49 (account-progress decision §4.4 and §4.6);
  - migration `0021` and `durability/account_achievement.rs` (`record_achievement_grant`, the
    outcomes `Granted`, `AlreadyHeld` and `Retired`);
  - the reward-claim grant and the chest `USE` granter (archived tasks
    `OTV2-20260930-reward-claim-achievement-grant`, `OTV2-20260930-chest-achievement-runtime`);
  - QUEST-STATE-0 §4-§5;
  - ENCOUNTER-RT-0 §6.4;
  - WORLD-INTERACTION-0 §3.5;
  - SKILLS-0 §3.2 and A13 §4.2 (build receipts);
  - owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162/#1622
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ACH-QUEST-1 | impl, persistence review | the `achievement` field on a quest transition and its grant in the transition transaction (§3.2) | QUEST-STATE-1 |
| ACH-ENC-1 | hard (persistence), persistence review | the `achievement` outcome consumer of an encounter and its durable outcome receipt table (§3.3) | ENC-OUTCOME-1 |
| ACH-COUNTER-1 | impl, persistence review | the batch grant API (§3.1), the threshold table and the grant at a counter's durable commit, first for level and skill receipts (§3.4) | the XP receipt; SKILLS-0's build receipts |
| ACH-NOTIFY-1 | impl, protocol review | capability 8 `ACHIEVEMENT_NOTICES_V1` and state domain 13 `ACCOUNT_ACHIEVEMENT_NOTICES` in the FND-02 registry, with the delta after a `Granted` commit (§5) | the FND-02 state-domain path |
| ACH-COVERAGE-1 | content tooling | the coverage report (§4) in `validate_achievements.py` | none |

The **Character progression lane** is the runtime owner of every child here.
- **Who calls the shared function.** The Achievement domain module stays the only writer of the
  account fact (owner contract §1). Each granter calls the one grant entry point (§3.1) from its own
  transaction, and none calls it from anywhere else.
- **Unchanged.** The reward-claim grant and the chest `USE` path stay as they are.

## 1. Question

The catalogue, the account fact, its migration, one granter (reward claim through a chest `USE`)
and the panel query exist. This decision answers how every other Tibia way of earning an
achievement reaches that same grant:
- quest steps;
- boss and encounter outcomes;
- counters such as level and skills;
- map interactions.

It also answers how the player is told that they earned an achievement. Each answer must keep the
single-owner, same-transaction rule.

## 2. Facts

- **Tibia** (manual §achievements, `CIPSOFT_OFFICIAL`, capture 2026-09-28):
  - achievements are optional goals that give no XP or items, only points;
  - "On completion, the client shows a pop-up notification naming the achievement";
  - secret achievements are not documented officially.
- **TibiaWiki** (`TIBIAWIKI_STRUCTURED`, 572 records, imported 2026-09-29). Earning conditions fall
  into a few families:
  - finishing a quest or a quest step;
  - killing a boss or winning an encounter (for example "Champion of Chazorai": every player in
    the arena);
  - reaching a count (level, skill, creatures killed, fish caught, items used);
  - using or doing something once (an item, an NPC dialogue, a map object).
- **Canary** (`OTS_HYPOTHESIS_ONLY`) grants from Lua scripts with
  `player:addAchievement(name)` and sends "Congratulations! You earned the achievement \"%s\"."
  as an advance message.
- **Repository:**
  - `record_achievement_grant` runs inside the granter's fenced character transaction, under the
    `character_root` lock. It returns:
    - `Granted` (fact inserted);
    - `AlreadyHeld` (first provenance kept);
    - `Retired` (no-op);
    - or fails closed on `UnknownAchievement`.
  - The runtime catalogue loads at Content activation and fails closed.
  - Quest transitions are content records with up to 8 track effects and `completes`
    (QUEST-STATE-0 §4).
  - Encounter outcomes go to bound consumers, idempotent by the outcome key (ENCOUNTER-RT-0 §6.4).
  - The quest gate decision already routes "an achievement with a claim" through the reward-claim
    grant (QUEST-GATE-0 §5.6).
  - No server-to-client message announces an earned achievement. The chat wire carries only chat
    lines (`chat_v1.proto`).

## 3. Granters (one rule: the grant rides the commit that earns it)

### 3.1 Common rule

- An achievement is granted only inside the fenced character transaction that durably commits the
  earning event, with exactly one grant request per (event, key).
- **One witness, one call per transaction.** `FencedGrantingCharacter` stays neither `Clone` nor
  `Copy`, and the grant consumes it. ACH-COUNTER-1 adds `record_achievement_grants(tx, granter,
  requests)`, the batch form of the same function:
  - it consumes the one witness and checks it once against the transaction (`pg_current_xact_id`)
    and the `character_root` lock, exactly as the single grant does today;
  - it takes 1 to `ACHIEVEMENT0-RL-01` (64) requests of the same character, with no duplicate
    (source, key) pair, and processes them in ascending (source event id, key) order;
  - each request has the single grant's semantics (`Granted`, `AlreadyHeld`, `Retired`), and it
    returns one outcome per request in input order;
  - any `UnknownAchievement`, invalid input or storage error fails the whole call, and the granter
    drops its transaction uncommitted, as owner contract §3.3 requires;
  - `record_achievement_grant` becomes the one-request form of it, so the witness, the fence and
    the atomicity rule are unchanged, and every granter makes at most one grant call per
    transaction.
- **Source event.** It is `(kind, event_id)`, where `kind` is one of:
  - `oteryn:reward-claim` (existing);
  - `oteryn:quest-transition`;
  - `oteryn:encounter-outcome`;
  - `oteryn:counter-threshold`.

  `event_id` is the granter's own occurrence id (at most 64 bytes, hashed as the reward claim does
  when it is longer).
- **Never from live state.** Nothing grants from a live-only value, a script, a timer, an admin
  command or a background job. An event that is lost before its commit grants nothing, and the
  next real commit of the same event grants it.
- **Failures.** `UnknownAchievement` fails the whole earning transaction, as the owner contract §3.3
  says. Content validation prevents it. `Retired` and `AlreadyHeld` let the earning transaction
  continue.

### 3.2 Quest transitions (ACH-QUEST-1)

- **Content.** A quest transition record (QUEST-STATE-0 §4) gains an optional
  `achievement: <oteryn:achievement/...>`.
- **Grant.** When the transition commits, its transaction grants that key with source
  `(oteryn:quest-transition, H(character_id, transition_key, cause occurrence))`. A refused
  transition grants nothing.
- **What this covers.** It covers quest completion (a transition with `completes: true`) and every
  quest step. It is also the only route for a dialogue node or a map interaction that earns an
  achievement:
  - such a node requests a transition, so a granting interaction always has a fenced character
    commit (QUEST-STATE-0 §4 request causes);
  - an achievement earned by "doing something once" outside any quest gets a one-track quest of its
    own in content.
- **No other child.** No separate interaction `Achievement` child exists. A map interaction that
  commits no character transaction (WORLD-INTERACTION-0 §3.5) cannot grant, and content validation
  refuses it.

### 3.3 Encounter outcomes (ACH-ENC-1)

- **Binding.** An encounter manifest may bind an outcome name to the `achievement` consumer with
  one key (the `achievement_domain` block already authored in the Tirecz sample).
- **Grant on a durable earning commit.** For each credited CharacterId, the consumer commits one
  fenced transaction of that character that composes two writes:
  - **the occurrence receipt**: one immutable row in `game_character_encounter_outcome_receipts`
    (ACH-ENC-1): `character_id`, the outcome key `(encounter instance, occurrence sequence, action
    index)`, the outcome name, the boss death key when there is one, and the commit time; unique
    per (outcome key, `character_id`); never updated or deleted; `oteryn_game_runtime` SELECT and
    INSERT only;
  - **the grant**, with source `(oteryn:encounter-outcome, H(outcome key, character_id))`.

  The receipt is the earning event's durable commit (§3.1, and the `0021` invariant that a grant
  request rides its granting event's transaction and fence). A deferred guard refuses a grant with
  source `oteryn:encounter-outcome` whose transaction has not inserted the matching receipt. A
  replay finds the receipt and grants nothing new; a crash before the commit leaves neither. No
  asynchronous grant exception is defined.
- **Delivery.** It follows ENCOUNTER-RT-0 §6.4:
  - a reward-boss death outcome resumes from its death record after a restart;
  - every other outcome is at most once. Its achievement is lost with the fight on a crash, as the
    rest of that outcome is.
- **Offline or elsewhere.** A credited character who is offline or on another channel is granted
  through that character's own fence, the way BOSS-REWARD-1's per-character MINT is. This applies
  only to reward-boss outcomes, which have a durable death record. For other outcomes the grant is
  skipped for that character.

### 3.4 Counter thresholds (ACH-COUNTER-1)

- **The table.** A threshold is a content row `{counter, value, achievement}`:
  - `counter` is a closed key of a durable per-character counter that has an owner (D48: counters
    stay per character with the domain that counts);
  - `value` is the first value that earns.
- **Grant.** The counter owner's own durable commit that moves the counter from `before` to
  `after` grants, in that transaction, every threshold with `before < value <= after`, with source
  `(oteryn:counter-threshold, H(character_id, counter, value))`.
- **Several thresholds at once.** One receipt can move several counters past several thresholds
  (SKILLS-0 §3.4). The commit collects every crossed threshold of every counter it moves and grants
  them in one `record_achievement_grants` call (§3.1). Content validation keeps the thresholds one
  receipt can cross within `ACHIEVEMENT0-RL-01`; a receipt that would exceed it fails closed and
  commits nothing.
- **Live counts.** A counter checkpointed from live state (tries, kills) grants at the checkpoint
  or receipt that crosses the threshold, never at the live tick.
- **First counters:**
  - `level`, from the XP receipt;
  - the seven skills and `magic_level`, from build receipts.
- **Later counters.** Bestiary kills and Bosstiary kills join with their owners. Fish caught and
  items used join when an owner keeps a durable count. A threshold whose counter has no owner is
  refused by content validation and stays unearned (§4).
- **Lost levels.** A level or skill lost at death keeps the fact. A fact is write-once (D48).

### 3.5 What stays as it is

- The reward-claim grant and the chest `USE` path.
- QUEST-GATE-0 §5.6 ("an achievement with a claim").
- The owner contract §3 rules: absent key fails closed, retired is a no-op, first commit wins.

## 4. Coverage (ACH-COVERAGE-1)

- **The report.** The achievement validator emits a coverage report. It lists every catalogue key
  with no granter in any admitted content (claim, transition, encounter binding or threshold), with
  the reason taken from §2's families.
- **Unearnable keys.** Such a key is `UNEARNABLE_PENDING`. It is shown when earned, like any key,
  but it cannot be earned until content binds it.
- **No substitute.** No admin, script or migration grant substitutes for content. A support grant
  or a retroactive backfill each needs its own decision.

## 5. Notification (ACH-NOTIFY-1)

- **Carrier.** The notice is a `StateDelta` (FND-02 message type 9, `SERVER_SEQUENCED`) of a new
  state domain, not a new message type. The FND-02 registry gains, in ACH-NOTIFY-1's registry
  commit:
  - capability **8** `ACHIEVEMENT_NOTICES_V1` (owner: the Achievement domain module), with no
    command type and state domain 13; not offered before ACH-NOTIFY-1 ships;
  - state domain **13** `ACCOUNT_ACHIEVEMENT_NOTICES`, with delta type 1
    `ACHIEVEMENT_EARNED_DELTA_V1` and snapshot type 1 `ACHIEVEMENT_NOTICES_SNAPSHOT_V1`, both gated
    by capability 8.

  The numbers are the control plane's next free leases in STATE (cmd 14, domain 13, cap 8). This
  decision uses domain 13 and cap 8 and no command type.
- **The domain's state** is the account's achievement watermark: `fact_count` and `total_points`
  (the display contract's watermark).
- **Revision (FND-02 §15).** The domain revision is one cumulative, monotonic `uint64` per
  GameSession: 0 at the session's initial snapshot, plus 1 for each delta sent. It is never reset,
  reused or wound back, not at a resync snapshot and not at a reconnect within the same
  GameSession. Every snapshot carries the current revision, and the next delta's `base_revision`
  equals it. A new GameSession starts its own stream.
- **When.** After a granting transaction commits with `Granted`, and only then, the earning
  character's live session, if it selected capability 8, receives one delta for each `Granted`
  outcome, in the session's server sequence after the commit, under the session's current
  connection generation.
  - `AlreadyHeld` and `Retired` send nothing.
  - A grant committed for an offline character, or a session that did not select capability 8,
    sends nothing (§3.3).
- **Delta payload.** `{key, name, fact_count, total_points}`: `key` at most 160 bytes, `name` at
  most 64 UTF-8 bytes (the display contract's bounds), at most 250 bytes encoded. Its
  `base_revision` and `new_revision` follow FND-02 like every domain.
- **Snapshot payload.** `{fact_count, total_points}` only, at most 16 bytes. It names no
  achievement.
- **Reconnect and resync.** These follow FND-02 unchanged. A resync or reconnect sends the snapshot
  and never re-sends a delta, so a pop-up is shown once, from a delta applied live; a notice lost to
  a resync is not replayed. This is the "never replayed" rule, and it is consistent with resync
  because the domain's state is the watermark, which the snapshot restores in full. The fact is
  durable and unaffected, and the panel query (command 10) shows it.
- **Compatibility.** A client without capability 8 never receives domain 13 and still reads its
  facts through command 10. Unknown fields and over-bound payloads fail closed as in every domain.
- **Client.** The client shows the Tibia pop-up naming the achievement and the log line
  "Congratulations! You earned the achievement "<name>"." This is client presentation (Canary text
  as the hypothesis).
- **Secret records.** A secret achievement is announced the same way once earned (display contract
  D223).
- **Registry.** The capability, domain and type numbers above are final once this decision is
  accepted. ACH-NOTIFY-1 registers them with its `.proto` and passes protocol review.

## 6. Rejected options

- **An asynchronous grant consumer.** Rejected: owner contract §3 chose same-transaction
  consumption, and every granter here has a fenced commit.
- **A separate interaction `Achievement` child.** Rejected: a quest transition already gives an
  interaction a fenced character commit and a cause. A second path would duplicate fences.
- **Granting from live counters or ticks.** Rejected: a crash would leave a fact with no durable
  earning event.
- **Script-style `addAchievement` calls from runtime code.** Rejected: there is no script engine
  (WORLD-INTERACTION-0 §3.1), and it bypasses content validation.
- **A new message type for the notice.** Rejected: FND-02 reserves message types 1-255 to the
  foundation, and a non-sequenced type would bypass ordering and connection-generation fencing.
- **One grant call per threshold.** Rejected: the witness is consumed by the first call, and
  minting a witness per call would weaken the fence.
- **Reusing the chat wire for the notification.** Rejected: chat lines are player speech under
  CHAT-0 bounds and moderation, while an earned achievement is a server event.

## 7. Architect rulings (owner rule 5905825574)

- **R1. Interactions.** a) Only through a quest transition (recommended); b) their own child.
  **Ruled a).**
- **R2. Encounter grants for absent characters.**
  - a) Only for reward-boss outcomes, with a durable record (recommended);
  - b) for all outcomes, through a new durable queue.

  **Ruled a).**
- **R3. Notification.** a) A dedicated post-commit push (recommended); b) a chat line; c) none.
  **Ruled a).**
- **R4. Runtime owner.** The Character progression lane (close-out ruling). The Achievement domain
  module keeps the fact and catalogue.

## 8. Owner questions

None. Every choice above is a reversible architect ruling, or it follows from D48, D49 and the
owner contract.

## 9. Decision test

- **Must decide now?** YES. Quest, encounter and level content keeps naming achievements (the
  Annihilator and the other classic quests, and the Tirecz sample), and none of it can be wired
  without a granter rule.
- **Blocked:** ACH-QUEST-1, ACH-ENC-1, ACH-COUNTER-1, ACH-NOTIFY-1, and the achievement fields of
  quest and encounter content.
- **Harder later:**
  - the source-event kinds and their id hashes become durable provenance in `0021` rows;
  - the "grant rides the earning commit" rule binds every later granter.
- **Supersede if:**
  - a granter cannot share its earning transaction (owner contract §3: then an asynchronous
    consumer);
  - Reference evidence shows achievements granted in a way none of §3 covers;
  - measured write contention on `character_root` from threshold grants.
- **Deliberately not decided:**
  - support grants and retroactive backfill;
  - the Bestiary, Bosstiary, fishing and item-use counters (their owners);
  - client art and layout;
  - character-page showcase and rankings (Platform, Atlas);
  - any gameplay value of points.

## 10. Before-freeze checklist

1. **Contracts:** owner contract §3 and §5 and the display contract are unchanged. This decision
   only adds granters under them.
2. **Owned path only:** this file and its task record. No edit to DECISION_INDEX or to the
   contracts.
3. **Split work:** ACH-QUEST-1, ACH-ENC-1, ACH-COUNTER-1, ACH-NOTIFY-1, ACH-COVERAGE-1.
4. **Rows:** `ACHIEVEMENT0-RL-01` grant requests per call = 64.
5. **Wire:** capability 8 and state domain 13 (§5), registered by ACH-NOTIFY-1; the domain
   revision is cumulative per GameSession.
6. **Persistence:** the encounter outcome receipt (§3.3) is the one new table, written by
   ACH-ENC-1 in the grant's transaction.
