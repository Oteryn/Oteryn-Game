# Oteryn Evolved — Master Product & Gameplay Roadmap

- Date: 2026-09-11
- Status: **OWNER-REQUESTED CONSOLIDATED PRODUCT ROADMAP / PROPOSED / NOT IMPLEMENTATION AUTHORITY**
- Protected repository snapshot used for consolidation: `Oteryn/Oteryn-Game@90a3f92434e32354ff1aaeac96d038bbc49eba9c`
- Primary target profile: **OTERYN EVOLVED**
- Reference rule: **Tibia Reference remains parity-first; Evolved-only mechanics must not drift into Reference accidentally.**
- Implementation authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This is the consolidated master roadmap for future Oteryn Evolved product, gameplay, client, economy, social, progression and quality-of-life systems.

It combines and de-duplicates:

1. the historical `blakinio/canary` future-systems roadmap;
2. its 128-item origin/type classification index;
3. the forum-derived Tibia player requests collected in the owner conversation;
4. the Oteryn QoL clustering work in `OTERYN_EVOLVED_QOL_FEATURE_CLUSTERS_2026-09-11.md`;
5. owner clarifications made during review of those requests.

The point is **not** to create hundreds of isolated backlog tickets. The point is to preserve every useful idea while mapping it into a smaller number of coherent systems with clear product boundaries.

## 2. Product philosophy

Oteryn Evolved should:

- **remove legacy friction that is irritating rather than meaningful**;
- **keep danger, difficulty, preparation and long-term progression meaningful**;
- modernize UI, input, social coordination, inventory and economy workflows without making the game play itself;
- prefer **server-authoritative semantic operations** over client macros for anything that changes gameplay or value;
- make convenience reusable and systemic rather than creating one-off exceptions;
- use **telemetry, bounded rewards, diminishing returns and anomaly detection** for systems exposed to abuse;
- preserve item/currency conservation and durable transaction safety even when reducing clicks;
- keep power systems understandable and avoid stacking dozens of mandatory vertical progression layers;
- separate **Reference parity**, **shared presentation QoL** and **Evolved mechanics** explicitly.

## 3. Priority and disposition legend

### Priority

- **S — Foundation / high return:** should shape architecture early or delivers exceptional player value.
- **A — Strong:** important product improvement after/with its owning domain.
- **B — Later / balance-heavy:** valuable but should not delay core delivery.
- **C — Low priority / caution:** weak value, high friction or dangerous product direction.
- **R — Research only:** retain for evaluation, not a promoted implementation direction.

### Profile disposition

- **SHARED-UX:** can usually exist in both Reference and Evolved because it changes presentation/ergonomics rather than game rules.
- **EVOLVED:** intended product difference from Tibia Reference.
- **REFERENCE-PARITY:** preserve/verify official behavior before redesign.
- **DEFER:** useful but later.
- **REJECT:** do not adopt in the proposed form.

---

# 4. Canonical programme map

## 1. **Client UX, Input, Workspace & Active Information**

**Priority: S**  
**Profile: SHARED-UX by default; semantic gameplay differences remain profile-gated.**

Goal: make the native client feel modern on 1080p, 1440p, 4K and ultrawide displays while reducing external-tool dependence and repetitive UI setup.

### 1.1 **Adaptive / scalable UI**

- independent UI scaling;
- independent font scaling where useful;
- runtime DPI/monitor changes;
- resizable game viewport, minimap, action bars and panels;
- no coupling between UI scale and world-camera zoom.

### 1.2 **Persistent workspace and layout presets**

- dockable/resizable panels;
- Hunting / Boss / PvP / Trading presets;
- persistent panel sizes, tabs, filters, search, selected categories and scroll position;
- safe recovery when resolution or layout data changes.

### 1.3 **Persistent Market / Stash / Depot workflow state**

- Market and Stash do not unnecessarily close each other;
- closing Market does not destroy Stash context;
- search/filter/category state persists;
- `stash all items of this type` must not close unrelated mailbox/depot UI.

### 1.4 **Modern input binding**

- native `Wheel Up` / `Wheel Down` gameplay bindings;
- UI scroll gets first chance to consume wheel input;
- high-resolution/free-spin wheels normalize into bounded semantic steps.

### 1.5 **Native simultaneous-WASD diagonal movement**

- `W+A -> northwest`;
- `W+D -> northeast`;
- `S+A -> southwest`;
- `S+D -> southeast`;
- resolve held direction state, not timing macros;
- opposite cardinal inputs cancel on that axis;
- server movement cadence, collision and legality remain unchanged.

### 1.6 **Persistent / pinnable chat views**

- pin tabs;
- auto-open on login;
- remember position/layout;
- clone read-only views;
- unread/mute behavior;
- typed message categories instead of one-off Event Chat plumbing.

### 1.7 **Active Effects UI**

Unify food timers, potion/buff icons and effect-duration requests into one system:

- effect identity/icon;
- magnitude/summary;
- source where permitted;
- remaining time;
- stack/refresh state;
- configurable compact HUD plus full Effects panel;
- no artificial small icon ceiling.

### 1.8 **Custom timers and reminders**

Client-local timers for Prey, XP boosts, Exercise Weapons, House Food and arbitrary activities:

- custom name/duration;
- repeat;
- optional notification;
- movable/pinnable timer panel;
- per-character presets;
- import/export later.

When the server already knows an authoritative expiry, prefer a game-aware reminder over manual duplicate timer entry.

### 1.9 **Appearance customization**

- larger outfit palette/bounded picker;
- favorite/recent colors;
- randomize all or selected parts;
- lock parts while randomizing others.

---

## 2. **World Map, Discovery, Huntfinding & Hunting Capacity**

**Priority: A/S**  
**Profile: Evolved with shareable client primitives.**

Goal: turn map and hunt discovery into a first-class navigation/information system rather than scattered external wiki/tool workflows.

### 2.1 **Huntfinder 2.0**

Search/recommend hunts using level, vocation/role, XP/profit goals, equipment, imbuement, charm and access context.

### 2.2 **Hunting Spot Availability**

Status model:

- free;
- probably occupied;
- occupied;
- unknown.

Use real activity signals rather than only player presence. Do not expose player identity by default.

### 2.3 **Notify when free**

A selected hunting area can trigger a user opt-in notification when availability evidence changes to free/probably free.

### 2.4 **World Map & Discovery 2.0**

- searchable/filterable POIs;
- discovered/undiscovered layers;
- navigation support;
- boss/hunt/service/depot/quest categories;
- scalable minimap and full map reuse one underlying map model.

### 2.5 **Permissioned live social map**

- party/guild/contact markers;
- exact placement only when policy allows;
- same/different channel and instance semantics;
- privacy/PvP aware.

### 2.6 **Shared discovery markers**

