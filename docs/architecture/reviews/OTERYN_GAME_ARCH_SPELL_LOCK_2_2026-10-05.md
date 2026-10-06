# ARCH-SPELL-LOCK-2-0: native spell casts without Channel guards across durable I/O

- Decision: `ARCH-SPELL-LOCK-2-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the two packets. No durable
  value, wire format, identity or public contract changes. The rulings fix an internal concurrency
  invariant of the Channel owner.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - Codex finding 4178405815 on #1534 (P1): Channel guards are held across durable combat I/O.
  - The SPELL-LOCK-1 BLOCKER (`docs/agents/tasks/archive/SPELL-LOCK-1.md`, Scope (1)). It moved
    the finding to SPELL-LOCK-2, "which starts with a design proposal".
- Builds on:
  - the existing spell owner reservation (`pending_owner`, `reserve_spell_batch`,
    `release_definitely_uncommitted_spell_batch`);
  - the slot-compare fence `validate_staged_spell_batch`;
  - each writer's retained retry path, with its AlreadyCommitted and `Applied` branches;
  - the Channel item advisory lock (key 33) and the guard order of ARCH-KILL-REWARD-LOGOUT-1 §1.2.
- Runtime, persistence and production authority: NONE. Each packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **Today.** The native cast pass (`native_combat_cast.rs` ~2613-2957) holds `runtime`,
   `spell_states` and `door` from the replay check to the install. Held across it are about
   30 awaits:
   - `begin`, the authority step with its `LOCK TABLE` and key-33 locks, and the fact loads;
   - the rune reserve, the prepare reads and the `read_tile` loops;
   - the item apply, the parameter write, the build write and `COMMIT`;
   - a second post-commit transaction.
   Every other session of the Channel waits for each cast's database round trips.
2. **Why it is not a small change.**
   - `ChannelRuntimeV1` has no revision a re-lock could check.
   - Prepare interleaves transaction reads with runtime reads.
   - SPELL-LOCK-1 stopped on both.
3. **Rulings (§1).**
   - The cast linearizes at one stage section S. In 2a, S may still contain today's prepare
     awaits, under the guards; 2b makes it await-free (§1.1).
   - A per-Channel **spell lane** mirrors key 33 in memory.
   - The existing slot reservation becomes complete: every mutator of a reserved slot honours it.
   - The caster stays visibly pending for the whole pass.
   - No revision counter is added.
4. **SPELL-LOCK-2a (hard worker, §2.1). Commit side.**
   - The lane is added. Every key-33 acquirer takes a lane permit, so the compiler finds every
     caller. The acquirers are the Rust functions that lock key 33 and the item writers whose
     rows fire the three key-33 triggers (§1.2).
   - Guards are released after S and re-taken to install or release.
   - For every key-33 writer that commits, an unknown `COMMIT` outcome, or a failure after a
     successful `COMMIT` and before the install, keeps the lane fenced until it is resolved (§1.6).
   - The unchecked mutators of a reserved slot are closed.
   - The caster stays pending throughout.
5. **SPELL-LOCK-2b (hard worker, §2.2). Read side. After 2a.**
   - The database reads before S run without guards, as a prefetch.
   - S evaluates prepare on the prefetch and the live runtime.
   - A plan miss retries a bounded number of times, then refuses with no side effect.
6. Other spell writers keep holding guards across their own transactions. That includes the
   world-item, parameter and familiar casts, timers, field step, familiar defence and party
   owner. 2a makes them take the lane first, and makes every writer that reaches
   `commit_spell_owner_transaction` hold the commit window (§1.6). Releasing their guards is later
   work that follows the same pattern.

## 1. Rulings

### 1.1 Linearization at the stage section S

A native cast takes effect at one critical section S, under `runtime`, `spell_states` and
`door`. S contains:

- every runtime and spell-state read that the outcome depends on: prepare, the `prepared.before`
  check, the post-stage tile, physical, field-policy and restriction reads, and the
  `validate_new_write` verdict;
- `stage_installation`, which includes `reserve_spell_batch`.

Facts from the database are valid in S only when they are read in the cast's transaction under
locks it holds until `COMMIT`:

- the authority step's admission-relation locks and key-33 lock;
- the tile rows read `FOR UPDATE`;
- the lane (§1.2).

`apply_spell_items_in_transaction_guarded` takes the verdict that S computed. It does not take a
closure that re-reads the runtime. The verdict stays valid because the reservation (§1.3) and the
locks above hold from S to the install.

**What S is in each packet.**

- **2a.** S is the guarded span from the pass start (the lane first, then the guards) through
  `stage_installation`. It contains today's prepare awaits: `begin`, the authority step, the
  fact loads, the rune reserve and the tile reads. That is correct because the guards are held
  for the whole span, so no runtime read in it can go stale before the reservation, and the
  database reads are under the locks listed above. The guards are released only after S, so 2a
  is correct on its own. What 2a removes is every guard across `COMMIT`, the post-commit
  transaction and every write after S.
- **2b.** S becomes await-free: the database reads move before S as a prefetch (§1.5). That is a
  latency property. It does not change why the outcome is correct.

### 1.2 The spell lane

- Each Channel owner holds one async mutex, the spell lane. It is the in-memory mirror of the
  Channel item advisory lock (key 33).
- Every in-process caller that takes key 33 takes the lane first and holds it from before
  `begin` to after its install or release.
- **Enforced at the shared boundary.** Every function that takes key 33, directly or through a
  trigger, requires a `&SpellLanePermit` for the same World and Channel. The sweep of `main`
  (`aa30141f`) for every key-33 acquirer, Rust and SQL, found:
  - **Direct locks in Rust:**
    - `assert_spell_item_scope_with_recovery`;
    - `assert_spell_item_authority_with_recovery`;
    - `native_map_items::initialize_in_transaction`.

    The `pub(crate)` `_in_transaction` wrappers delegate to the first two.
  - **SQL triggers that take key 33:**
    - `game_item_ground_owner_lock` on `game_item_ground_locations` (0034, replaced in 0049);
    - `game_spell_corpse_entry_owner_lock` on `game_item_corpse_container_entries` (0042). It
      takes the Channel of the corpse's Ground root;
    - `game_native_map_adoption_stamp` on `game_native_map_scope_adoptions` (0049).

    No other SQL function or trigger takes key 33. No foreign key cascades into these tables.
  - **Rust writers of those tables.** Each runs the trigger, so each is a key-33 acquirer:
    - already behind the boundary: the appliers in `spell_item_transaction.rs`, the deadline
      drains of `spell_item_temporal.rs` through `drain_spell_item_deadlines`, and
      `native_map_items::adopt_current_ground`;
    - not yet behind it, so 2a puts them there:
      - `item_transfer.rs` `apply_transfer` (Ground and corpse-entry DELETE), under
        `commit_item_transfer`;
      - `item_mint.rs` `insert_mint_with_corpse_attribution` (Ground INSERT) under
        `commit_item_mint` and `commit_corpse_mint`, and `insert_corpse_loot_mint` (corpse-entry
        INSERT) under `commit_corpse_loot_mint`;
      - `map_item_mint.rs` `apply_mint` (Ground INSERT), under `commit_map_item_mint`;
      - `item_decay_retire.rs` `apply_retire` (Ground and corpse-entry DELETE), under
        `commit_decay_retire` and `retire_decayed_corpse`.

  The only constructor of the permit acquires the lane. The permit's scope must match the
  Channel of every row the writer locks or writes: the Ground row's Channel, or the Channel of a
  corpse entry's Ground root. A mismatch is refused with no side effect, before the first
  statement where the scope is known from the request, and otherwise before the write that fires
  the trigger, with the transaction rolled back. One transaction holds one permit, so a writer
  whose rows span two Channels is refused. So a caller cannot reach key 33 without the lane, and
  the compiler lists every caller, present and future.

  The compiler sees only Rust signatures. Two 2a guard tests close the gap to SQL (§2.1, Tests):
  a PostgreSQL test pins the set of functions that take key 33 to the three triggers, and a
  source scan pins the set of production functions that lock key 33 or write those three tables,
  each of which must take a permit. A new trigger or writer fails one of them until it is put
  behind the boundary.
- **DB-only item writers.** The item transfer, mint, map mint and decay retire writers install
  nothing into the Channel runtime. They hold the permit from before `begin` to the end of their
  transaction, park nothing in `unresolved`, and resolve an unknown outcome through their own
  receipt path, as today. They take the lane like any other acquirer, so they wait while an
  attempt is parked in `unresolved` and run only after it is resolved (§1.6). A packet that makes
  one of them install runtime state after its `COMMIT` makes it a committing writer, with a
  commit window, a variant and a resolver (§1.6), in that packet.
- The callers on `main` (`aa30141f`), through the public functions above them, are:
  - the native, world-item, parameter and familiar casts;
  - `source_item_cycle.rs`: map-item initialisation and the spell item deadline drain, with its
    timer callbacks;
  - field step ingress, source floor change and the periodic standing-tile read;
  - familiar defence, the qualification wild spawn, and the party spell owner with its World
    party drains and presence refresh;
  - the item writers. None of them has a production caller on `main` yet. Their callers are
    `combat/pickup.rs` (`settle_ground_pickup`, `settle_corpse_pickup`, through `settle_pickup`),
    `combat/death_reward.rs` (`settle_loot`, through `commit_corpse_mint` and
    `commit_corpse_loot_mint`), the `let _ =` references in `durability/mod.rs`, and the
    PostgreSQL test support. 2a adds the permit parameter to `settle_pickup` and `settle_loot`
    and their entry points, and makes the test support acquire a lane. The corpse take of
    ITEM-MOVE-1 (`take_corpse_entry`, whose default in `connection.rs` rejects) is not changed:
    whichever packet implements it must acquire the lane to reach `settle_corpse_pickup`, under
    the lock order below.
- Since key 33 already serializes these writers in the database, the lane adds no new
  serialization. What it changes: no key-33 writer can run between any writer's `COMMIT` and its
  install. So no writer sees committed items that the runtime does not yet show.
- Lock order: lane, then `runtime`, then `spell_states`, then `door`, then `attack`. The lane is
  never acquired while a Channel guard is held.

### 1.3 The reservation is complete

From S until the install or release, a slot with `pending_owner` set changes only through:

- its own batch's install (`commit_spell_batch`); or
- its release (`release_definitely_uncommitted_spell_batch`, `rollback_physical`).

Every other mutator of a reserved slot either refuses retryably or defers to a later turn, as
`assert_slot_spell_unreserved` callers already do. The mutators that do not check today are:

- `commit_actor_condition`;
- the attacker lease bind, fence and lift, and the session rebind;
- `bind_continuation` and `mint_transition_fence`;
- the companion spawn reserve, rollback and install;
- `bind_semantic_creation`, `install_companion_policies` and
  `realize_native_qualification_spawn`;
- the fresh session reserve and commit;
- the attacker's own slot in the damage path.

For each one, 2a either adds the check or proves that the mutator cannot reach a reserved slot.
The install keeps `validate_staged_spell_batch` unchanged: whole-slot equality is now a guarantee
that is tested, not a race.

### 1.4 The caster stays pending

The pass keeps the cast visible to `has_pending_spell_commit` from the start of the pass to the
`*pending = None` that ends it.

- Today the cast is removed from `pending_native` at the pass start and pushed back at the end,
  so for the caster's own pending check the flag reads false during the pass.
- 2a keeps the cast visible through a marker, so the periodic, premium, movement-equipment,
  field-step and party checks defer for the caster while the guards are released.
- The pass ends as today only when it installs the batch or releases it as proven uncommitted.
  Every other exit after the `COMMIT` call goes to the lane's `unresolved` record (§1.6), not back
  into `pending_native`, and the marker stays. That covers `CommitOutcomeUnknown` and every
  failure after a known-successful `COMMIT` and before `prepared.commit`.
- The same holds for the other committing writers' lists: `pending_world_items`,
  `pending_parameters` and `pending_familiars`. An attempt parked in `unresolved` leaves only
  its marker in its list, so `has_pending_spell_commit` still reads true for its caster.
- The marker holds the attempt's identity only: actor, session, `CommandId` and the original
  intent. It holds no install field. Every reader that matched a pending attempt by those
  fields reads them from the marker instead. For the familiar cast these are in
  `familiar_cast_dispatch.rs`:
  - the dispatch result `Pending` for the caster;
  - the control-loss reconcile, which recovers the original command and intent;
  - the refusal of a competing command for the same caster;
  - `original_retained` for the retry of the original command.
  Those readers see the same caster state as today. A retry or control-loss reconcile of a
  parked attempt resolves it through the lane (§1.6), never by preparing it again.
- The replay fast path is unchanged.

### 1.5 Prefetch before S (2b)

1. Each prepare family builds a read plan from a short guarded read of the runtime: the caster,
   targets, positions and tiles.
   - The families are: ordinary, companion item, world item, and native source-self and
     from-owners.
   - For a `SpellTarget::AttackTarget` cast, the plan also records the held attack target. It
     is read from the separate `attack` state (`combat_state(..).target`), not from the runtime,
     and `attack` is taken last in the lock order (§1.2).
2. The guards are released. In the transaction, the pass runs `begin`, the authority step, the
   database part of the owned facts, the rune reserve and the planned tile reads.
3. S re-reads the runtime and, last in the lock order, the held attack target from `attack`. It
   first checks that the current plan is covered by the prefetch. A held target that differs
   from the plan's target, or is gone, is a miss. Prepare receives the target S read, never the
   prefetched one. Only then does S run the mutating steps:
   - `apply_current_premium`, the monk `tick_with_world` and `issue_owner_work`;
   - the presentation reservation;
   - prepare.
4. If S finds a miss, it releases the guards with no side effect, fetches the missing rows in the
   same transaction and retries.
5. After two retries the cast is refused with no reservation and no in-memory change, and the
   transaction is rolled back. The outcome is an existing one: the worker names it and adds no
   protocol result code.
6. Prepare reads the database only through the prefetch view. A request outside the prefetch is
   a refusal, never an await.

In 2a the held target is read inside S, as today, so no stale read can occur. The re-read is
needed only once 2b releases the guards before S.

### 1.6 Unknown outcome and restart

- The reservation is kept on `CommitOutcomeUnknown`, as today. The retry re-runs the retained
  attempt, when the client resends the same command or when control loss reconciles
  (`reconcile_pending_native_for_control_loss` for a native cast). It takes the AlreadyCommitted
  branch when the batch committed, and the `Applied` branch, which commits again, when it did not
  (`native_combat_cast.rs`, `world_item_cast.rs`, `parameter_cast.rs`; the familiar cast's
  `commit_familiar_spell` or `reconcile_familiar_spell` by `allow_new_mutation`).
  `commit_semantic_transaction` (`db.rs`) returns `CommitOutcomeUnknown` without sending `COMMIT`
  when the deadline has already elapsed, so an unknown outcome is often a rollback.
- **The fence covers every committing writer.** The writers that reach
  `commit_spell_owner_transaction` on `main` are:
  - the native cast (`native_combat_cast.rs`);
  - the world-item cast (`world_item_cast.rs`, and `WorldItemSpellCommit` in
    `spell_item_transaction.rs`);
  - the parameter cast (`parameter_cast.rs`, two calls);
  - the familiar cast (`familiar_cast.rs`, through `FamiliarSpellCommit` in
    `character_familiar.rs`).

  `commit_spell_owner_transaction` takes the permit's commit window as a parameter. So the
  compiler lists every committing caller, present and future, and none can `COMMIT` outside a
  window.
- **The lane stays fenced from `COMMIT` to install.** A `COMMIT` that succeeded, or may have
  succeeded, without an install opens the same window that §1.2 closes. Two kinds of exit leave
  it open, for every committing writer:
  - `CommitOutcomeUnknown`;
  - a known-successful `COMMIT` followed by a failure before the install. Today these failures
    are:
    - native: the training receipt handling, the post-commit transaction, the authority and
      reconnect checks, and the fresh owned-fact loads;
    - world-item and parameter: training `after_commit`, `prepare_install`, `rebind_training` and
      `commit_owner_batch`. Today they push the attempt back into `pending_world_items` or
      `pending_parameters` and return `Pending`;
    - familiar: every fallible step between its `COMMIT` and its install.

  The lane records both kinds the same way:
  - From the `COMMIT` call on, the permit owns the attempt in a commit window. The window can be
    consumed in only three ways: the install, the release of a batch proven uncommitted, or the
    move into `unresolved`. If the permit is dropped while the window is still held, `Drop`
    moves the attempt into `unresolved`. So an early return or a `?` after the `COMMIT` cannot
    skip the fence.
  - The lane's own state (inside the lane mutex, not in `spell_states`) holds
    `unresolved: Option<UnresolvedSpellCommit>`. It is an enum with one variant per committing
    writer, and each variant owns that writer's complete retained attempt, moved in whole:
    - `Native(PendingNativeCast)`: `prepared`, `owned`, `request`, `fence`, the reserved batch
      and every other field;
    - `WorldItem(PreparedWorldItemCast)`;
    - `Parameter(PreparedParameterCast)`;
    - `Familiar(PreparedFamiliarCast)`.

    Nothing in it is reconstructed, as each writer already requires for its retry.
  - On either kind of exit the pass moves the attempt into `unresolved` before it drops its
    permit. Dropping the permit releases the mutex, but not the fence. From then on the lane is
    the only owner of the attempt. The caster's pending list keeps only the §1.4 marker, which
    names the `CommandId`, so the caster stays visibly pending. No acquirer can win the lane while
    the attempt is still being republished, because it is moved before the permit is released.
  - Acquiring the lane returns either a permit or, while `unresolved` is set, an
    `UnresolvedLane`. Only one function turns an `UnresolvedLane` into a permit:
    `resolve_unresolved_spell_commit`. It dispatches on the variant to that writer's resolver,
    which lives next to the writer and resumes that writer's retained retry path. The compiler
    therefore makes every key-33 caller resolve first, and a new committing writer cannot
    compile without a variant and a resolver.
  - **Resolution resumes the full retained retry path.** Resolution holds the lane already, so
    it follows the lock order of §1.2 with no Channel guard taken first. The resolver runs, for
    the recorded attempt, the same path the caster's own retry of the original command runs
    today, from the retained attempt and under key 33:
    - in the mode that path computes from current facts: the familiar cast's
      `allow_new_mutation`, or the native control-loss mode when the caster has lost control,
      as `reconcile_pending_native_for_control_loss` does;
    - with a fresh deadline, as each retry has today;
    - with the retry's inputs derived for the attempt's actor and session the way the
      dispatcher derives them today (`refresh_spell_access`), never taken from the acquirer's
      own request.

    What the path finds decides the outcome:
    - Committed batch: the AlreadyCommitted branch runs the remaining post-commit steps, and
      the resolver takes the guards and installs.
    - No committed receipt, including the deadline case above: the `Applied` branch commits the
      retained attempt again inside a new commit window, as the caster's retry does. The
      resolver never just releases the attempt and never clears the marker without a result.
    - A definite rejection that the path itself proves, as today
      (`definite_familiar_rejection`, a rejected item verdict,
      `release_definitely_uncommitted_spell_batch`, `rollback_physical`): the resolver releases
      the attempt and returns that rejection. A retained training checkpoint changes only
      through `refresh_training_checkpoint_revision`, as on the retry.
    - Unknown again, or a post-commit step fails again: the window parks the attempt in
      `unresolved` again.

    On success the resolver replaces the caster's marker with the same result the caster's own
    retry would return, then clears `unresolved`. The result therefore does not depend on which
    acquirer resolved the attempt.
  - When the outcome is still unknown, or a post-commit step still fails, `unresolved` stays
    set. The acquirer's own work is refused retryably, with no side effect.
- If 2a finds a committing writer whose retained retry path cannot be resumed this way (an input
  only the caster's request carries, or no AlreadyCommitted branch that can finish its install),
  the worker returns a BLOCKER naming it. It does not invent one.
- **A parked attempt is always complete.** Today the world-item and parameter casts take `paid`,
  `player` and `physical` out of the attempt before later fallible steps
  (`world_item_cast.rs` after `COMMIT`, `parameter_cast.rs` likewise). A failure there would park
  an attempt the resolver can never finish, and the Channel's key-33 writers would stay fenced
  for good. So in 2a:
  - Each `UnresolvedSpellCommit` variant holds the install state by value, not as an `Option`
    that can be taken. An attempt with a field already taken cannot be built into a variant, so
    it cannot be parked.
  - Each writer's post-commit install runs in two phases:
    1. a fallible phase that only borrows the retained attempt. It runs the checking forms of
       training `after_commit`, `prepare_install`, `rebind_training` and `commit_owner_batch`,
       and of the native receipt, transaction, authority, reconnect and owned-fact steps. It
       consumes nothing and produces a checked install plan;
    2. an infallible phase that moves `paid`, `player`, `physical` and `presentation` (or the
       native `prepared` and `owned`) out of the attempt and applies the plan.
  - A failure in phase 1 parks the attempt unchanged. Phase 2 cannot fail, so no exit leaves a
    half-consumed attempt.
  - The shared install helpers are split the same way, because today the moved fields go into
    helpers that can still fail. `commit_owner_batch` (`actor_spell_commit.rs`) receives the moved
    `StagedSpellBatch` and `PlayerBatchPreflight`, then validates, binds and runs the physical
    commit fallibly, and `PlayerBatchPreflight::install` is private. 2a splits:
    - `commit_owner_batch` into a check that borrows the staged batch and the preflight
      (`validate_current`, `matches_batch`, the binding, `validate_staged_spell_batch`) and returns
      a checked token only it can build, and an infallible install that moves both, writes the
      runtime slots and installs the player states. `commit_owner_batch` stays as the
      composition of the two for callers that hold no retained attempt;
    - `ChannelRuntimeV1::commit_spell_batch` and `release_companion_touches_for_source_commit`
      (`runtime_actor_spell.rs`) into their existing borrowing checks and infallible writes, so
      no runtime slot or companion touch changes before a later phase-1 check fails;
    - `PreparedPlayerTraining::prepare_install`, which takes `installation` today, and
      `PlayerSpellState::rebind_staged_training` (`mana_training.rs`), with
      `PlayerBatchPreflight::rebind_training`, into borrowing checks and moves;
    - `commit_familiar` (`companion_lifecycle.rs`), which consumes `PreparedFamiliar` and can
      fail, into a borrowing check that also yields the `FamiliarApplyReceipt` the apply will
      return, and an infallible apply;
    - `FamiliarTimerInstallPreflight::finalize` (`delayed_execution.rs`), checked in phase 1
      against that receipt.

    Phase 2 keeps an `expect` only on an invariant that phase 1 proved under the same guards,
    and adds no check of its own. The direct and corpse companion installs, the presentation
    install and the timer install already follow their checks infallibly and stay as they are.
  - If a step cannot be split this way, the worker returns a BLOCKER naming it. It does not take
    a field before a fallible step.
  - A resolver failure that a retry cannot fix (for example, the committed batch contradicts the
    retained attempt) keeps `unresolved` set. The lane stays fenced, which fails closed, and the
    failure is logged with its error code. A Channel reload from durable truth clears it, as for
    a restart below.
- The caster's own retry and the control-loss reconcile are ordinary lane acquirers. They take
  the attempt from `unresolved`, never from `spell_states`. Each either resolves the attempt or
  finds it already resolved and returns the recorded result through the existing replay path.
- So no key-33 writer runs between any writer's `COMMIT` and its install, whether the outcome
  was known or not, and whether or not a post-commit step failed. No retry locks `spell_states`
  to reach the lane.
- Nothing durable is added. A process restart reloads the Channel from durable truth, so the
  in-memory reservation, the lane and its `unresolved` record need no recovery.

### 1.7 Checklist

1. **Amendments go in the owning contract.** No contract governs the in-process guards. This
   decision is the record, and 2a states the lock order in the module doc of
   `native_combat_cast.rs`. The guard order of ARCH-KILL-REWARD-LOGOUT-1 §1.2 is extended with
   the lane in front of it. It is not changed.
2. **Concurrency is serialized.**
   - The lane serializes the key-33 writers.
   - The lane's commit window and `unresolved` record keep that serialization from every
     committing writer's `COMMIT` to its install, across an unknown outcome or a post-commit
     failure (§1.6).
   - The reservation serializes access to the touched slots.
   - The pending marker serializes access to the caster's spell state.
   - The guard order is fixed and tested.
3. **Durable state is restart-sufficient.** No durable change is made (§1.6).
4. **References are typed.** The reservation is the existing `Arc<OwnerSpellReservation>` on
   exact slots.
5. **Older peers are gated.** One node holds a Channel under its scope assignment, and nothing
   changes on the wire or in storage.
6. **Split work is one unit.**
   - 2a is correct alone. Its S still contains the prepare awaits, but under the guards, which
     are released only after S (§1.1). Its commit window covers every committing writer (§1.6).
     2b only removes the awaits from S.
   - 2b depends on 2a.
   - Each packet is one PR.

## 2. Packets

### 2.1 SPELL-LOCK-2a (hard worker)

```yaml
task_id: OTV2-20261005-spell-lock-2a
decision: this decision §1.1-§1.4, §1.6
worker: oteryn-hard-worker
review: independent review on the final frozen head (Channel concurrency)
branch: claude/spell-lock-2a-20261005
base: main
depends_on: ["this decision merged", "the CP orders it against KILL-REWARD-COMP-1 (attack.rs, runtime_actor_carrier.rs) and SPELL-TARGET-1 (ordinary_combat.rs), and against any open packet that owns runtime_actor_spell.rs, mana_training.rs, companion_lifecycle.rs or delayed_execution.rs, or that wires or changes the item transfer, mint, map mint, decay retire, pickup, death-reward or ITEM-MOVE corpse-take paths"]
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs
  - apps/game-server/src/gameplay_transport/mod.rs                 # lane field, lane-first callers
  - apps/game-server/src/gameplay_transport/actor_spell.rs         # pending marker
  - apps/game-server/src/gameplay_transport/actor_spell_commit.rs  # two-phase commit_owner_batch and rebind_training
  - apps/game-server/src/gameplay_transport/world_item_cast.rs     # lane acquire, commit window, resolver
  - apps/game-server/src/gameplay_transport/parameter_cast.rs      # lane acquire, commit window, resolver
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks.rs  # lane acquire only
  - apps/game-server/src/gameplay_transport/field_step_ingress.rs  # lane acquire only
  - apps/game-server/src/gameplay_transport/familiar_defense.rs    # lane acquire only
  - apps/game-server/src/gameplay_transport/party_spell_owner.rs   # lane acquire only
  - apps/game-server/src/gameplay_transport/familiar_cast.rs       # lane acquire, commit window, resolver
  - apps/game-server/src/gameplay_transport/familiar_cast_dispatch.rs  # pending readers read the marker, retry and control loss resolve through the lane
  - apps/game-server/src/gameplay_transport/qualification_wild_spawn.rs  # lane acquire only
  - apps/game-server/src/gameplay_transport/spell_periodic.rs      # lane acquire only
  - apps/game-server/src/gameplay_transport/source_item_cycle.rs   # lane acquire only
  - apps/game-server/src/movement/source_floor_change.rs           # lane acquire only
  - apps/game-server/src/gameplay_transport/attack.rs              # reserved-attacker deferral
  - apps/game-server/src/foundation/runtime_actor_carrier.rs       # §1.3 checks
  - apps/game-server/src/foundation/runtime_actor_conditions.rs    # §1.3 check
  - apps/game-server/src/foundation/runtime_actor_spell.rs         # two-phase commit_spell_batch and companion-touch release
  - apps/game-server/src/spell/mana_training.rs                    # borrowing checks of prepare_install and rebind_staged_training
  - apps/game-server/src/spell/companion_lifecycle.rs              # two-phase commit_familiar
  - apps/game-server/src/spell/delayed_execution.rs                # familiar timer check against the predicted receipt
  - apps/game-server/src/durability/spell_item_transaction.rs      # verdict instead of closure, permit, commit window
  - apps/game-server/src/durability/spell_owner_commit.rs          # commit window parameter
  - apps/game-server/src/durability/native_map_items.rs            # permit parameter
  - apps/game-server/src/durability/character_familiar.rs          # permit and commit window pass-through
  - apps/game-server/src/durability/world_party.rs                 # permit pass-through
  - apps/game-server/src/durability/item_transfer.rs               # permit at apply_transfer and commit_item_transfer
  - apps/game-server/src/durability/item_mint.rs                   # permit at the mint and corpse-loot appliers and commit_* entries
  - apps/game-server/src/durability/map_item_mint.rs               # permit at apply_mint and commit_map_item_mint
  - apps/game-server/src/durability/item_decay_retire.rs           # permit at apply_retire, commit_decay_retire, retire_decayed_corpse
  - apps/game-server/src/combat/pickup.rs                          # permit parameter
  - apps/game-server/src/combat/death_reward.rs                    # permit parameter
  - apps/game-server/tests/support/attack_kill_reward_postgres_cases.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - apps/game-server/tests/support/combat_bestiary_postgres_cases.rs
  - apps/game-server/tests/support/combat_death_reward_postgres_cases.rs
  - apps/game-server/tests/support/combat_pickup_postgres_cases.rs
  - apps/game-server/tests/support/corpse_decay_postgres_cases.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs
  - apps/game-server/tests/support/item_mint_postgres_cases.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs
  - apps/game-server/tests/support/map_item_mint_postgres_cases.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs  # these cases acquire a lane to call the item writers
  - apps/game-server/tests/spell_lane_key33_pin.rs                 # the two key-33 pin tests (new)
  - the test modules next to these files
  - docs/agents/tasks/archive/OTV2-20261005-spell-lock-2a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - git diff --check
