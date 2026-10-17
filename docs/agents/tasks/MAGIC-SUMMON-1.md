# MAGIC-SUMMON-1

```yaml
task_id: MAGIC-SUMMON-1
title: "MAGIC-SUMMON-1: fenced familiar lifecycle and clean logout"
mode: IMPLEMENT
status: OPEN
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/magic-summon-1
issue: 1622
pr: 2199
base_sha: efecd2209e71e209878ac543cd70966c6300e8dd
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 3 MAGIC-SUMMON-1 (rows 57, 58); App. 7 LOOP-MONSTER-1, MAGIC-SUMMON-1; wave 2"
class: "hard (child graph)"
migration: maybe
registry: maybe
owner: family F
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/monster_summon.rs
  - apps/game-server/src/gameplay_transport/familiar_cast*.rs
  - apps/game-server/src/gameplay_transport/world_item_cast.rs
  - apps/game-server/tests/summon_*.rs (new)
  - apps/game-server/migrations/0107_familiar_state_versioned_lifecycle.sql
  - apps/game-server/src/durability/character_familiar.rs
  - apps/game-server/src/gameplay_transport/familiar_cast.rs
  - apps/game-server/src/spell/companion_lifecycle.rs
  - apps/game-server/src/spell/delayed_execution.rs
  - apps/game-server/src/spell/delayed_execution_tests.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/character_familiar_writer_postgres_cases.rs
  - apps/game-server/tests/support/character_proficiency_postgres_cases.rs
  - apps/game-server/tests/support/familiar_versioned_lifecycle_postgres_cases.rs
  - apps/game-server/tests/support/spell_familiar_runtime_postgres_cases.rs
  - docs/agents/tasks/MAGIC-SUMMON-1.md
slot_only_paths:
  - connection.rs
  - mod.rs
  - capabilities.rs (SUMMON-WIRE-1)
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - other unallocated migration numbers (0107 is owned under CP D1296)
  - content spell/creature pins
public_contracts: []
depends_on:
  - "MAGIC-CAST-1: MERGED #2112, #2119"
  - "SPELL_NPC_MAP0: ACCEPTED #2098"
  - "CREATURE_AI0 ACCEPTED (summon children): BLOCKING: CANDIDATE"
  - "FAMILIARS0 ACCEPTED: BLOCKING: CANDIDATE"
  - "CREATURE-MOVE-1 residue (SUMMON-1): BLOCKING: draft skeleton/creature-move-1"
  - "HOUSE-1b (`house_access`): BLOCKING: Wave 5"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Summons, familiars and magic fields; children MONSTER-SUMMON-1, SUMMON-1, SUMMON-WIRE-1, FAMILIAR-CONTENT-1, FAMILIAR-1, SPELL-RT-SUMMON-1, SPELL-RT-FAMILIAR-1, SPELL-RT-WORLD-1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAGIC-CAST-1 | MERGED #2112, #2119 |
| SPELL_NPC_MAP0 | ACCEPTED #2098 |
| CREATURE_AI0 ACCEPTED (summon children) | BLOCKING: CANDIDATE |
| FAMILIARS0 ACCEPTED | BLOCKING: CANDIDATE |
| CREATURE-MOVE-1 residue (SUMMON-1) | BLOCKING: draft skeleton/creature-move-1 |
| HOUSE-1b (`house_access`) | BLOCKING: Wave 5 |

**BLOCKING:** CREATURE_AI0 ACCEPTED (summon children); FAMILIARS0 ACCEPTED; CREATURE-MOVE-1 residue (SUMMON-1); HOUSE-1b (`house_access`). The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/monster_summon.rs`
- `apps/game-server/src/gameplay_transport/familiar_cast*.rs`
- `apps/game-server/src/gameplay_transport/world_item_cast.rs`
- apps/game-server/tests/summon_*.rs (new)

Current D1296 PR-owned paths (in addition to the historical skeleton allocation):

