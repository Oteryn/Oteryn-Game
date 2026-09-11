# Oteryn Evolved QoL Feature Clusters

- Date: 2026-09-11
- Status: **OWNER-REQUESTED PRODUCT DESIGN INVENTORY / PROPOSED / NOT IMPLEMENTATION AUTHORITY**
- Repository snapshot used for grouping: `Oteryn/Oteryn-Game@90a3f92434e32354ff1aaeac96d038bbc49eba9c`
- Applies to: future Oteryn Evolved product/UX planning and Reference-vs-Evolved separation
- Implementation authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This document consolidates three owner-provided Tibia/Oteryn QoL wishlists into a smaller set of coherent product initiatives. Several source ideas overlap and should be implemented as one reusable system rather than as isolated features.

The grouping is deliberately product-oriented:

- preserve the existing Reference-first architecture and do not silently change Reference behavior;
- prefer Evolved reliability/UX improvements that reduce friction without unnecessary power inflation;
- use reusable primitives instead of one-off exceptions;
- keep gameplay authority on the server and keep the native UI as a presentation/intent surface;
- treat economy, progression, PvP/privacy and monetization changes as explicit product decisions rather than hiding them inside a QoL patch.

The scores below are planning aids only:

- **Value**: expected player/product value, 1-10;
- **Complexity**: relative implementation/design cost, 1-10;
- **Adoption sense**: whether Oteryn should intentionally plan for the feature, 1-10;
- **Priority**: `S` highest, then `A`, `B`, `C`.

## 2. Source item map

### Wishlist A

| ID | Original proposal |
|---|---|
| A01 | Tibia Coin / market cooldown after placing or cancelling offers |
| A02 | Incomplete Bestiary indicator |
| A03 | Skip selected rewards with Collect All |
| A04 | Favorite / remembered Offline Training setup |
| A05 | Exercise / Training Wands usable while the character is offline |
| A06 | More outfit colors |
| A07 | Random outfit colors |
| A08 | Party member location on map/GPS |
| A09 | Full equipment-set hotkeys |
| A10 | Separate event chat |
| A11 | Auto-drop empty potion containers |
| A12 | Larger potion/rune stacks |
| A13 | Sell directly from Stash |
| A14 | Different mount attributes |
| A15 | Bank shorthand such as `2k`, `1.5kk` |
| A16 | Item Deck / item collection catalogue |
| A17 | Character level in VIP/contact list |
| A18 | Guild in chat/VIP/contact surfaces |

### Wishlist B

| ID | Original proposal |
|---|---|
| B01 | In-game UI scaling |
| B02 | Larger minimap |
| B03 | Party members on minimap |
| B04 | Mouse-wheel hotkeys |
| B05 | Protection-set management |
| B06 | Wheel of Destiny changes anywhere in PZ |
| B07 | More VIP groups |
| B08 | Persistent/pinnable read-only channel tabs |
| B09 | Better Stash filters for Weekly Tasks |
| B10 | Better Market filters for Weekly Tasks |
| B11 | Remove/clear ring or amulet enchants |
| B12 | In-game party loot split |
| B13 | Food-duration display |
| B14 | Individual food/potion buff icons |
| B15 | Track NPC food costs during party hunts |
| B16 | Echo Warden / Echo boss completion counter |
| B17 | Higher Bestiary/Bosstiary Tracker limit |
| B18 | Better Bestiary sorting/filtering/layout |
| B19 | Count prepaid future Premium time immediately toward Loyalty |

### Wishlist C

| ID | Original proposal |
|---|---|
| C01 | Equipment Presets triggered from the action bar/hotkeys |
| C02 | Diagonal movement from simultaneous WASD pairs such as `W+A`, `W+D`, `S+A`, `S+D` |

## 3. Canonical grouped initiatives

### QOL-01 — Native UI Workspace, Scaling and Input

**Source items:** `B01`, `B02`, `B04`, `B08`, `C02`

**Canonical feature family:**

