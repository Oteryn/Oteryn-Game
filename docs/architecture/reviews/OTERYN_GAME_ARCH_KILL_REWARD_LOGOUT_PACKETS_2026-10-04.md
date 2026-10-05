# ARCH-KILL-REWARD-LOGOUT-1: live kill rewards and the logout command

- Decision id: ARCH-KILL-REWARD-LOGOUT-1.
- Status: the §1 rulings and the §2 packets are accepted on merge. They allocate no number: the
  command type, capability and limit ids in §1.6 are proposals for the control plane to lease.
- Origin:
  - ATTACK-1b #1798: its task record names the follow-ups KILL-REWARD-COMP-1 and LOGOUT-WIRE-1.
    The live kill settlement was cut out of it under CP D655, after Codex 4179744870.
  - ATTACK-0 §4 (the in-fight logout block, `ATTACK0-RL-03`).
  - VSL-COMBAT-01 (`docs/architecture/VSL-COMBAT-01_MINIMAL_COMBAT_DEATH_LOOT_CONTRACT_CANDIDATE.md`)
    and D3 (`OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`, D3-2).
  - D3-7 (`OTERYN_GAME_D3_7_CORPSE_ITEM_ADMISSION_PACKET_2026-10-04.md`, corpse `i00005801`).
  - SPELL-LOCK-1 #1796: channel guards are released before durable I/O.
  - `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md` (a graceful logout creates no
    protection window) and CHAR-POSITION-0 §3.2 (the final position write in the terminal
    release).

## 0. Gaps and order

### 0.1 What is missing on `main` (and on the #1798 head)

- **No live caller settles a creature death.**
  - `settle_creature_death_rewards` and `settle_creature_death_rewards_with_bestiary`
    (`combat/death_reward.rs`) have only fixture callers
    (`tests/support/combat_death_reward_postgres_cases.rs`). `combat.rs` re-exports them under
    `allow(unused_imports, reason = "no production caller yet")`.
  - `top_damage_character` (`foundation/runtime_actor_carrier.rs`) is marked "D3-2 wires this
    into settle_creature_death_rewards".
- **Auto-attack projects the death and drops it.** In the #1798 `drain_auto_attacks`
  (`gameplay_transport/attack.rs`), a lethal hit runs
  `let _ = project_fixed_one_creature_death(...)` and clears the target. No loot, XP or
  Bestiary write follows.
- **A spell kill projects nothing.** `native_combat_cast.rs` discards the `CombatBatchReceipt`
  of `prepared.commit(...)`. A creature whose `EffectReceipt.health` reaches 0 is never
  projected as a death.
- **No content binds a creature to its rewards at runtime.**
  - Every `LootTableDefinition` in use is a test fixture.
  - The creature records in `content/creatures/definitions/*.json` carry
    `authoring.profile.experience`, `authoring.profile.details.corpse_item` and
    `definition.loot`, which is a `{family Loot, key oteryn:loot.creature.<slug>, revision}`
    reference. The loot records live in `content/loot/loot-*.json`.
  - The native gameplay pin (`content/native_gameplay.rs`) carries the creature profiles
    (`CreatureProfileRecord.profile`), and with them the experience and corpse item. It carries
    no loot table.
- **The settlement holds the runtime borrow across durable I/O.** Both settle functions take
  `owner: &mut CurrentOwnerCombatDeath<'_>` and await the loot, XP and Bestiary commits while
  holding it. `owner` borrows the locked `ChannelRuntimeV1`. Its only uses are reads:
  `projected_death`, `top_damage_character` and `reward_occurrence`. The
  `ComposedFreshAdmission.revision_sequencer` is documented as "never awaited while `runtime` is
  locked", and SPELL-LOCK-1 releases channel guards before durable I/O.
- **No logout command exists.** `docs/contracts/protocol-oteryn/v1` has no logout message, and
  neither the server nor the client has one. A session ends only through transport loss, then
  grace expiry (`TerminalRelease::Abandoned`) or a capability mismatch
  (`TerminalRelease::CapabilityMismatch`). #1798 adds the in-fight hold
  (`hold_while_in_fight`) to both paths.

### 0.2 Shared files

| File | Packets | Rule |
| --- | --- | --- |
| `apps/game-server/src/gameplay_transport/attack.rs` | KILL-REWARD-COMP-1: the lethal arm of the drain only | LOGOUT-WIRE-1 only reads `in_fight_until` |
| `apps/game-server/src/gameplay_transport/mod.rs` | KILL-REWARD-COMP-1: the settlement call after each drain, and the drain of the session's own queue before its terminal release. LOGOUT-WIRE-1: the `TerminalRelease::Logout` variant and its release path | the two packets touch disjoint functions; the second to merge merges `main` first and wires the §1.3 release handshake into the `Logout` path |
| `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json` | LOGOUT-WIRE-1: the command and capability rows. KILL-REWARD-COMP-1: the `KILLRW-RL-01` row | numbers leased by the control plane |