```

Builds:

- The lane and the permit (§1.2), taken first by every key-33 caller, including the item
  writers and their callers in `combat/`. If the compiler finds a caller outside the owned paths,
  the worker returns a BLOCKER naming it. It does not edit the caller.
- The scope check of every item writer against the Channel of each row it writes (§1.2).
- Release after S, then re-lock. The `COMMIT`, the post-commit transaction and every write after
  S run with only the lane held.
- The §1.3 checks and the §1.4 marker, with every pending-list reader moved to the marker's
  identity fields, including those in `familiar_cast_dispatch.rs`.
- S as the guarded span through `stage_installation` (§1.1). The prepare awaits stay in it.
- The commit window as a parameter of `commit_spell_owner_transaction`, the
  `UnresolvedSpellCommit` record with its four variants, `UnresolvedLane`,
  `resolve_unresolved_spell_commit` and one resolver per committing writer (§1.6).
- The two-phase post-commit install of every writer (§1.6): a borrowing, fallible check phase,
  then an infallible phase that moves the install fields. The variants hold those fields by
  value. The shared helpers are split too: `commit_owner_batch`, `commit_spell_batch`,
  `release_companion_touches_for_source_commit`, `prepare_install`, `rebind_staged_training`,
  `rebind_training`, `commit_familiar` and the familiar timer `finalize`. If another helper
  that phase 2 needs can still fail after a move, the worker returns a BLOCKER naming it.
- Each resolver resumes its writer's retained retry path (§1.6): the AlreadyCommitted branch
  when committed, the `Applied` branch in a new commit window when not, and release only on a
  definite rejection that the path proves.

Tests:

- A sequencer test fails if a Channel guard is held across any await after S, where S ends at
  `stage_installation`. Awaits inside S are allowed in 2a.
- During the unguarded window, the test runs each competitor and asserts the install succeeds
  and the reserved slots are equal. The competitors are:
  - caster movement;
  - a swing by the caster and a swing at the caster;
  - a condition tick on a target;
  - the caster's control loss;
  - a door use;
  - a periodic tick;
  - a second cast, which waits on the lane.
- An unknown commit outcome followed by a retry installs exactly once.
- For a familiar attempt parked in `unresolved`, while only its marker is in
  `pending_familiars`: dispatch returns `Pending` for the caster, the control-loss reconcile
  recovers the original command and intent, a competing command is refused, and the retry of
  the original command sees `original_retained` and resolves through the lane. The test fails
  if `pending_familiars` holds a second `PreparedFamiliarCast`.
- After an unknown commit outcome, a competitor that takes the lane at once finds the complete
  attempt in `unresolved` and installs or releases it with the original `prepared`, `owned`,
  `request` and `fence`. The test fails if the attempt is reconstructed or if `pending_native`
  holds more than the marker.
- After an unknown commit outcome, each other key-33 writer (a periodic tick, a world-item cast,
  the map-item deadline drain) resolves the recorded cast before it runs, so it never sees
  committed items the runtime does not show. The caster's later retry returns the same result.
- After a known-successful `COMMIT`, a fault injected into each post-commit step parks the
  attempt in `unresolved`. A competitor that takes the lane at once installs it exactly once. The
  test fails if any key-33 writer runs before that install. The steps are:
  - native: training receipt, post-commit transaction, authority check, reconnect check,
    owned-fact load;
  - world-item and parameter: training `after_commit`, `prepare_install`, `rebind_training`,
    `commit_owner_batch`;
  - familiar: each fallible step between its `COMMIT` and its install.
- An unknown commit outcome of a world-item, a parameter and a familiar cast parks the complete
  attempt in its variant. The test fails if the attempt is pushed back into its pending list or
  reconstructed.
- An early return after the `COMMIT` that does not consume the commit window still parks the
  attempt, through `Drop`.
- For each writer, a fault injected into each phase-1 step leaves the parked attempt holding
  every install field (`paid`, `player`, `physical`, `presentation`, or the native `prepared`
  and `owned`). The resolver then installs it exactly once. The test fails if any field is
  missing.
- For each split helper, a fault in its check leaves the staged batch, the preflight, the
  training installation and the `PreparedFamiliar` unchanged, and changes no runtime slot or
  companion touch. Callers of the `commit_owner_batch` composition behave as today.
- For each writer, `commit_semantic_transaction` returns `CommitOutcomeUnknown` because the
  deadline elapsed, with no `COMMIT` sent. A competitor that takes the lane at once makes the
  resolver commit the retained attempt exactly once through the `Applied` branch. The marker
  then carries the same result the caster's own retry returns in the same state. The test fails
  if the attempt is released, the marker is cleared without a commit, or the attempt is
  reconstructed.
- The resolver releases an attempt only on a definite rejection that the writer's retry path
  proves, with the same result as that retry. After control loss, it resolves a native attempt
  as `reconcile_pending_native_for_control_loss` does, and a familiar attempt with the
  `allow_new_mutation` its retry computes.
- A resumed commit that is unknown again parks the attempt again, with every field present.
- A resolver failure that a retry cannot fix keeps the lane fenced and logs its error code. A
  Channel reload clears it.
- While the outcome stays unknown, those writers are refused retryably with no side effect.
- No path reaches a permit from an `UnresolvedLane` except through the resolution.
- No call of `commit_spell_owner_transaction` compiles without a commit window.
- A test fixes the guard order.
- A permit for another Channel is refused before any statement runs.
- For each item writer (transfer from Ground and from a corpse entry, mint, corpse mint,
  corpse-loot mint, map mint, decay retire, corpse retire), a permit for another Channel is
  refused with no row written and no receipt. A transfer whose rows span two Channels is refused.
- While an attempt is parked in `unresolved`, each item writer waits on the lane and runs only
  after the attempt is resolved.
- **Key-33 pin, SQL.** A PostgreSQL test reads every function in the database whose body takes
  advisory lock key 33, and every trigger that calls one. The test fails unless the set is exactly
  the three triggers of §1.2 on their three tables.
- **Key-33 pin, Rust.** A source scan of the production sources of `apps/game-server` finds
  every function whose SQL locks key 33 or writes `game_item_ground_locations`,
  `game_item_corpse_container_entries` or `game_native_map_scope_adoptions`. The test fails unless
  the set equals a pinned list and each function on it takes a `&SpellLanePermit` or is called
  only from one that does.

### 2.2 SPELL-LOCK-2b (hard worker)

```yaml
task_id: OTV2-20261005-spell-lock-2b
decision: this decision §1.5
worker: oteryn-hard-worker
review: independent review on the final frozen head (Channel concurrency)
branch: claude/spell-lock-2b-20261005
base: main
depends_on: ["SPELL-LOCK-2a merged", "SPELL-TARGET-1 merged or ordered by the CP (ordinary_combat.rs)"]
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs
  - apps/game-server/src/gameplay_transport/ordinary_combat.rs
  - apps/game-server/src/gameplay_transport/native_companion_item_cast.rs
  - apps/game-server/src/gameplay_transport/native_world_item_cast.rs
  - apps/game-server/src/durability/spell_access_facts.rs
  - the test modules next to these files
  - docs/agents/tasks/archive/OTV2-20261005-spell-lock-2b.md
