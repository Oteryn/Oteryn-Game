# Oteryn Reference Playable — Official-First Evidence Pack

- Status: **EVIDENCE PACK / NON-AUTHORITY**
- Date: 2026-09-09
- Coordination: #483, parent #162, architecture #220
- Protected base at lane creation: `main@0e8a358f134693871d5140309cf778a28da97277`
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Runtime/client/server/protocol/DDL/migration/production authority: **NONE**
- Evolved work: **OUT OF SCOPE until REFERENCE_PLAYABLE**

## 1. Purpose

This pack narrows Reference research to the minimum loop needed to make the first Oteryn Reference profile physically playable.

It does not try to prove every Global Tibia mechanic and it does not mutate the accepted Reference target. Its job is to separate:

- facts that already have strong official/public support;
- current official behavior that is useful discovery evidence but still lacks target-cut continuity proof;
- Oteryn-native reliability/authority behavior that does not need proprietary-client/protocol parity;
- existing accepted Oteryn Reference differences that conflict with current official Global behavior;
- gaps where Canary/Crystal or other OTS sources may help discover questions/tests but may never supply Reference truth by themselves.

## 2. Binding target and evidence discipline

The accepted first Reference target remains Global Tibia production-observable behavior after the **2026-07-28 server-save/maintenance change boundary**.

The target is immutable until a later explicit owner-approved Reference revision supersedes it.

The accepted source hierarchy remains:

1. `OFFICIAL_PUBLIC` and provenance-cleared owner primary captures;
2. lawful controlled black-box observation;
3. reputable community corroboration;
4. `OTS_HYPOTHESIS_ONLY` for Canary, Crystal and other OTS/reference implementations.

Important fail-closed rules:

```text
current official page after 2026-07-28
!= automatic proof of exact 2026-07-28 behavior

no discovered patch note
!= proof that behavior did not change

Canary/Crystal implementation
!= proof of Global behavior
```

Current official pages are still preferred discovery evidence because they are primary CipSoft/Tibia surfaces. A material target-cut assertion needs target continuity, a target-era primary source/capture, a bounded observation with historical continuity proof, or an already accepted Oteryn Reference contract/declared difference.

## 3. Official primary source index

| Source | Locator | Current official evidence useful to Reference Playable | Target-cut disposition |
|---|---|---|---|
| Jul 28 2026 official news | `https://www.tibia.com/news/?id=8905&subtopic=newsarchive` | Confirms the selected date is a real server-save change boundary and records named hunting/boss changes on that date. | `PROVEN` for boundary identity only; not proof for unrelated mechanics. |
| Starting manual | `https://www.tibia.com/gameguides/?section=starting&subtopic=manual` | Character/account setup and current client start/title-screen/world-entry concepts; central game server role. | `CURRENT_OFFICIAL_DISCOVERY`; exact July-28 UI/flow continuity pending. |
| Interface manual | `https://www.tibia.com/gameguides/?section=interface&subtopic=manual` | Game window, minimap, equipment slots, combat controls, logout/title-screen behavior, skill display. | `CURRENT_OFFICIAL_DISCOVERY`; presentation details need not be copied literally by native Oteryn client. |
| Controls manual | `https://www.tibia.com/gameguides/?section=controls&subtopic=manual` | Orthogonal/diagonal movement, click-to-walk, floor transitions, item use, push, attack start/stop, corpse looting, container handling. | `CURRENT_OFFICIAL_DISCOVERY`; exact timing/legality/collision continuity pending. |
| Characters manual | `https://www.tibia.com/gameguides/?section=characters&subtopic=manual` | XP/levels, skills, current death/respawn/loss description, equipment progression. | `CURRENT_OFFICIAL_DISCOVERY`; exact arithmetic/rounding and July-28 continuity pending. |
| World manual | `https://www.tibia.com/gameguides/?section=world&subtopic=manual` | Creature/NPC/character distinction, creature-kill XP role, temples, depots and safe storage concepts. | `CURRENT_OFFICIAL_DISCOVERY`; target continuity pending. |
| Combat manual | `https://www.tibia.com/gameguides/?section=combat&subtopic=manual` | Attack rounds, range-driven attack attempts, damage types, equipment/skill relationship, PvP surface. | `CURRENT_OFFICIAL_DISCOVERY`; exact formulas/order/timing remain parity gates. |
| Trading manual | `https://www.tibia.com/gameguides/?section=controls_trading&subtopic=manual` | Safe trade, container trade, depot/Market entry and capacity-related constraints. | `CURRENT_OFFICIAL_DISCOVERY`; minimal playable slice does not need full Market parity. |
| Products/Blessings manual | `https://www.tibia.com/gameguides/?section=products&subtopic=manual` | Seven regular blessings, death-penalty/item protection role, Twist of Fate and blessing charges in current Global. | `CURRENT_OFFICIAL_DISCOVERY`; target continuity and exact target formulas must be evidenced. |
| Spells Library | `https://www.tibia.com/library/?subtopic=spells` | Current official spell catalogue and metadata; existing manifest already registers Light Healing/Ice Strike hypotheses. | Existing cases remain `PARITY_PENDING_EVIDENCE` until target continuity/provenance gates are satisfied. |
| Creatures Library | `https://www.tibia.com/library/?subtopic=creatures` | Current official creature catalogue; creature pages expose HP, XP, qualitative resistances/abilities and loot families. | `CURRENT_OFFICIAL_DISCOVERY`; exact selected creature case and target continuity required before parity promotion. |