### 0.3 Order

- KILL-REWARD-COMP-1 runs after ATTACK-1b #1798 and D3-7 merge.
- LOGOUT-WIRE-1 runs after ATTACK-1b #1798 merges. It does not depend on KILL-REWARD-COMP-1 or
  on CHAR-POSITION-1.
- The two packets run in parallel, on the disjoint functions of §0.2.

## 1. Rulings

### 1.1 The reward binding is pinned with the native gameplay content

- The native gameplay manifest gains one optional pinned section, `loot_tables`, with schema
  `OTERYN_NATIVE_LOOT_TABLES/v1`. It is a bounded list of `{identity, algorithm, entries}`
  records, copied from `content/loot/loot-*.json`. Its digest is bound into the native gameplay
  pin like every other section.
- **Per-creature binding.** The section also holds one `creature_loot` row per pinned creature
  profile: `{creature, loot}`, where `creature` is the creature definition reference and `loot`
  is the creature's `definition.loot` reference copied as is, or an explicit `null` when the
  creature has no loot. A pinned creature with no `creature_loot` row, or a row for a creature
  that is not pinned, refuses the manifest at decode. The runtime never derives a loot key from
  the creature slug.
- The section holds exactly the tables that the `creature_loot` rows reference, and no others.
  The native gameplay manifest producer (`tools/content-schema/native-gameplay/`) writes it,
  and the node boot staging copies it like the other sections.
- An existing pin without the section stays valid: it decodes to no loot tables and no
  bindings (serde default), and every creature row is then `NoSettlement(no_loot_binding)`.
- At generation activation, a pure function builds one immutable `CreatureRewardTable` per
  active generation, keyed by the creature definition reference that the runtime already uses
  for its creature policies (`creature_policies`). Each row holds:
  - `xp_amount`: `profile.experience`, converted to `ExactI64`. A value above `i64::MAX` is
    refused when the row is built.
  - `corpse_item`: the `LootDefinitionRef` of `details.corpse_item`, resolved through the D3-7
    `i00005801` alias. It must be an admitted, materializable container Item with capacity of
    at least 1 and at most `GAMEITEM01-CORPSE-CONTAINER-ENTRIES` (16).
  - `loot`: the `LootTableDefinition` and its reference when the pinned binding names one;
    none when the pinned binding is `null`.
  - `race`: the Bestiary race from `content/project/bestiary.rs`, or none.
- **Fail closed per creature.** A missing or inadmissible corpse item, a missing binding row
  (`no_loot_binding`), a bound loot reference without its pinned table (`loot_table_missing`),
  or a table entry naming an Item that is not admitted and materializable
  gives the row `NoSettlement(reason)`. The generation still activates. A kill of that creature
  settles nothing and logs one `kill_reward_refused` event line with the reason and the
  creature key.
- `experience: 0` (or absent) settles loot with no XP descendant. A table that rolls no item
  still mints the corpse. A pinned `loot: null` binding mints the corpse with no loot; it is
  not a refusal.

### 1.2 Facts are read under the lock; durable writes run after it is released

- KILL-REWARD-COMP-1 changes the two settle functions so that they no longer take
  `&mut CurrentOwnerCombatDeath`. They take an owned `ProjectedCreatureDeathFacts` value:
  - `death` and `corpse`, from `projected_death(actor)`;
  - the reward principal (§1.3), from the new `top_damage_contributor(actor)`;
  - the XP and Bestiary occurrence bytes for the reward principal, from
    `reward_occurrence(actor, principal)`.
  These are the same owner reads the functions make today, made earlier at the same projection.
  `top_damage_contributor` returns what `top_damage_character` returns today plus the winning
  contributor's stored high-water identity, so the settled principal is the same Character.
  The descendants keep their bindings, so a replay of a fixture death keys exactly as before.
- The caller captures these facts in the same owner turn that projects the death, while the
  runtime lock is held. It then releases every channel guard (`runtime`, `spell_states`,
  `attack`) before any `RevisionSlot` is acquired or any durable call is awaited.
- `settle_*` callers acquire the slot with `revision_sequencer.acquire(character_id)` after the
  guards are released. The slot is held across the whole loot, XP and Bestiary chain of one
  death, as today.

