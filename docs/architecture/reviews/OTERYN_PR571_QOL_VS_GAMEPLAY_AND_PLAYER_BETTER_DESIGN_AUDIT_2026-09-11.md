# PR #571 — QoL vs Gameplay Impact + Better-for-Players Design Audit

- Date: **2026-09-11**
- Repository: `Oteryn/Oteryn-Game`
- Related PR: `#571`
- Source PR head reviewed: `b2ef322791520130a2683f3004d26abd5c027ac8`
- Archive branch: `docs/pr571-audit-chat-archive-20260911`
- Status: **NON-AUTHORITATIVE PRODUCT/AUDIT EVIDENCE**
- `IMPLEMENTATION_AUTHORITY: NONE`
- `PRODUCTION_AUTHORITY: NONE`

## 1. Owner direction captured after the completed PR #571 audit

The owner added two product requirements after the first complete 192-entry audit:

1. Explicitly separate relatively small/simple QoL changes that should not materially alter Tibia Reference gameplay from changes that meaningfully modify mechanics, progression, combat, economy, rewards or other gameplay semantics.
2. Perform an additional audit of ideas that can be designed **better than originally described**, so Oteryn is genuinely better for players rather than merely implementing the historical request literally.

These requirements refine prioritisation. They do not approve implementation by themselves.

---

## 2. Add a second classification axis: Semantic Impact

The existing roadmap priority axis (`S/A/B/C/R`) answers roughly **how important / promising** an idea is. It does not sufficiently answer **how deeply it changes Tibia semantics**.

Every proposal should therefore carry two independent labels:

```text
Priority:        S | A | B | C | R
Semantic Impact: Q0 | Q1 | G1 | G2
```

### Q0 — Presentation QoL

Presentation/ergonomic improvement that does not intentionally change game rules, authoritative timing, reward output, cost, power or risk.

Typical examples:

- UI scaling and font scaling;
- larger/resizable minimap and panels;
- persistent layout presets;
- persistent filters/search/tab state;
- own-market-offer highlighting/sorting;
- Bestiary/Collection search, sorting and filtering;
- Active Effects presentation when it only displays already-authoritative state;
- appearance-editor convenience.

Default treatment: strongest candidates for early/shared delivery, subject to profile-safe verification.

### Q1 — Interaction QoL

Makes an already legal action easier or faster to express, but should preserve the authoritative result, legality and meaningful gameplay cost.

Typical examples:

- simultaneous WASD diagonal input mapped to already-legal diagonal movement;
- mouse-wheel bindings with correct UI precedence;
- exact quantity input / `k` / `kk` parsing;
- remembered offline-training selection;
- Equipment Loadout **planning/editor** UX;
- `Keep at least N` reservation policy;
- Party Finder lobby UX, scheduling and ready checks;
- quest blocker explanation.

Q1 needs explicit regression tests proving the convenience does not accidentally increase movement cadence, bypass equip restrictions, create value or disclose protected information.

### G1 — Gameplay-adjacent semantic change

Changes time, availability, logistics, economic efficiency, scope or the practical way a gameplay system is consumed even if it does not redefine the entire combat/progression model.

Examples:

- Offline Exercise sessions;
- direct Stash-to-NPC sale;
- substantially larger potion/rune stacks;
- automated/authoritative party settlement;
- account/world legacy access sharing;
- effective-use Prey/Imbuement time consumption;
- personal loot;
- some Residence/lifestyle convenience when it affects durable utility.

Default treatment: Evolved-only or separately reviewed declared difference unless parity evidence proves otherwise.

### G2 — Core Gameplay / Progression / Economy

Directly changes power, combat, progression, death, spawn throughput, reward generation, PvP, long-term economy or role balance.

Examples:

- Dynamic Spawn throughput scaling;
- Death/Recovery redesign;
- Forge `+N`;
- additional Equipment Proficiency;
- Charm Mastery / Grandmaster power;
- Boss pity/essence reward model;
- Elemental Attunement as a new power layer;
- PvP rules/reward redesign;
- new classic-skill gain formulas.

Default treatment: explicit Evolved design/balance gate, simulation/telemetry where applicable, and no silent Reference leakage.

---

## 3. Three-question semantic-impact test

For every roadmap entry, answer:

1. **At the same server state, can the player obtain a different gameplay result than under Reference?**
2. **Does the feature change time, cost, risk, availability or information required to perform the action?**
3. **Can it change XP, gold, item supply, damage, healing, survivability, progression speed or reward eligibility?**

