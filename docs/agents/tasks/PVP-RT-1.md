# PVP-RT-1

```yaml
task_id: PVP-RT-1
title: "PVP-RT-1: PvP combat runtime (skeleton)"
mode: IMPLEMENTATION
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-rt-1
issue: 1622
pr: 2204
base_sha: c3ad75d493945469c3f12b01363b4268c6215681
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 PARTY/PVP PVP-RT-1; wave 2"
class: "hard (combat), combat and security review"
migration: no
registry: no
owner: resource_resume
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pvp/lock.rs
  - apps/game-server/tests/pvp_rt_readmission_postgres.rs
  - docs/agents/tasks/PVP-RT-1.md
slot_only_paths:
  - mod.rs
public_contracts: []
depends_on:
  - "ATTACK-1b: MERGED"
  - "COND-1: MERGED"
  - "PVP-1a: MERGED #2032"
  - "PVP-1b: BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Current allocation (Root exclusive lease, 2026-10-16): implement only the existing native
PVP-1 read → runtime logout/PZ/kill readmission restoration in the three paths above, on
`8474c609f93f42ed806cfb5c63b51a48028a32fb`. Earlier card-only packets below are historical.
No shared mod, legality, durability writer, transport, Cargo or migration change is authorized.
Worker publication remains forbidden. The complete PvP activation dependencies remain open.

## Goal

Runtime wiring of PvP legality into attacks and spells.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ATTACK-1b | MERGED |
| COND-1 | MERGED |
| PVP-1a | MERGED #2032 |
| PVP-1b | BLOCKING: not merged |

**BLOCKING:** PVP-1b. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Original implementation paths (historical; not this allocation)

- `apps/game-server/src/combat/pvp/legality.rs`
- `apps/game-server/src/combat/pvp/lock.rs`
- apps/game-server/tests/pvp_rt_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- mod.rs
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/attack.rs` target legality
- `combat/pvp/legality.rs`

## Inseparable bundles

none.

## Original family acceptance criteria — OPEN

- [ ] Illegal PvP target refused before damage; protection zone respected.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Original implementation validation — NOT EXECUTED in this packet

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` App. 7 PARTY/PVP PVP-RT-1.

## Original builder and CP rules

- Work on branch `skeleton/pvp-rt-1`. For this allocation the worker performs no commit or push; root alone may publish the reviewed card to the existing draft.
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: native caller/residue packet prepared; runtime acceptance remains OPEN
status: prepared
branch: skeleton/pvp-rt-1
blocker: PVP-1b
next_action: CP reviews report and allocates exact gated runtime integrations
```


## Native caller and residue packet (2026-10-15)

### Qualification and source boundary

Inspected draft source `c3ad75d493945469c3f12b01363b4268c6215681`; read-only accepted main
`bda15eba7a84f65fcb963a86e8b2bf18d69c800a`. No implementation, new ID, content/ruleset row,
callback, writer, authority flag or capability is introduced. Source hashes, main deltas, external
prepared-input hashes, local checks and scope evidence are retained under
`/workspace/oteryn-prep/pvp-rt-1-evidence/`. No runtime, PostgreSQL, legality, replay, mutation,
protocol or client tests ran in this preparation. The matrix below is a future qualification plan.

Accepted authority is `OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md` (§4.2,
§6–11), through `ACCEPT_SOCIAL_MAP0` and main's 2026-10-15 status reconciliation. The draft's
candidate header is stale; the relevant rule body is unchanged. GUILD0 is accepted, but an
accepted design is not a current guild-membership service.

`OTERYN_GAME_PVP0_PVP_GAP_CLOSURE_CONTRACT_CANDIDATE_2026-10-10.md` remains **CANDIDATE** on
main. Its D971 owner answers are recorded, while §6 expressly keeps amendments pending until
acceptance. In particular: 15-minute white skull after unjustified kill; one-ruleset/field-policy
readiness reconciliation; no-PvP tile rule3b; pvp-zone exception removal; player field/wall hold
and PVP-FIELD0 contract; expert rule6b and black-skull mode reset; expanded go-live/client/block/
required-capability gate are not silently promoted by this packet. Preserve current refusals;
CP must resolve the accepted release contract before activating those paths. The existing pure
helper already contains candidate3b/6b behavior, so exporting/wiring it is not a harmless visibility
change. Do not invent a competing baseline helper; settle that contract boundary with its owner.

Accepted PARTY-PVP0's go-live gate already requires PVP-1 (1a **and**1b), PVP-RT-1,
PVP-DEATH-1 and PVP-WIRE-1 landed before character targets become attackable. The first World is
OPTIONAL, with no player harm before GUILD-WAR0; OPEN must be delivered, HARDCORE branches tested
but no World configured with it. No ruleset, field-policy default, unpopulated directory, supported
client bit or prepared evaluator grants activation. The proposed stronger PVP0 §8 gate remains a
separate pending release decision, not a reason to relax the existing gate.

### Rechecked family gates