### 1.3 One bounded settlement queue per Channel, drained by the principal's own session

- `drain_auto_attacks` runs in each session's serve loop, but it swings for every attacker in
  the Channel. A `ComposedFreshAdmission` holds the `ReconciledCharacterAuthority` of its own
  Character only, so it cannot settle another Character's kill.
- **Reward principal.** The principal is the top-damage contributor at the projection. With no
  tracked contributor, it is the attacker whose hit was lethal. This matches the current
  fallback in `settle_creature_death_rewards` and `COMBAT01-REWARD-PRINCIPALS` = 1.
- **Principal identity is captured under the lock.** The principal's identity is
  `(CharacterId, character_lease_generation, GameSessionId, ExactActorRef)`, all read in the
  owner turn that projects the death:
  - For the top-damage contributor, `runtime_actor_carrier.rs` gains the accessor
    `top_damage_contributor(actor)`. It returns the winner's `CharacterId` with the
    `(character_lease_generation, GameSessionId)` of its stored D141 high-water mark, and the
    `ExactActorRef` of the committed actor slot of that Character in the same carrier. A
    contributor whose slot is gone, or whose slot belongs to another session or lease
    generation, gives no principal: the death is projected with no entry and logs
    `reason=principal_gone`.
  - For the lethal-attacker fallback, the identity is the attacker's own, from its current
    command and fence.
  - Nothing looks the principal up by `CharacterId` after the guards are released. A successor
    session of the same Character never matches the captured session and lease generation.
- **Queue.** A lethal hit appends one `PendingKillSettlement` to a per-Channel queue: the
  principal identity above, the creature definition reference, the
  `ProjectedCreatureDeathFacts`, and the `DeathGroundContext`. The queue lives with the attack
  state behind the existing `attack` mutex.
- **One entry per death.** The queue is unique by the death key of the projected death. An
  append whose death key is already queued or in flight is a no-op and consumes no capacity.
  A death that was already settled may be appended again by a replay (§1.4). Its settle then
  finds every descendant committed and adds nothing, because the descendants are idempotent per
  `(death, character)`. The queue keeps no settled-death history.
- **Bound.** The queue holds at most `KILLRW-RL-01` = 64 entries per Channel, the same as
  `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE`. A death that finds the queue full is projected
  without a reward entry and logs `kill_reward_refused reason=queue_full`.
- **Loot MINT capacity.** `inflight_loot_mints_before_this_death` is the count of one-item loot
  MINTs actually in flight in that scope, from the loot plans of other settlements that are
  running now. It is read when the entry is taken from the queue. Queued entries and corpse
  MINTs are not counted, as `COMBAT01-INFLIGHT-LOOT-MINTS-PER-SCOPE` defines. A plan that the
  existing check refuses up front (`CAPACITY_EXCEEDED`) mints nothing. Its entry goes back to
  the queue and is retried on the next drain, so the refusal is backpressure, not a lost loot.
- **Drain.** After `drain_auto_attacks` and after each committed spell cast return, the session
  takes the entries whose captured `GameSessionId` and lease generation are its own, one at a
  time. It settles under its own `ReconciledCharacterAuthority`, whose fence carries the same
  lease generation. A
  settle that returns an unknown durable outcome is retried with the same facts on the next
  drain. The descendants are idempotent per `(death, character)`.