- independent in-game UI scale;
- resizable/floating minimap rather than a fixed one- or two-column special case;
- bindable wheel-up / wheel-down semantic actions with proper UI-scroll precedence;
- persistent/pinnable chat/read-only tabs as part of the general dock/layout persistence model;
- optional two-key WASD diagonal movement without requiring dedicated diagonal keys.

**Recommended design:**

- UI scale remains independent from world-camera zoom;
- minimap is a normal resizable dockable panel;
- high-resolution/free-spin wheel input becomes discrete gameplay steps only after UI scroll has had a chance to consume it;
- chat/tab layout is versioned and restored safely across sessions/resolution changes;
- diagonal movement is resolved from held cardinal key state, not from a fragile millisecond timing macro:
  - `W + A -> northwest`;
  - `W + D -> northeast`;
  - `S + A -> southwest`;
  - `S + D -> southeast`;
  - opposite cardinal inputs cancel on their axis;
  - releasing one key transitions cleanly back to the remaining cardinal direction;
  - UI/text/modal ownership still suppresses gameplay movement input when appropriate;
- diagonal input changes only client control ergonomics: server movement legality, cadence, collision and movement cost remain authoritative and unchanged unless a separate gameplay decision says otherwise.

The current input foundation already supports bounded multi-input chords and explicit Gameplay/Text/Modal contexts, so the feature should use that foundation while avoiding accidental double cardinal + diagonal command emission.

**Score:** Value **10/10** · Complexity **4/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-02 — Party Presence, Navigation and Privacy

**Source items:** `A08`, `B03`

These are one feature: a party-location projection rendered on the minimap/map.

**Recommended design:**

- same Channel/Instance: precise party marker may be shown when authorized;
- different Channel: show coarse `different channel` presence rather than a false coordinate;
- different Instance: show coarse instance state without leaking exact placement;
- preserve the accepted privacy-first social-presence boundary;
- permit stricter PvP/ruleset policies and user-controlled sharing where appropriate.

Do not implement public or unconditional GPS.

**Score:** Value **9/10** · Complexity **6/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-03 — Equipment Loadouts and Build Preparation

**Source items:** `A09`, `B05`, `B06`, `C01`

`A09 Full Set Hotkeys`, `B05 Protection Set Management` and `C01 Equipment Presets` are the same system and should become generic **Equipment Loadouts** rather than three parallel features.

**Canonical feature family:**

- named loadouts such as Fire, Energy, Death, Physical, Damage, Speed or Boss;
- action-bar button and hotkey activation;
- one semantic `ActivateEquipmentLoadout` operation rather than a client macro issuing many equip commands;
- deterministic server validation of participating item instances, slots, destinations and restrictions;
- default all-or-nothing behavior unless a later product decision intentionally allows partial activation;
- normal equipment/combat restrictions remain authoritative;
- missing/unavailable items produce an explicit result instead of silently leaving a dangerous half-set.

`B06 Wheel in PZ` belongs to the same broader **build preparation** experience but remains a separate ruleset policy:

- Reference may retain its target restriction;
- Evolved may allow build editing in an accepted safe-zone class;
- do not hard-code Temple/PZ geography into generic engine semantics.

**Score:** Value **10/10** · Complexity **7/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-04 — Active Effects, Buffs and Timers

**Source items:** `B13`, `B14`

Food duration and separate food/potion icons should be one generic **Active Effects** surface backed by authoritative effect/condition state.

Provide presentation-safe effect identity, icon, magnitude/summary, optional source label, remaining duration and relevant stack/refresh state. The server owns effect lifetime and stacking semantics; the client may count down locally between authoritative updates.

Recommended UI:

- compact configurable HUD icons;
- precise timer/tooltip;
- full Effects panel for overflow;
- clear buff/debuff distinction;
- no artificial six-icon architectural ceiling.

**Score:** Value **10/10** · Complexity **5/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-05 — Collection Framework: Bestiary, Bosstiary, Echo and Item Codex

**Source items:** `A02`, `A16`, `B16`, `B17`, `B18`

These should become one reusable **Collection Framework** instead of five independent implementations.

**Canonical consumers:** Bestiary, Bosstiary, Echo progression, Item Deck/Item Codex and future compatible collections.