- `apps/game-server/migrations/0107_familiar_state_versioned_lifecycle.sql`
- `apps/game-server/src/durability/character_familiar.rs`
- `apps/game-server/src/gameplay_transport/familiar_cast.rs`
- `apps/game-server/src/spell/companion_lifecycle.rs`
- `apps/game-server/src/spell/delayed_execution.rs`
- `apps/game-server/src/spell/delayed_execution_tests.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/character_familiar_writer_postgres_cases.rs`
- `apps/game-server/tests/support/character_proficiency_postgres_cases.rs`
- `apps/game-server/tests/support/familiar_versioned_lifecycle_postgres_cases.rs`
- `apps/game-server/tests/support/spell_familiar_runtime_postgres_cases.rs`
- `docs/agents/tasks/MAGIC-SUMMON-1.md`

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs
- mod.rs
- capabilities.rs (SUMMON-WIRE-1)
- PROTOCOL_OTERYN_V1_REGISTRY.json
- other unallocated migration numbers (0107 is owned under CP D1296)
- content spell/creature pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, unallocated migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/spell_parameter_cast.rs` closed gates `vertical_move`, `house_access` (BUNDLE-GATES-1 inventory `bundle_gates_tests.rs`)
- `monster_summon.rs`

## Inseparable bundles

SUMMON-WIRE-1 wire bundle; FAMILIAR-1 migration+writer. First unblocked child: SPELL-RT-WORLD-1 minus `house_access`.

## Acceptance criteria

- [ ] Every closed bundle gate it opens is updated in `bundle_gates_tests.rs` inventory.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

CREATURE_AI0; FAMILIARS0; SPELL_NPC_MAP0 §2-§3. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 3 MAGIC-SUMMON-1 (rows 57, 58); App. 7 LOOP-MONSTER-1, MAGIC-SUMMON-1.

## Builder instructions

- Work on branch `skeleton/magic-summon-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: D1296 scoped repair after CP D1299 main integration
status: OPEN
branch: skeleton/magic-summon-1
blocker: CREATURE_AI0 ACCEPTED (summon children); FAMILIARS0 ACCEPTED; CREATURE-MOVE-1 residue (SUMMON-1); HOUSE-1b (`house_access`)
next_action: new-head CI, then CP-owned external and final review
```

## Bounded caller and contract qualification — 2026-10-15

This appended packet is `REPORT_ONLY_RESIDUE_CALLER_PACKET` for existing draft #2199,
source `21488c73b1c184010b44fc9419748557b88de01a`. Its exclusive allocation owns this
card only. The skeleton metadata, historical dependency table and checkboxes above are
preserved as history, not asserted current delivery state. No runtime, content, migration,
wire, registration or activation changes accompany this packet. This is a concrete
implementation handoff for CP and architect review, not their approval.

### Accepted constraints and source admission

Protected-main object `bda15eba7a84f65fcb963a86e8b2bf18d69c800a` contains ACCEPTED
FAMILIARS0 and CREATUREAI0 under `ACCEPT-BATCH-2026-10-15`; the allocation's earlier
acceptance object is `ac9f172836f00226b68088aa04db95ccf1ab994e`. The historical
CANDIDATE holds above therefore do not describe those decisions now. Their accepted
rules still require separate child allocation, exact-head review and protected integration.
World and Channel remain distinct; a prepared source plan, saved receipt or matching
content digest does not prove current session, lease, scope or physical actor authority.

FAMILIARS0 §§3–7 supplies the five vocation familiars, 900 s initial remaining lifetime,
1800 s cooldown, ordered admission checks, two-summon aggregate cap, remaining-time
persistence and clean-end/crash rules. The 10,000 HP source value remains
`PARITY_PENDING`. CREATUREAI0 §8 supplies ordinary summon acquisition, placement,
role, ownership, attribution and removal rules. Do not replace either contract with an
upstream helper's semantics merely because the helper is pinned.

The existing native loader is an actual admission seam, not a global source-data lookup:
`content/native_gameplay.rs:588` validates versioned manifest pins, and `:970` compiles
catalogue plus explicit source selection into the spell book. Optional familiar config is
qualified under the outer artifact digest. `tools/content-schema/native-gameplay/README.md`
records the explicit `OTERYN_NATIVE_GAMEPLAY_MANIFEST` opt-in, independent staging/scope
issuance and baseline fallback. The checked-in example manifest is `/v1`, four pins and
no familiar-config section; the full producer can construct a richer explicitly supplied
artifact. Neither example files nor a successful local builder prove a deployment selected
that artifact. This report reads no live environment or control-plane issuance.

### Actual owner/caller trace and reusable implementation

| Family | Existing real source and boundary | Residue to integrate; do not duplicate |
|---|---|---|
| Monster summon | `monster_summon.rs` qualifies parent/child references, loader digest, current role/counts, bounded proposal replay and actual fixed-slot admission. Parent tile then qualified eight-neighbour placement uses native scheduling/RNG. `monster_combat_lane.rs:1473` calls `prepare_with_summons`, then `:1503` executes against the current fence, work stamp and map reader; new children enter its registration path. `:1852` retires native summons. | The live transport's first playable melee path, `gameplay_transport/monster_ai_cycle.rs:123`, clears attacks/defenses before admission; `ai/behaviour_profile.rs:100` also clears summons. The existing rich lane is registered source, but the repository search finds its owner type references confined to that module. Do not infer that its proposal resolution is the current transport consumer. Join the prepared LOOP-MONSTER boundary to the real actor owner under a CP allocation; do not enable summons in today's projection alone. |
| Player Summon/Convince/Animate Dead | `spell/companion_lifecycle.rs` prepares real source acquisitions; `PreparedCompanionSpawn`/assignment in `foundation/runtime_actor_companion.rs` bind the physical owner/session, current census and placement. Convince keeps the existing actor; named/corpse acquisition uses the existing qualified payment path. | Reuse these plans and existing caster/item transaction. Current content's summonable/convinceable flags, exact target/corpse custody, shared cap and actual destination admission must all survive reservation through commit. A new calculator or independent summon store would bypass the existing boundary. |
| Familiar cast/login | `gameplay_transport/mod.rs:4202` calls `cast_native_familiar`; `:4783` calls `initialize_familiar_login` after first entry only when active native content carries config. `familiar_cast_dispatch.rs` dispatches actual source-family casts and retained original intents; `familiar_cast.rs` has the real durable/physical commit path. `FamiliarSourceAdapter` at `:1264` combines the qualified config with independent current Premium and administrative projections. | Existing dispatch is not missing wholesale. Qualify accepted lifecycle semantics and content before activating this family; retain current fence, source revision, cost, physical reservation and unknown-COMMIT reconciliation. |
| Familiar current access | `spell_premium_coordinator.rs:743` refreshes the registered owner, applies the current transition and falls back to unavailable when refresh/application fails. `:762` resolves the current account binding and coordinator view. | The nearby old dispatch comment mentioning an unavailable producer is not evidence of no implementation: this real registered coordinator exists. A node without the coordinator or current binding still refuses. No request-derived Premium/God flag or new authority callback is appropriate. |
| Familiar death/end | `familiar_cast_dispatch.rs:77` reads actual health-zero owned slots, obtains Combat lethal receipts and performs qualified lifecycle cleanup. `gameplay_transport/mod.rs:1340` reads the genuine FreshAdmission session/grace deadline/control-loss epoch before `save_familiar_logout`; callers at `:2070` and `:2355` gate terminal actor release. | Extend these owners for accepted return-close/crash state rather than replacing them. A death-only not-applicable commercial projection is sealed cleanup evidence, never cast eligibility. A replayed historical receipt must not reacquire a physical companion. |
| World-item/field creation | `gameplay_transport/mod.rs:4210` calls `cast_world_items`; `world_item_cast.rs` retains the original normalized grant through uncertain COMMIT. `native_world_item_cast.rs` and `ordinary_field_items.rs` use current source Item definitions, actual tile/owner/custody and the original caster-cost transaction. | Reuse the current item writer and binding. Numeric source IDs alone cannot admit a definition. Do not create a detached field map, second payment receipt or new audit cause. |
| Field ingress/conditions | `field_step_ingress.rs:295` inspects the actual STEP destination and rechecks current actor/session/map/content in the same owner boundary. `ability/field_condition.rs` decodes qualified field recipes; `durability/spell_field_policy.rs` carries typed current World policy. Standing reads pause existing DOT only; they do not install/refresh entry conditions. | Preserve entry/standing distinction, creator lease/current World policy and unknown-metadata refusal. No caller-supplied PvP boolean or fabricated no-field authority. Share existing ConditionStore/damage calculation. |

The checked-in gate inventory is authoritative code evidence of bounded activation:
`bundle_gates_tests.rs:127` keeps house-access/vertical-move parameter casts closed;
`:140` keeps world-item casts closed; `:151` keeps field ingress closed under LOOP-WORLD;
`:162` keeps source-map initialization closed under MAP-CUTOVER. Those real call sites
must be qualified with their own content/current-owner producers. Presence of a call site
is not proof its gate is open; a closed gate is not absence of the underlying implementation.

### Resolved duration question and concrete lifecycle mismatch

No 900 s duration defect is established. The current config pins Canary
`99902524e052f37574194466c2949c576e4ab269`, source config SHA256
`099939f439c755171c18d8dadd23d82f50bd3ebd219099248e847481fd84cd15`
and VIP implementation SHA256
`be92f3b797a5692843b1cce09250287ce52f91afbcfd5edaf9c6fddccfbb9232`.
Its `familiar_minutes = 30` is a source setting name: all nine catalogue familiar profiles
at `spell-p2-r20` multiply by `seconds_multiplier = 30`; `native_companions.rs:389`
therefore produces 900 s, and the source cooldown multiplier 2 produces 1800 s with the
pinned rate 1.0 and VIP disabled. Changing that config to 15 would introduce a defect.
The source `summon_limit = 1` likewise refuses the ordinary non-God cast with any summon;
it does not impose a one-summon aggregate runtime cap.

**Actionable cross-domain finding: accepted clean-save and crash semantics have no
complete mapping in the current familiar state representation.**
`durability/character_familiar.rs:36` stores absolute `saved_expiry_unix`,
`last_logout_unix`, a lifecycle epoch and identity/look metadata; its cooldown snapshots
already store remaining microseconds. It does not represent the accepted familiar
remaining duration, `open`, writing lifecycle session/runtime owner and durable
`clean_end_game_session_id`. Migration 0033 defines the original eight-key JSON state;
0041 permits the ninth cooldown key, not the accepted lifecycle discriminator.
`companion_lifecycle.rs:1056` passes expiry/logout to the source plan; `:1094` copies
`StoreExpiry`; `:2350` saves logout Unix time and remaining cooldowns.

The actual selected Canary profiles for source IDs 194–197 use
`login.lifetime_basis = current_unix_time`; the unique Monk 282 profile does too. The
four explicitly retained Crystal alternatives use `last_logout_unix_time` instead.
`executable-spell-source-selection.json` selects the Canary identities, not those
alternatives. `native_companions.rs:403` executes that distinction. For example, a
clean save at Unix 1100 with expiry 1900 followed by login at 2000 yields zero from the
selected source plan, while accepted offline freeze retains 800 s. Selecting the
Crystal alternative alone would still not implement durable clean-end closure or
process-replacement crash classification; source selection is not the repair.

This is a source-to-accepted-contract finding with a concrete reachable dispatch chain,
not a claim that a currently deployed familiar already loses time. Runtime activation,
complete familiar content and current owner qualification are not proven by this packet.
The existing writer is genuinely recovery- and gameplay-fenced: at
`character_familiar.rs:488` it locks admission/occurrence, returns an exactly bound
historical receipt before new-mutation checks, and permits a new mutation only after
independent current gameplay/node checks and Character revision/CAS. Preserve that
historical-versus-current distinction. Generic fencing and a lifecycle counter do not
supply the missing clean-end discriminator or its return-admission ordering.

### Concrete proposed integration sequence for CP/architect

1. **FAMILIAR-CONTENT:** retain exact Canary/Crystal provenance and selected identities;
   record the accepted Oteryn lifecycle override explicitly rather than silently rewriting
   source evidence. Admit five linked Creature/Presentation/spell definitions, including
   the accepted eligibility flags and HP parity marker. Qualify the outer artifact and
   all referenced attack/defense policies; no placeholder familiar or automatic fallback.
2. **FAMILIAR persistence + caller, inseparable:** extend the existing row/writer with
   FAMILIARS0 §5's remaining durations, revision, full gameplay fence identity, runtime
   scope owner/generation, open and durable clean-end session. Allocate the migration
   under [CP D1296](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6228238402). CP approved lazy legacy absolute-expiry conversion on the next fenced write;
   expiry/logout alone does not reconstruct crash versus clean end. Do not invent
   a current owner. Preserve immutable exact
   receipt binding, Character revision ordering and replay reconciliation.
3. **Admission/return:** before costs, resolve current level/vocation/Premium/support
   cooldown, actual lever-boss room membership, complete owned-summon census, stored
   pending return and durable familiar cooldown in accepted order. Cast and return
   acquire the same physical owner slots and row atomically through the existing
   transaction/reservation path, opening the lifecycle only on committed acquisition.
   A blocked tile retains stored time; a lever-room refusal ends the familiar as specified.
   Unknown room/placement/source facts refuse; no guessed `is_boss_room` authority.
4. **Clean end/recovery:** close return admission before waiting for an in-flight return;
   CAS the last clean-end write and durable closing GameSession before authority release.
   Reopen only after proving no commit, not after a lost reply. On login/replacement,
   independently read current session/owner, then distinguish same-session reconnect,
   changed owner/session crash and durable terminal close exactly as §5 specifies.
   Missing row loads without insertion. Crash never shortens the saved cooldown.
5. **Current actor/lifecycle:** reuse owner timers and actual companion slots. Lifetime
   consumes online non-swimming time; cooldown consumes all online time; both freeze
   offline. Integrate actual swimming, owner-speed, lever-room arrival, travel,
   owner death, familiar lethal receipt, Channel transfer and WorldReset hooks.
   WorldReset closes and saves remaining time; familiar death/owner death/expiry end it;
   ordinary removals must not invoke familiar death. No synthetic lifecycle command from
   client or timer IDs is authority.
6. **Ordinary/monster movement:** adopt the existing profile schedule/catalog/owner path
   within the actual Channel actor owner, with its current target/path/census/fence and
   bounded replay. Ordinary summons use accepted follow/target/removal and ever-summon
   no-value lethal rules; familiars use their separate >15-tile/any-floor return and
   travel rules. Parent lethal death and parent retirement remain distinct operations.
   Share destination admission and the existing physical actor generation checks.
7. **Field/house/wire slots:** open only the specifically qualified gate and update its
   existing inventory. HOUSE owns current house-access membership; World policy owns
   field policy and creator lease. SUMMON-WIRE bundles existing observer NONE/OWN/OTHER
   relation semantics with the actual client consumer and registry owner; no new event
   or capability number is assigned by this packet. Existing familiar cooldown/warning
   messages are reused as the accepted contract directs.

### CREATURE-MOVE prepared boundary, not a merged dependency

The preserved `deliverables/creature-move-1/RESULT.json` describes a local prepared
subset at retained base `b2bf32341faf70c73b3bd264529e66b11bb4eab7` (#2184):
existing cardinal A* extended for exact spawn-cell walkback (50-tile bound, 1024
expansions, 128 steps), plus near-target cadence/duration helpers in the existing speed
module. Its two source hashes are bound in this packet's external manifest. Its previous
98 PASS/one ignored and focused 15 PASS are historical export evidence, not tests run
here and not acceptance. The handoff still needs real goal/content/actor-bound path
adoption, current destination admission, per-creature counter, invalidation/research,
step deadline ownership and shared chase budget/player ordering. No accepted relocation
API is inferred from those prepared files. Join that owner seam once allocated; do not
add another pathfinder or grant map authority through a caller-supplied admission callback.

### Required qualification when the allocated integration exists

| Boundary | Positive, edge and one-invariant negative qualification |
|---|---|
| Admission/payment | All five real content profiles; 0/1/2 owned summons; pending return with no actor; exactly enough mana/support/familiar cooldown boundary; wrong vocation/Premium, lever room, unknown room, stale content and full tile. Every refusal leaves costs, row, slots and cooldown unchanged. |
| Clean-end family | Execute every FAMILIARS0 §5.1/§5.2 case, including F-I1–F-I11, both direct and reconciled paths where specified. Real PostgreSQL concurrency/reload must cover close versus return, timer versus save, old owner versus takeover, lost COMMIT response and same-session replacement after close. Do not substitute matching request-derived authority. |
| Time/session | 900/1800 s arithmetic; 60/10 s warnings once; online/swimming/offline transitions, long offline interval, same GameSession raised connection generation, fresh sessions both generation 1, changed scope owner generation, missing row and overflow. Recovery never lowers cooldown or resurrects an open crashed lifecycle. |
| Physical/replay | Exact bound original cast/return replay once; changed command binding conflicts; occupied/removed target after prepare, stale actor/session/item/lease/content, uncertain commit then current successor validation, reservation rollback. Historical AlreadyCommitted must not independently spawn or consume again. |
| Ordinary/monster summons | Parent tile and eight-neighbour blocked boundaries; due/chance/cap/current-target-path checks using real schedule; master is itself a summon; per-entry/total limits; parent retirement versus lethal death; no descendant re-summon; ever-summon lethal event with no corpse/loot/XP/credits; damage/loot/PvP/party attribution through existing owners. |
| Movement/removal | Current floor/door/PZ/teleport/house restrictions; stale path after map/content change; >30 tiles/two-floor ordinary removal versus >15/any-floor familiar return; same-session reconnect retention; owner death/logout/Channel transfer/travel/WorldReset with distinct familiar outcomes. |
| Fields/client | Qualified versus unknown field metadata, STEP entry once versus repeated standing reads, safe/PvP World policy and expired creator lease, item cost/field rollback, source map gate, house denial, reconnect snapshot/change/removal and observer relation without new invented wire IDs. |

### Evidence and scope

External durable evidence: `/workspace/oteryn-prep/magic-summon-1-evidence/`.
`qualify_sources.py` reproduces 43 exact Git-source hashes (40 candidate sources and
three protected-main decisions), extracts nine familiar catalogue profiles/four explicit
selection records, and binds four retained movement export files. The extraction reads
existing inputs only; it does not regenerate content or qualify a deployment.
The final validation log records governance validator, 124 governance unit tests,
whitespace, exact owned-path scope and final card binding. No Rust/Cargo, PostgreSQL,
runtime, client E2E or performance tests were run for this documentation-only packet.
The whole appended diff was reviewed against the source trace; no code or shared files
changed. CP scope allocation and independent architecture/persistence/combat/content/
protocol reviews remain required before the proposed integrations can be accepted.

## #2199 D1126 / D1296 source continuation (2026-10-16)

OPEN / DRAFT / NOT_FULLY_QUALIFIED. [CP D1126](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225655585) owns `durability/character_familiar.rs`
and `spell/companion_lifecycle.rs`; [CP D1296](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6228238402) confirms next-checkpoint migration
`0107_familiar_state_versioned_lifecycle.sql`. This task-card entry is the scoped
migration allocation ledger; no separate migration-number registry was identified.
No historical migration or receipt is rewritten. The 0041 full legacy cooldown
predicate is retained verbatim under a helper; CREATE OR REPLACE retains the
existing validator function OID. Optional v2 serializes only when present, so old
canonical v1 before-state bytes and binding hashes remain unchanged.

The reader projects the actual linked familiar receipt `committed_at` milliseconds.
The real Cast/Login/Advance/Death compositor converts on the next fenced write:
legacy remaining is max(0, expiry*1000-checkpoint), never last_logout or read time.
V2 stores remaining/open and full session/connection/channel/scope-generation writer
identity. The actual writer checks that identity against the independently asserted
current gameplay fence in its existing receipt transaction. Existing cooldowns and
source configuration (900 seconds under current source parameters) are preserved.
Closed login uses remaining duration, with exact millisecond timer restoration;
open different-session/runtime login loses the familiar, keeping cooldowns.
Same-session same-runtime open reconnect is refused by this reconstruction path:
it must retain the current physical familiar/timer, not restart persisted remaining.

HELD: accepted FAMILIARS0 §5 clean-end needs the actual familiar timer remaining
and close/wait/reconcile return-admission protocol. `save_familiar_logout` previously
provided only cooldown state, actor loss epoch, DB Unix and current fences.
New writes from that incomplete port now return Unknown; already-prepared historical
logout receipts retain retry/reconcile. No boolean is treated as clean-end evidence.
`familiar_logout_snapshot` still represents the legacy cooldown-only API and refuses
v2; it cannot close v2. The durable clean-end marker is modeled/validated but no
unproven caller manufactures it. The writer rejects new non-null clean-end markers
until that proof port exists, even when the caller has a genuine gameplay fence. FAMILIAR gating/return-admission caller ownership
remains held; CP clarification is requested by the lead. This is not full offline
clean-end acceptance, production activation, readiness or merge authorization.

Prepared checks: one pure conversion/reconnect/ownership/boundary test, retained
legacy byte-identity test, and one canonical Character-authority PostgreSQL case
for actual linked checkpoint conversion, strict shape and independently current
writer rejection. Existing source-runtime cases receive required typed facts.
Rustfmt and diff checks are the only local validation; native/PG/Clippy/full package
are NOT_RUN for this source. Prior immutable-head evidence is historical only.

### Independent writer-boundary P1 disposition

Fixed in source: after historical receipt lookup, every NEW writer mutation requires
v2 after-state. New v1 and v2-to-v1 downgrade requests are rejected. Historical v1
replay retains its original before/after JSON and binding. The new native PG case
seeds historical v1 only in an explicitly isolated admin fixture transaction;
`SET LOCAL session_replication_role=replica` ends at COMMIT before any actual API
replay/negative/fresh write. This bypass is setup, not production qualification.
Existing fresh-write fixtures now explicitly carry v2 current writer identity;
D1164 permits the minimal proficiency fixture conversion. The accepted 0041 JSON
predicate still accepts historical v1; runtime new-write admission is stricter.

A persisted v2 non-null clean marker on a non-spawn event also remains held by the
current writer guard until the actual clean-end proof port exists; it is not silently
cleared or asserted current. D1212 clarified Logout versus GraceExpiry, but pinned
c248 has `save_familiar_at_actor_end` only for expired grace and raw loss epoch;
there is no FamiliarActorEnd enum. Timer due is private in SpellTimerOwner's Pending
and LifecyclePending; registered_familiar returns an event but no due. Lead has the
exact required caller/topology and timer accessor packet. No clean proof is guessed.

### D1216 / D1218 integration source continuation (supersedes held-port note)

The lead began a local merge of protected main b3e4fa0d into f0f9dfbc; this worker
resolved only its familiar_cast conflict, retaining actual main FamiliarActorEnd,
current transport/attempt, loss-epoch checks and pending retry/supersession behavior.
No merge/commit/stash/push is performed by this worker. Main's Logout entry is the
clean-end producer; GraceExpiry never supplies a clean marker or the new clean-end
commit port. Existing runtime then spell_states locks serialize the snapshot and
its pending receipt. Owner time is sampled once under both guards for actual expiry
remaining and reference cooldowns. A present familiar without exactly one matching
expiry is Unknown; an independently observed absence of physical familiars is zero.

D1218 adds only read-only SpellTimerOwner::familiar_expiry_due(self, owner, session,
creature, lifecycle_epoch), scanning actual cast and return Expiry entries. Warnings,
other full identities, absent and duplicate entries supply no deadline. No timer is
scheduled, canceled, advanced or consumed by this accessor. Existing hardlinked timer
source and test file were atomically detached before editing.

The crate-private clean-end writer port carries the authenticated Logout transport
into the same existing fenced receipt TX. Generic commits retain marker refusal.
The TX independently checks Active session/current transport and marker==fence session;
existing full authority checks hold the session row FOR SHARE. Historical receipt
lookup precedes these current-mutation checks; original bindings/history remain intact.
The source PG case now prepares direct-marker refusal, GraceExpiry/Reconnectable
refusal, wrong transport refusal, genuine Active/current-transport clean commit and
history replay after transport replacement. The existing real timer-owner unit case
adds cast full-key/warning/duplicate metadata assertions; return-path implementation
is prepared but has no separately executed qualification here.

Unresolved scope remains explicit: a prior committed clean marker on a subsequent
non-spawn/GraceExpiry path is held by generic writer refusal; no marker is silently
cleared. Same-session open Login reconstruction still refuses resetting a live timer.
No build/native/PG/strict Clippy/full package was executed for the integrated source.
The pre-integration f0f9dfbc bundle and all older evidence remain historical.


### 2026-10-17 source qualification continuation

NOT_FULLY_QUALIFIED. Exact-source native LIST initially rejected a stale shared-target artifact (2 tests versus the required 3); candidate-only detached metadata freshness retained all raw Git bytes and old proof metadata. A subsequent disk-guard stop occurred before any native test while preparing the separate source handoff; physical deduplication restored the reserve, with no cache clean or runner change. The actual fresh compiler then reported E0308 in the existing runtime PostgreSQL fixture helper (String error versus boxed TestResult error); the helper now uses the normal `Ok(...?)` conversion. These stopped attempts are retained, and no native test, strict Clippy or full-package PASS is claimed for this continuation. The complete source-bound native inventory and strict all-targets Clippy remain mandatory; earlier scoped evidence stays tied to its own source.


### 2026-10-17 qualified component checkpoint (D1122 / D1220)

NOT_FULLY_QUALIFIED. This checkpoint supersedes earlier NOT_RUN statements only
for the bounded native coverage and strict Clippy below. Tested source:
`12540672013a6312a79a43033ca589cab5b02b9f`, full raw inventory 18,864 files,
SHA256 fingerprint `039bb4c2e5c027f161f22d2af10e166922a192f5271dccbc09f903e9fece8f58`.
The publication successor changes only this task record; it does not change Rust,
SQL, fixtures, Cargo inputs or registration.

Coverage is 15 unique native cases (4 pure + 11 canonical PostgreSQL), using 14
retained passes with independently verified unchanged relevant inputs and one
re-executed conversion case after its same-value initializer lint repair. This is
not a fresh 15-case run on the final source. The earlier five-case run was followed
by a writer-fixture compile RED; the remaining ten PostgreSQL cases then passed
after its boxed-error conversion repair. That run's strict Clippy RED was repaired
only in the pure test initializer, without suppressing a lint. All stopped attempts
and their original logs remain distinct evidence.

The affected exact native LIST and test passed on the tested source. Strict
`cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`
passed in 737.6 seconds under Rust/Cargo 1.94.0, jobs1, debug0, incremental0.
Actual SQL before/after reports PostgreSQL 170006 and the same 14 database
name/OID/owner identities; no pre-existing database identity changed or was removed. Full source fingerprints are
identical before/after and all recorded owned process groups drained. Native logs
do not print a positive version marker; the real SQL and source-bound canonical
fixture checks supply that evidence. The dependency's Tokio warning is retained.

Evidence: `2199-clippy-initializer-fixed-native-result.json` SHA256
`33a95ad99fac97b19797b6ae8d300e77f1d5d05a07ed3f2d0f2fd556a75a1b5a`;
retained-case review `32798cef7439ad030880b75eba15fa66fe4a16d8fab22b82733bec894d812f76`;
final independent actual review
`fd40f7d2be0ed2ea605c750ab3feee3f0fcdcd4e566cc338eef016796ac9c68a`.
Evidence and verified Git bundles are preserved outside the author checkout in
the family evidence/recovery directories.

Full-package tests remain NOT_RUN for this component. This does not resolve the
retained shared full-package REDs or grant readiness, merge, queue movement or
production activation. Generic non-spawn/GraceExpiry writes with an existing clean
marker and same-session open Login reconstruction remain held as described above;
they do not silently reset a familiar timer or manufacture clean-end evidence.


### D1296 round-one repair

OPEN / DRAFT / NOT_FULLY_QUALIFIED. [CP D1296](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6228238402)
records the repair allocation. The [specific review decision](https://github.com/Oteryn/Oteryn-Game/pull/2199#issuecomment-6228238236)
approves lazy legacy conversion on the next fenced write and accepts a foreign
still-open lifecycle as crash: remaining lifetime becomes zero, with cooldowns
retained. The existing same-lifecycle reconnect and clean-end guards remain.
This repair adds one pure decision test and specific InvalidInput
assertions, and makes a missing checkpoint projection column fail closed.
The audit scan now projects the actual linked receipt checkpoint using the same
character/occurrence/revision join as the reader. No historical state is rewritten.

CP performed the main update in [D1299](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6228329035), producing
`cba8aa3ced137413361ecea73c48218bc15802de`. The clean author branch fast-forwarded
to that exact merge; its integrated main parent is
`efecd2209e71e209878ac543cd70966c6300e8dd`. These three owned files were unchanged
by the main update before applying this repair. F performed no merge or rebase.

Changed-case native execution and strict Clippy on this repaired source are
NOT_RUN locally; new-head GitHub CI, including Agent governance / validate, must
pass as required by D1296. The earlier 15-case coverage and strict Clippy belong
only to their historical source and do not qualify this repair. Full-package tests
remain NOT_RUN. CP owns the subsequent external-review trigger and final review.

The 0107 EXECUTE grants are intentional validation-only parity with 0041, which
already grants both validator functions to runtime and control. The new immutable
legacy helper needs matching invoker permission; none of these pure JSON functions
grants table writes or current gameplay authority. No live control caller is
invented as justification.

### D1308 round-three repair

OPEN / DRAFT / NOT_FULLY_QUALIFIED. [D1308](https://github.com/Oteryn/Oteryn-Game/pull/2131)
approves this last repair round for four Codex P1 threads (4301237612, 4301237622,
4301237626, 4301237630), repaired as one clean-marker / stored-lifetime state machine:

- Both actor ends are clean ends (FAMILIARS0 §5). The end of the in-fight deadline now
  closes the row with its session's marker through the clean-end port; its TX re-proves the
  session is Reconnectable at the exact control-loss epoch and past its original grace
  deadline on the database clock. Logout keeps its Active exact-transport proof.
- An ordinary write may carry a closed foreign session's marker unchanged (a Login with no
  free tile keeps the clean row, cooldowns and delayed return); only cast and return writes
  clear it. F-I11: after a session's own clean end it writes nothing more, in the TX, the
  owner-turn preparation and the actor-end save (a durable clean end is NotApplicable).
- With no physical familiar, the actor-end close keeps a waiting clean row's stored time;
  only an open row whose familiar already ended in this runtime stores zero.
- A cast while a stored familiar waits to return is refused before any cost (§4 check 4),
  with the source summon-limit message.

This closes the hold above on GraceExpiry and non-spawn writes over an existing clean
marker. Regression tests: one pure marker-decision test, three pure lifecycle tests and one
PostgreSQL grace clean-end test (gated by `configured_admin()`). Local Rust build, Clippy
and tests are NOT_RUN on this head (disk); new-head GitHub CI must pass.

### Round-four repair

OPEN / DRAFT / NOT_FULLY_QUALIFIED. The owner approved this round for Codex threads
4301955863 (P1) and 4301955868 (P2):

- A Cast or level Advance against an open lifecycle that another session or runtime wrote
  (not `same_lifecycle`) is refused before any cost or write, fail closed, because only Login
  crash classification (D1296) reconciles it. The Cast uses the source summon-limit refusal.
  Without this, a failed first-entry init left the row and its cooldown unrestored, and a
  cast replaced both. The transport still ignores that init error; the refusal makes that
  safe, and the next Login reconciles the row.
- D1126 conversion is for the next write only. A non-Cast event that changes nothing but
  the manufactured v2 lifecycle of an absent or legacy projection keeps it as loaded, so a
  no-op Login writes no row and no Character revision.

Regression tests: two pure lifecycle tests next to the round-three tests. Local Rust build,
Clippy and tests are NOT_RUN (no cargo); `rustfmt --check` passes. New-head GitHub CI must pass.

### Round-five repair

OPEN / DRAFT / NOT_FULLY_QUALIFIED. The owner approved this round for Codex thread 4302041461
(P2) and the ignored first-entry familiar init. Branch merged with `main` first (merge commit).

- A Login that keeps a positive `familiar_remaining_ms` but finds no free tile (the source
  spawn-room `CreationRefusal`) now records a stored return for that actor/session in the
  Channel `SpellTimerOwner` (data only). The 1000 ms owner cadence (`tick_vitals`, after the
  death drain) checks for a free tile under the held owner without any SQL read, then replays
  the source Login recreation through the fenced lifecycle writer as a new
  `FamiliarLifecycleOrigin::Return`, keyed by the waiting row's lifecycle epoch. It creates the
  familiar with the fenced stored lifetime only from the waiting row of this lifecycle at that
  epoch (`familiar_return_admissible`), never with a familiar present, and restores no cooldown
  again. A failed attempt leaves the row waiting and retries after 10 s; an unknown COMMIT
  resumes the retained original. A row no longer waiting ends the retry; the next Login still
  returns it. The Cast guard is unchanged, so it frees once the return commits.
- A first entry whose familiar Login did not complete now refuses its playable entry
  (`RefusedByChannel`, input-ineligible), like the other post-position owners, so the actor
  never plays with the stored familiar and its cooldowns unrestored. `gameplay_transport/mod.rs`
  is slot-only for this packet; the lead directed these two edits there for this round.

Regression tests: `stored_return_admits_only_this_lifecycles_waiting_row` and
`only_a_no_room_login_leaves_a_return_waiting` (lifecycle),
`stored_familiar_return_is_one_due_entry_per_actor` (timer owner) and
`an_incomplete_familiar_login_refuses_the_playable_first_entry` (transport). On a scratch
build of this tree, `cargo clippy -p oteryn-game-server --all-targets -- -D warnings` passes; the
game-server lib tests ran to about 2,600 of 3,281 with no failure before the build host went
offline, so the rest are NOT_RUN locally. `rustfmt --check` passes. New-head GitHub CI must pass.

### Round-six repair

OPEN / DRAFT / NOT_FULLY_QUALIFIED. The owner approved this round for Codex threads
4302326064 (P1) and 4302326065 (P1).

- FAMILIARS0 §4 check 3 / §7: a return, immediate (Login) or delayed (Return), now resolves
  lever boss room membership before it spawns. No BOSS-ROOM-1 owner exists yet, so membership
  comes from the owner's current session scope (`LeverBossRoomMembership::of_current_scope`).
  BOSS-RAID-0 §6.1 makes a boss room an activity instance while the lever room stays Channel
  map, so a Channel scope is outside (the fenced familiar TX re-proves `runtime_scope_kind = 1`)
  and an instance scope is unresolved and refuses (the row keeps waiting). Inside a resolved
  lever boss room, the waiting familiar ends in the same fenced write: remaining 0, row closed,
  cooldown kept, no spawn; the stored return then ends because the row no longer waits. The
  Cast now refuses unless the room is resolved outside. The `Inside` value stays unreachable
  until BOSS-ROOM-1 supplies the fact.
- CREATURE-AI-0 RL-19: the stored return replays the source Login recreation, whose planner
  skips the cast's owned-summon limit. It now checks the complete owned census against the
  aggregate cap of two summons, both in the owner-cadence precheck and when it prepares (the
  census is re-validated at commit). At the cap the row keeps waiting and retries.

Regression tests: `lever_boss_room_membership_comes_from_the_current_scope`,
`lever_boss_room_return_ends_the_stored_familiar` and
`familiar_return_respects_the_aggregate_summon_cap` (lifecycle). Local
toolchain 1.95.0: `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`
passes; `cargo test --locked -p oteryn-game-server --lib --bins` passes (lib 3,255 passed,
29 ignored, 0 failed; bins 7 passed), built without debuginfo to fit the disk. `rustfmt --check`
passes. PostgreSQL integration targets are NOT_RUN locally (no database). New-head GitHub CI
must pass.
