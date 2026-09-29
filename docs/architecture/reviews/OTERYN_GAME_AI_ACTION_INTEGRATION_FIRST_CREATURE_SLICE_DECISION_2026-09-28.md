# GAME-AI-01 AI Action Integration: first creature slice decision

- Decision: `GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1`
- Status: **CANDIDATE with owner decisions D53-D57 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Parent gate: `GAME-AI-01` (`ACCEPTED`, `LIFECYCLE_CLOSED`, implementation `NOT_STARTED`)
- Extends: `OTERYN_GAME_AI_BOOTSTRAP_SLICE_OWNER_DECISION_2026-08-25.md` (the bootstrap slice).
  The bootstrap stays valid; this decision adds the integration it deferred (§3).
- Owner decisions posted: #162 comment 5870406825
- Admission baseline: `main@356673f`
- Related: VSL-COMBAT-01 (§6, §14-§16, §24), FND-03 §10, spell wire contract (SPELL-D2), #1079
  (D52 death identity), #162 Combat C allocation 5869823681
- Runtime, migration, protocol and production authority: **NONE**. Each child in §5 needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The owner wants a real monster: one that spawns, notices the player, chases, attacks and comes
back after death. The bootstrap slice deliberately produced only AI-local results: `IDLE` or
`ACQUIRE_CANDIDATE`, plus at most one path proposal. It forbade Movement adoption, deferred the
Ability boundary (GAME-AI-XD-01) and excluded spawn and timers (bootstrap §3, §5, §6, §10).

What is the smallest integration that makes the native entry-room rat behave like a monster,
without breaking the accepted GAME-AI-01 authority rules?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D53 | The first creature chases a perceived player, bites when adjacent, and wanders when no player is in range. Behaviour is deterministic and bounded; no Reference AI parity is claimed. | "Goni, gryzie i chodzi" |
| D54 | Creature damage never takes a player's HP below 1 in this slice: no player death. | "HP nie spada poniżej 1" |
| D55 | The creature respawns on its spawn cell after a delay, as a new actor-local generation. | "Tak, po czasie" |
| D56 | AI is built in parallel with Combat C-F: this decision and the owner timer lane first; AI runtime code in `foundation/**` is serialized with the Combat D lease. | "Równolegle z walką" |
| D57 | Resource maxima (§4.9): up to 16 spawn sources per scope and up to 4 creatures per spawn now, so that more monsters do not need a new limit decision; 3 occupancy retries. | "Więcej spawnów od razu" (16 spawns, 4 creatures per spawn) |

## 3. Facts

**PROVEN** (main `356673f`)

- GAME-AI-01 is accepted (`GLOBAL_ARCHITECTURE_DECISION_REGISTER.md:49`;
  `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md:97-125`). The candidate
  file keeps its historical `PROPOSED` header (bootstrap §2).
- Accepted GAME-AI-01 rules this decision keeps:
  - one current Channel owner and generation per AI actor; workers, scripts and clients never
    commit (candidate §3);
  - staged, all-or-nothing resolution; over budget means zero mutation (§6);
  - no universal fixed tick; think work arrives as FND-03 owner timers (§7);
  - canonical target selection with a stable tie-break (§8);
  - path results are proposals that the owner revalidates (§10).
- `apps/game-server/src/ai/` implements the bootstrap (perception, resolution, path proposal) but
  is not declared in `lib.rs`. Only `tests/ai_bootstrap.rs` compiles it.
- Movement rejects creature slots (`runtime_actor_carrier.rs:1057-1083`,
  `MovementCreatureUnavailable`; `movement.rs:1-5`). `MovementOwnerTurn::try_step` is an
  `ExactActorRef`-keyed server-side step (`movement.rs:184-258`).
- Ability has `ProposalSource::Ai` and `AiAbilityAdapter` (`ability/intent.rs:4-9`, `:120-126`),
  but the issuer is a string atom, not an `ExactActorRef` (`intent.rs:31`, `:52`;
  `exact_actor_resolution.rs:8-35`). Damage commits only to creature slots (`NotCreature`,
  `runtime_actor_carrier.rs:1456-1464`); players have no HP.
- The carrier holds at most one creature (`runtime_actor_carrier.rs:1159`), with one committed
  damage receipt (`:1423-1480`). `admit_creature` is test-only.
- `native_entry_room.json` declares `oteryn:creature/rat` (behaviour `passive-idle`, ability
  `bite`) and `oteryn:spawn/entry-rat` on `cell/entry-east`, population 1,
  `EphemeralScopeReset`. No runtime code realizes the spawn.
- `cell/entry-east` is the east cell of the accepted Movement step-and-return proof, which
  VSL-COMBAT-01 §24.2 requires to be preserved.
