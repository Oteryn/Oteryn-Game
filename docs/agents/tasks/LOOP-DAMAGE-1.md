# LOOP-DAMAGE-1

```yaml
task_id: LOOP-DAMAGE-1
title: "LOOP-DAMAGE-1: native damage caller and capability residue packet"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-damage-1
issue: 1622
pr: 2195
base_sha: dfdaf256dacbc47fb2e3d953df81595cc5332e86
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-DAMAGE-1 (rows 23-26); App. 7 LOOP-DAMAGE-1; wave 2"
class: "hard (combat, authority, protocol integration design)"
migration: none_in_this_packet
registry: false
owner: family_F
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/LOOP-DAMAGE-1.md
public_contracts: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Result and source boundary

This is the allocated **card-only caller/residue packet**, not damage implementation or runtime
completion. No calculator, wrapper, authority callback, protocol ID, content fact, migration,
registration or offer is added. The historical skeleton's broad `combat/` and `ability/`
allocation is superseded for this preparation by `loop-damage-1-allocation.json` (one owned file).
The existing draft remains dependent on a future CP allocation and independent combat,
determinism, persistence and protocol review of the actual integrated code.

Native source inspected: `dfdaf256dacbc47fb2e3d953df81595cc5332e86`; allocation acceptance base:
`ac9f172836f00226b68088aa04db95ccf1ab994e`; refreshed read-only main:
`bda15eba7a84f65fcb963a86e8b2bf18d69c800a`. Relevant main/source differences are retained in the
evidence directory. Main adds logout capability 21; it does not offer capability 12. The inspected damage/equipment/condition bodies are unchanged. The manifest records three
changed native files: capability offers/tests for logout21, and `actor_spell.rs` Familiar
end-of-actor re-exports (no damage-body change). Acceptance text is read from main where the draft retains candidate
headers. No remote status or unmerged child is presumed delivered.

Evidence: `/workspace/oteryn-prep/loop-damage-1-evidence/` (`source-bindings.json`,
`acceptance-delta.patch`, `local-checks.log`, `scope-check.json`). This packet executes no combat,
PostgreSQL, RNG/replay, client or runtime tests. The test matrix below is required future work.

## Owning rules and corrected historical gates

Paths below are relative to this repository. Decision files are under `docs/architecture/reviews/`.

| Historical row | Exact current evidence and consequence |
|---|---|
| EQUIP-0 accepted | `OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_DECISION_2026-10-01.md` §3–4 and its 2026-10-13 acceptance are present. Six equipment abilities: skill/stat/speed/protection/suppression/light. It explicitly defers global equipment critical hits and life/mana leech; its title is not a contract for those. |
| CHARM4 candidate blocks all work | Stale: `OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md` §1 on main accepts CHARM-4. Its rule body is unchanged from the draft. Acceptance releases the decision gate, not CHARM-DEF-1 runtime delivery. |
| ITEM-MOVE-2a partly delivered | Still a concrete integration gate. `durability/character_equipment.rs` and `gameplay_transport/equipment_move.rs` implement fenced plain-item equip/unequip and receipts. `ARCH_BATCH_ITEM_EQUIP_PACKETS` §2.8 also requires occupied-slot swap, stack/B3 behavior and timed-host lifecycle as applicable; source explicitly documents free-slot-only movement and refuses timed equips. No cap12 offer follows just from these seams existing. |
| EQUIP-RT-1 | Typed `content/item_abilities.rs::item_ability_profile` exists; the architect's §2.10 `domain/equipment_effects.rs` active-set owner is absent in both inspected trees. Existing movement/cast reads do not establish the complete accepted current-active-set policy. No duplicate generic modifier engine is justified. |
| COND-1 absent / mana shield must be written | Obsolete: `ability/condition.rs`, `condition_spell.rs`, `spell/actor_conditions.rs` and owner consumers exist, including §3.5 gates and Agony. CONDITIONS0 acceptance describes earlier merged families; source, not its historical remaining-work prose, determines current residue. Reuse these stores/transitions. |
| FOOD-REGEN-1 not merged blocks any damage work | Too broad: `spell/food_regeneration.rs`, owner food application and periodic consumers exist. CONDITIONS0 §6.3 durable remaining food time is a separate release/restart obligation; no dedicated food durability module was found. This is not proof all possible save paths are absent, nor a reason to duplicate shield/damage code. CP must qualify the actual lifecycle writer before claiming COND-DUR-1 complete. |
| Crit/leech globally specified by generic Ability baseline | False: `GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md` requires typed stages and descendants, but explicitly leaves exact crit/lifesteal formulas and ordering to owning decisions. Accepted IMBUE-FORGE0 §5 supplies stage ownership; its single-`crit` amendment is conditional on PROF-EFFECT0, whose inspected main header is still CANDIDATE. Do not silently activate that amendment. |

EQUIP0 §3.2's 2026-10-14 amendment makes **every Extra-slot item LIGHT-only**, even an item whose
own designated slot is Extra. The older architect §2.10 acceptance row says an own-slot Extra item
grants all effects; use the newer accepted EQUIP0 rule, and include a regression rather than copy
that stale row. `agony` cannot be suppressed. No source lookup or Reference import supplies
current Premium, current equipment membership, current target eligibility or session authority.

## What already executes, and where composition stops

Source paths in this section are below `apps/game-server/src/`.

| Native entry / consumer | Existing behavior to preserve | Specific residue |
|---|---|---|
| `gameplay_transport/seam_rooms.rs::SeamRooms` and `node/serve.rs` | Bundle view opens attack target, auto-attack, native combat casts, ordinary timer continuation, monster AI and periodic damage gates. Field ingress and source-map initialization remain qualified-room-only. | No new global bundle combat enable is needed. Keep each gate and its matching content/map pin; do not open field/world-item gates to obtain missing facts. |
| `gameplay_transport/attack.rs::drain_auto_attacks`, `swing_weapon`, `creature_mitigated` | Actual held weapon read, melee/fist formula, fight mode, creature block budget/defence/armor, exact-actor resolution, owner swing commit and kill recording. Occurrence uses attacker, command lineage and swing ordinal. | Formula inputs still use `constants.starting_skill`; this is not a live skill/equipment-boost read. No complete resistance/current active equipment/crit/leech composition is established on this melee route. `run_charm_hook` reads `NoCharmAssignments` and discards outcomes. Do not count hook invocation as an applied effect. |
| `gameplay_transport/native_combat_cast.rs::cast_native_combat_inner`, `prepare_from_owners`; `ordinary_combat.rs::prepare_inner` / lowering | Real same-transaction caster/build/equipment/access reads, target/map/PZ/sight checks, bounded batch, preview before live draws, retained pending command, fenced durable cost/training commit and physical installation. Authored area and chain are already selected/lowered, not missing calculators. | Integrate new qualified providers into these callers; retain new-write vs historical-receipt distinction, complete roster and target predecessor checks. Broader formulas cannot be declared live because geometry executes. |
| `spell/magnitude_owner.rs::PreparedMagnitudeOwner::{qualify,finish_with_damage_healing,validate_current_with_lookup}` | Existing critical/fatal, condition attributes, creature immunity/armor/mitigation/resistance/pierce, Wheel and source stages; shared extension draw for area/beam. Rejects unknown prerequisites and damaging player targets; numeric qualification grants no legality. | `gameplay_transport/spell_magnitude_facts.rs::project` returns `None` for Strict: no advanced modifier owner is registered. `BaselineTest` is explicitly content-qualified approximation; raw unknowns/omitted stages remain visible and are filled only for that baseline. Neither path proves complete Reference combat. Nonempty equipment critical fields remain unknown. Do not set them all to zero or mark extra providers inactive to make Strict pass. |
| `ordinary_combat.rs::prepare_due` (re-exported as `prepare_ordinary_due_from_owners`); `spell_timer_callbacks.rs::{drain_native_ai_timers,validate_numeric_predecessor,apply_due_under_current_owners}` | Due chain/callback numeric work re-reads genuine current owners and validates retained payload, content, predecessors and exact target; retry retains resolved draw. | Extend due-time provider binding too; a cast-time equipment snapshot must not stand in for a current due-phase read. Preserve the owning source's frozen recipient/phase semantics; do not re-pick killed or replaced targets on replay. |
| `creature_chain_attack.rs::ChainOwner::{schedule,run_due}`; `spell/chain.rs::pick_chain` | Exact-actor frozen recipients, accepted picker, current map-facts seam, generation-bound owner timers, retained per-step outcomes. | A module and tests are not proof that every bundle creature profile reaches it. The current bundle `monster_ai_cycle` clears proposal attacks/defences for targeting-only thinking and uses the native bite path; broad creature-chain delivery requires its actual lane/caller qualification, not a second picker. |
| `gameplay_transport/monster_ai_cycle.rs::drain_monster_melee` → bite owner → `actor_spell.rs::apply_creature_damage_native` | Bundle source-bound targeting, qualified room/census/PZ/reentry and live actor checks, bounded think/swing scheduling, bite ledger and actual player damage commit. | Incoming hooks run **after** bite dispatch and use the empty/discarded helper. They cannot implement Dodge/Parry before HP/shield commit. Route the full occurrence through the current defensive composition before calling the vitals writer; preserve bite replay/targeting/cadence. |
| `creature_auto_attack.rs::AutoAttackOwner`, `creature_damage_spell.rs::DamageSpellOwner`, `monster_combat_lane.rs` | Separate native source-qualified melee/spell/condition/geometry/lane components and real owner/vitals interfaces already exist. `CurrentCombatFactsReader::read_attack` explicitly requires independently current physical facts and evaluated defence/armor. | These APIs accepting facts do not establish a live provider or bundle registration. Inventory and qualify each actual call; no all-true `AttackFacts` wrapper, default defence/armor, or content-derived current authority. Source critical baseline covers four closed profiles with **zero damage bonus**, not general critical parity. |
| `spell/actor_conditions.rs::{stage_creature_hit,stage_agony_hit}` and `actor_spell.rs` damage/periodic consumers | Current condition-based incoming reduction, mana shield `min(damage,remaining,mana)`, correlated mana/remaining/HP successor, revision/death lifecycle; Agony bypasses reduction/shield. `condition_spell.rs::consume_mana_shield` validates expected remainder. | Shield mechanics already execute when installed. Compose elemental/field discrimination and pre-shield mitigation/defensive charms, not a new shield calculator. Existing naked magnitude interface cannot attest an undefined element; retain that bypass before entering the splitter. |
| `spell/combat_execution.rs::commit` | Historical single-damage seam explicitly returns `AwaitingLiveAttackerFence`. | Do not revive it or mistake it for the active multi-effect transaction; live spells use the composed native writer. |

`content/item_abilities.rs` contains 27-item/32-field pinned facts and six closed types, with no
STAT_BOOST/LIGHT rows in v1. This is useful data, not an active equipped-item owner or proof that
any particular bundle item has all required values. Do not regenerate/pin data here or infer
unknown protections from an omitted fact. Missing leaf semantics and qualified absence differ.

## Concrete future deltas and owner handoff

These are proposed CP allocations, **not write permission or newly accepted protocol**. Rebase and
re-run the trace in the integration slot; retain existing native types and private constructors.

1. **Equipment provider, owned by EQUIP-RT / current item and Premium owners.** Implement the
   architect's existing `domain/equipment_effects.rs` active-set seam rather than allocate another
   damage calculator. Read actual `EquipmentSnapshot` from `character_equipment` under the current
   fence; bind Character, GameSession, connection/lease generations, distinct World/Channel,
   scope generation, equipment/character revisions, item identity/state revision and definition
   key/revision, and active content digest. The equipment owner must establish designated slot,
   admitted equip requirements and current Premium validity; a snapshot boolean is data only.
   Recompute at accepted EQUIP0 lifecycle triggers (equip/unequip, fresh/restart/reconnect,
   death/respawn/transfer, Premium pull/expiry/unavailability, release). Derive six ability stages
   in accepted slot→definition→ability order, stat percent then flat, checked i64/truncation,
   no A13 writes, maxima clamp, SUPPRESS admission/removal. Resolve definition order explicitly:
   `item_ability_profile` sorts its derived abilities, so do not assume its vector position proves
   the authored evaluation order required by EQUIP0. Keep Extra LIGHT-only and timed effects
   refused until their own live checkpoint/host contract is integrated.
2. **Magnitude/current-caller integration.** Extend
   `gameplay_transport/spell_magnitude_facts.rs::project` / `spell_access_facts.rs` qualification
   with actual admitted provider reads, not a permissive fallback. Keep `OwnedCastFacts` and
   `PreparedMagnitudeOwner` validation at PREPARE, NEW durable write, post-commit installation
   and due callbacks. Wire the same admitted stages into `attack.rs::drain_auto_attacks` using
   real skill/build/equipment owners instead of its starting-skill input. EQUIP0 protections sum
   per element across applicable sources, clamp [-100,100], apply once after defence/armor:
   `damage - trunc(damage * sum / 100)`. Field-only contributions need actual field provenance.
   Do not confuse creature intrinsic rational resistances with wearer equipment protections or
   change the already source-qualified spell arithmetic opportunistically.
3. **Incoming occurrence compositor, combat/CHARM-DEF owner.** Move defensive evaluation from
   post-bite observation in `monster_ai_cycle` to a prepared complete occurrence before damage.
   Obtain actual current charm assignment/catalogue, condition store, equipment active set,
   defender session and exact attacker/target from their owners. Adapt the existing source
   `AutoAttackOwner`/spell/monster-lane consumers as applicable, retaining their explicit fence
   and stamp; do not replace them with authority flags. Preserve CHARM4 §3 ordering below and
   publish one correlated vitals/conditions/death result through the existing physical owner.
4. **Crit/leech source owners.** Obtain an accepted scope/formula/RNG descriptor and admitted
   chance/share/units before enabling a new source. IMBUE-FORGE0 §5 owns imbuement stage inputs;
   EQUIP0 does not authorize equipment special skills. PROF-EFFECT0 is still candidate in the
   inspected main, so its combined `crit` purpose/other perk rules cannot be assumed. Charm
   critical/leech consumers still name missing systems in `combat/charm_effects.rs`; registering
   those outcomes without a live consumer is insufficient. Leech is a bounded typed descendant
   of the committed applied damage result, not requested damage or a second primary calculation;
   no charm/leech recursion from CharmDamage (CHARM0 answer 12b). Exact shares, multi-target
   distribution, rounding, clamps, RNG anchors and reaction budgets need the owning admitted
   descriptor/registered rows. No guessed percentage, global zero, provider callback or wire ID.
5. **Durable/lifecycle owner.** Qualify conditions' durable remaining food time separately at
   lease release/reload/death; preserve current food/condition runtime while this remains open.
   Reuse current source spell transaction/receipt and item guards for any new value consequence;
   no new schema or receipt writer is specified by this doc. Resource-row owner must qualify
   actual maximum/max+1 work, correlated effects and descendant budgets before admitting them.
6. **Final integration owner.** Allocate exact shared `gameplay_transport/mod.rs`, connection,
   capabilities, protocol/resource registries and any needed native private APIs only in the CP
   slot. Keep protocol-oteryn. No imports/content pins, migrations or registries change here.

### Required occurrence order and recovery

Accepted CHARM4 §3, with CONDITIONS0 §3.2/3.3/3.5, gives the creature→player order:
Dodge hook3 once (success cancels health, explicit drain and attached conditions; no later rolls)
→ block/armor/resistances → health-hit gate on positive post-mitigation damage → Parry before
minor evaluation (reflected base is pre-character-mitigation health-component sum; creature armor,
no resistances, neutral physical presentation, no chaining) → evaluate minor using post-Parry
creature liveness → decide explicit-drain Void Inversion before commit → shield/HP then explicit
drain or inversion at its slot then attached conditions → apply minor. Shield mana expenditure
is redirected health damage, never explicit drain. Cleanse sees newly attached conditions.
Agony skips mitigation and every charm; absent-source ordinary ticks have no attacker charm.
Do not roll all outcomes and then discard them on Dodge. R2/R3 parity qualifications in the
accepted brief remain qualifications, not a license to change the order.

Keep existing lock order: spell lane before Channel guards for paid spells; runtime → spell
states → attack for physical turns. Use `load_owned_cast_facts_in_transaction` /
`qualify_raw_owned_cast_facts` and current item fence/semantic transaction, not public fact
constructors. Revalidate current source/target/map/condition/equipment/access at the native
NEW-write boundary before the first write; prepare reserves bounded physical/outbox resources
before durable COMMIT. `PreparedNativeCombatCast::validate_new_write`, `check_commit` and
`install_commit` remain the installation seams. Post-commit current build/equipment reads and
native reconnect proof must match the accepted successor; a cached prepared record cannot mint
current authority. A historical exact command receipt settles its already committed decision
without reroll/recharge or reacquiring a live controller; substituted intent/source/target fails.

Unknown COMMIT keeps its exact pending marker, lane and physical reservations; only proven
rollback releases them. Current actor retirement, session replacement, scope transfer and content
change must refuse a NEW mutation without publishing part of its plan. Preserve exact occurrence,
revisions and draw results across retry; do not re-resolve a lethal target away. Area uses its
accepted shared draw/complete preflight and native atomic batch; delayed chains use ordered
sub-occurrences with fresh due-time owners and retained outcomes, not one immutable whole-cast
snapshot for all numerical phases. Descendant failure records its own outcome and never erases
an already committed parent; do not hide a leech/resource mutation inside magnitude finishing.

## Capability 12: registration, advertising and effect are distinct

`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` defines 12 **ITEM_EQUIP_DROP_V1**, requiring 4
**ITEM_VIEW_MOVE_V1**. It owns no new command/domain: it extends command type 9 and domain 9.
`GATED` already lists `(CAPABILITY_ITEM_EQUIP_DROP_V1, &[], &[])`; adding another table entry does
not enable it. Neither the draft nor refreshed main offers 12 in production, item-view or bundle
sets; client support alone cannot select it. Selected capabilities are retained on resume/transfer
and cannot widen; missing selected support makes resume unavailable/fresh fallback.

The accepted architect §2.8 gate is **working EQUIPMENT equip/unequip/swap behavior**, with its
storage legality/requirements/receipts and selected-client domain9/codec behavior, proven through
production-path fresh admission with requires4. GROUND may remain NOT_SUPPORTED until 2b.
Cap12 is not an assertion that all damage/crit/leech systems work; EQUIP0 itself adds no capability.
The historical skeleton conflated them. To close this draft's broader damage goal CP must
separately require the effects matrix, not use selection of12 as combat parity evidence.

Before an offer change: finish applicable item-move residue, current Premium and lifecycle gates;
qualify all promised destination/results and occupied slots; update registry offer gate and exact
qualified offer sets together, preserve ascending IDs/requires closure and refreshed main's
logout21; prove positive selection, unsupported omission, byte/field/result gating, reconnect and
transfer behavior. An offered-only code change is not an effective handler; an internal native
handler with12 unoffered is not reachable gameplay. Client drawing/negotiation still needs the
owning client qualification. No offer or registry change is authorized by this packet.

## Future qualification matrix — NOT EXECUTED

Each row needs a working real-native positive control, then one changed invariant per negative
case. Actual authority constructors/owner reads and isolated PostgreSQL transactions where
required; matching copied fields or `current=true` are not negative authority coverage.

| Invariant / mutation | Consumer boundaries and required observation |
|---|---|
| Character/account, actor/session, connection and lease generation, World vs Channel, scope generation missing/stale/substituted | Active-set load, NEW-write spell/cast, melee and incoming commit, due chain phase, post-commit install, resume/reload: no unauthorized effect or authority grant. Historical terminal receipt remains replay-only. |
| Content digest / definition key+revision / item state and equipment revision changed; same-slot replacement | Provider and every magnitude revalidation/commit: no stale bonus or stale protection. Mutation of one field preserves all unrelated native facts. |
| Premium expired, not current, unavailable; clock future/regression; equip before expiry then hit after | Active-set lifecycle and new/due commit: Premium-only benefit stops, item remains equipped; no zero/false default that claims a successful authority read. Accepted historical receipt is not re-decided. |
| Slot/equip order permuted, designated slot mismatch, Extra own-slot/non-own-slot, timed item, unknown leaf | Same active set gives identical derived stage values; Extra always LIGHT-only; timed/unknown never gain invented benefits. Known absence differs from missing content. |
| Mixed positive/negative protections, field-only versus direct hit, sum ±100 boundary, overflow | Current incoming stage: accepted clamp and one truncation; no sequential per-slot multiplication; no field immunity from unbound origin; invalid numeric state refuses before commit. |
| Dodge success, full block, shield-only health hit, drain-only/mixed hit | Complete occurrence before HP: exact hook gates and draw count; Dodge cancels conditions/drain; positive health component absorbed by shield remains a hit; shield loss cannot trigger inversion. |
| Parry kills source before Numb; Cleanse after new condition; inversion after shield | Native occurrence commit/descendant: no Numb roll against dead source; self minor still applies; correct inversion cap from mana room at drain slot. No retroactive compensation. |
| Shield capacity/mana/damage0/exact exhaustion/undefined element/expiry; attached condition failure; Agony | Existing typed splitter and composite writer: one correlated successor, no partial HP/mana/remaining/condition publication, correct bypass and lifecycle/death behavior. |
| Area target/census changed after preview; one illegal/PZ/reentry/unseen target; unsupported damaging player | PREPARE, NEW-write and physical install: accepted atomic/no-partial behavior; shared critical/fatal draw preserved; no calculator grants target permission. |
| Chain target dies/replaced/transfers; caster loses access; same due callback duplicated/cancelled | Due owner read / numeric predecessor / install: exact source frozen recipients and fresh due facts, retained draw, no new target or second damage. Exercise timer bound and continuation. |
| Same command/occurrence replay, intent substitution, unknown COMMIT, reconnect during install, death mint/outbox allocation failure | Native paid writer and bite/swing ledgers: one charge/effect/death/kill record; retain reservations until reconciled; no new live grant from immutable receipt. Concurrent equip/Premium/condition replacement cannot win between check and mutation. |
| Leech from overkill/fully blocked/no-op/CharmDamage, max health/mana, source death/replacement, descendant-budget max+1 | Only qualified committed applied damage feeds authorized descendant, capped by actual owner state; no recursive proc, duplicate credit, parent rollback or guessed formula. Gate tests until descriptor/rows exist. |
|12 registered/unoffered, supported without4, fresh selection, resume support lost, transfer, domain9 wrong extension | Real production admission/codec/dispatch/emission: offers/selected/effective operation independently asserted; old sessions never silently gain12. Keep registry/offer tests and client evidence. |

## Original family acceptance — OPEN

The original implementation family remains open. This report-only preparation does not satisfy
or close its runtime acceptance criteria:

- [ ] Damage formulas match the admitted Reference descriptors, with deterministic retry/replay
  proven through the actual native callers and composed effects.
- [ ] Capability 12 is offered only when its promised equip/unequip/swap extensions are live and
  production-admission qualified, with requires4 and the accepted GROUND exception. Separately
  qualify this family's damage effects; selecting12 does not attest combat parity.
- [ ] The implementation builder rechecks residue against the integration head before code and
  demonstrates closure of the allocated caller, content, authority and lifecycle gaps. This
  source-bound packet supplies the current trace; it does not waive that future recheck.

## Builder and CP integration rules

This allocation permits only the task card. Worker must not commit, push, mark ready or merge.
Root may publish the reviewed packet to the existing draft; CP controls readiness and integration.
No shared slot-only changes before the CP queue turn and its then-current allocation/rebase:
`gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, protocol and
resource registries, migration numbers, protocol `lib.rs`, content regeneration outputs and pins.
At that turn, integrate the exact inseparable handler/codec/offer/registry/client-qualified tests
under the owning accepted contracts and required independent exact-head review. Historical
skeleton paths or proposed deltas in this card grant no source or shared-file ownership now.