Share Echo Raid, Fiendish, rare boss, event or rally-point observations to selected scopes without converting the map into public surveillance.

### 2.7 **Dynamic Spawn & Hunting Capacity**

Use player pressure, effective power, spawn budgets and area capacity to improve hunt availability while preserving bounded farming throughput.

---

## 3. **Party, Social, Group Finder & Hunt Coordination**

**Priority: S**  
**Profile: Evolved; social presentation may be shared where parity-safe.**

Goal: make finding, forming, preparing and operating a party substantially easier without removing consent or social choice.

### 3.1 **Party Finder 2.0 / Group Lobbies**

One canonical system, not parallel finders.

Modes:

- Hunt;
- Boss;
- Soul Core / Soulpit;
- Quest/Event where useful.

### 3.2 **Scheduled lobbies**

Hosts may create immediate or future sessions with server-time scheduling and reminders.

### 3.3 **Role/vocation slots**

Visual EK/ED/MS/RP/Monk or flexible-role slots using the shared role taxonomy.

### 3.4 **Eligibility checks**

Server checks only what is necessary:

- level range;
- quest/access eligibility;
- required role/vocation;
- optional declared protection/imbuement requirements.

Do not expose a player's entire private quest state to lobby hosts.

### 3.5 **Whitelist, blacklist and contact integration**

Use the consent-based contact/block model rather than creating a separate Party Finder social database.

### 3.6 **Shareable lobby links**

Chat-safe lobby references open the lobby details/application view.

### 3.7 **Ready check and Form Party**

`Form Party` sends normal invitations to accepted members; it does not silently force players into party membership.

### 3.8 **Tactical hunt map**

Host may mark:

- lure routes;
- shooter positions;
- safe tiles;
- boss mechanics/rally points.

This is collaborative planning, not gameplay automation.

### 3.9 **Party System 2.0**

Rework active-party UX, shared objectives, roles and shared-EXP/bonus presentation without duplicating Party Finder.

### 3.10 **Party combat visibility**

- outlines/role/leader markers;
- low-HP/status emphasis;
- priority members;
- off-screen direction indicators.

### 3.11 **Shared Hunt Accounting & Settlement**

Unify Party Loot Ledger, food/supply accounting and forum loot-split requests:

- session participants and interval;
- loot and supply consumption;
- valuation policy;
- waste/manual adjustments;
- per-player balance;
- idempotent settlement.

No leader may debit another player's bank without that player's authorization.

### 3.12 **Modern Contacts / VIP**

- mutual consent where exact presence is shared;
- private tags instead of tiny fixed group caps;
- level/guild/party context where policy permits;
- block/reject/privacy controls.

---

## 4. **Inventory, Loot, Depot, Stash & Item Logistics**

**Priority: S**  
**Profile: mostly Evolved; many presentation improvements are shareable.**

Goal: reduce repetitive container micromanagement while preserving physical inventory gameplay where it adds meaning.

### 4.1 **Loot System Rework**

- Loot Nearby / Loot All;
- bounded multi-corpse queue;
- optional auto-loot policy where accepted;
- loot filters;
- destination containers by category;
- party loot rights;
- clear feedback;
- manual loot remains available.

### 4.2 **Summon Loot Assistant**

Optional controlled summon mode with limited radius/speed and normal loot rights. It must not become global instant auto-loot.

### 4.3 **Inventory / Depot / Stash 2.0**

- global search;
- smart containers;
- sorting;
- post-hunt deposit flows;
- cleaner storage navigation.

### 4.4 **Smart Item Reservation & Task Protection**

Canonical solution for forum `Protect Item` requests:

- protect all of an item type; or
- `Keep at least N` quantity;
- protected amount excluded from Withdraw All, Sell All, auto-sell, trade/drop automation and other value-losing batch actions;
- clear lock/reserve indicator;
- expose safe-to-sell surplus.

### 4.5 **Task-aware Stash / Objective Relevance**

Show owned/required/missing quantities for known active objectives without leaking unrevealed quest spoilers.

### 4.6 **Larger supply stacks**

Use item-definition-specific maximums. Evolved may use substantially larger values for high-volume potions/runes without imposing one universal max on every item.

### 4.7 **Reward collection exclusions**

- collect selected;
- collect all eligible;
- `keep in chest`/retention markers;
- visible expiry.

### 4.8 **Empty-container management**

Prefer configurable:

- keep;
- move to assigned container;
- discard.

Do not make ground-spam auto-drop the canonical behavior.

### 4.9 **Direct Stash-to-NPC sale**

Atomic authoritative value transaction, with locks/reserved quantities and optional keep-minimum policies.

### 4.10 **NPC sell filtering**

`Show only carried/sellable items` and equivalent useful filters.

### 4.11 **Altered equipment storage / normalization**

For partly used or enchanted rings/amulets, evaluate a generic Normalize/Discharge operation or Equipment Vault rather than isolated special-case NPC logic.

---

## 5. **Bank, Trade, Market, Postal & Financial UX**

**Priority: S/A**  
**Profile: Evolved.**

Goal: make money/item transactions explicit, safe, auditable and fast without weakening anti-duplication/value-conservation rules.

### 5.1 **Bank UI 2.0**

Dedicated transfer UI with recipient, amount, balances before/after, confirmation and duplicate-submit protection.

### 5.2 **Exact amount expressions**

Reusable exact-integer parser supporting forms such as:

- `55`;
- `2k`;
- `200k`;
- `1.5kk` where locale/parser rules make it unambiguous.

Market must also provide a direct exact quantity input instead of forcing massive-range sliders.

### 5.3 **Transaction history**

Durable audit for transfers, deposits, withdrawals, fees/system transactions and resulting balances.

### 5.4 **Trade System 2.0**

Allow item ↔ item, item ↔ bank gold and mixed item+bank-gold negotiation. Any transaction mutation resets acceptance.

### 5.5 **Market UX 2.0**

- persistent filters/categories;
- highlight own offers;
- sort My Offers by price/amount/time where useful;
- configurable/default anonymity policy;
- exact amount entry;
- task-aware deep links and filters.

### 5.6 **Anti-spam market controls**

Do not freeze a blanket five-minute cooldown. Prefer rolling-window rate limits, expiry and evidence-driven anti-abuse controls that ordinary users rarely encounter.

### 5.7 **Sale-only commission**

Evaluate commission on successful sale rather than punishing unsold offers with listing fees; spam controls remain separate.

### 5.8 **Multi-currency Market & Trade**

Explicit currency identity and filtering; no hidden exchange-rate conversion.

### 5.9 **Postal Network 2.0**

- item delivery;
- own-character transfers;
- COD;
- insurance/tracking;
- service tiers;
- guild/house/depot endpoints.

### 5.10 **Global Market + Local Logistics**

One liquid searchable market may retain stock origin, pickup/delivery choices and local logistics.