| Component | Exact available behavior and missing handoff |
|---|---|
| PVP-1a / #2032 | Migration0081 creates state, ledger, points and marks with World/deadline/revision guards. `durability/pvp_state.rs` has fenced deadline write-ahead, DB-clock remaining-time admission reader and bounded cleanup. None is a production combat caller. This is not all of PVP-1. |
| PVP-1b / #2203 | `/workspace/oteryn-prep/deliverables/pvp-1b/RESULT.json` says PREPARED_LOCAL_SUBSET: detached `combat/pvp/skull.rs` evaluator/tests, no merged registration, death writer, World job or ruleset delivery. Its age<window endpoint assumption and PARITY_PENDING still require review. Its reported12 tests are external artifact evidence, not tests executed here or proof of current authority. Source main has no skull module. |
| Ruleset / policy | `rulesets/pvp/{skulls,wars,arena}/index.json` and `rulesets/party/shared-experience/index.json` are READY_UNPOPULATED. `spell_field_policy.rs` reads immutable published field World policy, not an admitted `pvp_type` ruleset implementation. PVP0's mismatch-readiness adapter is still candidate and absent. |
| Legality/lock | `combat/pvp/{legality,lock}.rs` already implement pure ordered decisions,50% damage factor, deadlines and bounded aggression relations. `mod.rs` exposes them only under cfg(test); no production references to their APIs were found. A test fixture or matching data struct is not a live/current observation. |
| ATTACK/COND | Actual attack-target/swing, spell owner, condition/periodic damage and death consumers exist. Their present creature-only/PvP-unavailable behavior is an intentional integration boundary; COND presence does not make player DOT legal. |
| Party/guild | Genuine current World party SQL/private actor-binding ports exist; no native guild-membership owner/reader was found in inspected source. A party cache, source role group or Exiva preference cannot attest non-membership in a guild. This is a concrete provider gate for rule5 and group retaliation, not a default false. |

### Source/consumer map

Paths below are under `apps/game-server/src/` unless qualified. Main deltas are bound in the
manifest; main's newer logout path is inspected separately rather than assumed present in the draft.

| Source / actual consumer | What executes now | Precise PvP residue |
|---|---|---|
| `combat/pvp/legality.rs::check_pvp_legality`, `pvp_damage` | Pure World→level→PZ→candidate no-PvP→login immunity→party/guild→secure→candidate expert→black-skull decision. Unknown no-PvP/group facts refuse. Damage50% (floor),100% on black-skulled target, after draw before mitigation. | Current `PvpActor/PvpRelation` values hold data, no actor/session/Character/World/authority identity. Caller must establish each field under the actual owner. No production writer or observer is created by these predicates. Do not pass all-true visibility/current flags. |
| `combat/pvp/lock.rs::{CombatLock,AggressionRelations}` | Deadlines only rise; exact-deadline equality expires. Hit logout60s; aggression PZ60s; kill DB+15min; union also includes PvE in-fight.64 aggression relations, oldest expired eviction, full set refuses new aggression. | No live actor cell, durable write-ahead/restore invocation, white/yellow projection, ledger, Join Aggression, current time monotonicity or input sequencing. No skull/kill classification calculator should be duplicated here. |
| `gameplay_transport/attack.rs::{target_facts,ChannelAttackStates::set_target,drain_auto_attacks}` and `combat/attack/target.rs` | Exact actor, visibility, position, live creature/kind, PZ/reentry checks, server fight modes, held target/swing lineage and owner commit. Player targets fail `contains_live_creature`; player summons/familiars also excluded. `secure` is carried but has no current creature effect. | Add player/summon-owner admission only after release gate. Legality must run on set target and again for each harm, not only once when holding target. Resolve real player lease/session and PvP10s login provenance; the existing4s PvE reentry flag is not that immunity. Current secure mode is accepted server state; expert mode has no current protocol field. |
| `gameplay_transport/mod.rs::cast_spell` fallback → `actor_spell.rs::cast_in_channel`; `spell/combat_execution.rs::commit` | Fallback resolves/casts a player-state successor after durable-stance handling. The historical immediate single-target damage bridge explicitly returns AwaitingLiveAttackerFence; no callers make it a live multi-target writer. | Audit fallback routing and refusals when adding player-target support. Do not revive the old bridge or attach PvP solely there and leave the actual composed route unchecked. |
| Native dispatch in `gameplay_transport/mod.rs` → `native_combat_cast.rs::{cast_native_combat_inner,prepare_from_owners}` and `ordinary_combat.rs::prepare_inner` | Native authored spells use genuine source/access/build/equipment, current player/creature predecessors, tile/sight/PZ checks, preview, retained command, durable cost/training and physical batch installation. Ordinary aggressive legal-set only includes live attackable masterless creatures; native target legal flag says actual PvE only. | Add one qualified PvP legality observation in the existing stage before draws/payment; area skips refused player targets, single target refuses. Retain full census/target preflight and exact revalidation. A boolean `target.legal` produced by numerical/content plan is not independently current permission. |
| `spell/magnitude_owner.rs::PreparedMagnitudeOwner::{qualify,finish_with_damage_healing}` | Already handles qualified critical/armor/creature resistance and atomic lowerings; damaging Player target rejects because mitigation expects Creature. | Merely admitting players upstream still cannot execute player damage. Integrate real player defence/equipment/conditions and accepted PvP factor at the proper stage with LOOP-DAMAGE owner; do not reuse creature intrinsic-resistance values for players or add a second damage engine. |
| `gameplay_transport/ordinary_combat.rs::prepare_due`, `spell_timer_callbacks.rs` | Due chain phases re-read caster numeric owners and validate retained source/target/predecessors; resolved draw retained on retry. | Read applicable current player legality at the due phase and preserve source-defined frozen recipient/step semantics. Cast-time permission cannot authorize a later fresh hit after membership/PZ/skull/session changes. Replayed committed phase must not reselect a killed target or mint new aggression. |
| `gameplay_transport/spell_periodic.rs` → `spell/periodic_execution.rs::stage_player_periodic_turn` | Genuine creature-field source handled separately; player/unknown provenance receives `|_| false`. Unknown PvP grant leaves harmful DOT pending while independently qualified non-damage progression may proceed. | Add real per-tick player/summon owner/provenance+legality observation, accepted50% factor and ledger update in the actual turn. A historical field creator receipt is not current permission. Preserve Agony/condition-owner distinctions; do not infer a present source from its old bytes. |
| `field_step_ingress.rs`, `ordinary_field_items.rs` | Current NoPvp field branch already skips: historical PVP0 document's old both-pvp-zone grant is no longer present as effective code. Other player initial-field branches refuse NotQualified before HP; safe authored field creation is restricted to qualified NoPvP contexts. | Do not mechanically remove a nonexistent effective grant or turn later stale source comments into authority. Player initial/ticking field and wall legality needs the owning contract/activation decision; no broad gate opening while tracing. |
| `durability/world_party.rs`, `gameplay_transport/party_spell_owner.rs::SourcePartyWorld`, `durability/party_target_binding.rs` | Same-transaction strong World read, source party revision/members, genuine colocated session/account/lease/runtime guard binding and physical predecessors. Cache projected on actual actor is read-only. | Reuse private current transaction ports for rule5, but supply actor+all player targets and durable guild relation too. `SourcePartyWorld` is a source spell snapshot, not a universal PvP-current grant. Source `game_world_party_*` tables are actual ports; do not query historical `game_party_members` names blindly. Accepted §4.2 relation is taken at the strong read; later membership commits apply from next action. |
| `durability/spell_privacy.rs::decision`, `spell_familiar_group.rs`, `gameplay_transport/parameter_cast.rs` | Named-target source access/staff/privacy checks under current session; own-guild/party/VIP filters explicitly UnknownRelationships when no admitted relation exists. Ordinary Game group is access role, not guild. | Preserve disclosure/visibility limits, but never treat permission to locate/name a player, Exiva allow_all, ordinary role or absence from a local viewport as permission to damage/non-membership. Do not expose private yellow/orange skull relations through a global cache or name lookup. |
| `durability/pvp_state.rs::{write_ahead_pvp_deadlines,read_pvp_state,drain_pvp_expiry}` | Actual actor session/recovery/node fence,1–2 distinct targets including actor sorted CharacterId, monotonic DB+70s update; remaining deadlines read on DB clock. No CharacterRevision advance.0081 guard binds stored World to Character root. | No live callers, per-action receipt, 10s throttle orchestration, all-source/PvP ledger snapshot or current runtime restoration. Existing methods open their own transaction: composing paid casts/party/death requires exact owning in-transaction port or separately observed write-ahead with renewed caller checks; not a counterfeit SpellItemAuthority. |
| `durability/character_death.rs::{settle_player_death,commit_character_death}`, `gameplay_transport/mod.rs::{settle_death,commit_death}` | Existing victim progression/death receipt sequenced under current Character/session fence, pending respawn/replay. | No PvP classification/skull/mark/kill-block/ledger deletion or PvP receipt fields in this source. Extend the victim's actual death transaction with PVP-1b/PVP-DEATH owners; cannot make a post-death independent skull writer atomic retroactively. |
| `gameplay_transport/actor_movement.rs::bundle_step`, `map/floor.rs::{prepare_bundle_step,landable_of}` | Floor resolver accepts PZ-block input; bundle caller explicitly passes false because no current runtime PvP block exists. | Supply actual current union of PZ and kill blocks, checked before actual step/teleport/fall landing publication; include other movement paths, not only teleport. Do not keep false once PvP activates. |
| Draft `gameplay_transport/mod.rs::held_step`; refreshed-main `gameplay_transport/logout.rs::{logout_session,logout_in_fight}` | Existing disconnect hold reads PvE in-fight. Main adds explicit logout handler with pending-spell reconciliation and PvE deadline check. | Integrate full PvP/logout/PZ/kill union into release, voluntary transfer and party leave, preserving current release/loss semantics. `world_party.rs::PartyLogoutEligibility` is sealed, same-transaction and all-deadline; Leave already refuses without real proof. Do not replace the seal with caller bool. |