Interpretation:

- all three effectively **no** -> usually `Q0`;
- ergonomic execution change, authoritative outcome unchanged -> usually `Q1`;
- meaningful time/cost/availability/logistics change -> at least `G1`;
- power/progression/combat/economy/reward change -> usually `G2`.

This prevents features that merely *look* like QoL from silently changing gameplay.

---

## 4. Strong candidates for an early Safe QoL Wave

Subject to live ownership/allocation and Reference/profile review, the following are high-confidence Q0/Q1 candidates:

- UI scale and font scale;
- resizable minimap and panels;
- layout/workspace persistence;
- saved Market/Stash/Depot filters and selected views;
- pinnable/read-only chat views;
- Active Effects presentation;
- exact Market quantity input;
- Bank amount shorthand (`k`, `kk`) with exact integer parsing;
- own-market-offer highlighting and sorting;
- Collection/Bestiary search/sort/filter/incomplete views;
- larger saved tracker-interest lists while keeping the active HUD bounded;
- remembered Offline Training choice;
- favorite/recent outfit colours;
- mouse-wheel hotkeys with UI-scroll precedence;
- simultaneous-WASD diagonal intent mapped to normal legal diagonal movement;
- NPC sale filters such as `show only carried/sellable`;
- task-aware views/filters;
- quest prerequisite/blocker explanation;
- Party Finder lobby UI, scheduling and ready checks;
- consent/privacy-aware Contacts/VIP tags;
- Equipment Loadout editor/preset model while activation continues to obey server equip semantics.

---

## 5. Features that look like QoL but require semantic caution

### Larger stacks

Looks like inventory convenience but can materially change capacity constraints, hunting duration and supply logistics. Treat as `G1` until balance proves otherwise.

### Direct Stash-to-NPC sale

QoL only if it preserves the required service/NPC access boundary. Selling from anywhere would be a different gameplay service.

### Equipment Loadouts

The planning/editor surface is Q1. Instant full-set combat switching can become G1/G2 if it bypasses the timing/restrictions of individual equipment actions.

### Party GPS / exact map position

Improves coordination but can alter scouting, PvP information and privacy. Exact information needs permission/ruleset boundaries.

### Prey effective-use timer

Reduces wasted paid/earned time, but materially increases the effective economic value of a Prey hour. Treat as G1/G2, not simple timer QoL.

### Offline Exercise

Removes pointless always-online clients and is an accepted owner direction for Evolved, including simultaneous training of multiple characters when each consumes its own resources. It still changes how resources/progression are consumed, so classify as G1.

### Personal loot

May reduce party conflict but changes item-generation/reward allocation. Treat as G2 until the full encounter reward budget is modelled.

---

# Part II — Better-for-Players redesign audit

## 6. Product principle

The target is not merely to add more systems.

**Oteryn should reduce wasted time and unclear friction without removing meaningful decisions, risk, exploration, preparation, cooperation, world presence and item identity.**

A recurring product problem in MMORPGs is that excessive convenience can turn the world into a sequence of menus/queues, while excessive friction pushes players into external spreadsheets, wiki pages, Discord coordination or repetitive chores. Oteryn should deliberately target the middle ground.

---

## 7. Cross-programme redesign: Activity Plan as the UX spine

The strongest improvement over PR #571 is to stop treating Huntfinder, loadouts, Party Finder, access checks, Stash, Market, map, trackers, settlement and restocking as unrelated windows.

Create an **Activity Plan** that coordinates existing systems around the player's intent:

```text
choose hunt / boss / quest
-> check access/readiness
-> prepare loadout and supplies
-> form/schedule party
-> navigate/play
-> review results
-> settle/restock
```

Example readiness card:

```text
Activity: selected hunt

Access:              ready
Level / role:        suitable
Protection:          warning — weak Energy protection
Weapon/loadout:      ready
Imbuements:          warning — 2h remaining
Supplies:            below saved target
Charm/proficiency:   configured
Channel availability: probably free, freshness shown
Party:               missing healer role
```

The system informs and prepares; it does **not** automatically play, teleport, buy arbitrary resources or select the player's decisions.

This should be an orchestration/presentation layer over owning systems, not a twentieth independent progression authority.

---

## 8. Programme-by-programme redesign opportunities

