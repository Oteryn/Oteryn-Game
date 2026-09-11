# Oteryn Evolved QoL Feature Clusters

- Date: 2026-09-11
- Status: **OWNER-REQUESTED PRODUCT DESIGN INVENTORY / PROPOSED / NOT IMPLEMENTATION AUTHORITY**
- Repository snapshot used for grouping: `Oteryn/Oteryn-Game@90a3f92434e32354ff1aaeac96d038bbc49eba9c`
- Applies to: future Oteryn Evolved product/UX planning and Reference-vs-Evolved separation
- Implementation authority: **NONE**
- Production authority: **NONE**

## 1. Purpose

This document consolidates two owner-provided QoL wishlists into a smaller set of coherent product initiatives. Several original ideas overlap or are better implemented as one reusable system rather than as isolated features.

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

## 3. Canonical grouped initiatives

### QOL-01 — Native UI Workspace, Scaling and Input

**Source items:** `B01`, `B02`, `B04`, `B08`

**Canonical feature family:**

- independent in-game UI scale;
- resizable/floating minimap rather than a fixed one- or two-column special case;
- bindable wheel-up / wheel-down semantic actions with proper UI-scroll precedence;
- persistent/pinnable chat/read-only tabs as part of the general dock/layout persistence model.

**Why these belong together:** they all consume the native client layout/input/settings foundation. The current Native UI architecture already models `user_ui_scale`, logical UI units, DPI, dock trees, selected tabs and semantic wheel input; the feature family should use those primitives rather than add Tibia-client-specific exceptions.

**Recommended design:**

- UI scale independent from world-camera zoom;
- safe scale range such as 75-200%, with accessibility/text scaling separable later;
- minimap is a normal resizable dockable panel;
- high-resolution/free-spin wheel input is normalized into discrete gameplay steps only after UI scroll has had a chance to consume it;
- chat/tab layout is versioned and restored safely across sessions/resolution changes.

**Score:** Value **10/10** · Complexity **4/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-02 — Party Presence, Navigation and Privacy

**Source items:** `A08`, `B03`

These are one feature, not two: a party-location projection rendered on the minimap/map.

**Recommended design:**

- same Channel/Instance: precise party marker may be shown when authorized;
- different Channel: show coarse `different channel` presence rather than a false coordinate;
- different Instance: show coarse instance state without leaking exact placement;
- preserve the accepted privacy-first social-presence boundary;
- permit stricter PvP/ruleset policies and user-controlled sharing where appropriate.

Do not implement public or unconditional GPS. Party membership is the authorization context, not a reason to make exact placement world-public.

**Score:** Value **9/10** · Complexity **6/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-03 — Equipment Loadouts and Build Preparation

**Source items:** `A09`, `B05`, `B06`

`A09 Full Set Hotkeys` and `B05 Protection Set Management` are the same system and should become generic **Equipment Loadouts** rather than Fire/Death-only shortcuts.

**Canonical feature family:**

- named loadouts such as Fire, Energy, Death, Physical, Damage, Speed or Boss;
- one semantic `ActivateEquipmentLoadout` operation rather than a client macro issuing many equip commands;
- deterministic server validation of all participating item instances, slots, destinations and restrictions;
- default all-or-nothing behavior, with any partial mode requiring a separate explicit product decision;
- normal equipment/combat restrictions remain authoritative.

`B06 Wheel in PZ` belongs to the same broader **build preparation** experience but remains a separate ruleset policy inside the cluster:

- Reference may retain its target restriction;
- Evolved may allow build editing in an accepted safe-zone class, e.g. Any PZ or another explicit safe-state policy;
- do not hard-code Temple/PZ geography into generic engine semantics.

**Score:** Value **10/10** · Complexity **7/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-04 — Active Effects, Buffs and Timers

**Source items:** `B13`, `B14`

Food duration and separate food/potion icons should not become two parallel UI systems. They should be one generic **Active Effects** surface backed by authoritative effect/condition state.

**Recommended design:**

Each visible effect may expose presentation-safe fields such as:

```text
semantic effect identity
icon / presentation key
resolved magnitude or summary
source label where permitted
remaining duration
stack/refresh state where relevant
```

The server owns effect lifetime/stacking semantics; the client may locally count down between authoritative updates.

Provide:

- compact configurable HUD icons;
- precise timer/tooltip;
- a full Effects panel for overflow/history-relevant presentation;
- clear separation of buffs and dangerous debuffs;
- no artificial six-icon architectural ceiling.