**Shared capabilities:** search, sort, filter, grid/list/dense views, completion state, current/remaining progress, favorites, tracker presets, `incomplete only` and `nearest completion` views.

Additional rules:

- incomplete Bestiary indicators consume the same collection state rather than another counter model;
- saved/tracked interests are separate from the smaller active HUD set;
- Item Codex uses stable item-definition identity;
- `discovered/obtained` may be distinct from stronger provenance such as `earned/looted/crafted`;
- avoid progression power merely for collecting junk; cosmetic/title/achievement rewards are safer first use.

**Score:** Value **10/10** · Complexity **7/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-06 — Inventory, Stash, Rewards and Item-State Convenience

**Source items:** `A03`, `A11`, `A12`, `A13`, `B09`, `B11`

This cluster removes repetitive inventory manipulation while preserving item/value authority.

**Reward collection (`A03`)**: support collect selected, collect all eligible, keep-in-chest markers and visible expiry warnings rather than hard-coded item exceptions.

**Empty potion containers (`A11`)**: prefer configurable keep / move to assigned container / discard. Do not make auto-drop-to-ground the canonical solution because it creates world-item spam and cleanup/network cost.

**Larger stacks (`A12`)**: use the accepted per-item-definition stack maximum. Reference and Evolved profiles may select different values; a first Evolved target around 1,000 for high-volume supplies is preferable to a universal 9,999 without evidence.

**Sell directly from Stash (`A13`)**: make Stash-to-NPC sale an authoritative atomic value transaction. Add favorites/locks/keep-minimum protection before broad `Sell All Eligible` behavior.

**Task-aware Stash (`B09`)**: generalize to Objective Relevance using active known requirements and owned-vs-required quantities without unrevealed quest spoilers.

**Ring/amulet normalization (`B11`)**: prefer a generic `Normalize/Discharge Item` operation or later Equipment Vault. Do not refund unused temporary value unless an owning economy decision explicitly permits it.

**Score:** Value **9/10** · Complexity **7/10** · Adoption sense **9/10** · Priority **A/S by slice**.

---

### QOL-07 — Market, Bank and Task-Aware Economy UX

**Source items:** `A01`, `A15`, `B10`

**Market anti-abuse (`A01`)**: do not freeze a blanket five-minute cooldown. Prefer bounded per-account/per-instrument rate limits, rolling cancel/reprice limits, meaningful fees where appropriate and telemetry before tuning. Normal players should almost never encounter the throttle.

**Amount shorthand (`A15`)**: create one exact-integer `AmountExpression` parser reusable across bank/market/NPC/trade. Support safe forms such as `2k`, `200k`, `1.5kk`; never use binary floating point for money. High-risk confirmations display the expanded amount.

**Task-aware Market (`B10`)**: prefer deep links and smart query chips such as `active task`, `missing only`, `owned/required` rather than one-click unbounded auto-buy.

**Score:** Value **8/10** · Complexity **6/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-08 — Party Hunt Accounting and Settlement

**Source items:** `B12`, `B15`

These belong to one **Party Hunt Settlement** system.

The useful expense fact is consumption during the hunt multiplied by the active valuation policy, regardless of when the supply was purchased.

**Canonical flow:**

1. collect/reconcile loot and supply usage for the party hunt session;
2. freeze a settlement snapshot with participants, interval, valuation policy, loot, supplies and adjustments;
3. show transparent per-player balances;
4. execute settlement as a separate idempotent economy transaction;
5. never let a party leader debit another player's bank without that player's authorization;
6. make offline recipients, reconnect and crash recovery safe against double payment.

A first version may let the loot holder pay all positive balances from their own account. Multi-debtor settlement can later use explicit proposals/acceptance. Analytics may calculate values but is not transaction authority.

**Score:** Value **10/10** · Complexity **8/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-09 — Offline Training and Offline Exercise Sessions

**Source items:** `A04`, `A05`

This cluster intentionally adopts the owner's clarified product direction.

**Remembered Offline Training (`A04`)**: remember the last/default character-specific choice instead of repeatedly asking the same question.