### 5.11 **Consolidated market delivery**

Combine compatible purchases into one bounded delivery to a selected endpoint.

### 5.12 **Parcel convenience**

The forum's automatic-label request should be solved by a modern addressing/delivery flow, not necessarily by preserving manual label-item friction forever.

---

## 6. **Build, Equipment Loadouts, Itemization, Forge & Equipment Proficiency**

**Priority: S**  
**Profile: Evolved except explicit parity-integration surfaces.**

Goal: provide deep build choice while making build switching/comparison understandable and safe.

### 6.1 **Equipment Loadouts**

Canonical destination for:

- Equipment Presets;
- Protection Sets;
- full-set action-bar hotkeys;
- quick ring switching;
- future named boss/hunt builds.

Use one semantic server operation such as `ActivateEquipmentLoadout(loadout_id)`, with deterministic validation and default all-or-nothing behavior.

### 6.2 **Quick A/B slot toggles**

A two-ring forum request becomes a thin UI convenience over Equipment Loadouts, not a separate ring system.

### 6.3 **Itemization & Build System 2.0**

Connect equipment, item classification, Forge, Imbuements, Skill Wheel, classic skills, Weapon Proficiency and presets into a coherent build-selection layer.

### 6.4 **Build Impact & Comparison**

Show authoritative before/after damage, healing, mitigation and sustain effects where formulas support trustworthy projections.

### 6.5 **Training Arena / Combat Simulation**

Non-rewarding standardized targets for build testing with zero normal XP/loot/progression.

### 6.6 **Equipment Durability — optional / caution**

If retained, use soft maintenance and strong convenience; do not reintroduce constant irritating maintenance merely to manufacture a sink.

### 6.7 **Repair All / Auto Repair**

Only relevant if durability survives product review.

### 6.8 **Forge Slot Mastery**

Evaluate moving long-term Forge-style investment to equipment slot identity rather than losing it every time one physical item is replaced.

### 6.9 **Item Enhancement +N**

Bounded item-instance progression, deliberately separate from other progression layers.

### 6.10 **Equipment Proficiency**

Equipment-category specialization beyond weapons while official Weapon Proficiency remains a separate system.

### 6.11 **Item classification as progression ceiling**

Reuse existing item classes rather than inventing another rarity ladder merely to cap enhancement.

### 6.12 **Proficiency branches / qualitative capstones**

Opportunity-cost paths such as Fortification, Retaliation or Sustain rather than granting all bonuses simultaneously.

### 6.13 **Controlled enhancement RNG and pity**

Higher-end randomness may exist only with bounded bad luck and no routine catastrophic destruction.

### 6.14 **Controlled duplicate-item sink**

Duplicates may support selected high-end progression without requiring mass destruction of ultra-rare BIS items.

### 6.15 **Enhancement salvage / replacement protection**

Recover a bounded portion of item-specific investment when upgrading to replacement gear.

### 6.16 **Existing-tier migration contract**

Any redesign must preserve legitimate existing investment without duplicating permanent power.

### 6.17 **Gem Atelier / Fragment Workshop parity review**

Treat official systems as parity evidence first; custom redesign comes only after current behavior/support is reverified.

---

## 7. **Imbuement & Elemental Attunement System 2.0**

**Priority: A**  
**Profile: Evolved, built from verified Tibia baseline.**

Goal: reduce timer/item duplication friction while retaining meaningful preparation and recurring sinks.

### 7.1 **Imbuement System 2.0**

Modernize the existing foundation rather than creating a duplicate unrelated enchant system.

### 7.2 **Slot-Based Imbuement Library & Active Channels**

Store charged/unlocked choices at slot/profile level while equipped gear controls legal categories and active-channel capacity.

### 7.3 **Elemental Attunement Layer**

Separate bounded elemental protection flexibility from Crit/Leech/Skill competition where product balance supports it.

### 7.4 **Slot / Attunement progression**

Long-term convenience/flexibility progression, with exact levels/effects left for later balance work.

### 7.5 **Progression & maintenance sink**

Combine one-time unlock costs and recurring recharge/renewal so convenience continues to remove gold/resources.

### 7.6 **Effective-use lifecycle**

Evaluate consuming specialized duration only during relevant active use so expensive preparation is not wasted on unrelated content.

---

## 8. **Skill, Training, Weapon Proficiency & Skill Wheel**

**Priority: S**  
**Profile: mixed Reference parity + Evolved progression.**

Goal: make active play a strong progression path while keeping offline/exercise training useful and economically healthy.

### 8.1 **Skill Progression 2.0**

Classic skills remain understandable and separate from Weapon Proficiency.

### 8.2 **Real Combat Training**

Genuine combat should contribute meaningfully to classic skill progression instead of rewarding only raw repetitive hit counts.

### 8.3 **Threat coefficient**

Relative target threat may influence eligible classic skill gain; exact formula requires telemetry/simulation.

### 8.4 **Diminishing returns on one persistent training target**

Reduce abuse of immortal/AFK targets without forcing unnatural active gameplay.

### 8.5 **Combat Activity Score**

Use only as a bounded signal to distinguish genuine activity from repetitive training loops.

### 8.6 **Shielding in real combat**

Reward relevant incoming pressure instead of trivial AFK tanking.

### 8.7 **Offline training**

Keep as a slower convenience path.

### 8.8 **Exercise weapons**

Keep as a faster resource/gold-sink training path.

### 8.9 **Offline Exercise Sessions — owner clarification**

Oteryn Evolved should allow deterministic exercise progression while the character is offline:

- same per-character training rate;
- same per-charge effectiveness;
- same eligibility;
- same cost for the same result;
- one active session per character;
- **multiple characters on the same account may train concurrently when each consumes its own legitimate resources**.

This concurrency is intentional. It improves QoL, removes pointless always-online clients and may strengthen the exercise-weapon gold sink.

### 8.10 **Remembered/favorite offline training choice**

Persist the player's last/default training selection where character-specific.

### 8.11 **Classic skills vs Weapon Proficiency**

Do not invent a duplicate generic Weapon Mastery system. Natural hunts may advance classic skills and official-style Weapon Proficiency independently under their own rules.

### 8.12 **Skill Wheel safe-zone editing**

Reference can retain target Tibia semantics. Evolved may use an explicit safe-zone policy rather than hard-coding one permanent Temple rule.

---

## 9. **Quest, Adventure, Account-World Unlocks & Legacy Accessibility**

**Priority: A**  
**Profile: Evolved.**

Goal: preserve meaningful questing while removing obsolete repeated barriers and making long quest chains understandable.

### 9.1 **Quest Journal 2.0**

Searchable hierarchy:

`Campaign -> Chain -> Quest -> Mission -> Objective`

with dependency graph.

### 9.2 **Current Objective / blocker explanation**

Explain missing prerequisites, access, items, NPC interactions or party-stage incompatibility.