| # | Programme | Weakness of literal/original interpretation | Better Oteryn design | Impact |
|---:|---|---|---|:---:|
| 1 | Client UX, Input, Workspace & Active Information | Many isolated UI features | One contextual **Activity Workspace** with persistent state, progressive disclosure and safe input ownership | Q0/Q1 |
| 2 | World Map, Discovery, Huntfinding & Capacity | A recommendation engine can become a black box | Explain *why* a hunt fits; show confidence/freshness for availability; configurable spoiler levels | Q1/G1 |
| 3 | Party, Social, Group Finder & Hunt Coordination | Long lobby forms repeat information the system already knows | One-click lobby from Activity Plan, autofilled activity/level/vocation/access, role slots, language and ready check | Q1/G1 |
| 4 | Inventory, Loot, Depot, Stash & Logistics | Risk of dozens of unrelated automations | Two strong flows: **Prepare Hunt** and **Finish Hunt**, with preview and protected/reserved quantities | Q1/G1 |
| 5 | Bank, Trade, Market, Postal & Financial UX | Multiple disconnected money/item windows | A coherent **Value Workspace** with market/NPC/stash/bank context, history and explicit net effects | Q0–G1 |
| 6 | Build, Loadouts, Itemization, Forge & Proficiency | Mechanical swap mixed with multiple new power layers | Separate **Plan** from **Active Loadout**; activity-specific comparison; keep new vertical progression optional/research until justified | Q1/G2 |
| 7 | Imbuement & Elemental Attunement | Can remain maintenance-heavy while adding more power | Preparation profiles, recharge clarity and anti-waste convenience; do not grant an unbudgeted extra protection layer | G1/G2 |
| 8 | Skills, Training, Weapon Proficiency & Wheel | Threat coefficients/activity score can become opaque hidden math | Prefer explainable natural-combat progression and clear separate progression tracks | G2 |
| 9 | Quest, Adventure, Account-World Unlocks | Journal as a larger checklist still drives wiki dependence | **Access Passport + Quest Navigator** with user-selectable spoiler depth | Q1/G1 |
| 10 | Bestiary, Charms, Collections, Echo & Prey | Shared browser alone does not create good goals | Collection Goals: nearest completions, region goals, favourites; no universal mandatory power track | Q0/G1 |
| 11 | Boss & Encounter System | Difficulty/pity/personal-loot listed as independent knobs | Unified Encounter Framework: Practice -> Normal -> Challenge, with one transparent reward budget | G2 |
| 12 | Death, Recovery & Connection Resilience | Recovery Pool + Fatigue can punish twice | Prioritise **Death Report**, clear recoverable/permanent loss and reconnect-safe recovery; reconcile with existing #295/FND-04 | G2 |
| 13 | PvP, Arenas, Wars & High-Risk Zones | Too many possible modes at once | Ship one excellent structured/objective mode first; make risk/reward explicit | G2 |
| 14 | Tasks, Bounty, Weekly & Dynamic Spawn | Risks another FOMO checklist | **Activity Contracts**: choose from offers, allow limited banking/rerolling, integrate naturally with hunts | G1/G2 |
| 15 | Vocation Identity, Combat Rules & Balance | Scalar balance targets can homogenise classes | Preserve strong asymmetric strengths/weaknesses while ensuring no supported vocation is excluded from broad content categories | G2 |
| 16 | Housing, Lifestyle, Fishing, Cooking & Personal Spaces | Separate side systems can become mandatory buff chores | Residence as identity/social hub: trophies, records, cosmetics, cooking/fishing collections; avoid mandatory combat DPS buffs | G1 |
| 17 | Economy Health & Gold Sinks | "add sinks" can create arbitrary taxes | Source/Sink Budget; prefer sinks attached to desired services/cosmetics/training rather than artificial friction | G2 |
| 18 | Security, Anti-Bot & Telemetry | Detection platform can over-penalise legitimate patterns | Evidence-first risk scoring, bounded escalation/appeal, explicit distinction between legal Oteryn QoL and prohibited automation | G2 |
| 19 | Premium & Commercial Policy | Merely preventing direct P2W is insufficient | Do not sell solutions to friction intentionally created by the game; core social/UI/safety QoL should remain broadly available | G2/policy |

---

## 9. Group Finder: improve the complete flow, not only the feature list

A recurring lesson from existing MMORPG tooling is that simply having a Team/Group Finder does not ensure players use it.