**Offline Exercise (`A05`)**: Oteryn Evolved should support a server-authoritative Offline Exercise Session instead of requiring a client and PC to remain connected for hours while repeating deterministic training.

Per-character equivalence remains:

```text
same exercise item/charges
same per-charge effectiveness
same per-character training rate
same eligibility/restrictions
same total cost for the same training result
```

Multiple different characters on one account may each have an active session. This is **intentional** and not abuse by default.

Rationale:

- removes pointless always-online client/PC time;
- reduces idle session/runtime/network load;
- multiple characters legitimately consume more exercise resources;
- when those resources are bought with gameplay currency, concurrent alt training can be a useful voluntary **gold sink**;
- it does not secretly improve per-character rate or per-charge return.

Implementation envelope:

- at most one active offline-exercise session per character;
- reserve/escrow selected charges/item state before start;
- version training rules/rates;
- deterministic elapsed-time settlement rather than simulating an always-online actor;
- idempotent start/stop/claim/reconnect;
- no duplicate charge consumption or progression after crash/retry;
- measure gold removed, charges consumed and skill progression before later tuning.

Reference worlds may retain Global-compatible behavior; this is an explicit Evolved capability unless a later Reference revision says otherwise.

**Score:** Value **10/10** · Complexity **6/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-10 — Appearance Customization

**Source items:** `A06`, `A07`

More outfit colors and random colors should ship as one improved **Appearance Color Editor**.

Recommended capabilities: larger curated palette and/or bounded safe picker, recent/favorite colors, randomize all, randomize selected part, part locks, and optional harmonious/contrast/dark/light generation later.

Avoid freezing an unnecessarily narrow permanent color encoding before protocol/content representation is intentionally decided.

**Score:** Value **8/10** · Complexity **3/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-11 — Social Contacts, Identity Context and Chat Routing

**Source items:** `A10`, `A17`, `A18`, `B07`

These should consume one coherent contacts/presence/chat presentation model.

For contacts/VIP metadata, preserve Oteryn's consent-based relationship model while showing authorized/public context such as level, guild/badge/rank and party state. Replace a tiny fixed set of mutually exclusive VIP folders with user-owned tags; one contact may carry several tags such as `Friend`, `Healer`, `Boss Team`, `Trade`.

For event chat, prefer typed message categories that the client may route into separate tabs, mute or combine according to local preference instead of a one-off special packet.

**Score:** Value **8/10** · Complexity **5/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-12 — Mount Utility and Identity

**Source item:** `A14`

This is a later gameplay-system proposal, not a normal first-wave QoL item.

Giving every individual mount bespoke combat stats creates BIS pressure, balance/content maintenance and potential monetization fairness concerns. If pursued later, prefer a small set of gameplay-earned utility archetypes, terrain/travel/capacity identity before direct combat power, cosmetic appearance separated from utility where feasible, no store-exclusive power, and explicit PvP/movement review.

**Score:** Value **7/10** · Complexity **9/10** · Adoption sense **6/10** · Priority **B**.

---

### QOL-13 — Account Tenure, Loyalty and Commercial Entitlements

**Source item:** `B19`

The proposed rule — immediately count all prepaid future Premium time toward progression-relevant Loyalty — is **not recommended**.

It changes a time/tenure mechanic into an immediately purchasable progression accelerator and creates difficult refund/chargeback/revocation semantics after a player has already benefited from the higher progression multiplier.

Prefer keeping these concepts separate:

```text
prepaid entitlement balance
!=
elapsed account/premium tenure
!=
progression power
```

If Oteryn later adopts tenure rewards, cosmetic/prestige/account-convenience rewards are a safer first direction than combat/progression power. Platform remains the commercial entitlement authority; no monetization model is accepted by this document.

**Score for the original proposal:** Value **3/10** · Complexity/risk **8/10** · Adoption sense **2/10** · Priority **C / reject as proposed**.

## 4. Canonical de-duplication table