### 9.3 **Party Quest Sync**

Show party stage compatibility without granting skipped progress.

### 9.4 **Account-World Unlocks**

Refine the historical broad `account-wide quest progression` proposal:

- selected convenience/story access can be shared by characters on the same account/world;
- do **not** blindly copy every character quest flag;
- keep character-specific choices/progression where meaningful.

### 9.5 **One-time reward semantics**

A shared access/completion unlock may still grant iconic/valuable quest rewards only once per account/world when that is the intended economy rule.

### 9.6 **Legacy Quest Accessibility**

For selected outdated mandatory-team gates on low-population content:

- curated Story Mode for legacy narrative encounters;
- quest progression/access may remain;
- no rare farming/Bosstiary exploitation;
- economy/access consequences must be reviewed explicitly.

Do not automatically add solo mode to current endgame group content.

### 9.7 **Puzzle substitute actors / Mercenary Illusions**

For old mechanics requiring four vocation tiles, temporary non-combat puzzle actors may stand in for missing roles. Avoid turning this into permanent combat companion AI.

### 9.8 **Quest Renown / Adventure Points**

Long-term completion/exploration progression focused on identity, convenience, titles, cosmetics and services.

### 9.9 **Region Mastery**

Connect related regional quest lines to durable exploration/progression rewards.

### 9.10 **Legacy reward modernization**

Preserve iconic physical rewards while adding carefully bounded modern value where old fixed rewards no longer match effort.

### 9.11 **Postman modernization pilot**

Use a legacy chain to unlock meaningful postal rank/service privileges rather than requiring repeated low-value completion on every alt forever.

### 9.12 **Linked Tasks**

Retain chained/repeatable task progression as part of quest/task architecture, not as a disconnected parallel journal.

### 9.13 **Rejected: character vampirism / deleting extra alts**

The forum proposal where a main character absorbs XP from characters beyond a five-character limit and those characters cease to exist is **REJECTED**. It does not solve the underlying repeated-quest problem safely and creates destructive/exploitable progression semantics.

---

## 10. **Bestiary, Charms, Collections, Echo & Prey**

**Priority: A/S**  
**Profile: Evolved built from verified official systems.**

Goal: unify collection/progress UX and provide bounded mastery without unlimited linear power stacking.

### 10.1 **Collection Framework**

Shared browser primitives for:

- Bestiary;
- Bosstiary;
- Echo Warden progress;
- Item Deck/Codex;
- compatible future collections.

Capabilities: search, sort, filters, grid/list/dense, incomplete-only, nearest-completion, favorites and tracker presets.

### 10.2 **Incomplete Bestiary indicators**

Use the same collection state; do not maintain duplicate progression models.

### 10.3 **Tracker capacity redesign**

Separate large/unlimited saved interests from the much smaller active HUD display set.

### 10.4 **Echo Warden progress**

Per-creature completion plus global summary and incomplete/completed filters.

### 10.5 **Item Deck / Item Codex**

Account/character scope must be an explicit product choice. Keep discovered/obtained separate from stronger provenance such as earned/looted/crafted if needed.

### 10.6 **Staged Charm Point rewards**

Distribute the unchanged total across meaningful Bestiary milestones while keeping full completion important.

### 10.7 **Knowledge-gated effective Charm levels**

Charm effectiveness against a creature may depend on verified Bestiary knowledge of that creature.

### 10.8 **Charm Mastery / Grandmaster**

Prefer bounded qualitative mastery and specialization over unlimited percentage growth.

### 10.9 **Creature Family Mastery**

Full family knowledge can unlock stable family-level mastery/eligibility.

### 10.10 **Persistent Charm assignments/loadouts**

Remember hunting-ground configurations while preserving simultaneous-assignment limits.

### 10.11 **Drome Charm integration**

Existing Drome rewards may amplify/master systems but must not bypass Bestiary gates.

### 10.12 **Prey active-use timer**

Evaluate consuming Prey time only through meaningful server-authoritative selected-race activity.

### 10.13 **Dormant target reservation**

Expired active time can leave the creature reserved without providing a bonus.

### 10.14 **Separate target, bonus and renewal states**

Do not overload one lock mechanism with several unrelated economic responsibilities.

### 10.15 **Bankable free Prey Charges**

Bounded stored free charges may replace expiring reroll opportunity.

### 10.16 **Sustainable Prey maintenance**

Use transparent free/gold/gameplay-earned/optional premium paths rather than one mandatory recurring Store-linked path.

### 10.17 **Mixed-spawn / party-safe consumption**

Consume only from legitimate selected-race activity; party member slots stay independent; valid summon participation attributes to owner.

### 10.18 **Prey loadouts and spend controls**

Remember preferences and caps without giving client authority over availability/state.

### 10.19 **Prey migration/switching safeguards**

Preserve existing value with atomic spend, combat-safe switching and reconnect-safe state.

---

## 11. **Boss, Encounter & Group PvE System 2.0**

**Priority: A**  
**Profile: Evolved.**

Goal: make bossing repeatable, discoverable and fair without flattening challenge.

### 11.1 **Boss System 2.0**

One owning encounter framework for difficulty, access, loot and party semantics.

### 11.2 **Adventure Guild Boss Hub**

Earned convenience hub for discovered/unlocked content rather than an immediate universal teleport menu.

### 11.3 **Flexible boss party size**

Controlled encounter scaling where appropriate; do not imply every boss must support every group size.

### 11.4 **Boss difficulty tiers**

Practice/normal/scalable modes only where encounter design supports clear reward separation.

### 11.5 **Personal loot**

Per-player resolution may reduce party friction while preserving encounter economy budgets.

### 11.6 **Bad-luck protection / pity**

Bound extreme bad luck after current official mechanics are reverified.

### 11.7 **Boss Essence / guaranteed progression**

No-drop sessions may produce bounded fragments/currency toward long-term goals.

### 11.8 **Crash-aware cooldown compensation**

Only proven infrastructure failure should restore/compensate boss access automatically; avoid user-triggerable crash exploits.

---

## 12. **Death, Recovery & Connection Resilience**

**Priority: A**  
**Profile: Evolved.**

Goal: keep death frightening without making high-level progression unsustainably punitive.

### 12.1 **Death System 2.0**

Move away from blindly scaling loss as a percentage of enormous lifetime XP.

### 12.2 **Level/progression-relative permanent loss**

Exact values remain balance work.

### 12.3 **Blessings remain meaningful**

May reduce XP/skill/item-loss risk and possibly temporary fatigue.

### 12.4 **Recovery Pool**

A bounded portion of loss may be recovered through continued active play.

### 12.5 **Consecutive-death protection**

Prevent catastrophic spirals without creating intentional death exploits.

### 12.6 **Death Fatigue / temporary weakness**