### Exact CP handoff, sequencing and observation deltas

The following is a proposed integration allocation, not permission to edit these files now.
No global trusted snapshot flag, new grant or new schema is specified by this report.

1. **Decision and delivery gates first.** CP resolves PVP0 candidate amendment/activation scope;
   PVP-1b supplies actual current skull/consequence writer and ruleset provenance, not just its
   archived evaluator; PVP-DEATH/WIRE/client/block owners qualify their promised behavior.
   Retain current creature-only/unavailable paths until the applicable accepted gate is met.
   Optional World still gets no wars from this task. A ruleset change never clears PvP state.
2. **Current observation owner.** Allocate a private Channel PvP owner adjacent to existing
   attack/spell owners; consume native actor, GameSession, Character/account binding, connection,
   lease, distinct World/Channel and scope generation, content/ruleset/map revisions, current
   position/vitals/level, accepted fight-mode revision, login time, stored skull/deadline revision,
   and current party/guild observations. Read from actual session/physical/DB owners; facts do not
   grant authority by matching immutable intent. Use existing `SpellItemAuthority` or genuine
   `SpellItemScopeAuthority` for commandless phases, not a synthesized command.
3. **Single ordered decision at each real harm boundary.** Call the existing accepted legality
   component from `attack.rs::set_target` and `drain_auto_attacks`, native/ordinary prepare and
   NEW-write validation, due callbacks and periodic application; explicit current observation
   validates same source/target predecessor at physical installation. Resolve summons to their
   actual current owner and contribution lineage. PZ/visibility, condition gates and applicable
   action admission remain owned stages; legality must not be moved into magnitude finishing.
   Add pending3b/6b only once accepted; preserve first-refusal precedence. A whole-area read batches
   all character relations; one target's refusal skips that actor, not a global permission grant.