validation: as 2a
```

Builds:

- The read plan, the prefetch view and the coverage check (§1.5), for each family.
- Tests:
  - every family's prepare asks only for planned keys;
  - a runtime change between plan and S that widens the plan retries, then refuses with no
    reservation and no in-memory change;
  - for an `AttackTarget` cast, an attack-target switch and an attack-target clear between plan
    and S are each a miss. The retry plans for the new target, and the test fails if prepare
    receives the prefetched target;
  - the sequencer test of 2a extended to "no Channel guard across any await of the pass".

## 3. Rejected options

- **A Channel-wide runtime revision as a re-lock fence.** Every mutator would have to bump it,
  and any movement in the Channel would invalidate every cast in flight. That livelocks under
  load.
- **Re-lock before `COMMIT` and commit under the guards.** This keeps a durable await under the
  guards. Without the lane, it would also still leave other spell writers' windows open.
- **Narrow the install fence to spell-owned fields and merge.** This changes the install
  semantics of every batch. The complete reservation keeps the existing whole-slot fence.
- **Release every spell writer's guards in one packet.** That is too large to review as one
  unit. The lane makes the later per-writer packets mechanical.

## 4. Decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`:

1. **Must decide now?** YES.
   - The finding is a P1 on merged code.
   - SPELL-LOCK-1 stopped because it had no design. A narrower fix cannot be specified without
     one.