For Oteryn, avoid a large form requiring the player to manually re-enter data the game already knows.

Preferred flow:

```text
Activity -> Find Team
```

Autofill:

- selected activity;
- character level;
- vocation/role;
- known access eligibility;
- current World/Channel context.

Player chooses only the missing intent:

- now / scheduled;
- visibility scope (public/friends/guild where supported);
- wanted role(s);
- optional note/language preference.

Useful additions:

- recent successful teammates;
- favourites/contacts;
- private avoid/block list;
- readiness/access compatibility.

Avoid public star ratings for players unless separately justified; they can produce harassment, gatekeeping and a secondary reputation economy.

Do not automatically teleport players to the content merely because a lobby forms. Oteryn should reduce organisational friction without erasing world traversal by default.

---

## 10. Core social QoL should not become a monetisation wall

Party Finder, basic map/navigation, communication safety, UI scaling and core inventory safety are foundational retention/social systems.

Recommendation: do not make them premium-exclusive merely because comparable external games have monetised some convenience.

If Premium/paid entitlement later adds convenience, safer categories include cosmetics, prestige, additional non-power customisation and bounded secondary convenience, always subordinate to Platform commercial authority and accepted policy.

---

## 11. Activity Loadouts: better than a simple equipment preset

Use a named **Activity Loadout** that can reference:

```text
Equipment
Protection profile
Ring/amulet plan
Imbuement plan
Hotkeys/action layout where appropriate
Charm configuration
Wheel/proficiency plan
Supply targets
Tracker/workspace layout
```

Crucially separate:

### PLAN

What the player wants for the activity.

### ACTIVE STATE

What is actually equipped/configured now.

This lets the client report truthful readiness when:

- an item is missing;
- a ring/charge expired;
- an imbue is low/expired;
- a charm is unavailable/already assigned;
- required supplies are missing.

Example:

```text
Ready: 87%
Missing: Energy protection target
Warning: Imbuement expires in 1h42m
```

Activation remains a server-authoritative operation and must preserve combat/equipment restrictions and deterministic failure semantics.

---

## 12. Build comparison: do not collapse choices into one Gear Score

Avoid making one scalar `Item Power` or `Gear Score` the main decision surface.

Prefer context-based comparison:

| Attribute | Set A | Set B |
|---|---:|---:|
| Fire mitigation | 21% | 31% |
| Physical mitigation | 14% | 10% |
| Healing/sustain estimate | higher/lower with assumptions shown | higher/lower with assumptions shown |
| Damage estimate | context-specific | context-specific |
| Imbuement capacity | 3 | 4 |

This preserves meaningful tradeoffs.

**Imbuement slots are part of an item's total value budget.** A statistically weaker item may be better for a use case because it has an additional imbuement slot or a different defensive/sustain profile.

Do not automatically stack Forge, `+N`, Equipment Proficiency, Attunement, Charm mastery and every other possible system into mandatory vertical progression. Each new progression layer must create a genuinely different choice or replace an existing layer.

---

## 13. Inventory/Stash: replace many chores with two explicit flows

### Prepare Hunt

Potentially coordinates:

- loadout readiness;
- supply targets;
- runes/ammo/potions;
- food;
- imbue status;
- container layout.

The system may offer `restock to target` only through accepted authoritative sources/services and explicit player confirmation.

### Finish Hunt

Classify loot into:

```text
Keep
Stash
Market candidate
NPC-sale candidate
Reserved/protected
Quest/task relevant
Unknown/manual review
```

Before any value-losing batch operation, show a preview, for example:

```text
218 items selected
Expected NPC value: 412,300 gp
7 items protected by reservation
3 items have a potentially different market disposition
Confirm
```

This removes repetitive container work without becoming an autonomous trading bot.

---

## 14. Huntfinder: explain, do not dictate

Avoid an opaque recommendation such as:

```text
AI recommends Hunt X
```

Prefer:

```text
Hunt X — strong fit

Why:
- level/role fit
- relevant access owned
- strong protection profile
- desired XP/profit profile
- charm/proficiency status

Warnings:
- weak Death protection
- likely occupied on current Channel

Alternative:
- Channel 3 probably free
- last activity evidence 90 seconds ago
```

The player remains the decision-maker.

For availability, show **confidence and freshness**, not false certainty:

```text
Probably free
Confidence: 82%
Last significant activity: 4m12s ago
```

Do not expose player identity/location by default merely to implement hunt availability.