4. **Durable-before-action sequence.** For an admitted action calculate only typed intent, preflight
   bounded runtime/ledger/outbox effects, then ensure affected deadline columns and any new
   contributor are durably written ahead under acting Character's rule2 fence before damage,
   assist or aggression takes effect. `pvp_state` at most2 targets matches one actor/target write;
   area orchestration must qualify its complete footprint and native limits, not widen that API
   arbitrarily or publish damage before the final required write. Retain the accepted10s throttle
   and new-contributor exception. Existing standalone writeahead has no command receipt and a
   retry can raise deadlines: unknown COMMIT requires actual read/reconciliation and exact pending
   occurrence, not assuming rollback or applying the hit. CP must allocate a concrete replay-safe
   in-transaction/observation seam before activating paid casts; no fake durability writer here.
5. **Lock orders.** Retain paid spell lane → runtime → player/spell states → attack ownership
   order; do not acquire the lane from inside Channel guards. Existing World party owner serializes
   under advisory namespace45 and real transaction seal; preserve its ordering with membership
   writers. PvP write-ahead state rows ordered by CharacterId after actual actor fence. Death
   keeps victim CharacterRevision slot/root first, then victim+contributors' PvP states ordered
   CharacterId, then point/mark rows; do not lock contributors' progression roots or advance their
   CharacterRevision. Concurrent party/guild writes follow accepted relation-at-read semantics,
   not an invented permanent membership freeze; current physical predecessors still revalidate.
6. **Apply and retain one occurrence.** Commit live hit, correlated shield/HP/conditions/death
   consequences through existing native physical batch/vitals owners, then typed descendants as
   accepted. Preserve damage draw identity and apply PvP factor once before mitigation (not again
   in a DOT descendant that is already scaled). LOOP-DAMAGE packet owns current equipment and
   defensive-composition integration; this packet supplies target permission and PvP consequence
   input, not another protection/critical/shield calculator. Friendly-fire area hits between
   current shared-enemy actors create no aggression/skull/block/points. Retaliation and Join
   Aggression must use real aggression relations, not client-reported enemy sets.
7. **Ledger and death.** Add runtime ledger/all-source totals to actual committed harm/assist
   events including monster/field/condition damage while PvP window exists; flush at most10s
   behind plus mandatory new-contributor writeahead. Keep31 ten-second buckets,16 full+64 compact;
   demote least damage (assist-only first then oldest), evict oldest compact, never reject legal
   harm for ledger space. Aggression64-full refusal is a different bound. Final-blow contributor
   always classified; no loss of victim PvP ratio when a contributor demotes. At death classify
   from actual window and viewer-relative victim mark at death; points, skull evaluation, kill
   blocks, revenge marks and victim ledger deletion join the same victim receipt. No XP for PvP
   kill. Reuse reviewed PVP1b evaluator only after its endpoint/row provenance acceptance and
   current DB observation; it never attests points, skull or death authority by itself.
8. **Restoration and controls.** Admission/reconnect/crash/channel entry must read PVP state and
   ledger under current fences before accepting relevant input. Translate observed DB remaining
   duration to current monotonic owner time without shortening; unknown/overflow/regressed/future
   facts cannot mean clear. Restore union with `CombatLock::restore`; never lower stored deadlines.
   Runtime yellow/retaliation relations rebuild after crash while restored white remains public.
   Movement, voluntary switch, logout/release and party leave read that union; kill block remains
   strong Character+World across channels. Pending committed consequence notifications must be
   installed before subsequent affected actor input; no candidate15min-white/expert reset is
   declared accepted by this requirement. White/yellow/orange projection remains distinct from
   red/black stored skull and obeys viewer/privacy owner.
9. **Shared slot only.** Future exact allocation includes combat_pvp exports, actual private caller
   module, `attack.rs`, `native_combat_cast.rs`, `ordinary_combat.rs`, callbacks/periodic consumers,
   `pvp_state.rs` in-transaction joins, `character_death.rs` owning transaction, actual movement/
   release/party proof paths, and admission/connection/registry/client pieces under their owners.
   No `mod.rs`, protocol `lib.rs`, migration number, content pin, ruleset population or registry
   write now. Numbers for PARTY_V1/PVP_V1/expert/refusals remain CP leases; no invented wire IDs.
   A command selected or skull encoded is not proof of current effective combat.

### Required native negative matrix — NOT EXECUTED

Every consumer below needs a real-native positive control and one applicable invariant changed
per negative case, using independently current owners. Matching record-derived fixtures cannot
qualify authority. Runtime/SQL observations and replay/restore tests must bind final integrated
source; PVP1b's earlier tests and this source trace do not satisfy them.

