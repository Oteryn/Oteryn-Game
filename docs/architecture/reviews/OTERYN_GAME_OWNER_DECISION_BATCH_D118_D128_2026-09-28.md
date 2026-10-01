# Owner decision batch D109 and D118-D128

- Decision: `OWNER-DECISION-BATCH-D118-D128-V1`
- Status: **CANDIDATE with owner decisions D118-D128**. Acceptance requires exact-head validation,
  independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Origin: a proactive sweep of the open items in the 2026-09-27/28 decision records, batched so the
  owner answers once instead of per lane
- Owner decisions posted: #162 comment 5879395589 (as D98-D108), and D109 in 5879706425
- Numbering: renumbered D118-D128 because D98-D101 (5879348805) and D110-D117 (5879404970) were
  already taken on #162; D109 keeps its number
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- Admission baseline: `main@3dcf3c82`
- Runtime, schema, registry, Platform and production authority: **NONE**. Each lane below needs its
  own allocation; Platform work needs Platform authority.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Decisions

| # | Topic | Owner choice (2026-09-28) | Supersedes or amends |
|---|---|---|---|
| D118 | Creature-kill XP | Full Global in the first XP slice: base creature XP, party shared experience and stamina | VSL Combat rows decision §5 "XP values not decided"; D79 (no XP path) ends when this lane lands. Shared experience uses the D109 principal ceiling |
| D119 | Respawn place | Home town as in Global: a character has a home town and respawns at its temple | first player death decision §3 UNKNOWN (home town), D63 placement |
| D120 | Low-level death protection | As Global at 2026-09-27 | confirms D65 (§2.1); D59 (the XP-loss formula) is unchanged |
| D121 | Corpses and loot | As Global: loot rights first for the top-damage character or party, then everyone; the corpse is a container; decay time per creature | VSL Combat rows decision §5, WO-0 §7 (corpse loot link) |
| D122 | Blessings | NPC and prices as Global first; DEATH-4 waits for the NPC trade lane; no stopgap grant | first player death decision §5 DEATH-4 |
| D123 | Character creation | Sex chosen at creation; starter outfit per sex as Global | character appearance decision remaining unknowns |
| D124 | Premium for testers | A test-environment Premium entitlement fixture on the Platform side now; Store and payment later | Premium activation decision §8 next action |
| D125 | Mounts | Mount activation as Global, with its speed bonus; no longer deferred to the Store lane | character appearance decision §4 (mount activation deferred) |
| D126 | Achievements | Allocate an AccountAchievement domain owner now | account-scoped progress decision §6 (owner deferred) |
| D127 | Partial stack pickup | As Global | B3 decision §3 UNKNOWN (§2.2) |
| D128 | Players per Channel | Design target 500 concurrent players per Channel, verified by PERF measurements | MOVE-RL-11 decision remaining unknown |
| D109 | Reward principals for shared experience | As Global: the `COMBAT01-REWARD-PRINCIPALS` ceiling equals the Global party size, 50 (§2.4) | VSL Combat rows decision row 10 (1 per death) |

## 2. Global facts (reference evidence)

Sources: tibia.com manual notes in `docs/reference/tibia-manual/` and the tibia.com snapshot
`imports/official/tibia-com/2026-09-28-160207Z`; TibiaWiki where the manual is silent.

### 2.1 Death protection (D120)

- Characters of level 8 or below lose no items on death (`characters.md`). This is D65; D120
  confirms it.
- There is no XP-loss exemption by level. Global charges 10% of total experience up to level 23;
  Oteryn keeps D59 (the D58 formula from level 1), which D120 does not change.
- A TibiaWiki snippet mentions a Newhaven change (Update 15.12, 2025-10-21) for level 6 on the
  mainland; the manual snapshot does not state it. **UNKNOWN**: DEATH-1 keeps D65 and D59; the
  threshold changes only when tibia.com evidence at the target date proves it.

### 2.2 Partial stack pickup (D127)

No tibia.com or TibiaWiki source was found. **UNKNOWN**: B3-1 keeps the fail-closed behaviour (no
item moves) until Global evidence shows a partial move; then B3-1 follows it.

### 2.3 Loot rights (D121)

For 10 seconds after the kill only the top-damage character or its party may loot or move the
corpse; afterwards anyone may (`controls.md`). Player corpses are exempt. The behaviour when the
top-damage character is gone is **UNKNOWN** (keep the 10-second window).

### 2.4 Shared experience and stamina (D118)

- **Shared experience** (`combat.md`): the leader enables it; the lowest level must be at least
  two thirds of the highest; every member within 30 tiles of the leader (any floor), recently active
  (healed a member or attacked an aggressive creature); the leader without a battle sign. Bonus +20%
  for kills worth at least 20 XP, +30%, +60% or +100% for 2, 3 or 4 distinct vocations. The XP
  splits evenly, rounding up; a summon takes its share first. When a condition fails, sharing turns
  off and XP goes by damage share. The activity window and the remainder rule are **UNKNOWN**.
