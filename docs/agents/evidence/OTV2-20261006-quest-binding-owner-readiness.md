# OTV2-20261006 Quest Binding Owner Readiness

Status: retained evidence / execution work queue
Repository baseline: `Oteryn/Oteryn-Game@97c46d0e63fefe8a22a2053de7b596c50b7253a6`
Related merged PRs: #1875, #1876
Coordination: #1622
Runtime activation: **false**

## Purpose

Retain the post-lowering Quest backlog by **runtime owner boundary**, not by quest name or donor folder.

The completion work is no longer primarily a catalogue/import problem. All 373 pinned wiki quest titles map to canonical Oteryn Quest records. The dominant remaining work is binding typed Quest transitions to the real occurrences owned by NPC, world interaction, inventory, combat/Encounter and completion producers.

This document does not certify playability.

## Current all-373 state

**PROVEN / DERIVED from committed current-main artifacts**

- pinned wiki titles: **373**
- canonical Quest definitions: **352**
- wiki titles mapped: **373 / 373**
- canonical title mapping state: **373 SINGLE**
- completion candidate owners: **303**
- typed tracks: **3152**
- typed transitions: **5085**
- completion transitions: **299**
- completion binding-plan owners: **295**
- binding-plan stage rows: **1822**
- native event dispatch bindings: **0**
- native NPC dialogue bindings: **0**
- native reward delivery bindings: **0**
- runtime activated: **false**

Implementation backlog by wiki-title row:

| State | Count |
| --- | ---: |
| `NATIVE_BINDINGS_PENDING` | **322** |
| `NATIVE_LOWERING_PENDING` | **9** |
| `DEFINITION_READY_RUNTIME_UNKNOWN` | **42** |

Source fidelity remains an independent axis. Source holds do not erase a consciously chosen Oteryn typed-progress candidate.

## The nine remaining lowering holds

The remaining lowering blockers are exactly the chosen-source recipes whose terminal `complete` stage has a count greater than one:

1. Barbarian Arena Quest
2. Bear Room Quest
3. Behemoth Quest
4. Demon Helmet Quest
5. Dragon Tower Quest
6. Edron Goblin Quest
7. Opticording Sphere Quest
8. Rift Warrior Outfits Quest
9. The Ancient Tombs Quest

### Why they remain fail-closed

QuestState commits completion from the selected transition's plain `completes` flag.

A repeated terminal transition with `completes=true` would mark the quest complete on the **first** occurrence, not the final required occurrence.

A safe solution therefore needs explicit state-dependent transition selection, for example a non-terminal progress transition versus a final completion transition selected from current QuestState facts. This selection must belong to the real producing owner/occurrence. It must not be synthesized by the content compiler without an owning runtime contract.

The existing Crystal Herald path demonstrates that state-dependent transition selection is possible in a narrowly qualified source owner, but it does not create a general Quest dispatch authority.

## Binding owner matrix

Counts below are stage rows in `content/quests/missions/completion-binding-plan.json`. Quest counts overlap across kinds and must not be summed.

| Stage kind | Stage rows | Quest owners | Strong evidence already present | Missing executable owner |
| --- | ---: | ---: | --- | --- |
| `talk` | **326** | **163** | 79 rows have NPC dialogue candidates | NPC-TALK-1 -> NPC-QUEST-1 |
| `use` | **431** | **213** | 44 rows have all targets exact; 148 have at least one exact target | QUEST-TRIGGER-1 / WORLDINT-USE-1; ITEM-USE-1 where the event is Item-definition consumption/use |
| `collect` | **273** | **148** | 145 rows have all targets exact; 180 have at least one exact Item target | inventory-count / per-target-quantity occurrence owner |
| `explore` | **239** | **173** | 29 rows have all targets exact; 43 have at least one exact Area | movement/world entry occurrence via QUEST-TRIGGER-1 and applicable spatial owner |
| `kill` | **258** | **135** | 172 rows have all targets exact; 191 at least one exact Creature; 25 exact Encounter outcome seams | kill credit / Encounter outcome delivery |
| `complete` | **295** | **295** | chosen graph predecessor relation | real producing occurrence / completion reducer binding |

## TALK owner boundary

**PROVEN**

- NPC-WIRE-1 is merged and registers `NPC_SERVICE_V1`.
- The capability remains `offered: false`.
- Commands are not served before NPC-TALK-1.
- QUEST-GATE-0 assigns typed Quest dialogue outcomes to NPC-QUEST-1, dependent on NPC-TALK-1 and QuestState/predicate/value owners.

**DERIVED**

The 79 dialogue candidates are useful binding evidence, not executable NPC Quest bindings.