Shift part of the cost from permanent progression loss to bounded temporary damage/healing/defense penalties.

### 12.7 **Context-separated death rules**

PvE, boss/instance, PvP and proven server failure may require different loss/compensation policies.

### 12.8 **Connection Loss Protection**

Do not pretend a server can perfectly classify one disconnect as genuine/intentional. Use explicit logout vs abrupt loss, combat state, telemetry, reconnect and historical patterns.

### 12.9 **Fast reconnect**

Restore control safely without duplicating character ownership/state.

### 12.10 **Server-failure compensation**

Proven infrastructure failures may justify strong compensation; client-reported failures alone are not proof.

---

## 13. **PvP, Arenas, Wars & High-Risk Zones**

**Priority: B/A**  
**Profile: Evolved.**

Goal: increase skill expression and structured competition while reducing grief-driven frustration and abuseable reward loops.

### 13.1 **PvP System 2.0**

Separate rules for open world, arena, guild war and events where needed.

### 13.2 **Rating / prestige / seasons**

Competitive progression should emphasize rank, cosmetics, titles, achievements and controlled currencies rather than victim-value extraction.

### 13.3 **Objective-based PvP rewards**

Reward ranked wins, war objectives, Castle objectives and tournament outcomes more than raw repeated kills.

### 13.4 **Anti-farming diminishing returns**

Repeated kills of the same victim should lose reward value.

### 13.5 **High-Risk PvP zones**

Stronger loss/loot rules belong in explicit opt-in spaces rather than everywhere.

### 13.6 **Castle / Battleground**

Structured objective PvP/event system.

### 13.7 **Prestige Arena**

Ranked custom arena direction.

### 13.8 **PvP death/recovery integration**

Use Death System 2.0 rather than maintaining contradictory loss models.

---

## 14. **Tasks, Bounty, Weekly Systems & Dynamic Spawn**

**Priority: A**  
**Profile: Evolved built from verified Tibia task foundations.**

Goal: make task generation/progression appropriate to player power and server capacity while preventing market shocks.

### 14.1 **Bounty & Weekly Tasks Rework**

Preserve official-system identity but improve suitability, objectives and long-term integration.

### 14.2 **Task Suitability**

Use progression, expected effort, spawn reality and access rather than only broad difficulty buckets.

### 14.3 **Dynamic kill-count/reward scaling**

Exact formulas remain telemetry/simulation dependent.

### 14.4 **Dedicated Bounty equipment slot**

Keep Bounty-specific equipment semantics scoped instead of creating a second universal ring slot.

### 14.5 **Bounty Talisman redesign**

Evaluate bounded Combat/Sustain/Spoils/Knowledge dimensions while reusing verified official foundations.

### 14.6 **Bounty Spawn Allowance**

Legitimate overleveled task completion may temporarily relax custom dynamic-capacity eligibility without granting permanent farming throughput.

### 14.7 **Weekly-task-aware Stash/Market**

Active known objectives can expose required/owned/missing quantities and deep-link to relevant market views.

---

## 15. **Vocation Identity, Combat Rules & Balance Framework**

**Priority: A**  
**Profile: Evolved.**

Goal: make classes distinct and viable without one scalar power score dominating all contexts.

### 15.1 **Independent auto attack and spell casting**

Spell use should not silently cancel/reset/skip the normal weapon auto-attack schedule unless a deliberate combat rule says so.

### 15.2 **Vocation/Class Identity Framework**

Explicit strengths, weaknesses, solo expectations, party value and build boundaries.

### 15.3 **Shared role taxonomy**

Reusable role tags for Party Finder, Party System, markers and contribution analysis without forcing a rigid trinity.

### 15.4 **Balance target bands & telemetry**

Measure damage, healing, defense, sustain, utility and mobility by role/context.

### 15.5 **Solo + party viability principle**

Every supported class should retain meaningful solo viability while preserving differentiated party value.

### 15.6 **Context-separated PvE / Boss / PvP tuning**

Do not let one context automatically dictate damaging changes to the others.

### 15.7 **Threat-based monster collision**

Potentially reduce obsolete body-block frustration based on threat/relative power while preserving meaningful danger.

### 15.8 **Break Free / Emergency Breakthrough**

Limited active escape for genuine dangerous surrounds; requires careful PvP/PvE abuse review.

### 15.9 **Transition Safety / Guaranteed Escape Path**

Improve holes/ladders/stairs/teleports and forced landing states without making transitions danger-free.

### 15.10 **Controlled Retaliation / reflect safety**

Prefer proc/internal-cooldown/output-cap behavior over unrestricted percentage reflection that scales dangerously with attacker count.

---

## 16. **Housing, Lifestyle, Fishing, Cooking & Personal Spaces**

**Priority: B/A**  
**Profile: Evolved.**

Goal: create meaningful non-combat identity and optional sinks without turning lifestyle systems into mandatory combat power.

### 16.1 **Fishing System 2.0**

Mastery, biomes, hotspots, bait and active interaction.

### 16.2 **Fishing Codex / records / tournaments**

Collection/social competition layer.

### 16.3 **Cooking 2.0**

Preparation/build-support crafting without making every combat session require excessive buff maintenance.

### 16.4 **House Chef**

Ingredient/service/recipe loop using verified housing/hireling foundations where applicable.

### 16.5 **House upgrades, services and luxury sinks**

Optional convenience/prestige sinks rather than punitive basic costs.

### 16.6 **Personal Hideouts / instanced basic housing**

Forum-derived addition:

- scalable supply of personal accommodation;
- decoration/private social use;
- no need to compete for a scarce city tile;
- city houses remain scarce/prestigious/showcase locations;
- hideouts should not automatically inherit every strategic/commercial advantage of prime physical houses.

### 16.7 **Mount utility — caution**

If mounts gain mechanics, prefer bounded utility/travel archetypes before direct combat BIS stats. Avoid store-exclusive power.

---

## 17. **Economy Health, Gold Sinks & Server-Wide Economic Control**

**Priority: S architecture / A feature delivery**  
**Profile: Evolved.**

Goal: control inflation with desirable sinks and deterministic safety rather than arbitrary player friction.

### 17.1 **Economy Sink Framework**

Coordinate sinks across exercise training, housing, imbuements, progression, services and luxury features.

### 17.2 **Exercise training as a gold sink**

Offline Exercise multi-character concurrency is acceptable when every character legitimately consumes its own resources.

### 17.3 **Weekly Delivery Economy Controller**

Use server supply/liquidity and bounded per-item demand budgets so generated tasks do not create uncontrolled scarcity shocks.

### 17.4 **Weekly Delivery Price Shock Guard**

Reduce/stop new generated demand when item supply/prices destabilize.

### 17.5 **AI-assisted economy forecasting, deterministic authority**

AI may forecast supply/liquidity/price impact; hard caps remain deterministic and authoritative.