**Score:** Value **10/10** · Complexity **5/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-05 — Collection Framework: Bestiary, Bosstiary, Echo and Item Codex

**Source items:** `A02`, `A16`, `B16`, `B17`, `B18`

These should become one reusable **Collection Framework** instead of five independent feature implementations.

**Canonical consumers:**

- Bestiary;
- Bosstiary;
- Echo Warden/Echo progression;
- Item Deck / Item Codex;
- future achievements/collections where compatible.

**Shared UI capabilities:**

- search;
- sort;
- filter;
- grid/list/dense views;
- completion state;
- progress/current count/remaining count;
- favorites;
- saved tracker presets;
- `incomplete only` and `nearest completion` views.

**Bestiary indicator:** use the same collection state to render a small incomplete/progress marker on relevant creature surfaces rather than maintain a separate counter model.

**Tracker limit:** separate `saved/tracked interests` from the much smaller active HUD presentation set. Do not make a historic UI limit look like a server subscription limit.

**Item Codex:** stable item-definition identity is the collection key. Keep `discovered/obtained` separate from stronger provenance states such as `earned/looted/crafted` if those are later desired. Avoid power bonuses merely for collecting junk; cosmetic/title/achievement rewards are safer first use.

**Score:** Value **10/10** · Complexity **7/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-06 — Inventory, Stash, Rewards and Item-State Convenience

**Source items:** `A03`, `A11`, `A12`, `A13`, `B09`, `B11`

This cluster removes repetitive inventory manipulation while preserving item/value authority.

#### Reward collection (`A03`)

Prefer generic reward-retention state over hard-coded Crystal exceptions:

- collect selected;
- collect all eligible;
- mark selected reward/item type as keep in chest;
- visible expiry warning;
- expiry remains authoritative and auditable.

#### Empty potion containers (`A11`)

Do **not** prefer auto-drop-to-ground as the canonical solution because it creates world-item spam and cleanup/network cost.

Prefer configurable behavior:

- keep;
- move to an assigned container;
- discard.

If discarding intentionally loses deposit/value, the UI must make that consequence explicit.

#### Larger stacks (`A12`)

Use the accepted per-item-definition stack maximum rather than one universal constant. Reference and Evolved profiles may select different content/ruleset values. A first Evolved target around 1,000 for high-volume supplies is preferable to making 9,999 universal without evidence.

#### Sell directly from Stash (`A13`)

Treat Stash-to-NPC sale as an authoritative atomic value transaction, not as hidden client-side withdraw/move/sell automation. Add favorites/locks/keep-minimum protection before broad `Sell All Eligible` behavior.

#### Task-aware Stash (`B09`)

Generalize from `Weekly Task` to **Objective Relevance** where safe:

- active known task requirements;
- owned vs required quantity;
- no unrevealed quest spoilers.

#### Ring/amulet normalization (`B11`)

The underlying problem is organization of altered/partially consumed equipment. Prefer a generic `Normalize/Discharge Item` operation or, later, an Equipment Vault. Do not refund unused temporary effect/value unless an owning economy decision explicitly permits it.

**Score:** Value **9/10** · Complexity **7/10** · Adoption sense **9/10** · Priority **A/S by slice**.

---

### QOL-07 — Market, Bank and Task-Aware Economy UX

**Source items:** `A01`, `A15`, `B10`

#### Market anti-abuse (`A01`)

Do not freeze a blanket five-minute cooldown as the architecture. The intended problem is automated/reactive offer churn.

Prefer bounded anti-abuse throttling using explicit policies such as:

- per-account/per-instrument rate limits;
- rolling-window cancel/reprice limits;
- fees that remain meaningful where the economy design uses them;
- stronger controls for commercial/premium currency if such a product exists;
- telemetry before tuning.

Normal players should almost never encounter the throttle.

#### Amount shorthand (`A15`)

Create one exact-integer `AmountExpression` parser reusable across bank/market/NPC/trade surfaces. Support safe forms such as `2k`, `200k`, `1.5kk`, with locale-aware decimal input only if parsing remains unambiguous. Never use binary floating point for money. Confirmation must display the fully expanded amount for high-risk transfers.

#### Task-aware Market (`B10`)

Prefer deep links and smart query chips such as `active task`, `missing only`, `owned/required` rather than one-click unbounded auto-buy. Price/slippage decisions remain explicit.

**Score:** Value **8/10** · Complexity **6/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-08 — Party Hunt Accounting and Settlement

**Source items:** `B12`, `B15`

These belong to one **Party Hunt Settlement** system.