2. **What is blocked?**
   - SPELL-LOCK-2a and 2b.
   - Releasing the guards of the other spell writers, which reuses the lane and the reservation.
   - Until then, every cast holds the whole Channel across its database round trips.
3. **What gets harder later?**
   - Every new key-33 writer must take the lane. The permit makes that a compile error to
     forget in Rust, and the two pin tests catch a new trigger or a new writer of a locked table. Every new committing writer needs a commit window, a variant and a resolver, also
     enforced by the compiler.
   - Every new mutator of a slot must honour the reservation.
   - Both are in-process rules. No durable, wire or contract coupling is created, so reverting is
     a code change.
4. **What would justify superseding it?**
   - Measured lane contention that stalls casts, beyond what key 33 already imposed.
   - A reservation test failure that cannot be fixed with a check or a deferral.
   - A move of spell writes off the Channel owner, or a sharded Channel. Either removes the
     single in-memory owner the lane mirrors.
5. **What is deliberately not decided?**
   - The order and packets in which the other spell writers release their guards.
   - Any runtime revision counter.
   - Lane fairness and priority, beyond the mutex's FIFO.
   - Prefetch caching across casts.

The design holds when:

- after 2a, no Channel guard is held across the `COMMIT` of a native cast, and S holds the
  guards from the pass start until the reservation is staged;
- after 2b, no Channel guard is held across any await of the pass;
- a reserved slot changes only through its own batch;
- no key-33 writer runs between any committing writer's `COMMIT` and its install, also when
  the `COMMIT` outcome is unknown or a post-commit step fails;
- no code path takes key 33 without a lane permit, whether it locks key 33 in Rust or fires a
  key-33 trigger by writing a Ground, corpse-entry or adoption row;
- a parked attempt resolves with the same result as the caster's own retry, whichever acquirer
  resolves it, and is released only on a definite rejection.
