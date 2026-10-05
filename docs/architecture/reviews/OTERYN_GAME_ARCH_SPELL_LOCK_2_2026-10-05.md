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
  - the AlreadyCommitted reconcile path;
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
   - The cast linearizes at one stage section S.
   - A per-Channel **spell lane** mirrors key 33 in memory.
   - The existing slot reservation becomes complete: every mutator of a reserved slot honours it.
   - The caster stays visibly pending for the whole pass.
   - No revision counter is added.
4. **SPELL-LOCK-2a (hard worker, §2.1). Commit side.**
   - The lane is added, and taken by every key-33 writer.
   - Guards are released after S and re-taken to install or release.
   - The unchecked mutators of a reserved slot are closed.
   - The caster stays pending throughout.
5. **SPELL-LOCK-2b (hard worker, §2.2). Read side. After 2a.**
   - The database reads before S run without guards, as a prefetch.
   - S evaluates prepare on the prefetch and the live runtime.
   - A plan miss retries a bounded number of times, then refuses with no side effect.
6. Other spell writers keep holding guards across their own transactions. That includes the
   world-item and parameter casts, timers, field step, familiar defence and party owner. 2a only
   makes them take the lane first. Releasing their guards is later work that follows the same
   pattern.

## 1. Rulings

### 1.1 Linearization at the stage section S

A native cast takes effect at one critical section S, under `runtime`, `spell_states` and
`door`, with no await inside it. S contains:

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

### 1.2 The spell lane

- Each Channel owner holds one async mutex, the spell lane. It is the in-memory mirror of the
  Channel item advisory lock (key 33).
- Every in-process caller that takes key 33 takes the lane first and holds it from before
  `begin` to after its install or release. These callers are:
  - the native, world-item and parameter casts;
  - spell timer callbacks;
  - field step ingress, familiar defence and the party spell owner;
  - map-item initialisation.
- Since key 33 already serializes these writers in the database, the lane adds no new
  serialization. What it changes: no other spell writer can run between a cast's `COMMIT` and its
  install. So no other writer sees committed items that the runtime does not yet show.
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
- The replay fast path is unchanged.

### 1.5 Prefetch before S (2b)

1. Each prepare family builds a read plan from a short guarded read of the runtime: the caster,
   targets, positions and tiles.
   - The families are: ordinary, companion item, world item, and native source-self and
     from-owners.
2. The guards are released. In the transaction, the pass runs `begin`, the authority step, the
   database part of the owned facts, the rune reserve and the planned tile reads.
3. S re-reads the runtime. It first checks that the current plan is covered by the prefetch.
   Only then does it run the mutating steps:
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

### 1.6 Unknown outcome and restart

- The reservation is kept on `CommitOutcomeUnknown`, as today, and the retry goes through the
  AlreadyCommitted reconcile path.
- Nothing durable is added. A process restart reloads the Channel from durable truth, so the
  in-memory reservation and the lane need no recovery.

### 1.7 Checklist

1. **Amendments go in the owning contract.** No contract governs the in-process guards. This
   decision is the record, and 2a states the lock order in the module doc of
   `native_combat_cast.rs`. The guard order of ARCH-KILL-REWARD-LOGOUT-1 §1.2 is extended with
   the lane in front of it. It is not changed.
2. **Concurrency is serialized.**
   - The lane serializes the key-33 writers.
   - The reservation serializes access to the touched slots.
   - The pending marker serializes access to the caster's spell state.
   - The guard order is fixed and tested.
3. **Durable state is restart-sufficient.** No durable change is made (§1.6).
4. **References are typed.** The reservation is the existing `Arc<OwnerSpellReservation>` on
   exact slots.
5. **Older peers are gated.** One node holds a Channel under its scope assignment, and nothing
   changes on the wire or in storage.
6. **Split work is one unit.**
   - 2a is correct alone: the guards are held until S and released after it.
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
depends_on: ["this decision merged", "the CP orders it against KILL-REWARD-COMP-1 (attack.rs, runtime_actor_carrier.rs) and SPELL-TARGET-1 (ordinary_combat.rs)"]
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/native_combat_cast.rs
  - apps/game-server/src/gameplay_transport/mod.rs                 # lane field, lane-first callers
  - apps/game-server/src/gameplay_transport/actor_spell.rs         # pending marker
  - apps/game-server/src/gameplay_transport/world_item_cast.rs     # lane acquire only
  - apps/game-server/src/gameplay_transport/parameter_cast.rs      # lane acquire only
  - apps/game-server/src/gameplay_transport/spell_timer_callbacks.rs  # lane acquire only
  - apps/game-server/src/gameplay_transport/field_step_ingress.rs  # lane acquire only
  - apps/game-server/src/gameplay_transport/familiar_defense.rs    # lane acquire only
  - apps/game-server/src/gameplay_transport/party_spell_owner.rs   # lane acquire only
  - apps/game-server/src/gameplay_transport/attack.rs              # reserved-attacker deferral
  - apps/game-server/src/foundation/runtime_actor_carrier.rs       # §1.3 checks
  - apps/game-server/src/foundation/runtime_actor_conditions.rs    # §1.3 check
  - apps/game-server/src/durability/spell_item_transaction.rs      # verdict instead of closure
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

- The lane (§1.2), taken first by every key-33 caller. Callers outside the owned paths are
  reported, not edited.
- Release after S, then re-lock. The `COMMIT`, the post-commit transaction and every write after
  S run with only the lane held.
- The §1.3 checks and the §1.4 marker.

Tests:

- A sequencer test fails if a Channel guard is held across any await after S.
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
- A test fixes the guard order.

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

The design holds when:

- after 2a, no Channel guard is held across the `COMMIT` of a native cast;
- after 2b, no Channel guard is held across any await of the pass;
- a reserved slot changes only through its own batch;
- no other spell writer runs between a cast's `COMMIT` and its install.