---

## 15. Quest/Access design: reduce wiki dependence without deleting exploration

A better solution than a universally explicit quest arrow is a selectable hint depth.

Example three-level model:

### Exploration

`The priest in Edron may know more.`

### Guided

`Talk to Father Joachim in Edron.`

### Explicit

Show the exact marker / required item where product rules permit it.

Players choose how much guidance they want.

Add an **Access Passport** summarising meaningful access states without exposing raw internal quest flags:

```text
Yalahar                    ready
Roshamuul                  ready
Ferumbras Ascendant        ready
Soul War                   missing prerequisite
```

Selecting a missing access can explain the nearest known prerequisite without automatically completing or revealing hidden story choices.

This data can feed Party Finder compatibility safely.

---

## 16. Account-World progression: respect the player without erasing character identity

A useful model separates account/world knowledge/convenience from character power.

Safer Account-World candidates:

- selected legacy access;
- story knowledge/completion recognition;
- cosmetics;
- exploration history;
- compatible collections.

Character-scoped by default:

- level;
- skills;
- gear;
- vocation-specific progression;
- combat mastery;
- meaningful quest choices/rewards where economy or identity requires it.

Principle:

```text
"the player has done this before"
!=
"every character gets the full reward automatically"
```

---

## 17. Bosses: Practice is often more valuable than another numeric difficulty tier

Recommended framework:

### Practice

- no normal XP;
- no normal loot;
- no pity/essence progression;
- rapid learning/reset where technically safe.

### Normal

Main progression/reward experience.

### Challenge

Harder mechanics and/or lower margin for error, with explicitly modelled reward differences.

Avoid a design consisting only of `+40% HP, +20% damage`.

If pity or guaranteed progression exists, keep it transparent where product design allows:

```text
7 / 10 Essence toward guaranteed reward
```

Personal loot + pity + essence share one encounter reward budget. They are not three independent full-value reward systems.

---

## 18. Death: make failure informative

Before adding more punishment, give the player a useful **Death Report**.

Possible report:

```text
Last 10 seconds
- Physical: 42%
- Fire:     31%
- Death:    18%
- Other:     9%

Relevant events
- Energy Ring expired
- Fire-protection imbue inactive
- healing gap: 1.8s
- 6 hostile creatures adjacent

Loss
- permanent component
- recoverable component
- corpse/item state
```

Only show information the player is entitled to know; do not leak hidden server or enemy state.

Death/recovery design must reconcile with existing Oteryn #295 direction and FND-04 rather than creating a second independent debt/reconnect model.

---

## 19. Weekly/Bounty: prefer chosen contracts over FOMO checklists

A possible improvement is **Activity Contracts**:

- receive several offers;
- choose a bounded subset;
- optionally bank a limited number into the next period;
- bounded reroll;
- complete naturally during normal hunts where appropriate.

This gives players direction without making every weekly reset a mandatory chores list.

Exact rewards, banking and reroll rules require economy simulation before acceptance.

---

## 20. Lifestyle/Housing: identity first, combat obligation second

Accepted Oteryn housing already contains Residence and scarce physical houses. Do not introduce a third competing personal-property authority merely to satisfy a `Hideout` label.

A Residence can become an optional identity/social hub with features such as:

- trophies;
- aquarium/fishing records;
- cooking presentation;
- wardrobe/cosmetics;
- boss memorabilia;
- guestbook/social use;
- decoration collections.

Avoid making lifestyle systems a mandatory combat checklist, e.g. `everyone must maintain a chef because it gives required DPS`.

---

## 21. Economy: not every payment is a healthy gold sink

Track separately:

```text
gold created
gold destroyed
gold transferred player -> player
item created
item destroyed
system service fees
economy-linked progression consumption
```

A player-to-player payment is not automatically a sink.

Prefer sinks attached to things players voluntarily value:

- training;
- decoration/prestige;
- selected services;
- convenience that does not sell power;
- progression maintenance that is itself justified.

Avoid arbitrary friction taxes whose only purpose is to remove gold.

---

## 22. Progression-layer budget

Potential Evolved systems already include or propose:

- character level;
- classic skills;
- Skill Wheel;
- Weapon Proficiency;
- Forge;
- `+N` enhancement;
- Equipment Proficiency;
- Attunement;
- Charm mastery;
- Region mastery;
- Renown/adventure progression.

Even if each is individually interesting, enabling all of them as mandatory vertical power would create excessive prerequisite burden.