| Invariant / mutation operator | Real consumer and expected observation |
|---|---|
| Missing/stale actor or target session, lease/connection/scope generation, World/Channel substitution, retired/control-loss actor | Set target, fresh swing, NEW paid write, due/tick, physical install and restore refuse new harm/authority. Historical terminal receipt still settles once without a new live grant. |
| Optional without war; absent/unknown/mismatched ruleset/policy; unsupported Retro; arena-like tile | World admission and each legality stage cannot gain player permission from defaults. Existing accepted gate stays closed. Candidate reconciliation/arena rules require acceptance-specific tests, not a presumed ready flag. |
| Level7/8, either PZ, stale/moved target/map pin, visibility/name-only permission | Single refused before damage/payment; area skips refused actor; stale/unknown physical observation cannot produce an eligible player. Existing PvE behavior stays qualified separately. |
| Login time9.999s/10s, future/regressed time, genuine retaliation vs new aggression,4s PvE protection mistaken for10s PvP | Exact admission/current harm boundary uses correct time domain/provenance; no fabricated immunity or aggression bypass. |
| Party/guild read failed, stale/mismatched revision, missing guild owner, join/leave between phases, remote same-World member | Batched rule5 read and NEW-write/due observation follow relation-at-read, fail PARTY_STATE on stale/missing source, do not infer solo/no guild from local census. No channel-location disclosure through harm checks. |
| Secure toggled after target held; marked only toward another viewer; expired red/black/orange; counterfeit client mode/skull | Fresh harm uses current server modes and viewer-relative marks; no global yellow/orange grant. Candidate expert/default/RedFist-black-reset matrix added only under accepted revision. |
|64 live aggression relations/65th, expired eviction, retaliation refresh, shared-enemy friendly fire, Join Aggression partial eligibility | Actual action cannot publish partial relations; no new aggression when bound refuses; friendly fire produces no block/skull/points; group membership/aggressors not forged. |
| Writeahead fails, stale fence, unknown COMMIT, canceled await, duplicate command/source substitution, concurrent deadline raise | No harm before observed durable prerequisites; pending occurrence/resources retained and reconciled; replay does not reroll, recharge, double damage or reset timers. Conservative durable consequence never creates an action authority. |
|16 full+64 compact ledger boundaries,81st contributor, bucket expiry/endpoints, monster damage after last PvP hit, new-contributor failed flush | Never immunity from ledger full; all-source/PvP totals preserved independently; bounded demotion/eviction; no contributor lost before contribution; crash loss bounded per accepted rule. Test tie handling against exact owning descriptor. |
| Death classification mark changes, final-blow absent from oldest ledger, duplicate PlayerDeathOccurrence, concurrent contributor skull/points change | Victim sequencer/transaction owns one death+points+marks+kill-block+ledger deletion; classification source exact and no XP; no contributor CharacterRevision advance or post-commit fake atomicity. |
| Black/red expiry, threshold windows/new-point reset, clock skew, corrupted row World, readmission on another Channel | Actual DB reader/current fence and native restore preserve consequences; migration guard remains tested, remaining-time read cannot be treated as an unbound live capability. PVP1b age-window proposal qualified explicitly. |
| Active PZ/kill/logout union vs only PvE, exact expiry, step/teleport/fall, release, voluntary transfer, party leave | All real controls refuse while applicable block runs; reconnect/restart cannot clear it; sealed party proof tied to actual SQL transaction/current occurrence. Missing proof never means locks_clear. |
| Native vs ordinary/fallback spells, area shared draw, delayed chain target replaced, DOT player source gone, field historical owner, rune replay | Every effective player harm reaches same accepted legality stage and proper current phase; no legacy bridge bypass, no second PvP damage engine, no source-history grant, no unsupported field/wall activation. |
| PARTY/PVP wire unsupported, unknown fields/mode, capability selection vs effective handlers, private skull projection | Release gate/admission/codec/dispatch/emission/client tests prove promised behavior together. Do not mint expert/mark/Character or Party authority from payload; keep protocol-oteryn and distinct World/Channel. |

### Packet validation and remaining work

Only the owned card changes. Governance validation and its124 tests, `git diff --check`, exact
base/branch, source bindings, allocation-only diff, empty staging/untracked and unchanged push-deny
hook/URL are the local checks. Executed: governance passed (22 required documents,9 project
lanes); governance unittests124/124 passed; allocation/scope and diff checks passed. Evidence binds
49 repository inputs and4 archived prepared-input files to the exact inspected trees and card.
No Rust build, Clippy, formatting or runtime/PG suite is appropriate to this report-only allocation.

The original family checklist remains **OPEN**. This packet supplies the native residue trace;
it does not claim integration-head code confirmation, illegal-target runtime test, current-party/
guild/PvP authority, replay durability or release readiness. Pending: PVP0 contract disposition,
PVP1b/full death delivery, admitted rulesets, real guild/current-PvP observers, caller/transaction/
ledger/control integration, final source-bound native+PG+client matrix, independent combat/security/
persistence/privacy/protocol and architect reviews, and CP shared integration slot. Root owns any
publication to the existing draft; worker performed none.


## Native deadline readmission component (2026-10-16)

Actual source authored, runtime qualification **UNEXECUTED**. Accepted PARTY-PVP0 §8.1 admission
and restart semantics apply to this three-deadline component. PVP-1a's actual read and tables
are present; this increment consumes no pending PVP-1b/#2203 skull evaluator or fresh-harm writer.
Early independent clock/native-Root design review: `reviews/pvp-rt-readmission-early-design-vip.json`
(`0781d61326ec0431acad7248455a31aeef0e057413a4b82cb6f50e359496eef4`).

`CombatLock::restore_pvp_readmission` awaits the actual public `DurabilityRoot::read_pvp_state`.
That owner checks the real reconciled recovery capability and current session/node/lease/
World/Channel/scope through the existing guards. This adapter duplicates no SQL or authority
check. The existing `OwnerClock` is owner-axis DATA: sample it once after a successful row read,
explicitly convert the Foundation microsecond axis to the simulation axis with checked
milliseconds × 1000 and checked addition, calculate all three deadlines, then invoke the existing
monotone `restore` once. Missing rows, read errors and any overflow leave the previous lock
unchanged; elapsed values never clear it. A boolean return reports row presence, not a controller
grant. A clock from another owner axis is not a valid installation input.

The new ordinary native test wrapper uses unchanged physical Root/Foundation/durability and
`tests/support/bestiary_postgres_harness.rs` source, following the existing
`combat_bestiary_postgres.rs` composition. The existing `pvp_state_postgres_cases.rs` is a read-only
reference. No private helper is widened and no guard body is copied. Administrative fixture
consequence rows are test DATA; actual recovery, Root, node registration, scope assignment and
session fixture APIs are retained. Missing canonical PG configuration is an error; actual
PostgreSQL `170006` is mandatory. The top inactive wrapper attribute remains a CP routing slot.

