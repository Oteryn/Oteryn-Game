# MONSTER-META-1 — monster meta: bestiary D15, bosses, raids (skeleton task record)

```yaml
task_id: MONSTER-META-1
title: "MONSTER-META-1: monster meta: bestiary D15, bosses, raids"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MONSTER-META-1; §7 'MONSTER-META-1' (BOSS_RAID0, MONSTER_D15)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/monster-meta-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Bestiary D15 completion, boss encounters with rooms and rewards, the Bosstiary, and scheduled raids with announcements.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

MONSTER-D15B-1; BOSS-1 -> BOSS-ROOM-1 (SCOPE-HANDOFF-1) -> BOSS-REWARD-1 (PARTY-XP-1) -> BOSSTIARY-1; RAID-CONTENT-1 -> RAID-1 (CHAT-1); BOSS-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-MONSTER-1 (family entry gate) | BLOCKING (Wave 2, not merged) |
| acceptance of BOSS_RAID0 / MONSTER_D15 (plan §7 gate) | BLOCKING |
| CHAT-1 / CHAT-1b-2b (RAID-1) | MERGED (#2093) |
| PARTY-XP-1 (BOSS-REWARD-1) | BLOCKING (Wave 3, not merged) |
| SCOPE-HANDOFF-1 (BOSS-ROOM-1) | MERGED (#1782) |
| D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2 | MERGED (archive records) |
| ENCOUNTER-RT-0 (encounter runtime) | ACCEPTED (2026-10-14); runtime merge state to be confirmed by CP |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/bosses/**, content/raids/** (sources only)`
- `apps/game-server/src/durability/character_bosstiary.rs`
- `apps/game-server/src/durability/boss_encounter.rs`
- `apps/game-server/src/durability/boss_encounter_tests.rs`
- `apps/game-server/src/gameplay_transport/raid.rs`
- `apps/game-server/src/gameplay_transport/raid_tests.rs`
- `crates/protocol-oteryn/src/bosstiary.rs`
- `apps/game-server/tests/monster_meta_postgres.rs`
- `docs/agents/evidence/MONSTER-META-1-*.md`
- `docs/agents/tasks/MONSTER-META-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- kill_reward.rs (boss reward hook)
- world_runtime.rs raid scheduler registration
- connection.rs dispatch chain; capabilities.rs `GATED` l.165
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- BOSS-REWARD-1: reward split + loot + bosstiary progress in one transaction
- RAID-1: schedule + spawn + broadcast + restart resume

## Acceptance criteria

- [ ] Boss cooldowns and Bosstiary progress are durable and restart-safe.
- [ ] Boss room entry/exit uses SCOPE-HANDOFF-1; no duplicate rewards on reconnect.
- [ ] Raids fire on schedule once per window even across restart.
- [ ] Bestiary D15 values match reference.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server bosstiary
cargo test -p oteryn-game-server boss
cargo test -p oteryn-game-server raid
cargo test -p oteryn-game-server bestiary
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md
- PR #1782
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local BOSS-1 clock preparation — 2026-10-15

This allocation narrows the family to `durability/boss_encounter.rs`, its own tests and this card.
Protected main `e2da9805d296a27b06b32335bf67394509d4910b` contains ACCEPTED BOSS_RAID0 through
merged #2209. §Implementation brief makes the early BOSS-1 child depend directly on that decision;
LOOP-MONSTER, ROOM, REWARD and later family integrations do not block this detached clock subset.
The historical dependency table above is not a current acceptance assertion. No family acceptance
checkbox is completed here; cooldown/eligibility, rooms, rewards, raids and Bosstiary are absent.

### Implemented boundary

- Actual respawn-clock claim/recovery transaction primitive: independently compares the current
  recovery admission, holds native admission relation locks, validates the native assignment
  high-water/retained history (keeping its writer share lock), locks the exact World+Channel scope
  assignment `FOR SHARE`, compares its active generation/current holder registration and proves
  native node-secret possession. It reads portable DB wall-clock milliseconds **after** authority
  locks, then uses a full-row CAS (cycle/state/epoch/due/owner), returning typed unchanged/loser.
  Missing clocks fail closed; no guessed first-spawn initialization is introduced.
- Activation can precede runtime-readiness publication. Assigned current scope ownership proves
  clock ownership; it does not publish ready, admit a player, grant Character authority or install
  an actor. A record with an owner generation ahead of the independently current generation fails
  closed. Same-generation ALIVE is unchanged; older ALIVE transfers generation without changing
  cycle. A future due time remains DUE for the real owner timer.
- Pure proposals for respawn death, at-reset death and new-reset draw. Respawn death adds exactly
  one cycle, draws from the native simulation-determinism generator using `(clock, new cycle)`,
  and uses the existing native uniform range function. Time/second-to-ms/cycle arithmetic is
  checked. At-reset death becomes INACTIVE in the same epoch. A new epoch overrides every old
  state **before** recovery; equal epoch draws nothing, older epoch rejects, false draw stays
  INACTIVE and true draw becomes DUE at reset time. No pure proposal certifies a live-generation
  committed death or a current reset, and no production reset/death storage entry is exposed.

### Private schema / determinism packet (CP migration slot)

`game_boss_spawn_clocks` has primary key `(world_id UUID, channel_id UUID, spawn_key TEXT)`;
`spawn_key` is a nonempty qualified content key, not a creature name. `cycle` and `reset_epoch`
are unsigned-u64 `NUMERIC(20,0)` (zero permitted); private state encoding is SMALLINT DUE=1,
ALIVE=2, INACTIVE=3. `due_at BIGINT NULL` is nonnegative **Unix milliseconds**, never a process
owner-timer instant. `owner_generation NUMERIC(20,0) NULL` is a nonzero native scope generation.
DUE has due_at and no owner; ALIVE has owner and no due_at; INACTIVE has neither. Unknown states,
invalid ranges and malformed state/null combinations reject on both load and write. WorldId and
ChannelId keep separate native types and separate fixed key fields. The owned PG test creates
this isolated table after the actual native migrations; no production DDL/permissions are added.
Final admission/initialization must enforce BOSSRAID0-RL-06's 1,024 clocks/channel, the current
qualified content policy and the real initial-spawn/reset source; the single-row primitive does
not invent provisioning. Each call changes at most one row, below the 100-row World job cap.

The only generator is `oteryn_simulation_determinism::deterministic_decision_u64`; no PRNG,
wall-clock entropy or second source of randomness exists. Its root is SHA-256 of the literal
`oteryn:boss-clock:v1`, WorldId bytes, ChannelId bytes, spawn UTF-8 byte-length as u64 big-endian,
then spawn bytes. Native DecisionOccurrenceId bytes are eight zero bytes plus the new cycle (or
reset epoch) as u64 big-endian; native purpose is `boss_respawn` or `boss_at_reset`, draw index 0.
The accepted seed coordinates are thus stable across owners/restart and distinct across scopes,
spawns and purposes. Chance is a validated exact content ratio (numerator/denominator, 0..1),
not an invented percent/ppm unit. Range selection reuses `spell::uniform_draw` unchanged. These
are private encoding decisions for this child, not a new signed envelope or authority contract.

### Mandatory claim → physical realization handoff (not installed here)

A committed ALIVE clock is **not** proof that a physical actor exists. The future native Channel
owner caller must hold one owner lane for the clock from preparation through commit and install,
with an unresolved attempt keyed by `(World, Channel, spawn, cycle, reset_epoch, owner_generation)`.
The epoch is necessary for at-reset clocks: a reset can change the life without a committed death
incrementing cycle, and must not be mistaken for the previous epoch's install. Physical
realization must be idempotent under that exact identity and the independently still-current
`ScopeRuntimeFence`/work stamp. Losing CAS produces no realization. Never clear the pending
attempt merely because ALIVE committed; never publish ready while installation is unresolved.

On a definite uncommitted transaction, release the attempt. On failed install or uncertain commit,
retain it and reconcile the exact durable row plus the physical owner registry while holding the
same lane/current fence: if the exact actor already exists, complete once; if this exact claim is
proven committed and its actor is proven absent, finish **that retained attempt's** idempotent
install once. An ordinary same-generation ALIVE read or a fresh retry cannot authorize this.
If provenance/current ownership/physical outcome cannot be reconciled, keep the scope unavailable
and relinquish/fence the owner generation; only a new independently assigned generation can use
ended-owner recovery. A process loss loses its generation, not the durable cycle. The native
unresolved-commit owner pattern (`durability/spell_owner_commit.rs`, physical owner lanes) is the
existing integration precedent; no substitute commit/actor/death token is created here.

### Exact remaining native seams

1. The actual caller must hold its retained `CharacterRecoveryStore::seal_current` /
   `ReconciledCharacterAuthority` guard and run in `begin_semantic_transaction` plus semantic
   deadline/commit. This in-transaction primitive validates the record against current DB
   admission, but cannot replace the externally retained seal or own the caller's final commit.
2. The real WorldReset owner/current epoch and reset time are not installed as an independently
   readable durable source on this base: `reset_epoch` currently appears in map-item MINT
   reservations/receipts, not a current-reset authority table. No request or prior boss clock is
   substituted for that source. Wire reset draw+CAS before at-reset claim/realization only when
   that concrete native source exists; the production entry here is respawn-policy only.
3. Committed boss death has no consumer capability here. Its DUR-03 A4 live-generation death
   descendant must persist the planned cycle change in the **same** committed death transaction;
   a callback, retained history row or new request token cannot certify a committed death.
4. Runtime ownership install/reconciliation, qualified boss spawn definitions, portable-due to
   `OwnerTimerLane` scheduling, migration/permissions, module registration, reset/death hooks and
   any event/audit registration remain CP slot work. No shared file or content is edited. Actor
   health/fight state is not persisted/restored; actual ended-owner realization is full health.

### Validation scope

Tests exercise exact due boundary, generation recovery without cycle advance, reset override of
all three old states, once-per-epoch behavior, inactive reset death, deterministic per-clock draw,
range and overflow, and malformed shapes. The isolated PG17.6 case uses actual native node
registration, scope assignment/replace and retained recovery admission; checks stale/wrong scope,
wrong secret, recovery binding mismatch, assignment history/high-water drift, pending-claim
rollback, concurrent CAS loser, same-generation retry suppression, ended-generation recovery,
native node revocation and new-connection reload. It does not install
actors, current resets, a death descendant, production grants or a complete semantic owner caller.
No player session is applicable to scope-only activation; no CharacterRevision is advanced.
Native node registrations have no TTL here: revoked currentness is tested instead of inventing an
expiry. Reset/time validation is pure until its actual current-source seam is integrated.

Exact commands, retained raw outputs, source manifest, scope and local patch:
`/workspace/oteryn-prep/monster-meta-1-evidence/`. The effective native qualification uses a
byte-verified disposable snapshot with only the literal boss module registration added to its
copy of `durability/mod.rs`; the draft's shared source remains unchanged. The owned tests reuse
`tests/support/bestiary_postgres_harness.rs` unchanged, with genuine cfg(test) native Root,
recovery store, node and scope assignment APIs. No native type substitute or new authority
callback exists. The initial production-rlib setup failed because its test-only Root constructor
was unavailable; that failure and unused exact-excerpt attempt are retained as setup evidence,
not behavioral RED or a qualification pass. No TLS/service configuration was changed.

Production role compatibility, the final semantic caller/deadline/commit, actor realization,
reset/death integration, whole-family E2E and independent exact-head review remain unqualified.
No worker commit, push, ready, merge or review trigger is authorized or performed.

Focused behavioral RED → GREEN was executed on genuine PostgreSQL17.6: a one-field native
assignment high-water drift originally allowed this draft's claim despite valid live owner
facts; the existing native `game_runtime_scope_assignment_history_valid()` check now rejects
it before any clock write. Its runtime EXECUTE/SELECT/lock permissions already exist in migration
0006; no production grant is added. Final native test result: **5 pure + 1 PostgreSQL PASS**,
0 failed/ignored; 3,179 unrelated native unit tests were filtered, not claimed as run. The initial
production-rlib constructor and the later invalid fixture DB-tag failures were setup failures,
not behavioral RED. Native Clippy/format/governance outcomes are bound by the retained validation
manifest; no runtime readiness/actor installation or whole-family acceptance is claimed.

## Owned cooldown/eligibility source batch — 2026-10-16

Author: family worker `/root/boss_raids`, sole writer of this packet on
`skeleton/monster-meta-1`, starting from `8e2391e938f2dfcdab5b1579811925680c88b904`.
The direct family allocation and CP review `6216368031` authorize owned-source completion;
the newer ROOT-AJ-F instruction confirms that the migration slot blocks its installation,
not preparation of the accepted logical storage behavior. This is AUTHORING, not a frozen
candidate. The earlier report above describes only its original clock subset.

Source manifest: `apps/game-server/src/durability/boss_encounter.rs`,
`apps/game-server/src/durability/boss_encounter_tests.rs`, and this record. No other source,
migration, module registration, content, registry or runtime wiring is changed.
EVENTS-RAID-1 remains the sole raid scheduler/content owner; no second raid core is added here.

### Implemented source boundaries

- BOSS_RAID0 §9's private in-transaction respawn eligibility preparation validates the retained
  recovery record against current DB admission, holds the admission locks, validates native
  assignment history/high-water, and invokes the native `assert_item_only_fence`. This checks
  the admitted Channel session, Character lease, owner generation/node possession, admission
  guards and locked live Character root/current interpretation. This proves only contributor
  currentness, not a qualified death or admitted boss content. The function, error and outcome
  are private; no production entry/caller is exposed. It neither compares nor advances
  CharacterRevision (§12). DB wall-clock milliseconds are sampled after those locks.
- The singleton is keyed by `(CharacterId, boss key)`, without Channel or derived window IDs.
  Missing rows are initialized under the Character root lock; `FOR UPDATE` serializes the
  exact `now >= next_eligible_at` decision and `now + spawn min_s` update. A committed claim
  is never a reward, Bosstiary, actor realization or death capability.
- Raid/admission reference eligibility, at-reset eligibility, group cooldowns and conditional
  compensation are private SQL **data** helpers. There is no exported entry accepting raw
  reset epochs, boosted booleans or admission IDs as current authority. Their behavior follows
  §6.3/§9: one reference claim, strictly newer credited epoch, exact cooldown boundary, prior
  epoch boosted exception without rewriting a read, and refund only the matching admission.
- Group cooldown storage validates at most 15 distinct participants for one encounter, takes
  Character roots and rows in deterministic order and preflights every row before any write.
  An ON_COOLDOWN refusal can safely be committed without a partial group write. Exact replay
  preserves the stored admission binding; changed or partial admission replay fails closed.
  Even an expired cooldown cannot be rewritten with an older proposed reset epoch.
- Native post-grace/Instance sessions remain rejected. §7 explicitly requires post-grace to
  fail closed until the actor's own accepted death writer supports it. These helpers do not
  extend Channel authority to an activity instance or certify a current WorldReset.

### Pending production migration and consumer contract

The owned PG test DDL is the proposed private physical encoding of these accepted logical rows:

| Table | Key | Remaining fields |
|---|---|---|
| `game_character_boss_cooldowns` | Character UUID + encounter ProductionKey text | nonnegative Unix-ms BIGINT `available_at`, existing admission UUIDv7, unsigned-u64 NUMERIC `reset_epoch` |
| `game_character_boss_eligibility` | Character UUID + boss ProductionKey text + existing firing/admission UUIDv7 | none; successful reference claims are retained |
| `game_character_boss_open_world_eligibility` | Character UUID + boss ProductionKey text | nonnegative Unix-ms BIGINT `next_eligible_at`, nullable unsigned-u64 NUMERIC `last_credited_epoch` |

Character UUIDs reference native roots; keys use the native ProductionKey bound (512 bytes).
Unsigned epochs allow zero and end at 18446744073709551615. NULL `last_credited_epoch` means no
credit yet, so epoch zero's first credit works without a sentinel or invented epoch. Cooldowns
are checked against the accepted 30-day cap; time arithmetic is checked before any row write.
No table grants, mutation/immutability guards or production DDL are installed by this batch.
The numbered migration/role compatibility and final guards remain CP slot work, alongside the
existing clock-table migration contract above.

The real death owner must lock its existing occurrence and check its immutable receipt before
calling, compose eligibility + reward draw + Bosstiary receipt in that **same** semantic
transaction, retain its native recovery seal, and own semantic deadline/commit/reconciliation.
A timeout/unknown commit must reconcile the death receipt; a fresh eligibility call is not
receipt replay. The real lever owner must independently prove current reset/boosted facts,
level/quest/busy gates and group fences, then compose ADMITTING allocation + cooldown writes.
Compensation requires its independently locked no-arrival/ABANDONED predicate. Those native
callers and current fact producers remain outside this source packet and are not fabricated.

Compile-only qualification patch, permitted by this card's module-registration rule, for a
private disposable full-crate copy of `durability/mod.rs` (not installed on the draft):

```rust
#[allow(dead_code, reason = "BOSS-1 source qualification before CP runtime wiring")]
pub(crate) mod boss_encounter;
```

### Validation status

Owned-file rustfmt, `git diff --check` and `validate_governance.py` passed during authoring;
`tools/agents/tests` passed 124 tests (0 failures). The new PostgreSQL17.6 cases use the
existing unchanged `bestiary_postgres_harness`, real native node/assignment,
session and recovery fixtures. They cover rollback after a later SQL failure, concurrent claim
serialization/reload, changed native fences, exact time/epoch boundaries, group refusal/replay,
and admission-matching compensation. Raw-reset/cooldown cases qualify data behavior only.
The concurrency case uses one native Channel fence twice and qualifies singleton serialization,
not an actual cross-channel death or transfer integration. Independent review found and the
author repaired two material boundaries: the initially exported wrapper proved contributor
currentness without a qualified death/content binding, so it is now private preparation; an
expired cooldown could accept a lower supplied epoch, so epoch regression now rejects before
any write. These new cases have **not yet been compiled or run**: the family lead schedules the single
Rust build slot. The earlier 5+1 results do not qualify this changed candidate. Full reward,
room, reset, runtime and production-role integration remain unqualified; no acceptance box
is completed, and no commit, push or paid-review trigger has been performed by this worker.

### Native qualification compile repair — 2026-10-16

Returned from frozen `13935a6b40e5e20a80c5318868d1fbb0aa3032a3` to AUTHORING.
The lead's native listing compile failed with E0061 at the new wrong-secret fixture:
`NodeRegistrationFact::new` accepts exactly `(node_id, registration_revision)`.
Removed only the duplicate revision argument. The negative still preserves the genuine
node identity/revision and changes only the bootstrap secret. No authority, SQL, production
wiring or expected assertion changed. The failed native-list log
`20261016T163257.266570Z-44857-monster-meta-assembly-native-list.log` is retained.
Native listing, ten focused cases, strict clippy, full package tests and RED/GREEN remain
pending on the repaired successor; the earlier fmt-only failure is also preserved.