## 4. Minimum REFERENCE_PLAYABLE evidence matrix

### RP-01 — Client start, character selection and world entry

**Official discovery:** Starting/Interface manual documents a title-screen/client-entry flow and character/world interaction concepts.

**Reference requirement:** Oteryn needs a native client that can authenticate through accepted Oteryn Platform/Foundation contracts, select a Reference character/world/channel and enter gameplay.

**Parity boundary:** Reference fidelity does not require copying CipSoft launcher UI, proprietary network protocol or proprietary client implementation. Observable player intent/state transitions may be mirrored where evidenced, while transport/security remains native Oteryn authority.

**Status:** `DERIVED / TARGET_CONTINUITY_PENDING` for exact Global observable flow; Oteryn admission/runtime implementation is separately blocked on the active fresh-admission -> Server Seam chain.

**Smallest next proof:** target-era primary capture or continuity evidence for any Global-visible flow detail we intend to claim as parity; otherwise keep native Oteryn presentation as a non-gameplay implementation choice.

### RP-02 — Character baseline and progression vocabulary

**Official discovery:** Characters manual confirms current level/XP/skill concepts. Existing accepted `GAME-CHAR-01` Stage B already records Reference vocation families, pre-vocation state and skill vocabulary under the immutable target discipline.

**Status:** semantic envelope `ACCEPTED`; exact arithmetic/formulas remain `PARITY_PENDING_EVIDENCE` where unresolved.

**Smallest next proof:** only the exact progression arithmetic exercised by the first playable slice: XP grant -> total XP -> level transition, plus any selected skill advancement behavior.

### RP-03 — World presence and movement

**Official discovery:** Controls manual currently documents:

- click-to-walk toward a reachable field;
- keyboard orthogonal movement;
- diagonal movement;
- facing-direction changes;
- floor transitions through stairs/ramps/holes/rope spots/grates/ladders;
- follow and chase movement concepts;
- item drag/drop and pushing.

**Do not infer yet:** exact step duration, diagonal timing factor, pathfinding tie-breaks, collision priority, simultaneous move resolution, push delay, floor/LoS rules or prediction/reconciliation.

**Status:** qualitative movement surface `CURRENT_OFFICIAL_DISCOVERY`; executable Reference movement parity remains `PARITY_PENDING_EVIDENCE` until the exercised rules have target continuity or explicit declared-difference coverage.

**Smallest playable case:** one orthogonal step, one diagonal step, collision rejection, one simple floor transition, server-authoritative position persistence.

### RP-04 — Basic combat targeting/attack lifecycle

**Official discovery:** Controls/Combat manuals currently describe selecting `Attack`, a designated opponent, automatic attack attempts when in range, attack cancellation and combat-round concepts.

**Do not infer yet:** exact attack interval, weapon formula, hit chance, defence/armor order, target switching order, range metric, LoS, PvP legality or RNG.

**Status:** qualitative attack lifecycle `CURRENT_OFFICIAL_DISCOVERY`; quantitative combat remains `PARITY_PENDING_EVIDENCE`.

**Smallest playable case:** one creature, one player attack intent, server legality/range check, deterministic bounded damage fixture, creature HP reduction and death.

### RP-05 — Representative heal and attack ability

Existing manifest revision 3 already contains:

- `ability_combat.light_healing.cast_metadata.v1`;
- `ability_combat.light_healing.self_heal_semantics.v1`;
- `ability_combat.ice_strike.cast_metadata.v1`;
- `ability_combat.ice_strike.targeted_ice_damage_semantics.v1`.