- **Party size** (D109): tibia.com states that the Party Hunt Analyser tracks "up to 50 single party
  members" (\`imports/official/tibia-com/2026-09-28-160207Z/facts.json\`). No other party cap was
  found, so 50 is the ceiling until tibia.com evidence shows another value.
- **Stamina** (`characters.md`): 42 hours. The top 3 hours give Premium characters +50% XP. At 14
  hours or less XP is 50% and the top-damage character gets no loot. At 0 no XP. Regeneration
  starts after 10 minutes offline: 1 minute per 3 offline up to 39 hours, 1 per 6 from 39 to 42.

**Amendment (architect ruling, 2026-10-01, #162): creature-kill XP is a closed product.** XP gained on a creature kill is the base creature XP multiplied by the product of a closed list of multipliers, each owned by its own decision:

- stamina (D118, this section);
- the Prey XP bonus (PREY-0, `OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md`);
- party shared experience (PARTY-PVP-0, `OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md`).

Only a later decision extends the list. Store XP boosts and double-XP events are not in base; they come later with the Store and event decisions. No boost source exists now.

### 2.5 Mounts (D125)

Mounted characters gain +10 speed (TibiaWiki; the manual gives no number). Per-mount exceptions:
**UNKNOWN**.

### 2.6 Home town (D119)

Characters on Newhaven or Rookgaard have no citizenship; they pick a home city with a public port
when they leave the starter island. A Portal of Citizenship changes it. Death respawns at the home
city's temple. When Premium expires, a character whose home city is a Premium city moves to the Thais temple on
its next login, as the Premium activation decision already specifies; an online character stays
in place until then. The
respawn before a city is chosen is **UNKNOWN**; until the starter island exists, the first world
assigns one home town with a temple.

## 3. Routing

| Lane | Carries | Depends on |
|---|---|---|
| Combat D/E | D118: base XP and stamina with one reward principal first; shared experience lands in the same XP slice with `COMBAT01-REWARD-PRINCIPALS` raised to 50 (D109; 1 per death today, VSL Combat rows decision row 10), registered by that lane with max and max+1 tests. D121 | progression readiness; VIS-2 for observation |
| DEATH-1/2 | D119 (respawn at the home-town temple), D120 with D59 unchanged | DEATH-0; the home town lane |
| Home town and temple (new) | a Character home town and temple positions, the default town | a Character position lane (DEATH-0 §3.4) |
| NPC trade and dialogue (new, priority) | NPC services, blessing sale at Global prices | Interaction owners; content |
| DEATH-4 | blessing purchase receipts (DEATH-0 §3.3) | NPC trade lane |
| Character creation (new) | sex and starter outfit (D123) | APP lanes |
| Platform tester Premium (new, Platform repository) | a test-only entitlement fixture (D124) | Platform authority |
| Mount activation (new) | activation and +10 speed (D125) | APP lanes; movement speed |
| AccountAchievement (new) | domain owner, catalogue and grant path (D126) | account-scoped progress (D44) |
| B3-1 | D127 per §2.2 | none |
| VIS-1 / PERF | D128 as the dense-scene target | none |

## 4. Decision test

- **Must decide now:** YES. Combat D/E, DEATH-1/2 and B3-1 are allocatable and each hit one of
  these items.
- **Minimum sufficient:** the owner's choices, the Global facts that make them implementable and a
  routing table; no schema or value is registered here.
- **Superseding evidence:** tibia.com evidence for the §2 UNKNOWN items; PERF measurements (D128).
- **Deliberately not decided:** blessing prices (from Global evidence in the NPC lane), creature XP
  and decay values (content), town list (map content).

## 5. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "proactive sweep of 2026-09-27/28 decision open items"
owner_decisions: [D109, D118, D119, D120, D121, D122, D123, D124, D125, D126, D127, D128]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D118_D128_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false   # D124 needs its own Platform lane
implementation_may_resume: true
required_fresh_allocation: true
required_independent_review: "exact-head independent review (routing, Global facts)"
implementation_lanes: [Combat-DE, DEATH-1, DEATH-2, HOME-TOWN, NPC-TRADE, DEATH-4, CHAR-CREATE, PLATFORM-TESTER-PREMIUM, MOUNT, ACHIEVEMENT, B3-1, VIS-1]
remaining_unknowns:
  - Newhaven level-6 rule at the target date
  - Global partial stack pickup
  - shared-experience activity window and remainder rule
  - respawn before a home city is chosen
  - an explicit Global party-size cap (D109 uses 50 from the Party Hunt Analyser)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates the lanes in §3."
```