Nine new PG test families are written: each real stored deadline; missing/expired rows; repeated
shorter data; independently changed Character/session/connection/zero or stale lease/World/
Channel/scope generation; actual Node revocation with expected bindings unchanged; another actual
Root receiving the first Root's authority; an actual database query failure; later-slot overflow proving no partial mutation; and
an actual blocked native DB read while the existing virtual OwnerClock advances. The native
read deliberately consumes `item_fence`, not CharacterRevision; its non-use is a positive control,
not an invented rejection rule. Three pure tests cover explicit axes, multiplication overflow
and addition overflow. These tests are **written, not executed**; cheap formatting is not a native
pass. Full compiler, configured Clippy, actual PG, source/body/extern proof and final review remain
pending a reviewed recipe and explicit Root heavy GO.

This is an uninstalled native component in the original lock source. Actor/caller installation,
white skull, aggression reconstruction, damage ledger and full admission orchestration remain
with their accepted owners/CP. Fresh harm/write-ahead receipts, PVP-1b, PVP-DEATH, wire/client and
all full PvP activation gates remain open. No `pvp_state.rs` lease or second writer is inferred.

## Native readmission component — executed qualification (2026-10-16)

This increment adds `CombatLock::restore_pvp_readmission` within the original PVP-RT-1 ownership. It awaits the existing fenced `Root::read_pvp_state`, samples the existing `OwnerClock` once after the successful database read, checks millisecond-to-microsecond multiplication and owner-axis addition for all three deadline slots, converts explicitly to the simulation time type, and invokes the existing monotone `CombatLock::restore` once. Missing/expired data, refused reads, database errors and overflow preserve existing locks. Persisted data does not confer authority; no second SQL writer or replacement authority provider was added.

The original 35,194-byte task-card prefix is retained exactly (SHA256 `e5506c8d8509b703c1897690db593095c140e31472d0820c7b52c9e0742f7faf`). The qualified Rust source is checkpoint `58e1ad2d238ec24f9f07f3d59d008d3e44f5a903b8db6bbf5c09d92d79f4d79e` on native head `8474c609f93f42ed806cfb5c63b51a48028a32fb`: core `4d06e99885c65d91f5d547b52bd5dfab75eedb3e79a3be6bc3ce36e60417f88f`, test wrapper `cabf3a929b367aabe204e388eb8881fee5feccc0bb145251fda03e4afd956c67`.

Actual PC qualification used Rust 1.94.0, a fresh own-head locked/offline Cargo preparation and actual Cargo compiler capture, the exact 1,959 immutable native inputs, explicit physical Root/Harness module composition, and authenticated canonical PostgreSQL 17.6 (`server_version_num=170006`). The ordinary wrapper is outside the original-library namespace; its production-server dependency is a real own-head compiled library. The selected test dispatcher preserves all 12 reviewed brace bodies and their original current-authority guards.

Executed results:

- Original production strict Clippy metadata: exit 0; own production library codegen: exit 0. These are retained actual checkpoint `0bd8e8979859c85bc2133eab0fb03804212dae6095dd2a6e1423d0ea25075832` results, reused only after complete unchanged production-input, compiler, dependency, generated-input and artifact checks. The later owned-test-only timeout repair does not enter production `lib.rs` inputs.
- Original ordinary-wrapper metadata: strict exit 1 on unused items in this non-harness metadata context; configured exit 0. This remains a strict failure, separate from the selected strict result.
- Current selected ordinary-wrapper strict Clippy metadata: exit 0. Selected native library codegen: exit 0. Driver link: exit 0. The successful link uses a detached, full-hash-verified canonical-name byte copy of the retained production output; that copy is not a new compiler producer.
- Native execution: **12 passed, 0 failed, 0 ignored**, consisting of 3 pure conversion cases and 9 PostgreSQL families across **11 isolated actual PostgreSQL 17.6 fixtures**. Native driver invocation retained `--test-threads=1`. Families cover all three stored deadlines, absent/expired rows, monotone repeat restoration, actual current-session/lease/scope refusals, revoked Node, another Root's authority, actual database errors, all-or-nothing later-slot overflow, and owner-clock sampling after a real blocked database read.
- Literal source/body inventories, compiled input maps, actual compiler/extern/generated input hashes, and all native artifacts were checked before consumption and after execution. Source guards remained equal. The original production byte-copy operation preserved its source inode metadata. The source-card bytes consumed by qualification are the retained prefix above; this final evidence append changes documentation only.

Retained native output SHA256s: production `43a07b03b6371ee2aa71b48270f32609df5b3f980ba71e9c25127de61d642488` (399,283,878 bytes), selected wrapper `b53ded9cca31a28bff0e2d27f221c51da147da5d11e2841a208e207376d2f30a` (74,144,498 bytes), driver `3c9c1302e2e16f9285f0e362c4bd3113ca597a8d8972bbf0466628d03ca15c3d` (25,963,176 bytes).

Durable actual evidence: `reviews/pvp-pc/actual-proof-v7/actual-final-summary-v7.json`, SHA256 `f624e2f3b558d7f929de8274b69c398194394bd3b703dcb037a0c50f9bdb097a`; complete 140-file proof pack SHA256 `b9f647635b746f0760608244ce97fa641c1b4e8ba9807e4ac02ea1b2bbd50edc`. All failed intermediate attempts remain retained: unavailable `tokio::select!` in the test fixture was repaired with the existing borrowed-future timeout API; a generated dispatcher placement was repaired without changing owned source bodies; transitive library discovery was repaired with the detached canonical-name copy. Metadata/codegen successes were reused with their original provenance rather than relabelled as new executions.