Do not set `selected_NPC_branch` or a native transition solely because an NPC identity or source reply candidate is unique.

## USE owner boundary

**PROVEN**

QUEST-GATE-0 assigns `USE`, `ON_ENTER` and `ON_LEAVE` Quest trigger roots to QUEST-TRIGGER-1.

WORLD-INTERACTION-0 assigns map/local-object use to WORLDINT-USE-1.

ITEM-USE-WIRE-1 registers capability 15, but the server explicitly remains fail-closed before ITEM-USE-1.

**DERIVED**

An exact canonical Item target does **not** prove:

- which placed object was used;
- which LocalObject/Interaction fired;
- whether the operation is Item-definition use, map-object use, use-with or another owner;
- the CommandRef / occurrence root.

The 44 all-exact-target USE rows therefore remain candidates, not executable bindings.

## COLLECT owner boundary

A canonical Item identity proves only the target identity.

It does not prove the event that should increment Quest progress. Collection may be produced by a RewardClaim, inventory transaction, NPC exchange, loot/death, world pickup or another owning domain.

The binding must therefore name an actual committed occurrence and exact quantity semantics. Polling current inventory and treating a threshold crossing as Quest authority is not accepted.

## EXPLORE owner boundary

An exact Area identity is not an entry trigger.

The executable producer must bind an actual movement/world occurrence to a defined region, tile, transition or other spatial boundary under the accepted movement/world owner.

The 29 all-exact-target Area rows are identity evidence only.

## KILL owner boundary

There are **25** exact existing Encounter outcome seams in the chosen-source event/reward packet.

They remain:

- `execution_verified: false`
- `native_dispatch_binding: null`
- `runtime_admitted: false`

### Crystal Herald precedent

Current server code contains a narrow source-qualified Herald death path:

- reads current QuestState;
- selects one explicit transition from current track values;
- freezes `QuestCause::CreatureDeath`;
- revalidates the current source/runtime pin;
- commits through the CharacterRevisionSequencer and existing QuestState writer.

This is a valid pattern for a **qualified owner**, not general authority.

The Herald implementation explicitly chooses the **top-damage Character only** and excludes party fan-out.

The 25 Encounter seams include credit policies such as party or damage-contributors. Those cannot be promoted by reusing Herald's recipient policy.

Real Encounter/credit delivery still needs its owning runtime allocation/contract, including ENC-RT-1 / ENC-OUTCOME-1 or an explicitly accepted successor.

## COMPLETE owner boundary

A chosen `complete` stage is a graph intent, not an occurrence.

Every completion transition requires a real cause from the owning producer. Completion must not be invoked simply because the preceding chosen counter reached its goal.

This is especially critical for the nine terminal-count holds.

## Highest-leverage allocation order

**RECOMMENDATION**

1. **QUEST-TRIGGER-1 and current prerequisites**
   Unlock the generic world/movement trigger root needed by large portions of USE and EXPLORE.

2. **NPC-TALK-1 -> NPC-QUEST-1**
   Unlock the 326 TALK rows, starting with the 79 already-qualified dialogue candidates.

3. **Inventory collection occurrence owner**
   Define the accepted event/quantity boundary for the 273 COLLECT rows without polling state into authority.

4. **Encounter outcome / kill-credit owner**
   Admit exact recipient/credit semantics for KILL rows; consume the 25 exact Encounter seams only after their owner is executable.

5. **Progress-versus-final selector contract**
   Close the nine non-unit terminal-count recipes using an explicit producer-owned selector, not a content-only completion shortcut.

## Do not regress

- Canonical identity is not executable event binding.
- Source association is not runtime readiness.
- Typed progress is not playability.
- NPC-WIRE-1 is not NPC-TALK-1.
- ITEM-USE-WIRE-1 is not ITEM-USE-1.
- Area identity is not an entry occurrence.
- Creature identity is not kill-credit selection.
- Encounter identity/outcome declaration is not outcome runtime delivery.
- Inventory state is not a Quest transition occurrence.
- A chosen terminal graph node is not a cause.
- Crystal Herald's top-damage-only path must not be generalized to party/damage-contributor credit without owner approval.

## Next executable work

Until the missing runtime children are allocated, the Quest lane can continue safely by producing exact, fail-closed candidate joins:

- canonical stage target -> concrete Interaction/placement/occurrence candidates for USE/EXPLORE;
- canonical Item target -> exact acquisition/RewardClaim/transaction occurrence candidates for COLLECT;
- exact NPC stage -> verified dialogue branch candidates without runtime admission;
- exact Encounter seams -> credit-policy evidence without changing recipient authority.

Every such packet must keep native binding and runtime-admission fields false until its owning consumer is accepted.