| Original items | Canonical Oteryn initiative |
|---|---|
| `A08 + B03` | Party Presence, Navigation and Privacy |
| `A09 + B05 + C01` | Equipment Loadouts |
| `B13 + B14` | Active Effects UI |
| `A02 + A16 + B16 + B17 + B18` | Collection Framework |
| `B12 + B15` | Party Hunt Accounting and Settlement |
| `A04 + A05` | Offline Training and Offline Exercise Sessions |
| `A06 + A07` | Appearance Color Editor |
| `A17 + A18 + B07` | Consent-based Contacts + Tags + context |
| `B01 + B02 + B04 + B08 + C02` | Native UI workspace/scaling/input family |
| `A03 + A11 + A12 + A13 + B09 + B11` | Inventory/Stash/Reward convenience family |
| `A01 + A15 + B10` | Market/Bank/Task-aware economy UX |

## 5. Recommended programme order

### S — design early / high product return

1. `QOL-01` Native UI Workspace, Scaling and Input, including native WASD diagonal chords.
2. `QOL-03` Equipment Loadouts and Build Preparation.
3. `QOL-04` Active Effects, Buffs and Timers.
4. `QOL-05` Collection Framework.
5. `QOL-08` Party Hunt Accounting and Settlement.
6. `QOL-09` Offline Training and Offline Exercise Sessions.
7. High-value slices of `QOL-06`: larger stacks and direct Stash-to-NPC sale semantics.

### A — strong improvements after/with owning domains

- `QOL-02` Party Presence and Navigation;
- remaining `QOL-06` inventory/stash/reward improvements;
- `QOL-07` market/bank/task-aware UX;
- `QOL-10` Appearance Customization;
- `QOL-11` Social Contacts and Chat Routing.

### B — later explicit gameplay design

- `QOL-12` Mount Utility and Identity.

### C — do not adopt as currently proposed

- `QOL-13` immediate Loyalty credit for future prepaid Premium.

## 6. Cross-cutting implementation rules

1. **Reference vs Evolved is explicit.** QoL presentation improvements can often be shared, but mechanics that intentionally differ from Global must be profile/ruleset decisions rather than accidental Reference drift.
2. **No client macros as transaction authority.** Loadouts, Stash sale and party settlement become server-validated semantic operations.
3. **No one-off UI data islands.** Active effects, collection progress, contacts and task relevance should expose reusable typed projections.
4. **No silent economic value creation.** QoL may remove clicks but item/currency conservation and durable transaction rules remain authoritative.
5. **Offline exercise is an intentional Evolved convenience/gold-sink capability.** Multiple characters may train concurrently when each legitimately consumes its own exercise resources; per-character rate/return remains unchanged unless a separate balance decision says otherwise.
6. **Exact location remains permissioned.** Party navigation must not weaken the accepted social-presence privacy model.
7. **Input QoL must not change movement power.** WASD diagonal chords produce the same semantic diagonal movement already legal to the character; they must not increase movement cadence, bypass collision/exhaustion or emit extra movement commands.
8. **Prefer configurable local presentation over server special cases.** Chat routing, panel layout, tracker display and contact tags belong primarily to user-facing presentation/settings unless they affect authoritative gameplay.
9. **Do not confuse architecture registration with implementation authority.** Each initiative still requires its owning gate/allocation, concrete acceptance tests and live repository coordination before code changes.

## 7. Net result

The current **39 source proposals** should not become 39 unrelated backlog items. They reduce to **13 coherent initiatives**.

The newest proposal does not create a fourteenth system:

- Equipment Presets are already the canonical `QOL-03 Equipment Loadouts` system;
- simultaneous WASD diagonal movement extends `QOL-01 Native UI Workspace, Scaling and Input`.

The highest-value product architecture remains concentrated in modern native UI ergonomics, atomic equipment/build preparation, active-effect presentation, one reusable Collection Framework, safe party navigation, first-class party hunt settlement, inventory/Stash/economy friction reduction, and server-authoritative offline exercise that removes pointless always-online training while increasing voluntary gold consumption when players train multiple characters.

This grouping is product-design input only. It records the owner's requested direction and de-duplication rationale without granting runtime, economy, progression, monetization, production or merge authority.