- **Session end: the release handshake.** A queued entry is never dropped by its own
  session's release. Every terminal release (`Abandoned`, `CapabilityMismatch`, and `Logout`
  from LOGOUT-WIRE-1) runs this handshake before its terminal release transaction, as it already
  saves familiar and spell training at actor end:
  1. **Drain.** The session takes and settles its own entries, as in the drain above, until a
     take finds none.
  2. **Seal.** In one owner turn holding the `attack` mutex, the session checks that the queue
     holds no entry and no in-flight settlement for its principal identity
     `(GameSessionId, character_lease_generation)`. If one exists, it releases the guard and
     returns to step 1. Otherwise it marks that identity `releasing` in the attack state, in
     the same turn. The check and the mark are one critical section, so no append can fall
     between them.
  3. **Attribution is kept.** The principal is always the D132 top-damage Character, captured
     at death by `top_damage_contributor(actor)`. The handshake never moves the reward to the
     next contributor. Every append runs in the projecting owner turn with the `attack` mutex
     held, so it sees the mark. An entry whose principal is marked `releasing` is not queued.
     It is parked under that identity in the attack state, with its D132 facts unchanged, and
     the projecting turn does not wait.
  4. **Abort before the transaction.** The seal is the last step before the terminal release
     transaction. Immediately before it sends the transaction, the release takes `attack`
     again. If an entry is parked for its identity, it clears the mark, moves the parked
     entries to the queue and returns to step 1. So a kill credited to the session before the
     transaction is sent is settled before `session_state = 3`.
  5. **Commit.** The terminal release transaction writes `session_state = 3`. An entry can be
     parked only while the transaction is in flight. Its outcome decides the entry:
     - **Committed.** The session ended when the transaction was sent: it reads no commands and
       its actor is fenced from the seal. A kill parked during the transaction is therefore a
       kill after the session ended, like a kill after its slot is removed. The entry logs
       `reason=principal_gone` with the D132 winner. No other Character is credited. The mark
       stays until the actor slot is removed, and no successor matches its session and lease
       generation.
     - **Retryable failure before commit.** An owner turn holding `attack` clears the mark and
       moves the parked entries to the queue. The resumed session settles them on its next
       drain, and its later kills enqueue as before.
     - **Unknown outcome.** The mark and the parked entries stay. The reconciliation (§1.6)
       reads the row. `session_state = 3` is handled as Committed. Any other state is handled as
       a retryable failure, and the `Abandoned` release then runs the handshake again.
  - Lock order: the projecting turns and the seal take the guards in one fixed order
    (`runtime`, then `spell_states`, then `attack`). The worker states it in `kill_reward.rs`
    and tests it. No guard is held across the terminal release transaction.
  - `kill_reward.rs` owns the seal and the clear. The packet that merges second wires them into
    the `Logout` path: KILL-REWARD-COMP-1 if LOGOUT-WIRE-1 is already on `main`, otherwise
    LOGOUT-WIRE-1 (§0.2).
  - An entry whose principal has no live session when it is taken (a node-local release that
    bypassed the handshake, which a test proves cannot happen) is logged
    `reason=principal_gone`, never settled under another authority.
- A node crash loses unsettled entries. The creature's death and health are runtime state, so no
  durable record is left half-written. Rejected alternative: §3.

### 1.4 Spell kills use the same path

- After `prepared.commit(...)` in `native_combat_cast.rs`, the caster's owner turn walks
  `CombatBatchReceipt.effects`. For each `EffectReceipt` whose `health` result is lethal on a
  creature target, it projects the death, captures the facts (§1.2) and appends a queue entry
  (§1.3), under the guards it already holds.
- **Replays recover.** A receipt with `applied = false` is an idempotent replay. It returns the
  retained lethal effects of the first commit, and it is walked like an applied one, so that a
  first attempt interrupted between the health commit and the projection or the append is
  repaired. The projection uses the existing idempotent replay
  (`committed_lethal_receipt_inner`, `project_committed_lethal_inner`), which returns the same
  death key. The append is deduplicated by that key (§1.3). A replay of a death that was
  already settled re-runs a settle that adds nothing. The receipt's `applied` flag is never
  used to skip the walk.
- **Delayed kills.** The same applies to the due and delayed spell paths that commit creature
  damage. `ordinary_combat::prepare_due` only builds the `OwnerCombatEffect`. The health commit
  and its `CombatBatchReceipt` happen in
  `spell_timer_callbacks.rs::apply_due_under_current_owners`, which returns
  `FireReport.receipts` while the `runtime` and `spell_states` guards are still held. That
  function walks each `FireReceipt.batch` like the cast path: project, capture the facts (§1.2)
  and append (§1.3), in the same owner turn, before it returns. This covers ordinary-chain and
  every other delayed creature kill, native owner effects included.
  - The worker lists each site. Any other site that commits creature health without a receipt
    the caller can read is reported as a `BLOCKER`, not rebuilt.
  - **Leases.** Neither SPELL-LOCK-1 #1796 nor ATTACK-1b #1798 changes
    `spell_timer_callbacks.rs`, so the file has no lease conflict. KILL-REWARD-COMP-1 already
    runs after #1798 merges (§0.3). If either PR adds a change to the file before it merges,
    KILL-REWARD-COMP-1 merges `main` after it and edits only the receipt walk.
- Familiar deaths (`familiar_cast_dispatch.rs`) and player deaths (`actor_spell.rs`,
  `apply_creature_damage`) are not creature kills and are unchanged.

### 1.5 Progression binding

- `RewardProgressionBinding` is composed from the active World's progression revisions (the
  `ProgressionRevisionContext`, policy and reward revisions and the finite policy). These are the
  same revisions the session's Character admission already pinned.
- The Bestiary binding uses the active `BestiaryProgressionBinding` of the same generation.
- A missing binding is a configuration error at activation, not at the kill. The session then
  settles loot only and logs `kill_reward_refused reason=no_progression_binding` for XP.