This is a qualified readmission deadline-restoration component. Actor/caller wiring, full PvP activation, fresh-harm writer composition, skull/ledger restoration and pending PVP1b integration remain separate owners and gates. No full workspace/Cargo-test or complete governance PASS is inferred from this selected native execution; the sparse native snapshot lacks the complete governance input set. The original aggression code and existing guarded reader remain unchanged.


## Default library unit lint repair — qualification pending (2026-10-16)

Actual CI job `116931881038` rejected three `clippy::expect_used` assertions in
`readmission_explicit_owner_to_simulation_axis_and_elapsed_data` (`lock.rs` lines
299, 302 and 304 at published `087f98040edbcfe8ff820e1fe2117a09e1ab416f`).
The retained CI log reports the Rust 1.95 Clippy documentation URL and a library-test
compile failure with three errors. The earlier selected-wrapper strict result did
not qualify the default library unit lint context; its successful scope remains
as recorded above.

This repair replaces only those three assertions with equivalent Result matching:
`Ok(Some(9000))`, `Ok(None)` and `Ok(Some(7000))`, with all error results still failing.
No error trait, production helper, lint allowance, fixture or PostgreSQL body changes.
The two existing pure overflow tests remain literal. The prior native PostgreSQL
execution retains its original source and artifact provenance; this test-only repair
does not claim another PostgreSQL execution.

Ordinary Cargo native library-unit strict Clippy and the three existing conversion
unit tests are pending actual execution on the explicitly recorded local toolchain.
Local qualification will not itself establish a new CI Rust 1.95 result. The original
40,286-byte task-card prefix is preserved exactly.

## Default library unit lint repair — executed qualification (2026-10-16)

The three `expect()` assertions reported by CI job `116931881038` are replaced by
Result matching with the same required successful values and error rejection.
Only those three assertion expressions changed in Rust. The entire production
prefix, both overflow test bodies and the complete PostgreSQL wrapper remain
byte-for-byte equal to published `087f98040edbcfe8ff820e1fe2117a09e1ab416f`.

The local qualification uses ordinary Cargo and Rust/Clippy 1.94.0 against the full
native 18,809-file task-head context plus the three frozen owned files. There are
no module mounts, custom drivers, signature transforms or added lint allowances.
The earlier selected-wrapper strict result did not cover this default library-unit
lint context. The original CI Rust 1.95 `--all-targets` failure remains retained;
this narrower local run does not establish a new CI 1.95 or all-targets result.

Executed results on source checkpoint `ce5189b2011f38c140314aee87d77d2eac4b3d9b467209efd2ead25d5ff4d537`:

- `cargo clippy --locked --offline -p oteryn-game-server --lib --profile test --no-deps --jobs 4 --message-format=json -vv -- -D warnings`: exit 0, zero compiler errors. The actual single primary invocation contains `--test` and the unchanged manifest denials, including `clippy::expect_used`; Cargo reports `profile.test=true`, `fresh=false` and debug info disabled. No configured fallback or lint suppression was added.
- `cargo test --locked --offline -p oteryn-game-server --lib readmission_conversion_tests:: --jobs 4 --message-format=json -vv -- --test-threads=1`: native unit codegen and execution exit 0. **3 passed, 0 failed, 0 ignored; 3,179 filtered out.** The actual module is `combat_pvp::lock::readmission_conversion_tests`; all three existing test names are retained. This is focused execution, not a complete unit-suite run.
- All 18,809 compiled-source hashes and all three owned source inputs remained equal before and after execution. Earlier qualified production, selected-wrapper and native-driver outputs retained their full hashes and inode metadata; their historical producers were not relabelled.

The ordinary Cargo native unit ELF is 376,707,736 bytes, SHA256 `944a53933afebc3076203fe2c04cdcf050f472c004eac4f01716d06d5c63b8b9`. Its verified persistent lossless gzip is 59,296,557 bytes, SHA256 `5374706d57572bb9f4cba65cf25226e8eddeed3a1090b017fa4270c34ad880ca`; full decompression matched the original SHA and size. Raw RAM output remains retained. This execution used the explicitly activated RAM-target budget; earlier disk-floor stops remain distinct failures before Cargo, with no success inferred from them.

Complete raw actual evidence: `reviews/pvp-ci3-pc/actual-native-unit-proof-v1.json.gz`, SHA256 `4282e19cb053b1ca43b867ffed165841bc4ef739dd5beecbedaafc6987e2ff0c`, containing 47 losslessly reconstructed proof files. Source equivalence review: `peer-reviews/pvp-ci3-source-equivalence-ground-review.json`, SHA256 `b5c0104b45735a9d1546dfb821e68f289209cae0ea938b97cc9c2327fb47e636`. The 41,643-byte source-checkpoint-1 card prefix remains exact; this final append changes documentation only.

The previous 12-case readmission qualification retains its original source and
artifact provenance. PostgreSQL was not rerun for this assertion-only repair; the
production restoration code and all PostgreSQL bodies are literal. Actor/caller
installation and the full PvP activation gates remain unchanged.


## D1124 same-transaction clock preparation — NOT_FULLY_QUALIFIED (2026-10-16)

Authoring resumed from exact `4e4660762405315b9ae64cbed29c3cedcf0ec25f` on
`skeleton/pvp-rt-1`; no new candidate has been committed or published. D1124 assigns
`durability/pvp_state.rs` to this writer. This checkpoint does not alter the
readmission caller (`fresh_admission_composition.rs` remains deferred until #2187
merges), shared authority guards, production activation or the other PvP owners.

