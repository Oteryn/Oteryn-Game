# OTERYN-20261011-spawn-ceilings-1

```yaml
task_id: SPAWN-CEILINGS-1
title: "SPAWN-CEILINGS-1: the Game carrier realizes spawns up to the registered CREATURE-AI-0 ceilings"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spawn-ceilings-1-20261011
pr: 1984
base_sha: 407b0f2b80034925fc2070eae308850e49ede264
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622; origin Codex P1 on #1981 (SPAWN-1b)"
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTERYN-20261011-spawn-ceilings-1.md
  # Scope extension directed by the control plane for the creature-ceiling closure audit
  # (Codex 4242381779 and 4242381774 on #1984):
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/channel_owner_auto_attack_tests.rs
  - apps/game-server/src/foundation/channel_owner_creature_ai_tests.rs
  - apps/game-server/src/foundation/channel_owner_creature_bite_tests.rs
  - apps/game-server/src/ability/creature_bite.rs
  - apps/game-server/src/ai_monster_melee.rs
  - apps/game-server/src/ai_think/channel_owner.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  # Scope extension directed by the control plane (Codex P1 4242954089 on #1984): the native
  # and defence summon owners keep the same D57 64 x 8 memo ceiling that this task lifts in
  # the carrier's summon admission. No other open PR touched this file when it was added.
  - apps/game-server/src/monster_summon.rs
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
external_repositories: []
```

## Outcome

`ChannelRuntimeV1::realize_activation_spawns` no longer refuses readiness above 16 sources or 4
cells. Spawn realization reads the registered CREATURE-AI-0 §10 rows: `CREATUREAI0-RL-02`
65,536 sources per channel, `CREATUREAI0-RL-03` 64 points per source and `CREATUREAI0-RL-01`
131,072 points per channel. Above any of them activation refuses with zero mutation.

## Architecture and source of truth

- CREATURE-AI-0 decision §10 (`OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`):
  RL-02 replaces `AI01-SPAWN-SOURCES-PER-SCOPE` 16, RL-03 replaces `AI01-SPAWN-POPULATION` 4 and
  `AI01-SPAWN-PLACEMENT-CELLS` 4.
- The carrier's spawn state and spawned population are bounded by RL-01 (a sum of points), not
  by the RL-02 × RL-03 product (4,194,304).
- `RESOURCE_LIMITS_REGISTRY.json` names the carrier constants as consumers of RL-01/02/03; the
  three AI01 rows note that they are superseded for spawn realization.

## Excluded scope