`B15` should not track only foods purchased during the hunt. The useful economic fact is **consumption during the hunt multiplied by the active valuation policy**, regardless of when the supply was purchased.

**Canonical flow:**

1. Party/hunt session collects authoritative or reconciled loot/supply usage inputs.
2. A settlement snapshot freezes participants, interval, valuation policy, loot, supplies and manual adjustments.
3. The UI shows transparent per-player balances.
4. Settlement becomes a separate idempotent economy transaction.
5. No party leader may debit another player's bank without that player's authorization.
6. Offline recipients and reconnect/crash recovery must not cause double payment.

A simple first model may allow the loot holder to pay all positive balances from their own account. Multi-debtor settlement can later use explicit proposals/acceptance.

Analytics may calculate/propose values, but analytics is not transaction authority.

**Score:** Value **10/10** · Complexity **8/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-09 — Offline Training and Offline Exercise Sessions

**Source items:** `A04`, `A05`

This cluster intentionally adopts the owner's clarified product direction.

#### Remembered Offline Training (`A04`)

Prefer `remember last choice` and an explicit favorite/default over repeatedly asking the same question. The preference is character-scoped where the training mode is character-specific.

#### Offline Exercise (`A05`)

Oteryn Evolved should plan for a server-authoritative **Offline Exercise Session** rather than requiring a client and PC to remain connected for hours while the character repeats a deterministic training action.

The desired semantic equivalence for one character is:

```text
same exercise item/charges
same per-charge effectiveness
same per-character training rate
same eligibility/restrictions
same total cost for the same training result
```

The player may start offline exercise on multiple different characters. This is **intentional**, not treated as an abuse case by default.

Product rationale:

- it removes the artificial requirement to leave a client/PC online for many hours;
- it can reduce pointless idle gameplay sessions/runtime/network load;
- if exercise weapons are purchased with gameplay currency, multiple concurrently training characters increase voluntary exercise-weapon consumption and therefore can be a useful **gold sink**;
- it accelerates account-wide alt development only because the owner intentionally spends the corresponding resources on several characters; it must not secretly improve the per-character rate or per-charge return.

**Recommended safety/implementation envelope:**

- at most one active offline-exercise session per character;
- multiple characters on one account may each have their own active session;
- reserve/escrow the selected charges or item state before the session starts;
- use versioned training rules/rates so later balance changes do not reinterpret an already-started session ambiguously;
- deterministic elapsed-time settlement/catch-up rather than simulating an always-online character hitting a dummy;
- idempotent start/stop/claim/reconnect behavior;
- no duplicate charge consumption or progression after crash/retry;
- telemetry should measure gold removed, charges consumed and skill progression before any later tuning.

Reference worlds may retain Global-compatible behavior; this offline exercise model is an explicit Evolved product capability unless a later Reference revision says otherwise.

**Score:** Value **10/10** · Complexity **6/10** · Adoption sense **10/10** · Priority **S**.

---

### QOL-10 — Appearance Customization

**Source items:** `A06`, `A07`

More outfit colors and random colors should ship as one improved **Appearance Color Editor**.

Recommended capabilities:

- larger curated palette and/or bounded safe color picker appropriate for pixel-art readability;
- recent/favorite colors;
- randomize all;
- randomize selected part;
- lock selected parts while randomizing the rest;
- optional harmonious/contrast/dark/light palette generation later.

Avoid making the client assume an unnecessarily narrow permanent color encoding before the protocol/content representation is intentionally frozen.

**Score:** Value **8/10** · Complexity **3/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-11 — Social Contacts, Identity Context and Chat Routing

**Source items:** `A10`, `A17`, `A18`, `B07`

These should consume one coherent contacts/presence/chat presentation model.

#### Contacts/VIP metadata (`A17`, `A18`, `B07`)

Oteryn's accepted social direction is consent-based rather than unilateral legacy VIP tracking. On top of that relationship model, the client may display authorized/public contextual information such as:

- level;
- guild/badge/rank where policy permits;
- party state;
- private local organization metadata.

Replace a tiny fixed number of mutually exclusive VIP folders with **user-owned tags**. One contact may carry multiple tags such as `Friend`, `Healer`, `Boss Team`, `Trade`.

#### Event chat (`A10`)

Do not introduce one special `Event Chat` packet type if the general message system can expose typed message categories. The client should be able to route categories into separate tabs, mute them, or combine them according to local preference.

**Score:** Value **8/10** · Complexity **5/10** · Adoption sense **9/10** · Priority **A**.

---

### QOL-12 — Mount Utility and Identity