- FND-03 §10 defines the timer contract: owner-scoped inputs, a scheduling key bound to scope,
  ownership generation and target generation, staleness rejection, no zero-delay recursion, and
  a declared catch-up policy per timer family. No timer lane exists in code.
- VSL-COMBAT-01 §15: the first combat slice accepts a bounded deterministic fixture behaviour
  that uses the real Ability path and claims no Reference parity.
- SPELL-D2: player HP, mana and soul are runtime-actor-local and non-durable in V1; they arrive
  with spell step P3b-2.
- D52: a creature death key includes `ActorLocalGeneration`, and actor slot generations are never
  reused (`runtime_actor_carrier.rs:1310-1317`).

**UNKNOWN**

- Reference rat values (speed, bite interval and damage, perception range, respawn delay). These
  are content inputs; §4.8 routes them.

## 4. Decision

### 4.1 Scope

The first content is one creature definition (the rat) in the native entry room, on one Channel,
population 1. The engine supports the D57 envelope from the start: up to 16 spawn sources per
scope and up to 4 creatures per spawn. The same code must not branch on the rat's identity: it
dispatches on the typed creature definition, so a second definition works without code changes
(VSL-COMBAT-01 §24.2 genericity). The carrier's current one-creature limit
(`runtime_actor_carrier.rs:1159`) is lifted to this envelope in AI-2.

### 4.2 Channel owner timer lane (FND-03 §10)

This is the first implementation of the FND-03 timer contract in `ChannelRuntime`. It serves AI
now and spell cooldowns and regeneration later.