### 1.6 The logout command

- **Command** `LOGOUT_INTENT`, proposed command type 23, with an empty body. It is sent on the
  ordinary command path, under capability `LOGOUT_V1` (proposed id 19). A server that offers the
  capability answers every logout command with one `LogoutResultV1`:
  - `LOGOUT_RESULT_ACCEPTED` (1);
  - `LOGOUT_RESULT_IN_FIGHT` (2), with `retry_after_ms`, the remaining in-fight time from
    `in_fight_until`, rounded up and at most `ATTACK0-RL-03` (60,000);
  - `LOGOUT_RESULT_BUSY` (3), while the session has a pending durable spell commit or an
    unsettled kill entry of its own (§1.3), or when the terminal release returned a retryable
    failure before it committed (below). The client may retry.
  - 0 is invalid. Codes are append-only.
- **Rate.** At most one logout command is outstanding per session (`LOGOUT-RL-01` = 1). A second
  one before the first result is a protocol error, as for other single-flight commands.
- **Release, then accept.** When the checks pass, the server stops reading commands from the
  session and runs the terminal release as a new `TerminalRelease::Logout(transport)`. It runs
  at once, with no grace period, on the same path as `Abandoned`:
  - the actor-end saves (familiar, spell training) and the kill queue release handshake (§1.3);
  - the terminal release transaction, which writes `session_state = 3`. Once CHAR-POSITION-1
    merges, its final position write runs there, as CHAR-POSITION-0 §3.2 requires. LOGOUT-WIRE-1
    does not build it.
- **`ACCEPTED` is sent only after the terminal release has committed.** The server then closes
  the transport cleanly. The other outcomes:
  - **Retryable failure before commit.** Before it answers, the server reads the session's
    current durable state and runs `settle_unended`, as the existing release does on
    `NotApplicable`. That read lifts the `TransitionFence` through `FenceHolders`, so character
    writes are not left fenced:
    - `Lifted`: the session, actor and transport are unchanged. The handshake's `releasing`
      mark is cleared (§1.3), the server answers `BUSY` and resumes reading commands.
    - `Terminal`: the release had committed after all. The server retires the session, answers
      `ACCEPTED` and closes the transport.
    - The read fails: the fence is kept, and the server follows the unknown-outcome path below.
  - **Unknown outcome.** The server sends no result, closes the transport and keeps the fence.
    The session then enters `control_loss_lifecycle`. Today `commit_control_loss` returns
    `NotApplicable` for a row that is not `Active`, and the lifecycle releases only on
    `ResumedHistory`, so a committed release whose acknowledgement was lost would never retire
    the actor. LOGOUT-WIRE-1 adds an explicit terminal reconciliation:
    - `commit_control_loss` returns a new `ControlLossResult::Terminal` when its current read
      shows `session_state = 3`.
    - On `Terminal`, the lifecycle calls a new `reconcile_terminal(session)`. It reads the
      current row again. When the row is TERMINAL, it runs `retire_reconciled`: Premium release,
      `remove_terminal_session` on the exact actor, and `FenceHolders::forget`. Its result is
      `Released`. An unreadable row keeps the fence and is retried with the lifecycle's backoff,
      like `Unknown`.
    - A row in any other state is an ordinary loss of the transport, because the client never
      received `ACCEPTED`. The lifecycle records the loss as today. The handshake mark is
      cleared and the parked entries are queued (§1.3).
    No new column is needed. The client treats a close with no result as an ordinary connection
    loss, not as a logout.
- **No protection window.** A graceful logout is not an unexpected loss of control. The next
  login is an ordinary admission with no PvE re-entry protection interval
  (`DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`). This needs no logout marker. The
  committed release leaves `session_state = 3` (TERMINAL), which is absorbing. The recovery
  path already refuses to resume a terminal session, and PvE re-entry protection is granted
  only on such a resume (`current_player_reentry_protection`). So a committed logout cannot
  be resumed and grants no protection, and recovery does not need to tell a logout from an
  abandoned release. There is no migration and no migration lease.
- **Store release.** `release_abandoned_session` refuses a session that has no control-loss
  epoch, and a session that was never lost has none. So LOGOUT-WIRE-1 adds
  `release_logout_session(session, account_id, transport)` in `fresh_admission.rs`. It accepts
  an `Active` session on its exact current transport, with or without a control-loss epoch, and
  calls `release_current_claims`. A replay after the commit returns `Terminal`. Any other state
  returns `NotApplicable`.
- **In fight.** The refusal leaves the session and the actor unchanged. Closing the client
  instead keeps today's #1798 behaviour: the actor stays until the deadline ends.