Those cases intentionally remain `UNKNOWN / PARITY_PENDING_EVIDENCE` for the immutable target because their official Library evidence was post-boundary and target continuity was not proven.

**Action:** do not duplicate them. Improve their provenance/continuity later or replace the representative slice with another case only through normal manifest authority.

### RP-06 — Creature kill -> XP

**Official discovery:** World manual says characters earn experience by killing creatures. Current creature Library pages provide explicit XP yields for named creatures; for example current Dragon/Skeleton pages expose their XP values and qualitative combat/loot facts.

**Do not infer yet:** target-era XP value for a selected creature, party sharing, stamina modifiers, boosted creature multipliers, summon attribution or kill-credit ordering.

**Status:** qualitative kill->XP relation `CURRENT_OFFICIAL_DISCOVERY`; exact first creature fixture `UNKNOWN` until target-era continuity is established.

**Smallest playable case:** select one low-complexity creature only after its target HP/XP/loot facts are evidenced sufficiently for a bounded fixture.

### RP-07 — Creature corpse -> loot interaction

**Official discovery:** Controls manual currently documents corpse opening/looting and quick loot. It also documents a current first-10-seconds creature-loot ownership rule and party access.

**Reference Playable minimum:** normal creature death materializes a corpse/loot container; authorized player can move one item from corpse to inventory when capacity/container legality permits.

**Defer from first slice unless required:** Quick Loot Nearby Corpses, 30-corpse aggregation, configurable loot categories and full party loot ownership.

**Status:** qualitative corpse-loot loop `CURRENT_OFFICIAL_DISCOVERY`; exact target ownership/timer and selected loot table remain `PARITY_PENDING_EVIDENCE`.

### RP-08 — Inventory, containers and equipment

**Official discovery:** Interface/Controls manuals currently provide body slots, a container slot, equip-by-moving, slot legality concepts, level/vocation restrictions, capacity and nested container operations.

**Existing Oteryn authority:** `GAME-ITEM-01` and `DUR-03` own native item identity/location/conservation. Global-visible restrictions remain Reference evidence inputs, not reasons to copy legacy storage implementation.

**Status:** native semantic boundary `ACCEPTED`; exact target item catalogue/slot restrictions for selected items remain parity/content evidence work.

**Smallest playable case:** one backpack/container, one stackable loot item, one eligible equipped item and one ineligible equip rejection.

### RP-09 — Death, loss/protection and respawn

**Official current discovery:** Characters manual currently states that zero HP causes death, dead characters return to the temple of their home city, and Global death includes experience loss, skill loss and possible item loss. Products/Blessings manual currently states there are seven regular blessings and that blessings reduce death penalty/item-loss risk.

**Existing Oteryn Reference difference:** protected `OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md` explicitly sets:

```text
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
```

and defines XP-only Reference death progression loss. This is not what the current official Global manual describes.

**Classification:** `DECLARED_DIFFERENCE / OWNER_REVIEW_REQUIRED FOR STRONG GLOBAL-FIDELITY GOAL`.

This pack does **not** supersede the protected owner baseline. Before `REFERENCE_PLAYABLE` is promoted as a strong Global-parity profile, the owner/architecture lane should decide whether to retain that declared difference or supersede it with evidenced Global target behavior.

**Smallest playable case:** HP reaches zero -> death occurrence -> accepted Reference loss policy -> temple/home respawn -> reconnectable Character state, with item/corpse behavior only to the extent already evidenced/accepted.

### RP-10 — Minimal NPC/depot/trade service

**Official discovery:** World manual currently describes temples and personal depot lockers; Trading manual describes safe trade and Market access through a depot locker.

**Reference Playable minimum:** one NPC interaction/service sufficient for restock or one depot storage operation. Full Market, bank, house and travel parity are not prerequisites for the first physically playable slice.

**Status:** qualitative service surfaces `CURRENT_OFFICIAL_DISCOVERY`; exact target NPC dialogue, prices, inventory and market rules remain content/economy parity work.

### RP-11 — Logout, reconnect and persistence

**Official discovery:** Interface manual currently describes normal logout returning to title screen and notes that logout block/disconnection can leave a Character in game.

**Oteryn authority:** durable session/admission/reconnect, generation fencing and PostgreSQL correctness are native Oteryn reliability/security contracts. Reference parity does not justify reproducing proprietary implementation weaknesses.

**Status:** `SHARED OTERYN AUTHORITY + CURRENT_OFFICIAL_DISCOVERY` for visible concepts. The active WP3/WP4/WP5/Server-Seam chain is the implementation prerequisite.