### 17.6 **Imbuement maintenance sinks**

Recurring recharge/renewal can remove resources while improving flexibility.

### 17.7 **Controlled duplicate-item sinks**

Use carefully so valuable item identity is not destroyed by mandatory mass consumption.

### 17.8 **Prey maintenance sinks**

Transparent free/gold/gameplay-earned/premium choices rather than one opaque mandatory payment loop.

---

## 18. **Security, Anti-Bot, Anti-Abuse & Behavioral Telemetry**

**Priority: S strategic**  
**Profile: platform-wide.**

Goal: defend the economy and gameplay against modern external automation without treating AI output as automatic guilt.

### 18.1 **Disconnect Abuse Detection**

Long-term pattern analysis rather than pretending one disconnect can be classified perfectly.

### 18.2 **AI Anti-Bot / Anti-Cheat Platform**

Layer:

- client integrity signals;
- server behavioral telemetry;
- statistical/AI behavior analysis;
- cross-account graph analysis.

### 18.3 **Modern bot threat model**

Include classic macros, cavebots, pixel/computer-vision automation and external-device visual agents.

### 18.4 **Risk-score enforcement principle**

Do **not** use `AI says bot -> automatic ban` as the primary model.

Prefer:

`signals + evidence + correlation + risk score + challenge/review/escalation`.

### 18.5 **Cross-system abuse detection**

The same platform may surface:

- kill farming;
- win trading;
- suspicious economy transfers;
- exploit behavior;
- multibox abuse;
- disconnect abuse.

---

## 19. **Account Tenure, Premium & Commercial-Progression Policy**

**Priority: C/B policy**  
**Profile: Evolved/Platform decision; no monetization implementation authority.**

Goal: prevent convenience monetization from accidentally becoming immediate purchasable progression power.

### 19.1 **Separate prepaid entitlement from elapsed tenure**

```text
prepaid premium balance
!=
elapsed account/premium tenure
!=
progression power
```

### 19.2 **Rejected: immediate Loyalty for future prepaid Premium**

Do not instantly grant years of progression-relevant Loyalty for future time that has merely been purchased. It converts tenure into immediate buyable power and creates refund/chargeback/revocation debt.

### 19.3 **Safer tenure rewards**

Prefer cosmetics, prestige, account convenience and identity before combat/progression power.

---

# 5. Canonical programme priority summary

| # | Programme | Priority | Product stance |
|---:|---|:---:|---|
| **1** | **Client UX, Input, Workspace & Active Information** | **S** | Build early |
| **2** | **World Map, Discovery, Huntfinding & Capacity** | **A/S** | Strong differentiator |
| **3** | **Party, Social, Group Finder & Hunt Coordination** | **S** | Strong differentiator |
| **4** | **Inventory, Loot, Depot, Stash & Logistics** | **S** | Build early |
| **5** | **Bank, Trade, Market, Postal & Financial UX** | **S/A** | Architecture early |
| **6** | **Build, Loadouts, Itemization, Forge & Proficiency** | **S** | Architecture early |
| **7** | **Imbuement & Elemental Attunement 2.0** | **A** | Later domain package |
| **8** | **Skill, Training, Weapon Proficiency & Wheel** | **S** | Architecture early |
| **9** | **Quest, Adventure, Account-World Unlocks & Accessibility** | **A** | Strong retention value |
| **10** | **Bestiary, Charms, Collections, Echo & Prey** | **A/S** | Major progression/UX system |
| **11** | **Boss & Encounter System 2.0** | **A** | Major gameplay system |
| **12** | **Death, Recovery & Connection Resilience** | **A** | Balance-heavy but important |
| **13** | **PvP, Arenas, Wars & High-Risk Zones** | **B/A** | Later structured gameplay |
| **14** | **Tasks, Bounty, Weekly & Dynamic Spawn** | **A** | Economy/capacity aware |
| **15** | **Vocation Identity, Combat Rules & Balance** | **A** | Continuous balance programme |
| **16** | **Housing, Lifestyle, Fishing, Cooking & Personal Spaces** | **B/A** | Expansion/lifestyle |
| **17** | **Economy Health, Gold Sinks & Economic Control** | **S/A** | Architecture early |
| **18** | **Security, Anti-Bot & Anti-Abuse** | **S** | Strategic platform capability |
| **19** | **Account Tenure, Premium & Commercial Policy** | **C/B** | Policy first; no instant Loyalty power |

---

# 6. Source de-duplication — 128 historical classified proposals

Every numbered proposal from the historical classification remains represented below. The canonical programme is the owning destination, not an assertion that implementation is authorized.