## Local validation and closeout

Only governance checks are appropriate to this doc-only preparation. Actual command results and
full logs are retained in `local-checks.log`; source SHA256 values and main comparison are in
`source-bindings.json`. `git diff --check` and the allocation scope check must pass before handoff.
Executed: governance validation passed (22 required documents, 9 project lanes); governance
unittests passed (124/124); allocation-only scope and `git diff --check` passed. No runtime-test
count, formula parity, current-authority execution or end-to-end completion is claimed by a source
trace or this future matrix. No Rust build, formatting, Clippy, runtime or PostgreSQL suite ran.

Pending: CP's exact split between EQUIP-RT, CHARM-DEF, item-move and LOOP callers; accepted
crit/leech descriptor/content and reaction budgets; actual current-provider/lifecycle wiring;
source-bound native/PG/replay/negative tests; client and capability qualification; independent
architect/combat/determinism/persistence/protocol reviews; integration slot. Root alone may
publish the reviewed card to the existing draft. Worker performed no publication.


## D1125 current build/condition provider — NOT_FULLY_QUALIFIED

CP D1125 (2131#issuecomment-6224930545) supersedes the historical card-only scope:
`gameplay_transport/spell_magnitude_facts.rs` and `spell/actor_conditions.rs` may be
written now. `attack.rs` composition follows #2192; `monster_ai_cycle.rs` follows
#2200. Shared wiring and capability offers remain closed.

The current provider checks the native Content digest, Character/Equipment revisions,
command/session/lease/connection bindings, current PlayerSpellState revision and level.
It reads all seven skills from the independently loaded durable build; missing or
mismatched build/vocation/magic refuses. Existing native condition owners supply the
current skill deltas and source modifiers, with their existing checked arithmetic,
exact expiry and nonmonotonic-time refusal. No creation-time starting_skill fallback,
new formula, equipment bonus, crit/leech evidence or attack authority is fabricated.
The result is point-in-time data: the eventual owning caller must requalify actor and
binding and recompute after awaits. Equipment effects and CHARM4 pre-hit composition
remain separate unimplemented requirements. Narrow dead_code allowances document only
the two new provider entry symbols pending the explicitly ordered caller integration.

Added a meaningful native unit regression for unavailable/mismatched build, current
melee/distance condition modifiers and exact expiry before cleanup. Source review found
one P2 dead-code lint issue, addressed with those bounded pending-caller annotations.
Touched-file rustfmt and diff checks PASS. Actual native unit4/4 PASS, including
the new current-build regression, on the full18808-source native crate with exactly
the owned delta; source-before/after equal. Clippy and full package remain NOT_RUN;
no earlier family test result is imported as this delta's qualification. No production activation or full LOOP-DAMAGE completion is claimed.

Current checkpoint governance validation and all124 governance unit tests PASS.