Require every new progression system to satisfy at least one of:

1. creates a genuinely different qualitative choice/opportunity cost; or
2. replaces/simplifies an existing layer.

If a proposed system merely adds another mandatory small percentage on top of all existing systems, default disposition should be `BADANIE` or `ODRZUCIĆ` until justified.

---

## 23. Alt-friendly, not alt-mandatory

Preserve the accepted owner direction for concurrent Offline Exercise across multiple characters when each legitimately consumes its own resources.

At the same time, account design should not make many alts mandatory for optimal economy/progression.

Monitor at account level where relevant:

- generated value;
- daily/weekly eligibility;
- boss rewards;
- market impact;
- exercise-resource consumption;
- any cross-character unlocks.

QoL for alts must not become an exploitative multibox/account-farming meta.

---

## 24. Vocation balance: asymmetry is desirable, exclusion is not

Do not force every vocation toward equal scalar DPS in all contexts.

A supported class should have clear:

- strengths;
- weaknesses;
- party contribution;
- solo viability boundaries;
- mobility/sustain/utility identity.

But no supported vocation should be effectively excluded from a broad major category of content because its kit cannot contribute meaningfully.

Balance telemetry should evaluate solo hunts, coordinated team hunts, bosses and PvP separately.

---

## 25. Ten highest-value upgrades over the literal PR #571 proposals

1. **Activity Plan** as the preparation/activity spine.
2. **Social Assist Group Finder** instead of a large manually completed form.
3. **Prepare Hunt / Finish Hunt** inventory flows.
4. **Activity Loadouts** with `Plan` vs `Active` state.
5. **Quest/Access Navigator with selectable spoiler depth.**
6. **Death/Wipe Analysis** so failure teaches the player.
7. **Transparent boss progression/pity** if pity exists.
8. **Selectable/bankable Activity Contracts** rather than pure weekly FOMO.
9. **Alt-friendly Account-World unlocks** that preserve character identity and reward/economy rules.
10. **Fewer mandatory vertical power layers, more meaningful qualitative choices.**

---

## 26. Required roadmap metadata for the next pass

Every material PR #571 item should gain fields equivalent to:

```text
PLAYER_PROBLEM:
PRIORITY: S | A | B | C | R
SEMANTIC_IMPACT: Q0 | Q1 | G1 | G2
BETTER_DESIGN:
DO_NOT_CROSS:
SUCCESS_METRIC:
REFERENCE_BASELINE_OR_DECLARED_DIFFERENCE:
OWNING_SYSTEM/CONTRACT:
```

Example:

```text
FEATURE: Team Finder

PLAYER_PROBLEM:
Finding a suitable team takes too much time and external coordination.

SEMANTIC_IMPACT:
Q1/G1

BETTER_DESIGN:
Create lobby directly from Activity Plan and autofill activity,
character role/level/access context. Player chooses only missing intent.

DO_NOT_CROSS:
- no automatic teleport by default;
- no public player star rating by default;
- no disclosure of private raw quest state;
- no forced party membership.

SUCCESS_METRIC:
- time from Find Team to formed party;
- successful lobby completion rate;
- repeat use;
- abandonment rate;
- report/block/privacy incidents.
```

---

## 27. Recommended next audit artifact

The original 192-entry audit is complete. The next useful artifact is not another broad idea search.

Perform an **Implementation Readiness Pass** that adds to each canonical feature:

- `Semantic Impact` (`Q0/Q1/G1/G2`);
- exact player problem;
- improved design;
- Reference/Evolved disposition;
- owning accepted contract/domain;
- server/client/protocol/persistence impact;
- minimum acceptance test;
- telemetry/success metric;
- dependencies and implementation slice.

This will allow Oteryn to create an early **Safe QoL Wave** while keeping G1/G2 mechanics behind appropriate gameplay/economy/balance decisions.

---

## 28. Evidence / limitations

This document records the additional owner direction and the resulting product-design audit from the conversation after the full PR #571 audit.

It does not establish:

- implementation authority;
- exact numerical balance;
- final economy parameters;
- exact Global Reference behaviour for every concrete mechanic;
- runtime readiness;
- production readiness;
- approval of every proposed redesign.

External MMO examples and community discussions are product evidence/heuristics, not authority over Oteryn. Live Oteryn accepted architecture, Reference evidence, repository governance and owner decisions remain controlling.