| Legacy # | Proposal | Canonical programme |
|---:|---|---|
| 1 | Huntfinder 2.0 | **2** |
| 2 | Hunting Spot Availability | **2** |
| 3 | Party Finder 2.0 | **3** |
| 4 | Account-wide quest progression | **9** |
| 5 | Independent auto attack and spell casting | **15** |
| 6 | Character markers on minimap | **2 / 3** |
| 7 | Loot System Rework | **4** |
| 8 | Summon Loot Assistant | **4** |
| 9 | Bank UI 2.0 | **5** |
| 10 | Transaction History | **5** |
| 11 | Trade System 2.0 | **5** |
| 12 | Death System 2.0 | **12** |
| 13 | Recovery Pool | **12** |
| 14 | Death Fatigue / Post-Death Weakness | **12** |
| 15 | Connection Loss Protection | **12** |
| 16 | Disconnect Abuse Detection | **18** |
| 17 | AI Anti-Bot / Anti-Cheat Platform | **18** |
| 18 | PvP System 2.0 | **13** |
| 19 | PvP rating / prestige / seasons | **13** |
| 20 | High-Risk PvP Zones | **13** |
| 21 | Adaptive UI / Scalable Client | **1** |
| 22 | Layout Presets | **1** |
| 23 | Boss System 2.0 | **11** |
| 24 | Adventure Guild Boss Hub | **11** |
| 25 | Flexible Boss Party Size | **11** |
| 26 | Boss Difficulty Tiers | **11** |
| 27 | Personal Loot | **11** |
| 28 | Bad Luck Protection / Pity | **11** |
| 29 | Boss Essence / Guaranteed Progression | **11** |
| 30 | Threat-Based Monster Collision | **15** |
| 31 | Break Free / Emergency Breakthrough | **15** |
| 32 | Transition Safety | **15** |
| 33 | Guaranteed Escape Path | **15** |
| 34 | Fishing System 2.0 | **16** |
| 35 | Fishing Codex / Records / Tournaments | **16** |
| 36 | Cooking 2.0 | **16** |
| 37 | House Chef | **16** |
| 38 | Economy Sink Framework | **17** |
| 39 | House upgrades / services / luxury sinks | **16 / 17** |
| 40 | Equipment Durability | **6** |
| 41 | Repair All / Auto Repair | **6** |
| 42 | Skill Wheel changes outside temple | **8** |
| 43 | Skill Progression 2.0 | **8** |
| 44 | Real Combat Training | **8** |
| 45 | Threat coefficient for classic skill gain | **8** |
| 46 | Diminishing returns on one persistent training target | **8** |
| 47 | Combat Activity Score | **8** |
| 48 | Shielding progression in real combat | **8** |
| 49 | Offline training as slower convenience path | **8** |
| 50 | Exercise weapons as faster gold-sink path | **8 / 17** |
| 51 | Classic skills and Weapon Proficiency remain separate | **8** |
| 52 | Natural hunts advance classic skills and Weapon Proficiency independently | **8** |
| 53 | Equipment Presets | **6** |
| 54 | Linked Tasks | **9 / 14** |
| 55 | Better Map UX | **2** |
| 56 | Castle / Battleground | **13** |
| 57 | Prestige Arena | **13** |
| 58 | Crash-aware boss cooldown compensation / Obelisk concept | **11 / 12** |
| 59 | Dynamic Spawn and Hunting Capacity | **2 / 14** |
| 60 | Bounty and Weekly Tasks Rework | **14** |
| 61 | Task Suitability and progression-aware Bounty offers | **14** |
| 62 | Dynamic Bounty/Weekly kill-count and reward scaling | **14** |
| 63 | Dedicated Bounty Equipment Slot | **14 / 6** |
| 64 | Bounty Talisman Combat/Sustain/Spoils/Knowledge redesign | **14 / 6** |
| 65 | Weekly Delivery Economy Controller | **17 / 14** |
| 66 | Weekly Delivery Price Shock Guard | **17** |
| 67 | AI-assisted economy forecasting with deterministic caps | **17** |
| 68 | Bounty Spawn Allowance for overleveled task players | **14** |
| 69 | World Map & Discovery System 2.0 | **2** |
| 70 | Live Social Map & Permissioned Position Sharing | **2 / 3** |
| 71 | Shared Discovery Markers | **2** |
| 72 | Friends/VIP Mutual Consent & Privacy 2.0 | **3** |
| 73 | Party System 2.0 | **3** |
| 74 | Party Combat Visibility | **3** |
| 75 | Shared Hunt Accounting / Party Loot Ledger | **3 / 5** |
| 76 | Itemization & Build System 2.0 | **6** |
| 77 | Build Impact & Comparison System | **6** |
| 78 | Training Arena / Combat Simulation | **6 / 15** |
| 79 | Inventory / Depot / Stash 2.0 | **4** |
| 80 | Smart Item Reservation & Task Protection | **4** |
| 81 | Market sale-only commission | **5 / 17** |
| 82 | Multi-Currency Market & Trade | **5** |
| 83 | Imbuement System 2.0 | **7** |
| 84 | Slot-Based Imbuement Library & Active Channels | **7** |
| 85 | Elemental Attunement Layer | **7** |
| 86 | Imbuement Slot / Attunement Progression | **7** |
| 87 | Imbuement Progression & Maintenance Sink | **7 / 17** |
| 88 | Effective-Use Imbuement Lifecycle | **7** |
| 89 | Vocation/Class Identity & Role Framework | **15** |
| 90 | Shared Role Taxonomy & Party Contribution Model | **15 / 3** |
| 91 | Vocation Balance Target Bands & Telemetry | **15** |
| 92 | Solo and Party Viability Balance Principle | **15** |
| 93 | Context-Separated PvE/Boss/PvP Balance | **15** |
| 94 | Gem Atelier & Fragment Workshop parity/balance review | **6** |
| 95 | Bestiary staged Charm Point rewards | **10** |
| 96 | Bestiary knowledge-gated effective Charm levels | **10** |
| 97 | Charm Level 3 Mastery and Level 4 Grandmaster | **10** |
| 98 | Creature Family Mastery | **10** |
| 99 | Persistent Charm assignments and loadouts | **10** |
| 100 | Drome Charm amplifiers and optional Grandmaster catalysts | **10** |
| 101 | Quest Journal 2.0 and quest dependency graph | **9** |
| 102 | Current Objective and quest blocker explanation | **9** |
| 103 | Party Quest Sync | **9 / 3** |
| 104 | Quest Renown / Adventure Points | **9** |
| 105 | Region Mastery | **9** |
| 106 | Legacy quest reward modernization and bounded XP scaling | **9** |
| 107 | Postman Quest modernization pilot | **9 / 5** |
| 108 | Postal Network 2.0 | **5 / 9** |
| 109 | Global Market plus Local Logistics | **5** |
| 110 | Consolidated market delivery | **5** |
| 111 | Forge Slot Mastery | **6** |
| 112 | Item Enhancement +N | **6** |
| 113 | Equipment Proficiency | **6** |
| 114 | Item Classification as progression ceiling | **6** |
| 115 | Equipment proficiency branches and qualitative capstones | **6** |
| 116 | Controlled Enhancement RNG and pity | **6** |
| 117 | Controlled duplicate-item sink | **6 / 17** |
| 118 | Enhancement Salvage and equipment-replacement protection | **6** |
| 119 | Existing tier migration contract | **6** |
| 120 | Controlled Retaliation / reflect safety | **15 / 6** |
| 121 | Selected-creature active-use Prey timer | **10** |
| 122 | Dormant Prey target reservation | **10** |
| 123 | Separate target reservation, bonus preservation and time renewal | **10** |
| 124 | Bankable free Prey Charges | **10** |
| 125 | Sustainable Prey maintenance paths | **10 / 17** |
| 126 | Mixed-spawn and party-safe Prey consumption | **10** |
| 127 | Prey loadouts and cost controls | **10** |
| 128 | Prey migration and switching safeguards | **10** |

---

# 7. Forum / conversation source requests — canonical destinations

The forum posts are treated as **player-problem evidence**, not automatically correct specifications. Several independently validate directions already present in the historical roadmap.

## A-series — earlier QoL forum requests

| ID | Request | Destination |
|---|---|---|
| A01 | Market/Tibia Coin anti-bot cooldown | **5 / 17** — evidence-driven throttle, not blanket 5 min |
| A02 | Incomplete Bestiary indicator | **10** |
| A03 | Collect All exclusions | **4** |
| A04 | Remember Offline Training choice | **8** |
| A05 | Exercise weapons while offline | **8 / 17** |
| A06 | More outfit colours | **1** |
| A07 | Random outfit colours | **1** |
| A08 | Party map/GPS | **2 / 3** |
| A09 | Full equipment-set hotkeys | **6** |
| A10 | Separate Event Chat | **1 / 3** — typed routing |
| A11 | Auto-drop empty flasks | **4** — keep/move/discard preferred |
| A12 | Larger potion/rune stacks | **4** |
| A13 | Sell directly from Stash | **4 / 5** |
| A14 | Mount attributes | **16** — utility-first caution |
| A15 | Bank `k/kk` shorthand | **5** |
| A16 | Item Deck / Codex | **10** |
| A17 | Level in VIP/contact list | **3** |
| A18 | Guild in chat/VIP/contact surfaces | **3** |