**Smallest playable case:** normal logout, fresh login, connection loss/reconnect and restart readback preserve the same authoritative Character state without duplicate session/value authority.

## 5. Current blocking evidence classes

### Can move forward as native/shared implementation inputs

These do not require copying Global internals:

- native client/server transport and admission;
- durable session/reconnect fencing;
- authoritative one-location item conservation;
- semantic world position/tile ownership;
- server-authoritative movement/combat command processing;
- deterministic bounded simulation framework;
- native content loading and presentation boundaries.

### Need target evidence before a strong `PARITY_CONFIRMED` claim

- exact movement timing/collision/floor-transition rules;
- exact basic weapon attack formula/order/RNG;
- exact selected creature HP/XP/loot facts at the target cut;
- Light Healing/Ice Strike target continuity and exact quantitative semantics;
- exact Reference death XP/skill/item-loss behavior if Global-faithful behavior is desired;
- exact starter/selected equipment and NPC/depot content used in the playable slice.

### Existing owner decision requiring deliberate review

The most material known mismatch discovered in this pass is:

```text
current official Global manual: death includes skill loss
protected Oteryn Reference: DeathSkillLoss = 0, DeathMagicLevelLoss = 0
```

Do not silently rewrite either side. This remains a named owner-review point.

## 6. Canary/Crystal and other OTS use

Permitted Tier-4 inputs:

- `blakinio/canary`;
- pinned Crystal/legacy migration evidence already used by Game migration work;
- other OTS repositories when their exact source/revision is recorded.

Allowed uses:

- enumerate likely server state and edge cases;
- identify candidate creature/NPC/item definitions to research officially;
- discover candidate tests for movement, combat, death, loot and persistence;
- compare alternative interpretations after official evidence exposes ambiguity;
- assist migration/source-field mapping when provenance permits.

Forbidden uses:

- copy an OTS number/formula and label it Global truth;
- mark a manifest case `PARITY_CONFIRMED` from OTS code similarity;
- choose an unknown Global rule because Canary/Crystal already has code for it;
- import OTS runtime architecture as production authority;
- launder proprietary/uncertain assets or source through an OTS repository.

Recommended pattern:

```text
OTS observation
-> candidate question / edge case
-> official-source search
-> controlled observation if needed
-> explicit evidence classification
-> Oteryn fixture/test
-> only then parity promotion
```

## 7. Manifest promotion recommendation

Do **not** mutate `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` from this pack alone.

Current recommendation:

- keep the four existing Light Healing/Ice Strike cases as the only registered mechanic cases until their evidence is strengthened;
- next manifest additions should be a very small set of executable `REFERENCE_PLAYABLE` cases, not a broad catalogue;
- each new case should name an exact target-era source/continuity argument or remain explicitly `UNKNOWN / PARITY_PENDING_EVIDENCE`;
- prefer one movement case, one creature kill/XP/loot case, one item/equip case and one death/respawn case once their evidence is mature;
- treat the existing XP-only Reference death rule as a declared-difference/owner-review dependency rather than pretending it matches current Global documentation.

## 8. Execution priority consequence

Until `REFERENCE_PLAYABLE` is reached:

```text
AUTHORIZED PRIORITY
= shared foundation required by Reference
+ Reference-specific evidence/content/gameplay

DEFER
= new Evolved-only mechanics, balance and product differentiation
```

The current critical path remains:

```text
WP3 durability/TLS/resource closure
-> WP4 durable fresh admission
-> required WP5 owning-source composition
-> fresh G0/readiness
-> resume existing Server Seam #247
-> native client world entry
-> movement/world
-> combat/ability
-> creature XP/loot/items
-> death/respawn
-> minimal NPC/depot/content
-> persistence/reconnect physical QA
-> REFERENCE_PLAYABLE
```

Evidence work in this pack may proceed path-disjoint from that runtime chain.

## 9. Exit criteria for this evidence lane

This pack is complete when it truthfully provides:

- an official-first source map for the first playable loop;
- a clear distinction between current official discovery and target-cut proof;
- explicit minimum behavior gaps rather than broad feature wishlists;
- separation of native Oteryn safety/reliability authority from Global-visible parity;
- OTS hypothesis-only usage rules;
- named manifest candidates;
- named owner-review conflicts, especially Reference death skill loss;
- no Evolved scope expansion and no runtime mutation.

`IMPLEMENTATION_AUTHORITY: NONE`
`PARITY_PROMOTION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