- `map/boot.rs` and `node/serve.rs` (#1981), the world pin and bundle.
- `ai/spawn.rs` (`RespawnSchedule`), death paths in other carrier modules, `smelly_cheese.rs`.
- RL-15/RL-16 activation time and memory measurement (owed by SPAWN-1).

## Acceptance criteria

- 64 points per source pass, 65 refuse (`InvalidSpawnDefinition`).
- 65,536 one-point sources realize at the runtime; 65,537 refuse with nothing admitted.
- 131,072 points pass the channel check; 131,073 refuse at the runtime with nothing admitted, and
  the single-source path refuses one point past the ceiling.
- No quadratic activation work and no per-removal scan of the spawn cells.
- `CREATUREAI0-RL-14` (Codex 4242094682 on #1984): no owner window realizes more than 4,096
  points. One owner turn (`realize_activation_spawns`) refuses a larger activation with nothing
  admitted; `begin_activation_spawns` keeps it as a persisted cursor and
  `realize_activation_spawn_window(now)` realizes the next `spawn::realization_windows` window,
  at most one per 50 ms owner window, in canonical order.
- The full respawn scan runs once per Channel owner window, not once per session drive
  (Codex 4242094686 on #1984).
- Activation leaves `SPAWN_ACTIVATION_PLAYER_HEADROOM` (500) carrier slots free for players
  (Codex 4242245064 on #1984): on the 131,072-actor pre-production carrier at most 130,572
  points are accepted and 500 players are then admitted.
- Respawn work per owner window is bounded (Codex 4242270322 on #1984): one window runs at most
  `SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX` (32) respawn steps however many occurrences fall due
  together; the rest stay pending, run first in the next windows, and every point respawns.
  A refused activation window removes its creatures with one pass over the occupied index.
  The budget is registered as `SPAWNCEIL-RL-01` (owner decision 2026-10-11).
- Creature-ceiling closure (Codex 4242381779 and 4242381774 on #1984): no production creature
  path is capped by the old 64-entry or 256-actor envelope. `CREATUREAI0-RL-04` (262,144
  creature actors per Channel) is registered and enforced by `admit_creature`; the creature
  bite ledger and the melee owner never refuse a live issuer (tested with 300 issuers);
  the monster pass has no channel-wide `AI01-ACTIVE-ACTORS` gate and swings at most
  `CREATUREAI0-RL-05` targeting monsters per pass, round robin (tested with 300 monsters).
- Merge with main (CREATURE-MOVE-1): the swing window keeps the targeting filter and the
  RL-05 budget and swings from each creature's post-step position. The respawn drive's
  admission also checks `CREATUREAI0-RL-04` once the dead creature has left (Codex
  4242715014), and a refused admission costs its `SPAWNCEIL-RL-01` step, so one window retries
  it at most 32 times (Codex 4242715016). Neither has a dedicated test: the first needs a
  262,144-actor carrier with a pending respawn, and the second needs a carrier with no free
  slot during a due admission.
- Wake fan-out (Codex 4242808144): the monster pass wakes for all its players in one indexed
  pass (`wake_for_players`): one walk of the think table and one position read per idle
  creature build a (floor, x, y) index, and each player examines only its view box (at most
  16 floors of 19 rows), so the work no longer grows with players times creatures. The result
  equals one wake per player in turn. Tested: the view box holds every cell a player sees on
  every floor pair, and 41 players over 800 rats wake the same creatures as wakes in turn,
  read each rat once and examine only the rats in their boxes. The registry no longer lists
  the creature bite ledger among the 64-entry D57 ledgers (Codex 4242808147).
- Summons past the D57 envelope (Codex 4242954089): native summon and masterless encounter
  admission check `CREATUREAI0-RL-04` (the occupied count `admit_creature` uses) in place of
  64 live creatures; the native summon links are bounded by RL-04 (one per child actor);
  `retire_native_summons` no longer refuses at the 65th dead child (it is now bounded per
  call, see the next item). The native
  summon owner's memos are bounded by RL-04 creatures times the 8 entries of
  `CREATUREAI0-RL-08`, and the defence summon owner's and the creature self-heal ledger's by
  RL-04 times the 8 defences of `CREATUREAI0-RL-07`, in place of 64 x 8. `CREATUREAI0-RL-08` is
  registered. Tested: 100 summoners each summon, retire their 100 dead children in one call and
  summon again; 100 defence summoners each summon.
- Summon link and memo work (Codex 4243039217, 4243039220): the native summon links are indexed
  (`NativeSummonLinks`: ordered maps by admission ordinal, by child actor and by (parent,
  ordinal)), so a lookup, a child removal and a parent's children cost O(log n) instead of a
  scan of every link. `retire_native_summons` examines at most `SPAWNCEIL-RL-02` = 256 links
  per call from a persisted cursor (wrapping), and the dead children found are removed with one
  pass over the occupied index and the attacker leases; the links left over are examined by
  later calls. A summon is never refused for retirement backlog: admission first retires the
  dead children of its own parent (at most 16, targeted through the parent index), so the
  per-parent RL-08 count is exact. The native summon owner's memos, the defence summon owner's
  memos and the creature self-heal ledger are `ActorMemoTable`s: ordered maps by carrier slot,
  each holding its exact creature and that creature's memos by entry or ability, so a lookup is
  O(log n), a memo of an older generation of the slot is dropped when the slot is next used, and
  the full-table prune runs only when the RL-04 x RL-07/RL-08 cap is reached; the caps,
  replay, superseded and per-creature limits are unchanged. Tested: 40 summoners x 16 dead
  children are retired 256 links per call, an admission retires its parent's dead children
  first and later calls retire the rest; 300 creatures x 8 memos are looked up by exact
  creature, and a reused slot's stale memos are dropped.

## Implementation / findings

- Activation checks every ceiling and the free-slot count once, then admits all sources under
  one snapshot (`admit_spawn_source`); the old per-source `slots.clone()` made activation
  O(sources × slots).
- `spawns` stays in ascending source order; every source lookup is a binary search.
- Removal no longer scans every spawn cell; `creature_alive` now checks the exact actor-local
  generation, so a stale cell reference never reads as the slot's later occupant. The summon
  move rollback therefore no longer snapshots `spawns`.
- The 16/4 constants remain only as the fixed D57 64-entry envelope of
  `MAX_RETAINED_CORPSE_PROJECTIONS` and the test-only smelly cheese ledger.
- RL-14 (review round on #1984): the activation is split into owner windows by
  `spawn::realization_windows` at point granularity (`admit_spawn_point`; a source is recorded
  at its point 0 and may span two windows). `SpawnActivation` persists the plan, the next-window
  cursor and the last owner window used; a second call in one owner window realizes nothing.
  The Channel must not admit players until `remaining` is 0; the respawn drive and the
  qualification spawn do nothing or refuse while an activation is pending. A refusal in a later
  window removes every creature the activation admitted (by removal, which relinks the free
  list, so owner work between windows is never undone) and a new activation may begin. The
  one-turn path keeps its exact snapshot restore. No current caller passes more than 4,096
  points (bundle worlds pass none until SPAWN-1b, #1981, which consumes the windowed API).
- Spawn drive coalescing (review round on #1984): the owner calls `drive_spawn_respawns` once
  per connected session (`tick_vitals` -> `drain_monster_melee` -> `drive_owner_spawns`, before
  the monster-AI pass gate). The gone scan over every realized point and `RespawnSchedule::due`
  now run once per owner window (the first drive with `now / 50 ms` past the last one), which
  keeps the occurrences due by the window's end in a deadline-ordered queue; later drives of
  the window pop only the occurrences whose deadline they reach, each still run at its exact
  microsecond, and successors due inside the window are inserted in order. A death is now
  observed at the first drive of the next owner window (up to 50 ms later than before; the
  occurrence is still due one full delay after its observation).
- Player headroom (review round on #1984): the pre-production carrier capacity
  (`PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY`, 131,072, `node/config.rs`) equals
  `CREATUREAI0-RL-01`, so a maximum activation filled every slot and `reserve_player`
  (`admit_inner`) refused the first player. `check_spawn_activation` now requires the slots left
  after the activation to be at least `SPAWN_ACTIVATION_PLAYER_HEADROOM`, the D128 design target
  of 500 players per Channel (half the carrier for carriers under 1,000 slots, for tests and
  qualification). Sizing the carrier to `CREATUREAI0-RL-04` instead needs `node/config.rs` and
  an owner decision on the capacity; it is outside this task. The headroom does not cover
  monster or player summons or NPCs (owner question in the report).
- Respawn work bound (review round on #1984): each due step cost O(slots) (the census of every
  positioned actor for the Blocked check; `cell_occupied` twice, a deep `slots.clone()` and an
  `occupied.clone()` per admission), and every occurrence due in a window ran in it, so a mass
  death of up to 131,072 creatures cost one window O(points × slots). The drive now runs at most
  `SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX` = 32 steps per owner window (stale queue entries are free
  and bounded by the queue built once per window); the occurrences left over are collected
  again, overdue first, by the next window's `RespawnSchedule::due`. Their successors are timed
  from the step that runs, so a backlog delays them by the windows they wait. A step now reads
  the present players once per drive from the occupied index (`present_player_positions`,
  the census's player rule, no census of every creature), checks the cell once and admits
  through `respawn_into_free_cell`: every foreseeable refusal precedes the first mutation and
  no slot snapshot is taken; after the dead creature's best-effort removal only the protected
  generation exhaustion of the freed slot (#541) can refuse. `resolve_respawn_timer` keeps its
  snapshot restore and is no longer called by the drive. A position index would make each
  step O(1) but positions are written in carrier modules outside this task; the owner moved it
  to its own task, CREATURE-POSITION-INDEX-1. The budget is registered as `SPAWNCEIL-RL-01`
  (owner decision 2026-10-11).
- Audit at the 131,072 ceiling (review round on #1984), all under the Channel runtime mutex:
  - activation (`check_spawn_activation`, `realize_activation_window`, `admit_spawn_point`):
    one free-slot count per activation, then O(1) per point (free-list admission, binary-search
    source lookup, in-order source insert), ≤ 4,096 points per window; policy binding searches
    the content policy table (content-bounded, `runtime_actor_companion.rs`);
  - activation refusal: was one O(occupied) `unindex_occupied` per removed creature (quadratic,
    up to 130,572 creatures); `remove_all` now removes them and drops their slots from the
    occupied index and the attacker leases in one pass each;
  - gone scan and `RespawnSchedule::due`: O(points) once per owner window (coalesced);
  - respawn steps: bounded as above; a removal is one pass over the occupied index;
  - player admission (`reserve_player`, `commit_reserved_player`): O(1) free-list admission,
    no scan; the headroom keeps 500 slots for it;
  - other `cell_occupied`-style scans: `native_summon_cell_free`, `current_cell_occupied_except`,
    `position_occupied_by_other`, `committed_player_positions`, the census and
    `npc_actor.rs` placement remain O(slots) per call, bounded by their callers' own budgets
    (summons, movement, RL-05 thinks); a position index for them needs paths outside this task.
- Creature-ceiling closure audit (Codex 4242381779 and 4242381774 on #1984). Every fixed
  limit, ledger, budget or table of 16 × 4, 64 or 256 that depends on the creature or issuer
  count:
  - `CreatureBiteLedger` (`CREATURE_BITE_LEDGER_MAX` 64) and `MonsterMeleeOwner` (same cap):
    the 65th live monster's bite and melee were refused `LedgerFull`. Both are now ordered maps
    capped at `CREATUREAI0-RL-04`. The melee owner is keyed by carrier slot (`actor_local_id`):
    an entry of an older generation of the slot is replaced in place, which also drops the old
    retain-live scan per insert. The bite ledger is keyed by the issuer's exact generation
    (`placement_identity`), because the `ability_engine` integration test compiles
    `src/ability` against a Foundation shim without slots; a full ledger drops dead issuers'
    entries first, and the carrier admits at most `CREATUREAI0-RL-04` creatures, so a live
    issuer is never refused.
  - `monster_ai_cycle`: the `census.len() > ResourceLimit::ActiveActors` (256) gate returned
    before every think and attack at 257 positioned actors. Removed; the per-pass work is
    bounded instead: think admission runs only for monsters new to the table, thinks are
    `CREATUREAI0-RL-05`-bounded by `CreatureAiTable::take_due`, and swings run for at most
    RL-05 targeting monsters per pass, round robin from a persisted cursor (the old
    `EvaluationWork` 8 was a per-think envelope, not a channel swing budget).
  - `CreatureAiTable`: idle dead creatures were never removed (only thinks clean up), so the
    table leaked with deaths. `forget_dead` drops them once per pass; the table holds live
    creatures only, bounded by RL-04.
  - census: was a scan of every slot; it now walks the occupied index in slot order.
  - `ChannelAttackStates` (player attack entries) were capped at `ActiveActors` 256; the cap is
    now the carrier slot count.
  - `CREATUREAI0-RL-04` is enforced at `admit_creature`; the check counts every actor, so it is
    conservative and never binds on the 131,072-slot pre-production carrier.
  - `AI01-ACTIVE-ACTORS` (256) stays only on `ai::AiSnapshot`, which only tests construct; the
    registry row says so.
  - Not changed: the 64-entry caps of the test-only monster lanes (`MonsterCombatLane`,
    `MonsterThinkLane`: `AI01_CREATURES_PER_SCOPE`, smelly encounter, source encounter seven,
    delayed damage spell, threshold heal, area-heal memos, bone encounters, smelly cheese
    16 × 4) are not on the production path; they move with the lane wiring task. Correction
    (Codex 4242954089): the summon memos and the callback spawn are served by the carrier's
    own `admit_native_summon` and `admit_native_encounter_masterless`, which kept a 64
    live-creature refusal; those, the summon memos, the summon retirement and the
    self-heal ledger's 64 x 8 now follow RL-04, RL-07 and RL-08 (see Acceptance). The native
    summon death ring (64) evicts, never refuses: a lookup of an evicted death fails closed
    without corpse or reward. FIFO retention rings (`MAX_RETAINED_CORPSE_PROJECTIONS`, native summon
    deaths, 64) evict, never refuse. `KILLRW-RL-01`, `COMBAT01_INFLIGHT_LOOT_MINTS_PER_SCOPE_MAX`
    and `COMBAT01_CORPSES_PER_SCOPE` (64) are registered limits of their own contracts (owner
    question in the report).
  - Residual per-pass O(N) work (`wake_for_player` per player over the table, the melee
    qualification of non-melee monsters, `forget_dead`) is bounded by RL-17, measured by
    CREATURE-MOVE-1, and is the scope of CREATURE-POSITION-INDEX-1.
- Summon links and memos (Codex 4243039217, 4243039220): the retirement made unbounded in the
  previous round cost O(links) per dead child (the death projection and the removal each
  scanned the link Vec, plus one occupied-index pass per child), and admission ran it before
  every summon; the memo ledgers did a `retain` and a `position` scan of up to RL-04 x 8
  entries per proposal. Now indexed and bounded as in Acceptance. `SPAWNCEIL-RL-02` (256) is
  registered at control-plane request; unlike `SPAWNCEIL-RL-01` it has no owner decision yet
  (owner question in the report). The link and memo maps allocate on insert like the other
  ordered-map ledgers of this task; the old `try_reserve` on the link Vec is gone, and a
  failed link insert (duplicate child or ordinal overflow) restores the admission snapshot.
- At reference scale the 64-entry corpse projection FIFO may evict before a lost-response
  replay; that is an owner question, not changed here.
- The test-only cheese source uses `SpawnSourceId(u16::MAX)`; at 65,536 realized sources it
  refuses as a duplicate (fail-closed).

## Validation

- Review round on #1984 (RL-14 windows, drive coalescing), on the owner's PC (WSL Ubuntu 24.04,
  Rust 1.94.0) at the pushed head:
  - `cargo fmt --all -- --check`: pass
  - `cargo clippy -p oteryn-game-server --all-targets --locked -- -D warnings`: pass
  - `cargo test -p oteryn-game-server --lib --locked`: pass at a2a1a060 (2,740 passed, 0
    failed, 28 ignored); the first run failed one new test assertion (a removed slot keeps its
    generation, so liveness, not `validate_ref`, shows the removal), fixed in a2a1a060
  - Player headroom round (Codex 4242245064, 3b393bcc): pushed before its PC run finished;
    CI found an unresolved `crate::node::config` path in the test (this file is also built
    into crates without the node configuration) and a clippy `manual_is_multiple_of`, both
    fixed in the respawn-bound round
  - Respawn-bound and audit round (Codex 4242270322), on the PC before the push, in a
    private checkout of 3b393bcc plus this round's diff (code and registry identical to the
    pushed commit): `cargo fmt --all -- --check` pass; `cargo clippy -p
    oteryn-game-server --all-targets --locked -- -D warnings` pass; `cargo test -p
    oteryn-game-server --no-run --locked` pass (every test target, including the integration
    tests CI refused); `cargo test -p oteryn-game-server --lib --locked` pass (2,742 passed,
    0 failed, 28 ignored)
  - Ledger-ceiling round (Codex 4242381779, 4242381774), on the PC before the push, in a
    private checkout of 8dacdb0c plus this round's diff (tree hash checked against the
    pushed code): `cargo fmt --all -- --check` pass; `cargo clippy -p oteryn-game-server
    --all-targets --locked -- -D warnings` pass (every test target, including the
    `ability_engine` shim and the postgres targets). The PC's WSL VM restarted during `cargo
    test --no-run`, so the test run is left to CI; earlier attempts found and fixed the
    `ability_engine` shim mismatch and a `assertions_on_constants` lint
  - Merge with main 3fd270f3 plus Codex 4242715014 and 4242715016, on the PC before the push,
    in a private checkout that reproduces the merge (apps tree hash equal to the pushed merge
    commit's): `cargo fmt --all -- --check` pass; `cargo clippy -p oteryn-game-server
    --all-targets --locked -- -D warnings` pass; `cargo test -p oteryn-game-server --no-run
    --locked` pass; `cargo test -p oteryn-game-server --lib --locked` pass (2,756 passed, 0
    failed, 28 ignored); governance validation and `tools/agents/tests` pass
  - Wake fan-out round (Codex 4242808144, 4242808147, clippy `while_let_loop`), on the PC
    before the push, in a private checkout of main 9080588d plus this branch's delta (apps and
    registry diff hash equal to the pushed head's): `cargo fmt --all -- --check` pass; `cargo
    clippy -p oteryn-game-server --all-targets --locked -- -D warnings` pass; `cargo test -p
    oteryn-game-server --no-run --locked` pass (before the bound test's carrier capacity
    fix, which the clippy and test runs then covered); `cargo test -p oteryn-game-server --lib
    --locked` pass (2,758 passed, 0 failed, 28 ignored)
  - Summon round (Codex 4242954089), on the PC before the push, in the same private checkout
    of main 9080588d plus this branch's delta (apps and registry diff hash equal to the pushed
    head's): `cargo fmt --all -- --check` pass; `cargo clippy -p oteryn-game-server
    --all-targets --locked -- -D warnings` pass; `cargo test -p oteryn-game-server --no-run
    --locked` pass; `cargo test -p oteryn-game-server --lib --locked` pass (2,760 passed, 0
    failed, 28 ignored); governance validation and `tools/agents/tests` pass
  - Ledger/retirement round (Codex 4243039217, 4243039220), on the PC before the push, in the
    same private checkout of main 9080588d plus this branch's delta (apps and registry diff hash
    equal to the pushed head's): the first runs found the carrier's derived `PartialEq` needing
    one on the link index (now the links in admission order, as the list compared) and two
    clippy `explicit_counter_loop`s, fixed before the push; then `cargo fmt --all -- --check`
    pass; `cargo clippy -p oteryn-game-server --all-targets --locked -- -D warnings` pass;
    `cargo test -p oteryn-game-server --no-run --locked` pass; `cargo test -p
    oteryn-game-server --lib --locked` pass (2,762 passed, 0 failed,
    28 ignored); governance validation and `tools/agents/tests`
    pass
- Earlier round: `cargo fmt --all --check`, `python tools/agents/validate_governance.py` and
  `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: recorded by the control plane at merge