- **PZ.** The accepted room has no protection-zone tiles. The PZ block and the 15-minute kill
  block belong to PARTY-PVP-0 and append their own result codes.
- **Client.** `apps/client` sends the command on the Tibia binding (Ctrl+L, Ctrl+Q). It shows the
  in-fight refusal as a status line with the remaining seconds, and on acceptance returns to the
  character list without a reconnect attempt.

## 2. Packets

### 2.1 KILL-REWARD-COMP-1 (live loot and XP settlement for a creature kill)

```yaml
task_id: OTV2-20261004-kill-reward-comp-1
decision: ARCH-KILL-REWARD-LOGOUT-1 §1.1-§1.5; VSL-COMBAT-01; D3-2
depends_on: [ATTACK-1b #1798, D3-7]
worker: oteryn-hard-worker
review: persistence (Codex), on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - apps/game-server/src/content/creature_reward.rs        # new: CreatureRewardTable (§1.1)
  - apps/game-server/src/content/creature_reward_tests.rs  # new
  - apps/game-server/src/content/native_gameplay.rs        # the optional loot_tables section and its pin only
  - apps/game-server/src/content/mod.rs                    # the module line only
  - tools/content-schema/native-gameplay/**                # the manifest producer: loot_tables and creature_loot, and its tests
  - tools/qualification/node_boot/**                       # the loot_tables staging only
  - apps/game-server/src/combat.rs                         # drop the "no production caller" allows that become used
  - apps/game-server/src/combat/death_reward.rs            # ProjectedCreatureDeathFacts (§1.2)
  - apps/game-server/src/foundation/runtime_actor_carrier.rs  # the top_damage_contributor accessor (§1.3) and the allow removal only
  - apps/game-server/src/gameplay_transport/kill_reward.rs    # new: the queue, the drain, the settle call
  - apps/game-server/src/gameplay_transport/kill_reward_tests.rs
  - apps/game-server/src/gameplay_transport/attack.rs      # the lethal arm of drain_auto_attacks only
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs  # the receipt walk after commit only
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs  # the due path receipt walk only, if it commits creature health
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks.rs  # the FireReport receipt walk in apply_due_under_current_owners, and its test module line only
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks_tests.rs  # new
  - apps/game-server/src/gameplay_transport/mod.rs         # module line, the drain after each drain or cast, the release handshake before every terminal release
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs  # the new settle signature
  - apps/game-server/tests/support/kill_reward_live_postgres_cases.rs     # new
  - apps/game-server/tests/combat_death_reward_postgres.rs                # registration only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # KILLRW-RL-01
  - docs/agents/tasks/archive/OTV2-20261004-kill-reward-comp-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - the combat_death_reward_postgres suite against PostgreSQL (repository CI service)
  - python3 -m unittest discover -s tools/content-schema/native-gameplay -p 'test_*.py'
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:**
  - the `loot_tables` pinned section with its `creature_loot` bindings, written by the native
    gameplay manifest producer and its tests, its staging from `content/loot`, and the
    `CreatureRewardTable` with its fail-closed rows (§1.1);
  - `ProjectedCreatureDeathFacts`, and the two settle functions taking it instead of the owner
    borrow (§1.2). The existing PG cases move to the new signature with their assertions
    unchanged;
  - the per-Channel queue, its `KILLRW-RL-01` bound, the principal rule, the session drain and
    the release handshake (§1.3);
  - the auto-attack lethal arm: project, capture the facts, enqueue, clear the target;
  - the spell receipt walks, on the cast path and in `apply_due_under_current_owners` (§1.4);
  - the progression and Bestiary bindings from the active World (§1.5);
  - removal of the `allow(unused...)` attributes that the live caller makes unnecessary.
- **Acceptance:**
  - A unit test per fail-closed reason of §1.1, one for `experience: 0`, and one that tells a
    pinned `loot: null` (corpse, no loot) from a bound table that is absent from the pin
    (`loot_table_missing`). A decode test refuses a pinned creature with no binding row.
  - A producer test that the native gameplay manifest carries the rat binding and the rat loot
    table, and nothing for a creature with `loot: null` beyond its `null` binding.
  - A carrier test that `top_damage_contributor` returns the winner's session, lease generation
    and actor ref, and gives no principal when the winner's slot belongs to a successor session.
  - A queue test that a second append of the same death key is a no-op, and that a replayed
    receipt (`applied = false`) after an interrupted first attempt enqueues the death once.
  - A capacity test: with 64 queued one-loot deaths, the first taken entry settles its loot
    (queued entries and corpses are not counted), and a plan refused for capacity stays queued.
  - A unit test that no `RevisionSlot` is acquired and no durable call is made while `runtime`,
    `spell_states` or `attack` is locked. It uses a test sequencer that fails if a channel guard
    is held.
  - A queue test at 64 entries and at 65 (the 65th death is projected with no entry and logs
    `queue_full`).
  - A live-path PG test (`kill_reward_live_postgres_cases.rs`) on a composed admission with the
    pinned rat profile and the rat loot table:
    1. An auto-attack kill mints the corpse `i00005801` and the rolled loot inside it, awards
       XP 5 once and records the Bestiary kill once.
    2. A spell kill (an area damage spell next to the creature) does the same.
    3. Re-running the drain with the same entry (an unknown outcome retried) adds no item, XP or
       kill.
    4. A kill whose principal is another session's Character is settled by that session, not by
       the attacker's.
    5. A session that logs out or is released with an unsettled entry of its own settles it
       before `session_state = 3`.
    6. A delayed kill (an ordinary-chain spell whose due step is lethal on the creature)
       settles once through the `apply_due_under_current_owners` receipt walk.
  - A delayed-path unit test (`spell_timer_callbacks_tests.rs`): a lethal due receipt appends
    one entry in the same owner turn, and a non-lethal one appends none.
  - Release handshake tests:
    - An append that lands after the session's last take and before its seal is found by the
      seal check, and is settled before `session_state = 3`.
    - A kill whose D132 winner is the releasing session, captured between the seal and the
      pre-transaction check: the entry is parked with that winner and the release returns to
      the drain. The entry is settled for the winner before `session_state = 3`, and a second
      contributor is never credited.
    - The same kill captured while the transaction is in flight: on commit it logs
      `principal_gone` with the D132 winner and credits no other Character. After the commit,
      no entry for the released identity is left in the queue or parked.
    - A retryable release failure clears the mark and moves a parked entry to the queue. The
      resumed session settles it, and its next kill enqueues for it.
    - An unknown outcome reconciled as non-terminal moves the parked entry to the queue; one
      reconciled as terminal logs `principal_gone`.
  - The loot roll is the existing `plan_creature_loot` with the death key. No new RNG.
- **Not in scope:** the loot window and corpse opening (D3-3), corpse decay (D3-4), party and
  multi-principal sharing, PvP kills, charm kill hooks beyond the existing Bestiary call, and a
  durable kill journal (§3).

### 2.2 LOGOUT-WIRE-1 (the logout command)

```yaml
task_id: OTV2-20261004-logout-wire-1
decision: ARCH-KILL-REWARD-LOGOUT-1 §1.6; ATTACK-0 §4; DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION
depends_on: [ATTACK-1b #1798]
worker: oteryn-hard-worker
review: protocol (Codex), on the frozen head
branch: allocated by the control plane
base: main
migration_lease: none
owned_paths:
  - docs/contracts/protocol-oteryn/v1/logout_v1.proto      # new: LogoutIntentV1, LogoutResultV1
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # the command and capability rows, as leased
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # LOGOUT-RL-01
  - crates/protocol-oteryn/src/logout.rs                   # new codec
  - crates/protocol-oteryn/src/logout_tests.rs             # new
  - crates/protocol-oteryn/src/lib.rs                      # the module line only
  - crates/session/src/lib.rs                              # the logout command and result only
  - apps/game-server/src/gameplay_transport/logout.rs      # new: the command handler and dispositions
  - apps/game-server/src/gameplay_transport/logout_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs         # module line, TerminalRelease::Logout and its release path, the Terminal arm of commit_control_loss and control_loss_lifecycle, and reconcile_terminal only
  - apps/game-server/src/gameplay_transport/connection.rs  # the command dispatch arm and ControlLossResult::Terminal only
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/durability/fresh_admission.rs     # release_logout_session only (no migration)
  - apps/client/src/input.rs                               # the Ctrl+L and Ctrl+Q binding
  - apps/client/src/**                                     # the refusal status line and the return to the character list only
  - docs/agents/tasks/archive/OTV2-20261004-logout-wire-1.md
validation:
  - cargo fmt --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-session
  - cargo test --locked -p oteryn-game-server
  - cargo test --locked -p oteryn-client
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Builds:**
  - the schema, the strict codec and the registry rows (§1.6), with the result codes
    append-only;
  - the server handler: the in-fight refusal with `retry_after_ms`, the busy refusal, the
    single-flight rule, and on acceptance the result, the end of command reading and
    `TerminalRelease::Logout`;
  - `release_logout_session` (§1.6), the store release for the exact current transport, with
    no migration: a committed logout is `session_state = 3`;
  - on a retryable failure, the current durable read and `settle_unended` before `BUSY`;
  - `ControlLossResult::Terminal` and `reconcile_terminal`, which retire a session whose
    terminal commit was not acknowledged;
  - the session crate command and result, and the client binding and its two presentations.
- **Acceptance:**
  - A codec test per result code, and refusals for code 0 and a non-empty intent body.
  - A server test of a logout while in fight: `IN_FIGHT`, `retry_after_ms` within 1 ms of the
    remaining deadline, the actor still in the Channel and the session still reading commands.
  - A server test of a logout after the deadline: `ACCEPTED`, `session_state = 3`, the transport
    closed, and no grace period. The test asserts that `ACCEPTED` is written only after the
    terminal release transaction has committed.
  - A server test with a retryable release failure: `BUSY`, the `TransitionFence` lifted, the
    session still reading commands, and a character write (a movement step or a damage commit)
    that succeeds after the `BUSY`.
  - A server test with an unknown outcome: no result and the transport closed. The session is
    reconciled from the row on the `Abandoned` path, with one case where the release committed
    (`session_state = 3`, retired) and one where it did not (an ordinary transport loss).
  - A server test of a committed release whose acknowledgement is lost: the transport closes
    with no result, `control_loss_lifecycle` gets `Terminal`, and `reconcile_terminal` removes
    the exact actor slot and stops Premium. A variant where the reconciliation read fails first
    keeps the fence and retires the actor on the retry.
  - Store tests for `release_logout_session`: a session with no control-loss epoch is
    released, a wrong transport returns `NotApplicable`, and a replay returns `Terminal`.
  - A server test that a login after an accepted logout is an ordinary admission with no
    protection interval, and that the recovery path refuses to resume the ended session.
  - A server test of `BUSY` with a pending durable spell commit.
  - A second logout command before the first result is a protocol error.
  - A capability test: without `LOGOUT_V1`, the command is refused as an unknown command.
- **Not in scope:** the CHAR-POSITION-1 position write, the PZ and PvP kill blocks
  (PARTY-PVP-0), a server-initiated kick, and a logout during a pending respawn (DEATH-2).

## 3. Rejected options

- **Settle inside the drain, holding the runtime lock.** This awaits durable writes while
  `runtime` is locked, stalls every other session of the Channel for each kill, and breaks the
  sequencer rule and SPELL-LOCK-1.
- **Acquire every possible principal's slot before the drain.** The drain does not know which
  swings will be lethal, and it may cover many Characters. Taking all their slots would
  serialize unrelated Characters on every tick.
- **Let the attacker's session settle another Character's kill.** That session holds no
  authority for the other Character and cannot fence its writes.
- **A durable kill journal.** It would add a migration and a replay path for state whose source,
  the creature's death, is itself runtime state. A crash loses the creature and its pending
  reward together. This can be revisited if a measured loss rate or a player-facing promise
  needs it.
- **Read loot tables from the full content project at runtime.** The live node runs from the
  pinned native gameplay content. A second unpinned source would let rewards drift from the
  pinned digest.
- **Treat a missing loot table as an empty table.** That would silently mint a corpse with no
  loot for a content error. Fail closed makes the error visible.
- **Logout as transport close.** A close cannot carry a refusal, so the client cannot learn it is
  in fight. It also cannot be told apart from an unexpected loss, which would wrongly grant the
  protection window.
- **Queue the logout until the in-fight deadline ends.** The ATTACK-0 §4 rule is a refusal. A
  queued logout would also hide the remaining time from the player.

## 4. Decision test

1. **Must it be decided now?** Yes. ATTACK-1b makes creatures killable live, and every kill
   until KILL-REWARD-COMP-1 merges gives nothing. The in-fight block has no player-facing
   command until LOGOUT-WIRE-1.
2. **What is blocked?** Live loot and XP, the corpse loot window (D3-3) and decay (D3-4), and a
   clean logout for the client.
3. **What becomes harder later?**
   - The `loot_tables` section is part of the native gameplay pin format.
   - The single-principal queue is replaced by party sharing when PARTY-PVP-0 needs it.
   - The logout result codes are wire codes and can only be appended.
4. **What would justify superseding it?**
   - A measured loss of settlements from crashes or from `KILLRW-RL-01`.
   - A shared reward or party rule.
   - A content pipeline that pins loot outside the native gameplay manifest.
5. **What is deliberately not decided?** The loot window, decay, party loot, PvP kills, the PZ
   logout block, and the final position write (CHAR-POSITION-1).