## B-series — later QoL forum requests

| ID | Request | Destination |
|---|---|---|
| B01 | In-game UI scaling | **1** |
| B02 | Larger/resizable minimap | **1 / 2** |
| B03 | Party members on minimap | **2 / 3** |
| B04 | Mouse wheel hotkeys | **1** |
| B05 | Protection Set Management | **6** |
| B06 | Skill Wheel in broader PZ/safe zones | **8** |
| B07 | More VIP groups | **3** — tags preferred |
| B08 | Persistent/pinned read-only tabs | **1** |
| B09 | Weekly-task Stash filters | **4 / 14** |
| B10 | Weekly-task Market filters | **5 / 14** |
| B11 | Remove/clear ring/amulet enchant | **4 / 6** — generic normalization preferred |
| B12 | In-game party loot split | **3 / 5** |
| B13 | Food duration display | **1** — Active Effects |
| B14 | Individual food/potion buff icons | **1** — Active Effects |
| B15 | NPC food/supply cost in party hunt | **3 / 5** |
| B16 | Echo Warden completion counter | **10** |
| B17 | Larger Bestiary/Bosstiary Tracker | **10** |
| B18 | Better Bestiary UI/sort/filter/layout | **10** |
| B19 | Immediate Loyalty from prepaid Premium | **19 — REJECT** |

## C-series — Equipment / input forum request

| ID | Request | Destination |
|---|---|---|
| C01 | Equipment Presets | **6** — same Equipment Loadouts system |
| C02 | Diagonal movement from simultaneous WASD | **1** |

## D-series — latest forum batch

| ID | Request | Destination |
|---|---|---|
| D01 | Decouple Stash and Market; retain filters/views | **1 / 4 / 5** |
| D02 | Anonymous Market offers by default | **5** — product preference/configuration |
| D03 | Highlight own Market offers | **5** |
| D04 | Sort My Offers | **5** |
| D05 | NPC Sell: hide items not carried | **4 / 5** |
| D06 | Auto-label purchased parcel | **5** — modern addressing may supersede literal label |
| D07 | Custom Timers & Reminders | **1** |
| D08 | Party Finder lobbies/scheduling/tactical map | **3** |
| D09 | Legacy Story Mode solo encounters | **9** — curated only |
| D10 | Puzzle/vocation companion illusions | **9** — temporary puzzle actors |
| D11 | Exact Market quantity input | **5** |
| D12 | Protect Stash item | **4** |
| D13 | Protect/reserve only N quantity | **4** |
| D14 | Two-ring quick switch | **6** — thin Loadout toggle |
| D15 | Personal Hideouts / basic instanced housing | **16** |
| D16 | Account/world shared legacy quest access | **9** |
| D17 | Shared quest unlock with one-time rewards | **9** |
| D18 | Main character absorbs >5 alts and deletes them | **9 — REJECT** |

---

# 8. Research-only candidates retained from the historical classification

These remain **R — research only** unless separately promoted.

1. **Faction & Reputation System 2.0** — potentially useful world/quest progression; requires explicit approval.
2. **Professions 2.0** — avoid mandatory vertical stat stacking.
3. **Guild Progression 2.0** — prefer utility/prestige over mandatory combat power.
4. **Adventurer Caravan / Expedition System** — custom exploration/event candidate.
5. **Region Discovery + Quest Chains** — candidate that overlaps programmes **2** and **9**.
6. **Item rarity / random affixes** — caution; may undermine Tibia item identity and current itemization direction.
7. **Awakening / Prestige reset** — **not recommended** for Oteryn's unlimited long-term progression philosophy unless that philosophy changes explicitly.

---

# 9. Critical owner decisions preserved by this consolidation

1. **Oteryn Evolved is the target for mechanical redesign.** Tibia Reference stays parity-first unless a shared-UX feature is explicitly proven not to alter semantics.
2. **Offline Exercise may run on multiple characters of one account simultaneously.** Each character must legitimately consume its own resources; this is accepted as convenience and a potentially strong gold sink.
3. **Equipment Presets, Protection Sets, full-set hotkeys and two-ring switching are one Equipment Loadouts system.**
4. **Food timers and separate buff icons are one Active Effects system.**
5. **Bestiary UI, Echo progress, Item Deck and trackers share one Collection Framework.**
6. **Party hunt expense tracking and loot split share one Hunt Accounting & Settlement system.**
7. **Stash protection is quantity-aware reservation, not only a favorite icon.**
8. **Market anti-bot behavior should use evidence-driven throttling rather than a blanket fixed delay that annoys normal players.**
9. **Legacy Quest Accessibility is selective.** It must not silently convert current endgame group content into solo content.
10. **Personal Hideouts complement scarce prestige housing; they do not replace it.**
11. **Immediate progression-relevant Loyalty for prepaid future Premium is rejected.**
12. **Character-vampirism/deletion for extra alts is rejected.**
13. **AI assists detection/forecasting; deterministic game/economy authority remains explicit.**

---

# 10. Architecture timing guidance

## Decide/shape early

These have high migration cost if ignored until late:

- **Client workspace/input/settings primitives**;
- **typed item/container/value transactions** for Stash sale, reservations and loadouts;
- **economy ledger/idempotency** for bank, trade, party settlement and delivery;
- **party/social privacy and exact-location policy**;
- **typed character progression** for skills, collections, quests and build state;
- **effect/condition presentation contract**;
- **account-vs-character-vs-world scope vocabulary**;
- **telemetry/audit foundations** for economy/security/balance.

## Can remain product candidates until owning systems exist

- Fishing/Cooking depth;
- Castle/Prestige Arena;
- mount utility;
- Equipment Durability;
- deeper Forge +N/proficiency tuning;
- Story Mode encounter list;
- faction/profession/guild progression research candidates.

---

# 11. Implementation authority boundary

This document is a **product roadmap and de-duplication authority only**. It does not itself authorize runtime/schema/protocol/economy/production changes.

Before implementation of any numbered programme or feature:

1. refresh protected `main` and applicable repository governance;
2. locate the owning accepted contract/architecture and live allocation;
3. verify current Tibia Reference behavior when parity is relevant;
4. define Reference-vs-Evolved profile semantics explicitly;
5. define durable authority, failure, idempotency and abuse constraints;
6. define concrete acceptance tests and required E2E evidence;
7. allocate bounded implementation work through the live coordinator programme.

`IMPLEMENTATION_AUTHORITY: NONE`

`PRODUCTION_AUTHORITY: NONE`
