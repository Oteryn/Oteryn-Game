# PARTY-XP-1 — party shared experience and party spells (skeleton task record)

```yaml
task_id: PARTY-XP-1
title: "PARTY-XP-1: party shared experience and party spells"
mode: IMPLEMENT
status: partial
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PARTY-XP-1; §7 'PARTY-XP-1' (PARTY_PVP0 ACCEPTED)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/party-xp-1
needs_migration: false
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Parties share experience under PARTY_PVP0 rules (level range, activity) and party spells work with feedback through CHAT-2.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; feeds BOSS-REWARD-1 (MONSTER-META-1).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | BLOCKING (Wave 2, not merged) |
| D118 XP lane (LOOP-PROGRESS-1) | BLOCKING (Wave 1, not merged) |
| D3-4 (LOOP-DAMAGE-1) | BLOCKING (Wave 2, not merged) |
| CHAT-2 (party spell feedback) | BLOCKING (CHAT-2B #2128 open) |
| PARTY_PVP0 | ACCEPTED (plan §7 status list) |
| DEATH-1, ATTACK-1b, COND-1, SPEED-1, VIS-2, ATTACK-WIRE-1 | MERGED (archive records, #1798, #2122) |
| DEATH-3 | BLOCKING (death module batch C, not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

Current CP6219643346/D1101 removes both `durability/world_party.rs` and
`durability/world_party_tests.rs` from this card: sole ownership is PARTY-1 #2201.
The exact inverse removing only this branch's original World hunks is authorized
reconciliation, not continuing ownership of those files.

- `apps/game-server/src/gameplay_transport/party_spell_owner.rs`
- `apps/game-server/src/gameplay_transport/party_xp.rs`
- `apps/game-server/src/gameplay_transport/party_xp_tests.rs`
- `apps/game-server/tests/party_xp_postgres.rs`
- `docs/agents/evidence/PARTY-XP-1-*.md`
- `docs/agents/tasks/PARTY-XP-1.md` (this record: status and notes only)

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

- apps/game-server/src/gameplay_transport/kill_reward.rs (XP split hook)
- D118 XP lane writer (LOOP-PROGRESS-1)
- party_spell_owner.rs spell registration
- GAME_EVENT_FOUNDATION_REGISTRY.json

## Inseparable bundles (never split across PRs)

- XP split + kill reward + audit in one transaction
- party spells + bundle data + tests

## Acceptance criteria

- [ ] Shared XP split exactly once per kill, only to eligible active members.
- [ ] Ineligible members (range/activity) receive solo-rule XP or none per brief.
- [ ] Party spells affect only party members; feedback via CHAT-2.
- [ ] Concurrent kills serialize per Character XP lane.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server party_xp
cargo test -p oteryn-game-server world_party
cargo test -p oteryn-game-server kill_reward
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
- docs/reference/spells/r21-local-candidate/bundles/instant-*_party/
- PR #1798
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation (2026-10-15; incomplete integration)

Exact authoring base: `3789b07c449ca016335f733b61ec299e7a3aab7d`,
`skeleton/party-xp-1`, draft #2172. The allocation narrows writes to its seven paths;
its prohibition on shared registrations overrides the historical registration allowance above.
Accepted owning input: PARTY_PVP0 §5.1/§13, resolved by the coordinator on
`main@3ef900c160c9713092a86bfcd1a8377c7587c898`; D118 §2.4 amendment fixes
party → per-character Prey → per-character stamina order.

Source inspection found real residue: native PARTY-1 membership/strong reads and
party-spell selection exist, but no shared-XP activity, eligibility or equal split.
Migration 0046 already supplies `game_parties.shared_xp_enabled`; no schema is invented.
The shared `PartySnapshot` deliberately remains unchanged. The new same-transaction
read returns that toggle and the native membership predecessor under existing World
and party/member locks. The command variant reuses independently checked
`SpellItemAuthority`; the autonomous scope-actor variant reuses native
`SpellItemScopeAuthority` and current colocated actor resolution, without a synthetic command.
The read writes no receipt, revision, resource or event.

The existing source party owner now retains a separate XP projection of every actual
colocated member, including a dead member omitted by spell-target selection. Each
Character/session association comes from its existing native journal resolution, each
level/position from actual player state/runtime. Spell target selection stays intact.
The XP consumer rejects a known retired runtime and changed physical/player predecessors,
and compares the strong membership predecessor. This does not independently prove
current database/session/visibility authority after locks or the SQL transaction end.

The detached XP module provides one bounded activity cell per Character/session on a World/Channel for actual qualified
healing or aggressive-monster attack events;
50-principal bound; 120-second inclusive freshness; all-participant 30-tile Chebyshev
range on any floor; exact two-thirds level test; leader-presence/toggle/PZ conditions;
and equal shares rounded up without `pool + count - 1` overflow. Foreign-Channel
members do not participate or disable sharing. A failing participant disables sharing
for the entire kill; missing/mismatched source inputs refuse planning. Empty/restarted
activity grants nothing. No partial XP write or redistribution occurs here.

The integer pool input is explicitly **after admitted party bonus and summon deduction**,
before the equal split. The source has a private base-vocation normalizer in an unowned
module, and the native kill writer has no integrated party/summon calculation. This
preparation does not duplicate that normalizer or invent intermediate rounding. It does
not implement bonus calculation, summon accounting, Prey, stamina or soul awards.

### Current-authority and CP integration gates

- The real kill owner must hold current runtime/player locks, independently current
  session/lease/node/Channel/World fences and the source transaction through planning
  and commit. A matching stored snapshot or supplied deadline is never a current grant.
- Read the toggle/membership through the new real SQL port in that same transaction;
  construct the source physical view using the existing current colocated actor journal
  resolution. Recheck actual predecessors at the consuming owner boundary.
- Produce activity only from committed, qualified events: a heal needs its actual source
  party member target; an attack needs aggressive-monster qualification and may be solo.
  Use the committed event's actual
  semantic timestamp; retries keep that timestamp and never refresh activity as a new event.
  No client flag or arbitrary enum value proves those facts. Keep each cell per
  Character/session on a World/Channel; replaced sessions use a fresh cell and recovery
  starts empty. Party changes preserve recent activity, including a solo aggressive attack
  before joining. These event hooks remain absent.
- Supply the current leader's actual PZ deadline under its lock for that same semantic
  instant. The private native `CombatLock` is not a production accessor here; its
  test-only re-export is not used. The logout/battle block is never substituted.
- Add the actual leader-authorized shared-XP toggle command through PARTY-1's owner
  revision/receipt/event rules. The new port only observes the existing stored setting.
- Compose the accepted vocation bonus and summon share first, then this split, then
  each character's Prey and stamina. Use deterministic CharacterId lock order and native
  Character sequencers/receipts to atomically settle each death exactly once; retry,
  concurrent kills and recovery must not duplicate awards. The shared kill writer,
  actor events, registrations, audit/feedback and CHAT-2 remain slot-only work.

No content, imports, pins, migrations, registry, shared module or production wiring
is edited. Party buff/Serene source behavior already exists; its accepted content and
CHAT-2 feedback remain their owning integration, not a second spell implementation.
The skeleton dependencies/checkboxes are not represented as completed by this subset.

### Validation and evidence

Actual native qualification: **15 passed, 0 failed, 0 ignored**: 13 owned rule
bodies and two PostgreSQL 17.6 strong-read cases. The original, untransformed
corrected unit sources pass strict native Clippy metadata; normal cfg(test)-off native
library Clippy and owned/workspace formatting also pass. The command-authority read is executed; the autonomous scope-actor read is
compile/lint qualified only. No full-runtime retirement/current-source, PZ/activity
producer, kill receipt, exactly-once/concurrency/recovery or playable E2E is claimed.

Preliminary review found an unaccepted PartyId reset losing recent solo activity on
joining. **Fixed**: one per-Character/session+World/Channel cell; the native regression
covers solo attack → joining → party change, solo-heal refusal, and immutable cell
binding. Earlier Clippy evidence covers intermediate bytes only. Its unstarted bridge
was cancelled without executing tests. The first full lib-test codegen SIGKILL and
cfg(test)-off fixture setup failure remain explicit resource/setup evidence, not REDs.

The approved final route builds the actual native cfg(test) library without the global
unit-test harness. A disclosed outside-tree bridge removes selected test attributes, exposes the unchanged
bodies and adds forwarding exports for a tiny driver. Its PG helper excerpt adds only
an unused-import allowance; every transformation is manifested. Native
private types, genuine recovery/session/node fixtures, SQL authority checks and PG
migrations remain actual source. Original-source strict unit-context Clippy runs with
normal test attributes intact; no fabricated SourcePartyWorld or authority type exists.

Evidence, manifests, exact native commands and hashes live at
`/workspace/oteryn-prep/party-xp-1-evidence`. `corrected-tests.log` contains the actual
15-case output; `corrected-bridge-manifest.json` and `corrected-bridge-only.diff` disclose
all bridge mutations. Five final Rust hashes match the untransformed native snapshot.
The unchanged accepted §5.1/resource rows and D118 §2.4 plus seven native inputs are
bound to exact accepted main hashes. `self-review.md` records the ownership/provenance
limits. Obsolete snapshots and completed private rlib/ELF are retained losslessly under
`owned-compression.json`; no shared-target dependency was manually edited or deleted.

Final governance: validator PASS (22 documents/9 lanes), 124/124 governance unit tests
PASS. Exact six-path ownership, unchanged shared registrations/push deny, no staged
changes or commits, and whitespace checks PASS. The exported patch is verified against
the exact authoring base; this packet stays a draft preparation pending CP/architect
review and the integration gates above.


## Family native qualification increment — 2026-10-16

AUTHORING on existing draft #2172, `skeleton/party-xp-1`, exact starting head
`7bae3d2067ae7380abf809edef4609950c73159c`. Sole author `/root/party_owner`,
also sole family author of the PARTY-1 World owner; no concurrent World writer exists.
The lead refreshed the live task/CP lane and supplied bound META3.1.0 policy. PARTY_PVP0
is ACCEPTED on protected main24fc332f256638e3b784bff1b3c85b18e2665a76; CHAT-2B
#2128 is closed unmerged and needs an owning successor, rather than being open.

Initial exact file plan before edits: append native cases to owned
`durability/world_party_tests.rs`, add meaningful boundary properties in owned
`gameplay_transport/party_xp_tests.rs`, and update this status/notes record.
No production XP source, shared registration, external owner, content, registry or
migration was changed. The original contents of both test files remain literal
prefixes of the new files, including all 13 original Party PostgreSQL bodies, two
prior XP strong-read bodies and 13 planner bodies.

### New native cases

Four appended native PG bodies use the already registered real library module at
`src/lib.rs:145`. They exercise the previously compile-only autonomous scope-actor
XP read with genuine current recovery/Node/scope proofs and actual Fresh-derived
Channel actors. Character facts, progression, vocation/build and monk state come
from existing native writers. No synthetic SourcePartyWorld, widened helper,
substitute authority or copied crate wrapper is introduced.

- Autonomous solo, disabled and enabled reads and the command read are compared
  against committed membership/revision/toggle rows in the same native transaction.
  The actual physical source resolves both current member Character/session pairs,
  levels and runtime positions, and its XP predecessor validation succeeds. Reads
  leave all receipt/audit/membership row counts unchanged.
- Current eligibility is revoked independently for the acting Character and then,
  in a separate fresh fixture, its colocated participant. The actor scope read refuses;
  a durable roster observation alone does not attest a participant, and the actual
  participant resolver separately refuses the invalid current claim. Immutable Fresh
  history, runtime actors and unrelated current facts remain valid.
- The scope-actor reader refuses both a different physical SQL transaction and the
  unrelated member's current session substituted for the actor's session.
- A genuine projection refuses missing player state and a retired actual runtime;
  retained membership/player DATA does not restore owner currentness.

Membership and toggle rows are explicit constraint fixtures, not a claim to implement
PARTY-1's toggle command, the XP award transaction or activity/PZ producers. Every
Character creation/history/session/actor is native. Existing ordinary PG fixture routing
is retained: only an actual configured PostgreSQL17.6 run counts as execution evidence;
a unit lane without the routed service supplies no PostgreSQL qualification.

Two appended pure rule bodies cover equal rounded-up sharing for every participant
count1..50 and boundary pools through i64::MAX with an independent wide-integer oracle,
and the exact level ratio at u32::MAX plus opposite integer-limit coordinates. These
are DATA rule tests, not current-authority or event producers. No new lint suppression
was added.

### Source validation and exact native context

Direct pinned Rust1.94 rustfmt/whitespace checks pass. Governance validation and
124 agent tests are the required cheap documentation checks. No worker Cargo/PG job
ran; the lead owns the single native cloud executor slot. Source totals are now
19 PostgreSQL bodies in the registered World module (13 original + 2 priorXP + 4 new)
and 15 planner bodies (13 original + 2 new). Runtime counts must be recorded from
actual configured native execution, not from these declarations.

For native PG regression execution, the existing library mount needs no registration
change: select `party_xp_native_` for the four new bodies, or the existing World module
for all19. Planner/production consumption still requires the actual CP-owned
`mod party_xp;` line in `gameplay_transport/mod.rs`; a disclosed validation-only native
copy may add that line. No copied large `party_xp_postgres.rs` wrapper was created:
the native cases already run in the real library context, and a dedicated wrapper
would need shared Cargo `test=false`/duplicate-source routing rather than solving the
private-owner context. Dedicated-target composition remains a CP slot item, not a
silently disabled test or a new allowance.

### Combined family owner and exact integration dependency

Under the lead's explicit combined-family instruction, PARTY-1's own
`durability/world_party.rs` now also contains the existing accepted `WorldPartyXp`
and its command/autonomous strong-reader functions, while preserving PARTY-1's
current toggle/consent/default/cache source behavior. Its union source SHA256 is
`99d3c9eaf8843cc7897e9c2ab32ab62c59ba893ff249db58d95a2b38413c65a0`.
The PARTY branch's `foundation/runtime_actor_party.rs` supplies its current PartyAction
variants and shared-XP snapshot field.

The XP card does **not** allocate that foundation file. The lead explicitly forbids
carrying it across this card, so the XP production owner/projection/planner remain
unchanged at their canonical starting bytes. CP composition must reconcile the exact
two-file provider (`durability/world_party.rs` plus `foundation/runtime_actor_party.rs`)
after upstream PARTY and required main. At that point the XP-owned pure fixture helper
needs the added `shared_xp_enabled` field; its unchanged test bodies must remain literal.
Independent qualification of the old XP layout is not qualification of the final
combined provider. No hidden cross-card write or invented compatibility interface
was used. PARTY publication also remains held on its separate literal-test/default
semantics clarification; this increment does not resolve that constraint.

### Remaining full XP scope and real owner triggers

The real `kill_reward.rs` transaction and `combat/death_reward.rs` still admit one
principal, not a native atomic50-principal party settlement. Those files and actual
committed attack/heal event owners are outside this card. Looping singleton settlements
would violate exactly-once party accounting. A production leader PZ accessor is absent;
a cfg(test) CombatLock export is not authority. These are exact cross-domain source/
allocation dependencies, not a reason to skip these native owned read tests.

Required before full completion: accepted party bonus and summon accounting before
this split; each Character's sequenced receipt with Prey/stamina after it; original
semantic-time attack/heal producers; actual leader PZ source; concurrent kill/retry/
rollback/restart exactly-once qualification; immunity/loot-right owners and party spell
feedback through the actual CHAT successor; shared activation/registries/content
regeneration at CP slot. No full XP gameplay, reward mutation, live feedback or task
completion is claimed. No commit/push, new PR, ready/merge, production change or paid
review was performed.


## CP6219643346/D1101–D1102 authoring increment — 2026-10-16

Current allocation supersedes the older overlapping World ownership and historical
hold/integration statements above. Starting head is #2172
`c617044bf3a1f2de6ee4d0587f840775b411bfbc`; sole author is `/root/party_owner`.
The lead explicitly released AUTHORING after the three Guild writers released.

D1101: applied the reviewed exact inverse of this branch's own World hunks only:
46 lines in `world_party.rs` and562 in `world_party_tests.rs`. Both files now match
immutable parent3789b07c exactly (Git blobs2703a9c9 and6767695e); all original13
Party cases remain literal. The removed two older/four newer XP cases and genuine
fixture helpers are preserved in the external inverse/evidence packet. No broad
restore, shared foundation change, main merge or rebase was used. ONLY #2201
continues World owner/test edits. Merge main here only AFTER #2201 merges; do not
recreate its reader types locally to make this intermediate branch compile.

D1102: implemented `plan_shared_xp_in_transaction` in the owned `party_xp.rs` leaf.
It checks the native death World/Channel/scope generation against the actual runtime,
then reads sole2201's strong XP membership/toggle port and the existing current
physical member port on the caller's same actual SQL transaction, before invoking
the existing planner. Native source refusal remains distinct from a valid sharing-Off
or planner-input error. Returned death/time/party/revision/shares are DATA, never
Character write authority, reservation, receipt or an atomic award.

The sole pure fixture helper gains `shared_xp_enabled` for the actual2201 provider;
all prior15 planner bodies remain unchanged. Two added owned DATA regressions cover
original microsecond kill time within one millisecond and original-time event replay
without extending the inclusive two-minute activity window. They do not qualify
actual combat/heal event producers. No new lint suppression is introduced.

A proposed native consumer test, using the original real Fresh/current-session,
recovery/Node/scope, player-build and committed-death fixtures, is routed to sole2201
as the outside-checkout `party-xp-D1102-world-consumer-tests.patch`. It checks actual
same-TX planning, native Character/session output, unavailable PZ vs Source refusal,
wrong current session, absent player state and physical-TX substitution. The new
case is not silently disabled or presented as executed. It needs the actual2201
World/foundation provider and an explicitly disclosed CP module-registration slot
(`pub(crate) mod party_xp;`) in native qualification; no private fixture is widened.

Exact E08266 source bindings, existing types and the implemented D consumer handoff
are in `/workspace/oteryn-family/party-xp-D1102-E-handoff.md`. E #2192 alone owns the
real50-principal atomic progression/death/kill settlement and actual attack/heal
engagement ports. E must independently resolve every recipient's current fence,
policy and stable `(death,Character)` occurrence, obey sequencer/runtime lock order,
retain original kill/activity/PZ snapshots and semantic microseconds, preserve replay
and unknown-outcome convergence, and apply party bonus/summon before split then
individual Prey/stamina. A singleton-award loop is not this integration. Actual PZ
producer, heal owner routing, atomic rollback/replay qualification and shared wiring
remain open; this card stays partial, with no gameplay completion claim.

Validation for this new increment: direct pinned rustfmt and whitespace checks plus
cheap governance checks only. Prior XP34 native PASS is historical evidence for the
superseded detached increment; it does not qualify these new bytes. The lead owns
native Clippy/test execution against the actual2201 source in a private disclosed
qualification composition. No Cargo, commit, push, E/shared-module write, ready,
merge, production action or paid review was performed by this author.


## D1102 executed composition and E handoff — 2026-10-16

This current entry supersedes the preceding execution-pending wording, retaining it
as history. Actual #2172 source `bd45762e7617de9f614a5fc729fed20b63915bbf`
passes **17 pure XP bodies and1 real native consumer body**, each invoked exactly
once with0 failures/0 ignored. The lead executed the preserved whole-crate native
ELF against configured PostgreSQL17.6 (`server_version_num=170006`). Independent
`D1102-execution-review.md` verifies exact invocations, results and compiler inputs:

- Native ELF SHA256: `889ab44ebd5c27b203d921940ced69ed7cb25aa01c15b92a2d5218c84bdc1dd1`.
- Compiler-source archive SHA256: `b67d061700bf01e7620a54eaf7355d23a1b249cfc92e86df865ac4993b4d17d7`.
- XP planner/consumer SHA256: `8f848ee1a0c3f18f87f69a9637228eea94ecdec91a3d2c5e6cce9c3efa6d25ce`;
  tests: `25888c8edfed9f48e022655883415e362e981040e95228acfc5872d478358fca`.
- Actual sole2201 provider at `d2f390fdd2957630614f0334c15a8dcfdac269ac`:
  World SHA256 `99d3c9eaf8843cc7897e9c2ab32ab62c59ba893ff249db58d95a2b38413c65a0`;
  foundation SHA256 `eb8ca50bfc4f181987e859c1d1857285ef15386418469f5990b67e774929f039`.

The executed World test is actual2201 source plus outside consumer patch SHA256
`826c05c129faa2fb6286d91a2ab7d2614cfc8643e329812785e3563ec0fb86e5`,
in the private qualification composition with explicit CP module registration.
That patch/registration is not canonical #2172 source; D1101's inverse remains
intentional. The real native case qualifies same-TX source planning and its negative
fences, not an E kill settlement or activity/PZ producer. Prior XP34 evidence remains
historical. Strict native unit Clippy is a separate lead qualification; no global
Clippy PASS or complete shared-XP gameplay is inferred.

### Implemented typed consumer for E #2192

These actual private types live in `apps/game-server/src/gameplay_transport/party_xp.rs`;
this inline handoff is usable without access to the author's local artifact paths.
`SemanticTimeMicros` and `ExactI64` below are from `oteryn_simulation_determinism`;
`CreatureDeathOccurrenceKey`, actor/session and runtime are native foundation types.

```rust
struct PartyXpKillInputs<'a> {
    death: CreatureDeathOccurrenceKey,
    at: SemanticTimeMicros,
    activity: &'a [PartyXpActivity],
    leader_pz: Option<LeaderPzObservation>,
    pool_after_bonus_and_summon: ExactI64,
}
struct PartyXpKillPlan {
    death: CreatureDeathOccurrenceKey,
    at: SemanticTimeMicros,
    party_id: Option<[u8; 16]>,
    party_revision: Option<u64>,
    decision: PartyXpDecision,
}
enum PartyXpConsumerError {
    Source(SpellItemError),
    Plan(PartyXpError),
}
async fn plan_shared_xp_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    caster: (crate::foundation::ExactActorRef, GameSessionId),
    input: PartyXpKillInputs<'_>,
) -> Result<PartyXpKillPlan, PartyXpConsumerError>;
```

The consumer checks death World/Channel/scope generation, calls actual2201
`read_world_party_xp_for_scope_actor_in_transaction` and the actual physical
`read_source_party_world_for_scope_in_transaction` on THAT caller transaction, then
calls the existing `plan_shared_xp`. Source errors preserve native refusal; valid
`Off(SharingOff)` selects the existing whole-kill damage-share path. `Shares` contains
`PartyXpShare { character:[u8;16], session:GameSessionId, amount:ExactI64 }`.
Every returned field is DATA, not a reservation, current Character grant or receipt.

E #2192 remains sole writer of the real combat/progression/death/kill settlement and
attack/heal engagement ports; D supplies this consumer, not an E writer. Required
caller composition:

- Resolve each of1..50 distinct recipients' OWN current Character/session/connection/
  lease/scope/revision fences, policy and stable `(death,Character)` occurrence.
  Settle all admitted recipients and receipts in ONE physical transaction, with
  rollback/unknown-outcome recovery; a loop of singleton commits is insufficient.
- Retain original at-kill membership/toggle/recipients, qualified aggressive-monster
  attack or committed member-heal activity and actual leader PZ observation. Preserve
  original microseconds and event timestamps on retry/replay. E's captured death
  milliseconds cannot safely be multiplied back to reconstruct a same-ms event;
  foundation owner-timer and simulation time types require explicit same-incarnation
  conversion. The death key does not attest input time or kill-time provenance.
- `LeaderPzObservation` binds actual leader Character/session/observed_at and optional
  `pz_until`. Outer `None` is unavailable; absent inner deadline is known unblocked.
  Do not substitute logout/battle block or invented producer data. Replayed heals
  retain their original committed timestamp instead of renewing activity at retry.
- Obey actual sequencer ordering: no runtime lock while waiting for Character slots
  or across durable writes. E must supply genuine reservation/revalidation and a
  stable total recipient lock order; this borrowed read adapter supplies neither.
- Apply real party bonus and summon share BEFORE this pool/split, then each own Prey
  and stamina AFTER, without redistribution. Zero pool legitimately produces zero
  shares; the current positive-only award writer rejects zero. E must account for
  zero awards through its actual batch semantics, never clamp to1 or redistribute.
- Replay the retained original recipient set, amounts, policies and occurrences;
  never re-plan from changed party/activity/PZ or mint new occurrences. Reject
  conflicting occurrence input and reconcile absolute committed facts, preserving
  later progression/Bestiary revisions. Qualify1/50 bounds, duplicates, second/last
  recipient rollback, overlap, lost commit outcome/restart and timestamp boundaries.

Real E atomic integration/producers/modifiers, shared activation and eventual main
merge only AFTER #2201 remain open. This docs-only update changes no Rust or owner
allocation and claims no additional execution beyond the exact composition above.

## ROOT-D-NEW-CONCRETE-PORT — retained activity DATA getter

On isolated #2172 base `a3b1ca4d1596bf7a1201b815b1cf343895acbf90`, implement
E's exact request in [comment6221205479](https://github.com/Oteryn/Oteryn-Game/pull/2192#issuecomment-6221205479):
`PartyXpActivity::recorded_activity(&self) -> Option<(WorldId, ChannelId, [u8;16], GameSessionId, SemanticTimeMicros)>`.
It copies only the existing private stamp, preserving original microseconds and
identity; it neither renews/clears activity nor restores state or grants authority.
Existing identity/original-time/replay bodies gain exact tuple, empty-cell and
non-mutating/non-extending assertions; the17-body inventory is unchanged.
The separate827 E-binding source and its RED remain preserved outside this candidate.
No E binding, shared source or current-authority constructor is added. Direct fmt,
whitespace and cheap governance checks qualify source only; native execution remains
with the lead. This getter does not qualify E's whole-kill writer or joint runtime.

## D #2172 native E activity consumer — 2026-10-16

The lead explicitly reopened AUTHORING on isolated `skeleton/party-xp-1` at
`b856170ed896ab9b8d8cdbbd554a9c1aa49d3850`, allocating only `party_xp.rs`,
`party_xp_tests.rs` and this task record. This increment reuses the focused owned
827c9e54 binding delta without cherry-pick, merge, rebase, reset or a foreign write.
The b856 `recorded_activity` getter and the entire existing 17-test source remain
literal; three native consumer boundary bodies are appended (20 total).

Production `plan_shared_xp` and `PartyXpKillInputs.activity` now consume the actual
`ChannelAttackStates` owner. The planner resolves each strong-source member's native
actor by current session, compares the current physical position and World/Channel,
and calls E's real `shared_experience_active(runtime, actor, session, original_at)`.
No activity stamp is constructed from a successful predicate, renewed, or restored.
The retained recorded-cell evaluator is private rule-test support; it is not the
production kill consumer's input. D continues to decide membership, 30-tile range,
level ratio and actual leader-PZ observation independently.

Exact provider: #2192 `8b79b5d31b20d8143705785838786bfcc1d8340a`, `attack.rs`
SHA256 `ddff75a96e857d4abd5ce2b27dfe10e95a1ab68dede3e16b4819aff68642f73b`,
handoff [comment6222804645](https://github.com/Oteryn/Oteryn-Game/pull/2192#issuecomment-6222804645).
Its real predicate checks current owner/control/Character lease, original semantic
microseconds and inclusive 120-second age. E alone owns actual committed attack/heal
recording and its positive activity/expiry/replay tests. This source does not add
that provider, its foreign spell/heal hook, or shared module registration.

The three new bodies use native committed-assignment/session/position/Character-lease
fixtures and E's actual public(crate) methods. They exercise preallocation without
activity grant, unavailable/blocked leader-PZ precedence, missing current session,
physical-position substitution, retired owner continuity, and strong membership
removal. Each rejection changes its relevant input independently; no substitute
provider boolean, synthetic native stamp or widened foreign fixture is introduced.
They qualify consumer boundaries only when executed, not a positive committed event
through the full World/TX/E/D path. Positive native engagement and inclusive expiry
must additionally run E's own existing activity bodies against the same provider.

Frozen Rust bytes for independent review/native composition:

- `party_xp.rs` SHA256 `17828137d2e8057748935c79aa27a58ba538f01327d3eefb9c8bee967db1a7ef`.
- `party_xp_tests.rs` SHA256 `cdd03d6311f416d9318bd4c1c00a36ccd599c0006d555f2eba5fc6ac09f4e467`.

Direct pinned Rust 1.94 rustfmt, whitespace and cheap governance checks qualify
source only. Native Cargo/Clippy/test execution is reserved to the lead's private
composition with this exact E and actual #2201 sources; this author did not run
Cargo. The old 827c binding RED caused by the absent E port remains historical and
does not describe this now-concrete dependency. No PostgreSQL, full E-D/World path,
atomic whole-kill settlement, gameplay completion, commit/push, readiness, merge,
production action or paid-review PASS is claimed by this increment.