The fenced `read_pvp_state` transaction now returns `PvpReadmissionClock`: a typed
PostgreSQL Unix-microsecond observation and absolute logout/PZ/kill deadlines from
one materialized `clock_timestamp()` sample in the same query/transaction. Existing
remaining-millisecond fields remain available for their previous readers. Recovery,
current session, Node, scope and lease checks and the transaction commit boundary
remain unchanged.

The incorrect `OwnerClock.now()`-after-await translation is removed. PostgreSQL
Unix microseconds are not the owner's monotonic clock, so they are not cast into
runtime `SemanticTimeMicros`. No accepted database-to-owner axis binding is present
in this component: active stored locks now return `ClockBindingUnavailable` and
leave all existing runtime slots unchanged. An absent row still returns false;
a row with only elapsed/unset locks returns true without clearing prior slots.
This is fail-closed clock-port preparation, not completed lock restoration or a
claim that P2 is fully resolved. CP still needs to select the actual clock-axis
binding and its runtime consumer; caller integration remains pending #2187.

The old disabled standalone PG target is replaced by owned support cases, registered
with the single D1160 `#[path] mod` line in the existing canonical
`character_authority_postgres.rs` root. Its existing harness/production namespaces
are reused. No new duplicate-test allowlist entry or copied shared authority source
is added. This routed target
prepares same-query sample/deadline/remaining consistency and a real relation-lock
wait before the DB sample, active-refusal atomicity, absent/elapsed rows and the
existing current-fence, revoked-Node, different-Root and real-query-error families.
The old tests expecting node-time projection/overflow are superseded by explicit
missing-binding cases; historical execution evidence above keeps its original
source provenance and is not imported as evidence for this material change.

Validation: touched Rust files formatted with Rust 1.94 rustfmt; final format/diff
checks recorded in the review packet. Native unit tests, the PostgreSQL target,
strict Clippy and full package are **NOT_RUN** for this authoring delta. Governance
validation and all124 governance unit tests PASS.
The shared family full-package RED remains retained. Per D1122 this is at most one
coherent draft checkpoint before full checks; no ready/review/CI restart, complete
qualification, merge or production activation is authorized. Source review by the
lead precedes any commit or push.


## D1180 supersedes missing-clock-binding refusal — NOT_FULLY_QUALIFIED

D1180 ([CP2131#6225403220](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225403220))
requires no database-to-owner epoch binding. It selects same-transaction typed
remaining duration `max(0, deadline_DB - clock_timestamp())`, followed by
`OwnerClock.now() + remaining` in a synchronous apply, with zero meaning no lock.
This supersedes the earlier `ClockBindingUnavailable` preparation above; it does
not authorize a database epoch cast or node sampling around an await.

The same materialized PostgreSQL clock sample now also produces typed exact
microsecond remaining values for logout/PZ/kill. Fenced reads and query errors
remain under the existing Root transaction. The completed typed read is passed
to synchronous `CombatLock::restore_pvp_readmission`: no async body or I/O exists
there, the owner clock is sampled once, and all three checked deadline additions
finish before the monotone runtime mutation. Read/overflow failures preserve all
prior slots; absent rows and zero durations create no locks. Caller composition
in `fresh_admission_composition.rs` remains deferred until #2187 merges.

Three prepared pure cases cover one clock sample, positive duration and exact
expiry, zero/absent data, monotone repeat restore and a later-slot overflow/read
error with all-or-nothing preservation. Canonical PG cases now finish the real
Root await before calling the synchronous apply, retaining current-authority
negative cases and same-query duration/sample consistency. They are NOT_RUN for
D1180. The earlier clock-port qualification job runs against its separate
immutable source and retains only that older source's provenance; it was neither
interrupted nor modified. No new PG job/build/test is launched by this author.
Touched formatting/diff checks are cheap source checks only. No commit/push,
full qualification, ready status, CI restart or production activation is claimed.


## D1180 actual focused evidence and D1213 new fact — NOT_FULLY_QUALIFIED

The immutable D1180 source (full18809-file fingerprint
`3efd196dcd9c96fa5e5cdc699bf34741d8e2cc88d09347be0de55e67f8db2d68`)
passed canonical native listing10 and execution10/0/0: seven real PostgreSQL17.6
families plus three pure clock/atomicity cases. Source inventory before/after
was identical. Focused log SHA256
`f9824ff52effe51f0ea641e5ee36dc87672a79bff315df496093355a11ac205c`;
`evidence/2204-D1180-native-result.json` is the lead's external custody record.
This supersedes the D1180 NOT_RUN statement above only for that precise source.
Strict Clippy and the full package remain NOT_RUN; retained family full RED
is not resolved by these ten cases.

D1213 assigns `gameplay_transport/combat_lock_fact.rs`, its own module item,
and the existing `logout.rs::logout_in_fight` caller. The fact uses the actual
actor/session-indexed AttackState and the current spell-state dead-actor check;
locks remain spell_states then attack, with semantic-clock sampling after both.
Its PvP source is exhaustively `NotWiredInRuntime`, never a forged actor-bound
scalar CombatLock. This new fact/caller increment is not covered by the preceding
D1180 run; native/Clippy remain NOT_RUN until its own source is checked. D1216
selects main `b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16` for the actual logout
caller, as a local integration merge commit; no PR merge/ready/rebase is done.
Actual admission/readmission composition remains held after #2187.

D1216 local integration uses exactmain b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16;
merge conflicts: none. The existing logout_in_fight now consumes the one fact
and exhaustively handles NotWiredInRuntime before computing retry on the same
semantic sample. No held_step/hold_step_within or other root caller was edited.
This changed consumer and main integration require fresh scoped native/lint
evidence; earlier ten-case D1180 evidence remains a separate source generation.