**Source item:** `A14`

This is not a normal QoL item; it is a gameplay-system proposal and should remain isolated from the first UX package.

Giving every individual mount bespoke combat stats creates strong BIS pressure, substantial balance/content maintenance and potential monetization fairness concerns if any mount source is commercial.

Prefer, if pursued later:

- a small number of gameplay-earned **mount utility archetypes**;
- terrain/travel/capacity-oriented identity before direct combat power;
- cosmetic appearance separated from the gameplay utility choice where feasible;
- no store-exclusive power;
- explicit PvP and movement-balance review.

Examples of safer utility directions include terrain slow reduction, limited environmental traversal utility or bounded non-combat carrying convenience rather than permanent damage/resistance BIS.

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

If Oteryn later adopts tenure rewards, cosmetic/prestige/account-convenience rewards are a safer first direction than combat/progression power. Platform remains the commercial entitlement authority under the accepted architecture; no monetization model is accepted by this document.

**Score for the original proposal:** Value **3/10** · Complexity/risk **8/10** · Adoption sense **2/10** · Priority **C / reject as proposed**.

## 4. Canonical de-duplication table

| Original items | Canonical Oteryn initiative |
|---|---|
| `A08 + B03` | Party Presence, Navigation and Privacy |
| `A09 + B05` | Equipment Loadouts |
| `B13 + B14` | Active Effects UI |
| `A02 + A16 + B16 + B17 + B18` | Collection Framework |
| `B12 + B15` | Party Hunt Accounting and Settlement |
| `A04 + A05` | Offline Training and Offline Exercise Sessions |
| `A06 + A07` | Appearance Color Editor |
| `A17 + A18 + B07` | Consent-based Contacts + Tags + context |
| `B01 + B02 + B04 + B08` | Native UI workspace/scaling/input family |
| `A03 + A11 + A12 + A13 + B09 + B11` | Inventory/Stash/Reward convenience family |
| `A01 + A15 + B10` | Market/Bank/Task-aware economy UX |

## 5. Recommended programme order

### S — design early / high product return

1. `QOL-01` Native UI Workspace, Scaling and Input.
2. `QOL-03` Equipment Loadouts and Build Preparation.
3. `QOL-04` Active Effects, Buffs and Timers.
4. `QOL-05` Collection Framework.
5. `QOL-08` Party Hunt Accounting and Settlement.
6. `QOL-09` Offline Training and Offline Exercise Sessions.
7. High-value slices of `QOL-06`: larger stacks and direct Stash-to-NPC sale semantics.

These either align closely with work already being architected or become substantially more expensive if their extension points are omitted until after UI/economy/progression contracts harden.

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

Any future implementation package should preserve these principles:

1. **Reference vs Evolved is explicit.** QoL presentation improvements can often be shared, but mechanics that intentionally differ from Global must be profile/ruleset decisions rather than accidental Reference drift.
2. **No client macros as transaction authority.** Loadouts, Stash sale and party settlement become server-validated semantic operations.
3. **No one-off UI data islands.** Active effects, collection progress, contacts and task relevance should expose reusable typed projections.
4. **No silent economic value creation.** QoL may remove clicks but item/currency conservation and durable transaction rules remain authoritative.
5. **Offline exercise is an intentional Evolved convenience/gold-sink capability.** Multiple characters may train concurrently when each legitimately consumes its own exercise resources; per-character rate/return remains unchanged unless a separate balance decision says otherwise.
6. **Exact location remains permissioned.** Party navigation must not weaken the accepted social-presence privacy model.
7. **Prefer configurable local presentation over server special cases.** Chat routing, panel layout, tracker display and contact tags belong primarily to user-facing presentation/settings unless they affect authoritative gameplay.
8. **Do not confuse architecture registration with implementation authority.** Each initiative still requires its owning gate/allocation, concrete acceptance tests and live repository coordination before code changes.

## 7. Net result

The original 37 proposals should not become 37 unrelated backlog items. They reduce to **13 coherent initiatives**, with the highest-value product architecture concentrated in:

- modern native UI ergonomics;
- atomic equipment/build preparation;
- clear active-effect presentation;
- one reusable Collection Framework;
- safe party navigation;
- first-class party hunt accounting/settlement;
- inventory/Stash/economy friction reduction;
- server-authoritative offline exercise that removes pointless always-online training while increasing voluntary gold consumption when players train multiple characters.

This grouping is a product-design input only. It records the owner's requested direction and de-duplication rationale without granting runtime, economy, progression, monetization, production or merge authority.