- A timer is an owner-scoped input. Its key is (scope, ownership generation, target actor and
  actor-local generation, timer family, timer occurrence identity, due deadline, the scheduling
  resolution's `RuntimeExecutionOrdinal`, deterministic within-resolution sequence). Equal
  deadlines order by the scheduling ordinal, then the sequence (FND-03 §10.1). When due, the owner
  accepts it as a normalized input with a new execution ordinal.
- Occurrence identities: a think occurrence is (creature `ExactActorRef`, per-actor think
  sequence number); a respawn occurrence is (spawn source, cell, the dead actor's
  `ExactActorRef`), and each retry adds its attempt index 1 to 3. The same identity is never
  scheduled twice.
- A timer whose ownership generation or target generation no longer matches is dropped without
  mutation (FND-03 §10.3).
- Time comes from an injectable owner clock (`SemanticTimeMicros`), with a deterministic virtual
  clock in tests (FND-03 §11).
- Timer families in this slice:

| Family | Policy (FND-03 §10.5) | Bound |
|---|---|---|
| AI think, one per live creature | `SKIP_TO_LATEST` | at most one pending per creature |
| Respawn, one per dead creature of a spawn | `DEADLINE_STATE` | at most the spawn's population pending |

- Both timers are process-local. They need no durable state: the spawn is `EphemeralScopeReset`,
  so a restart realizes the spawn again at Channel activation (FND-03 §9 applies only to timers
  whose semantics cross a process lifetime).

### 4.3 Spawn realization and respawn (D55)

- A spawn declares its population (1 to 4) and one placement cell per creature. Placement never
  searches beyond the declared cells.
- At Channel activation the owner realizes each spawn of the active content generation: it admits
  one creature actor per declared cell, each with a fresh actor-local generation, bound to the
  creature definition and content revision.
- Live plus pending creatures of a spawn never exceed its population.
- After a creature's death commits (D52), the spawn schedules one respawn timer for that creature's
  cell. When it is due, the owner admits a new actor there with a new actor-local generation. The
  dead actor's generation is never reused, so its death key cannot recur.
- If the cell is occupied when the timer is due, the attempt is postponed by the spawn's retry
  interval, at most `AI01-SPAWN-OCCUPANCY-RETRIES` (3) times, under the same respawn occurrence
  with attempt indices 1 to 3 (GAME-AI-01 §13.2). It never displaces an actor and never picks
  another cell.
- **Terminal disposition.** When the third retry fails, that respawn occurrence ends as `SKIPPED`,
  with nothing admitted. The spawn then schedules one new respawn occurrence for the same cell,
  due one full respawn delay later, with a new identity (spawn source, cell, the skipped
  occurrence's identity, successor index). It is bounded like the first: one pending occurrence
  per cell, at most 3 retries, and a terminal `SKIPPED`. A content revision change or scope
  retirement cancels pending occurrences (`CANCELLED`). There is never more than one pending
  occurrence per cell, so the chain is bounded in concurrency and runs at most once per respawn
  delay.
- A despawn or scope retirement creates no death and schedules no loot (VSL-COMBAT-01 §7).
- The respawn delay and retry interval are content inputs of the spawn definition (§4.8).

### 4.4 Behaviour (D53)

Each think timer runs one bounded, deterministic resolution over an immutable snapshot of the
owner state:

1. **Attack.** If a legal target player is adjacent and the bite is not cooling down, draw the
   bite chance. On success, propose one bite intent (§4.6). On a failed draw the resolution ends
   idle for this think: it neither chases nor wanders.
2. **Chase.** Otherwise, if a target player is perceived, propose one cardinal step along a path
   proposal toward that player (§4.5).
3. **Wander.** Otherwise, with the definition's wander chance, propose one random cardinal step;
   otherwise stay idle.

- Perception: players within the definition's perception range in the same Channel scope,
  canonicalized by stable identity; the nearest wins, ties by stable identity (GAME-AI-01 §8).
- **Re-entry protection.** A player inside the active four-second PvE re-entry protection window
  (`DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`; GAME-AI-01 §9) is not a legal attack
  target. The creature may still perceive and chase that player, but it proposes no bite, and no
  bite is buffered for when protection ends. The creature keeps its current target.
- Randomness (wander direction, wander chance, bite chance) draws from a SIM RNG stream bound to
  the think occurrence. A retry of the same occurrence never redraws.
- A resolution proposes at most one action. The next think timer is scheduled from the
  definition's think interval whatever the outcome.
- The owner rejects every proposal it cannot legally apply; a rejection changes nothing except
  scheduling the next think.

### 4.5 Creature Movement adoption

This supersedes the bootstrap's `movement_adoption: FORBIDDEN_IN_BOOTSTRAP` for this slice only.

- Creature slots become Movement-capable actors of the same Movement owner. AI never writes a
  position: it submits one step for its own `ExactActorRef` through the Movement owner turn, which
  revalidates legality, collision and occupancy exactly as for a player.
- At most one step per think resolution. A path proposal is used only for its first step and is
  revalidated against the current owner state before use.
- Route validity may not depend on doors or other Interaction-owned dynamic facts
  (GAME-AI-XD-02 stays deferred). A route that would cross such a cell fails closed.
- **Movement proof preservation.** The spawn must not sit on the accepted start/east proof cells.
  The Content/Seam owner revises the room so that the spawn cell and the creature's wander area
  are outside the proof path, and the qualification keeps the start/east step-and-return green
  (VSL-COMBAT-01 §24.2). That room revision needs its own allocation.

### 4.6 AI to Ability boundary (closes GAME-AI-XD-01 for this slice)

- AI proposes a typed intent through the existing Ability pipeline with `ProposalSource::Ai`. The
  issuer is the creature's `ExactActorRef` (owner, generation, actor-local id and generation),
  not a string atom. Ability resolves issuer and target exactly and rejects either if stale.
- Ability checks legality exactly as for a player intent: the issuer is alive, the target is a
  live player in range 1, the target is not inside the PvE re-entry protection window (revalidated
  at commit from the owning protection fact), and the bite's interval has elapsed. The effect commits through the
  current owner in the same mutation as the cooldown.
- The intent's occurrence identity is (creature `ExactActorRef`, think occurrence). A retry of
  the same occurrence returns the first result and never applies damage twice.
- AI reads the result only through its next snapshot. It keeps no success-dependent state.

### 4.7 Player damage floor (D54)

- Creature damage lowers the player's runtime-actor-local HP (SPELL-D2), but never below 1. The
  committed effect records the applied (clamped) amount.
- No player death, corpse, respawn or loss exists in this slice. The death consequences decision
  stays separate and Reference-gated (VSL-COMBAT-01 §16). This is a declared V1 limitation, not a
  Reference behaviour.
- This part depends on spell step P3b-2 (player vitals). Until then the bite may be exercised only
  against a test vitals fixture, never as a shipped behaviour.

### 4.8 Content inputs

- The rat definition changes from `passive-idle` to a hostile behaviour that carries: perception
  range, think interval, wander chance, bite interval, chance and magnitude, and speed. The spawn
  definition carries the respawn delay and occupancy retry interval.
- These values come from the monster authoring pipeline (`OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md`
  `behavior.targeting`, `behavior.movement`, `behavior.attacks[]`), with the same source rule as
  spells. Code never hardcodes them. A missing value fails content validation.

### 4.9 Resource rows (D57)

The slice makes bootstrap-excluded dimensions reachable (bootstrap §5). Owner-selected maxima
(D57):

| Row | Maximum | Failure |
|---|---|---|
| `AI01-SPAWN-SOURCES-PER-SCOPE` (AI-RL-11) | 16 | content validation rejects |
| `AI01-SPAWN-POPULATION` (AI-RL-11) | 4 live or pending creatures per source | content validation rejects; admission rejects |
| `AI01-SPAWN-PLACEMENT-CELLS` (AI-RL-12) | 4 declared cells per source, one per creature | content validation rejects |
| `AI01-SPAWN-OCCUPANCY-RETRIES` (AI-RL-13) | 3 per creature per respawn window | wait for the next window |
| `AI01-PENDING-TIMERS-PER-ACTOR` (AI-RL-06) | 1 think timer per creature actor | reject the second schedule |

Derived bounds, which the registration must state and test:

- creatures per scope: 16 × 4 = 64, within the registered `AI01-ACTIVE-ACTORS` (256);
- pending AI timers per scope: 64 think plus 64 respawn = 128;
- think resolutions per owner cycle: at most 64, each within `AI01-EVALUATION-WORK` (8).

Placement search beyond the declared cells stays unreachable.
- Repath windows (AI-RL-10) stay unreachable: one path request per think, within the registered
  `AI01-PATH-REQUESTS-PER-ACTOR` (2).
- Registration goes through the `RESOURCE_LIMITS_REGISTRY.json` single-writer lease, with max and
  max+1 tests. A larger value needs a new owner decision.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on | Serialization |
|---|---|---|---|
| AI-1 | Channel owner timer lane and injectable clock (§4.2) | this decision | `foundation/**`; serialize with Combat D's exclusive lease |
| AI-2 | Spawn realization, respawn, the carrier's creature envelope, creature Movement adoption (§4.3, §4.5) | AI-1 | `runtime_actor_carrier.rs`, `movement.rs`, content activation; serialize with Combat D; the room revision goes to the Content/Seam owner |
| AI-3 | Compile `ai/` into the server; perception, chase and wander over owner snapshots (§4.4) | AI-2 | `lib.rs`, `ai/**` |
| AI-4 | Typed AI issuer, bite through Ability, player HP floor (§4.6, §4.7) | AI-3, spell P3b-2, Combat D's multi-hit creature receipts where it overlaps | `ability/**`; serialize with P3b-2 |
| E (Combat) | Client observation of creature appear, move and disappear | Combat child E protocol lane | protocol registry lease |

Resource rows (§4.9) register with AI-1 and AI-2 under the registry lease.

## 6. Rejected options

- **Stationary rat.** The owner chose chase and wander (D53).
- **AI writes positions or damage directly.** It violates GAME-AI-01 §3 and VSL-COMBAT-01 §15.
- **A global fixed tick for AI.** It violates FND-03 §8.4 and GAME-AI-01 §7.
- **Durable respawn timers.** The spawn is `EphemeralScopeReset`; activation realizes it again.
- **Player death now.** The owner chose the HP floor (D54); death consequences need their own
  Reference-gated decision.

## 7. Decision test

- **Must decide now:** YES. The owner wants monsters in parallel with Combat (D56), and the timer
  lane is shared with spell cooldowns and regeneration.
- **Minimum sufficient:** one think timer per creature, one step or one bite per resolution,
  declared spawn cells only (the 16 × 4 envelope is the owner's choice, D57); existing Movement and Ability owners; no new framework, pathfinding library or
  durable state.
- **Superseding evidence:** a creature that needs threat memory, leashing, summons or group
  behaviour; more than 16 spawns per scope or 4 creatures per spawn; Reference AI parity work; player death consequences.
- **Deliberately not decided:** AI framework or representation, the production pathfinding
  algorithm, threat and aggro memory, leash, flee, summons, scripts, bosses, loot and XP values,
  player death.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source: "#162 owner direction 2026-09-28 (monsters in parallel)"
owner_decisions: [D53, D54, D55, D56, D57]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md
extends: docs/architecture/reviews/OTERYN_GAME_AI_BOOTSTRAP_SLICE_OWNER_DECISION_2026-08-25.md
resource_values_changed: true   # §4.9, registered by AI-1/AI-2
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false   # until this decision is protected-integrated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (AI authority, timers, Movement and Ability boundaries)"
implementation_lanes: [AI-1, AI-2, AI-3, AI-4]
required_revalidation:
  - "AI-1: stale timers (ownership or actor generation changed) mutate nothing; equal deadlines from different resolutions order by scheduling ordinal then sequence; an occurrence identity is never scheduled twice; SKIP_TO_LATEST collapses missed think timers; a virtual clock drives the tests"
  - "AI-2: activation realizes each spawn once; live plus pending never exceed the population (4 accepted, 5 rejected); 16 spawns accepted and 17 rejected; respawn uses a new actor-local generation; an occupied cell postpones at most 3 times, then the occurrence ends SKIPPED and one successor occurrence is scheduled a full delay later; the start/east Movement proof stays green"
  - "AI-3: identical snapshots give identical decisions under shuffled order; chase moves one revalidated step; wander draws from the occurrence RNG and a retry never redraws"
  - "AI-4: a stale creature issuer is rejected; one think occurrence applies at most one bite; a protected (re-entry window) target gets no bite and none is buffered; a failed bite-chance draw ends the think idle; player HP never drops below 1; AI never mutates Movement, Ability or vitals directly"
remaining_unknowns:
  - Reference rat values (routed to content, §4.8)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates AI-1."
```
