# WEATHER-1

```yaml
task_id: WEATHER-1
title: "WEATHER-1: weather state and change events (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/weather-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 WEATHER-1 (row 17); wave 2"
class: "impl"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_weather.rs (new)
  - apps/game-server/src/world_weather_tests.rs (new)
  - crates/protocol-oteryn/src/world_weather.rs (new)
slot_only_paths:
  - connection.rs (`world_clock_delta` l.1355 pattern)
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "LOOP-CLOCK-1 slice: MERGED #2071"
  - "CLOCK-1 residue: MERGED #2114"
  - "registry order ITEM-LOOK-1, SEC-1, WEATHER-1: BLOCKING: ITEM-LOOK-1 and SEC-1 not merged; SEC-1 gated on SEC_CLIENT01 -> CP decides whether WEATHER-1 may pass SEC-1"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

World weather state and change events (owner direction 2026-10-13: in scope).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-CLOCK-1 slice | MERGED #2071 |
| CLOCK-1 residue | MERGED #2114 |
| registry order ITEM-LOOK-1, SEC-1, WEATHER-1 | BLOCKING: ITEM-LOOK-1 and SEC-1 not merged; SEC-1 gated on SEC_CLIENT01 -> CP decides whether WEATHER-1 may pass SEC-1 |

**BLOCKING:** registry order ITEM-LOOK-1, SEC-1, WEATHER-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/world_weather.rs (new)
- apps/game-server/src/world_weather_tests.rs (new)
- crates/protocol-oteryn/src/world_weather.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs (`world_clock_delta` l.1355 pattern)
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `world_clock.rs` (pure World clock)
- `connection.rs` `world_clock_delta` (l.1355) / `world_clock_utc_ms` (l.504)
- `capabilities.rs` `WORLD_LIGHT` (l.65) pattern

## Inseparable bundles

wire+registry+capability+handler+tests.

## Acceptance criteria

- [ ] Weather deterministic from World seed and clock; no brief: builder writes packet section first, architect review.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

WORLD_INTERACTION0 §11 (clock); Reference weather. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 WEATHER-1 (row 17).

## Builder instructions

- Work on branch `skeleton/weather-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/weather-1
blocker: registry order ITEM-LOOK-1, SEC-1, WEATHER-1
next_action: CP rechecks BLOCKING rows
```

## Required first design packet — 2026-10-15 (PROPOSED; not architect approval)

Current lease: **DESIGN_PACKET_ONLY**, this record only, branch `skeleton/weather-1`
source `172ad99048d947f055e46fc1bbfd1983d5303830`. No product source, contract,
registry or content is edited. The original skeleton and its historical status
are retained. Its acceptance criterion explicitly requires “builder writes
packet section first, architect review”; implementation waits for that review.
This packet is a concrete recommendation and allocation handoff, not a request
to treat the proposed values as already accepted.

### Accepted constraints and released gate

- Protected-main `ac9f172836f00226b68088aa04db95ccf1ab994e`,
  `OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` §2 item10:
  **“WEATHER-1 is visual only, with no gameplay effect.”** §3 fixes registry
  order **ITEM-LOOK-1 → WEATHER-1 → SEC-1**. The old SEC-before-WEATHER question
  above is released; this does not allocate numbers, merge a prerequisite or
  approve this design. CP still owns the actual registry/integration turn.
- The task requires determinism from the World seed and clock. World and Channel
  remain distinct; moving process/owner/Channel must not reseed a World result.
- SIM-DETERMINISM-01 §§10–12/15 require stable semantic occurrence/purpose,
  revision binding and explicit recorded/injected time facts. Neither live wall
  time during replay nor a new process seed is an acceptable substitute.
- No accepted brief supplies weather presets, probabilities, interval, visual
  assets, serialization or resource rows. Every value below marked proposed
  requires architect/owning-contract review; no content default is inferred.

### Exact native residue and reuse boundaries

| Actual source | Available interface / missing qualification |
|---|---|
| `apps/game-server/src/world_clock.rs:91` | `WorldClockSource::{tibian_minute,reading}` uses checked elapsed UTC/2,500ms modulo1,440; its epoch is private. The wire reading loses the absolute day/slot. `now_utc_ms` anchors wall time once plus monotonic elapsed. Weather must consume the same input/tick's normalized recorded fact, not call another clock or infer a day counter from the modulo minute. |
| `gameplay_transport/connection.rs:504`,`:1355`,`:1772`,`:2272` | `FreshAdmissionAuthority::world_clock_utc_ms`, domain snapshot, `world_clock_delta` and pacer are real transport patterns. The current composition constructs the clock with DEFAULT epoch0 while no ruleset field exists. That fallback and a typed RNG root are not proof of an active qualified World-weather seed/clock binding. Snapshot continuity changes only after the whole snapshot succeeds. |
| `crates/simulation-determinism/src/lib.rs:288`/`:338` | `GameplayDecisionRoot`, `DecisionOccurrenceId` and `deterministic_decision_u64(root,occurrence,purpose,draw_index)` provide the actual primitive. They accept data; they do not resolve/prove a current World seed, config revision or owner. Preserve profile/purpose isolation; do not copy the SHA/RNG algorithm or use content digest/WorldId/NodeId as a substitute seed. |
| `crates/protocol-oteryn/src/world_clock.rs` | Strict bounded snapshot/change codec exists for the clock (9-byte bound), but its uint32/default-zero helper is not automatically sufficient for a weather u64 slot, fixed binding bytes and presence-aware union. Weather needs its own reviewed codec/domain; never overload the existing clock IDs. |
| `crates/session/src/lib.rs:128`,`:191`,`:1266`,`:1377` | Capability selection, generic gated snapshot/revision stores and ordered deltas exist. The default client capability set does not include World light; no typed weather/clock consumer is present. Merely appending a server capability will not route weather to the client scene. Resume selection cannot widen. |
| `apps/client/src/{play,scene}.rs`, `crates/renderer/src/{batch,scene_gpu}.rs` | The shell coalesces pushed position/entity/marker state; Scene owns tile/sprite batches. The renderer samples the atlas with alpha blending, bounded at81,920 quads/batch and32 bytes/quad. There is no weather store, admitted preset table or weather pass. Reuse the atlas/batch path after its owner qualifies assets and draw ordering; do not claim current weather rendering or add a particle engine by assumption. |

The current World-config/seed admission and replay-time provenance seam must be
provided by its actual owner before feature offer. This inspection establishes
no active production World-weather seed binding. Fixture byte arrays, the clock
DEFAULT constant, immutable admission records or a caller's `is_current` boolean
cannot supply one. A read-only absolute elapsed/clock-binding API in the clock
owner and real current-owner/session checks in transport are cross-domain
allocation requirements, not edits hidden in the three original weather paths.

### Proposed model: direct slot evaluation (recommended)

Choose a **World-scoped deterministic visual preset**, replaced at fixed
absolute clock slots. Alternatives are a stateful Markov chain with durable
schedule/checkpoint/RNG cursor, or client-local random weather. The first adds
recovery/authority/storage without an accepted need; the second does not meet
World determinism. Direct evaluation needs no weather migration, history,
background timer owner, per-Character write or separate global authority.

Proposed v1 parameters, all awaiting review:

| Decision | Concrete recommendation | Alternative / review impact |
|---|---|---|
| Slot interval | 360 Tibian minutes =900,000ms, using the native2,500ms unit | A150,000ms Tibian-hour slot gives six times as many changes. No interval is currently accepted. |
| Policy | 1..16 explicit admitted visual presets and an explicit16-entry selector table referencing them | No built-in “clear/rain/snow”, weights, climate, region rules or default asset is supplied. Table repetitions permit1/16 selection shares without an unbounded sampler. |
| Transition | Instant replacement of the visual preset at a slot boundary | Crossfade/particle phases are deferred; they require additional previous-state/time/renderer semantics and are not necessary for the first visual-only model. |
| Observation | Full replacement `Unavailable` or `Active{binding_token,absolute_slot,preset_index}`; same semantic shape for snapshot/change | No client weather command, authoritative client time, partial patch, future-weather forecast, seed or asset path. |
| Wire/resource proposal | ≤64 payload bytes, fixed32-byte non-secret binding token, u64 slot, preset1..16; at most one current observation per World memo/session/client | These are proposed engineering caps, not registered resource values. Schema owner must prove the actual encoding fits before assigning the cap. |
| Pacing/render | At most one weather change per session per10,000ms (initial/replacement snapshot excepted); at most one additional visible quad/frame,32 instance bytes, within existing batch/atlas limits | Stable policy normally changes once/900s. Availability changes coalesce; no increase to global renderer limits or per-weather asset downloads is implied. |

A preset resolves only to a qualified client presentation descriptor/atlas asset.
Its texture/alpha/appearance must be authored and admitted by that owner; this
packet chooses no visible weather content. The preset does not alter server
World light, map visibility/LOS, collision, speed, combat, conditions, spawn
rates, loot or economy. Client rendering/animation/accessibility choices never
feed back into simulation or act as weather inputs.

### Proposed deterministic binding and calculation

The owning World configuration must bind actual WorldId, private World-seed/root
identity, immutable clock epoch/identity, weather-policy revision and its preset/
selector table. The owner must establish that these are independently current
when emitting an observation. Pure evaluation accepts them as semantic data;
possessing a cached binding or RNG root grants no owner/session authority.

For normalized UTC input/tick fact `t` and the same qualified clock epoch `E`:

1. Checked `elapsed = t − E`; before-epoch/missing time has no active reading.
   `k = elapsed /900000`, using the **absolute** elapsed value, never the modulo
   wire minute. No multiplication/addition overflow or private modulo-day counter.
2. Proposed occurrence encoding is a versioned, fixed-width preimage of WorldId,
   clock-binding identity, policy revision digest and big-endian u64 `k`, domain
   separated as weather. Its owner-reviewed derivation supplies the existing
   `DecisionOccurrenceId`; the bytes/derivation and golden vectors must be fixed
   before code acceptance, not replaced by a fixture-derived “current” context.
3. Call existing `deterministic_decision_u64` with the real World root, candidate
   stable purpose `weather_visual_v1` and draw_index0. The top4 result bits select
   one of the16 explicitly authored selector entries. One draw, no rejection
   loop/modulo bias, no dependence on how many sessions/Channels sampled it.
4. The selected entry yields preset1..N. At `k`'s exact end the next slot applies.
   A delayed tick computes only its current `k`: no history scan, catch-up burst,
   skipped-slot events, persisted schedule or RNG advancement loop.

The binding token sent to the client identifies **public revision identities**;
it must not contain or hash/expose the secret seed bytes as a substitute for a
proper seed-identity contract. It is correlation/compatibility data, not current
authority evidence. Seed/root retention and canonical occurrence derivation
belong to the existing World/SIM owner; none is activated by this packet.

Proposed v1 forbids silently changing the seed/policy/clock binding mid-slot or
rerolling an already issued occurrence under a newer revision. Keep the admitted
binding fixed for its activation; a future policy/seed replacement needs an
explicit owner-approved activation/cutover rule and replacement snapshot. Clock
epoch cannot be changed contrary to WORLD_INTERACTION0§11.1. A process restart,
owner generation or Channel move never creates a new semantic binding.

### Snapshot, change, replay and retirement semantics (proposed)

- **Initial/join/reconnect/resync/Channel transfer:** after current native scope/
  session admission and actual binding resolution, evaluate the current recorded
  time fact and send one full domain snapshot in the ordinary atomic snapshot.
  Initialize presentation/pacer only from the successful snapshot; do not replay
  old weather changes. Same World+binding+slot gives the same preset on every
  Channel; permitted clock skew can straddle a boundary without changing RNG.
- **Change:** compare the current semantic tuple with last sent tuple; unchanged
  tuple emits nothing. Keep at most one latest pending tuple and one in-flight
  observation per session, paced as above. Use the existing session's global
  sequence/domain revision chain with checked increments. Encode/snapshot/write
  failure follows native connection/unknown-write handling, never silently
  advances a separate weather ACK or creates a second ordering stream.
- **Unavailable:** no qualified binding at admission means no weather feature
  offer. If a still-current emitting owner loses a weather input/config after
  selection, propose one full `Unavailable` observation, clearing the visual
  overlay without selecting a fictitious default preset or changing gameplay.
  Recover only with an independently current binding. Loss of owner/session
  authority is different: retire/close via native Foundation; do not let a stale
  owner emit even an `Unavailable` packet. Availability fault/compatibility policy
  is an explicit architect decision below, not implemented behavior.
- **Replay/restart:** replay receives recorded time/config/seed identity and
  reproduces the original slot result. Live restart samples the current native
  fact with the retained actual World binding; natural elapsed slots may differ,
  but the same slot is never reseeded. No weather DB row is needed; existing
  seed/config/replay retention remains mandatory and is not proven by a snapshot.
- **Retirement/replacement:** cancel/discard the old owner's pending observation;
  independently revalidate active ownership and current session generation at
  publication. Old callbacks/leases cannot publish after retirement. Replacement
  owner recomputes from the same semantic binding, sends current snapshot and
  starts a fresh transport pacer under native continuity. Clear old client visual
  state on session end or committed replacement snapshot; no late frame revives it.
- **Monotonicity:** within a binding, a regressed time/slot is an input/provenance
  fault, not an instruction to rewind or clamp the visual model. Native current
  time source and replay context establish provenance; arbitrary future/request
  UTC numbers do not. Revision/sequence exhaustion refuses before publication.

### Proposed codec/client integration and decision gate

The codec must reject payload>64, unknown/repeated fields, invalid status,
wrong binding length, mixed Unavailable/Active fields, missing required Active
presence (slot0 is valid), preset0/>16/absent-from-policy and integer overflow.
No raw strings/assets/seeds or variable lists are carried. Exact protobuf fields,
capability/domain/snapshot/change numbers and registry resource rows are **not
allocated here**. A new domain uses the native FND snapshot/sequence framing.

Recommend depending on existing World-spatial context, rather than requiring the
client World-light capability: the proposed payload needs no client clock. The
protocol owner must approve that dependency. Complete server offer/gate/codec,
Session gated route plus typed observation/event, shell latest-weather coalescing,
Scene overlay and admitted compatible preset data **before** offering weather.
Session applies every ordered delta before presentation coalesces; never drop
network sequence entries. The client does not predict the next preset or treat a
local clock as authority. Unknown policy/preset or malformed payload fails closed
before application through the native session fault/resync path; never silently
choose “clear”. A fresh admission may omit the optional capability; a resume may
not alter its selected set. No-capability/older clients receive no weather domain.

Architect decisions required as one review of this packet:

1. Approve direct-slot model versus a justified stateful alternative, proposed
   interval, selector-table model, bounds and discrete visual replacement.
2. Name the actual World-config/private seed/replay-time binding owner and
   allocate its clock/SIM/current-owner seam. Missing active binding is a real
   activation gate, not permission to populate a constant or a synthetic callback.
3. Approve the Unavailable/compatibility behavior and frozen-binding/cutover rule.
4. Allocate client Session/Play/Scene/render-presentation ownership and qualifying
   preset assets; approve the minimum capability dependency. These client/shared
   paths are outside the original three weather code paths and need explicit CP
   allocation. No weather content defaults or implied client acceptance exist.
5. Then lease implementation; keep wire+registry+capability+server handler+typed
   client consumer+tests inseparable. CP reserves identifiers in accepted
   LOOK→WEATHER→SEC order and owns shared registration/pins/wiring. No new weather
   capability is offered merely because this design document is merged.

### Executable validation matrix for the later implementation

| Boundary | Concrete positive and negative cases required |
|---|---|
| Determinism/time | Native root+clock golden vectors across supported targets; same World/slot after retry/restart/Channel or owner move; unrelated RNG purpose does not perturb weather;0/P−1/P/P+1, native day wrap without weather repeat, before-epoch, u64 bounds, delayed tick across many slots remains one draw, missing/regressed/foreign-provenance time; replay must not consult live clock. |
| Binding/owner | Change exactly one WorldId/seed identity/policy/clock identity or actual owner/session/lease generation from a valid case; missing/stale facts, retired callbacks, old-world state and replaced owners refuse at the emission consumer. Do not derive matching “current” facts from the same immutable proposal under test. Mid-slot revision change must not reroll; no implicit default seed or epoch acceptance. |
| Codec/compatibility | Every union shape, slot0/u64 maximum, preset1/16 and0/17, payload64/65,31/32/33-byte binding, missing/repeated/unknown/overflow/truncated fields; exact registered types/dependencies once reserved; selected/unselected capability, unknown policy/assets, old client, resume set mismatch and fresh snapshot replacement. No secret root in payload/ordinary logs. |
| Transport/recovery | Initial snapshot completeness, failed encode/partial snapshot/write, same tuple suppressed, exact pacing boundary, latest-only stalled reader, domain/global revision gaps and exhaustion, reconnect/transfer current snapshot with no historical storm, old-generation frame rejected, retirement racing a prepared observation and live Unavailable/recovery behavior. |
| Resources/client |1/16 presets and0/17, exactly16 selector entries and15/17/dangling entries; fixed cache/mailbox bounds; one extra quad within existing full batch (capacity/capacity+1), atlas/view bounds and absent assets; UI coalescing after sequence application; owner shutdown clears presentation. Test GPU-free batch shape first, then real renderer/client composition and supported-platform compatibility. No particle budget or measured performance claim is invented here. |
| Visual-only isolation | Weather changes leave server World light, visibility/LOS, movement/collision, damage/conditions, spawn scheduling and value state unchanged; no gameplay function consumes weather as an authoritative predicate. Client local animation/reduced visuals cannot issue simulation writes. |

This task ran governance validation and its124 tests plus owned-path/hash and
whole-diff whitespace checks only. No Rust/Cargo, runtime/network/GPU/PG/parity
or performance tests are claimed. Actual source hashes, accepted ruling and local
nomination are retained under `/workspace/oteryn-prep/weather-1-evidence/`.
No product code, schema/content, registry numbers, current seed binding, commit,
push, ready/merge action or architect approval was produced.
