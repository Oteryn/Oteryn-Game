# SPELL-NPC-MAP-0 mapping the spell and NPC source handoffs to accepted owners

- Decision: `SPELL-NPC-MAP0-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (spell and
  content) and protected integration. Rulings M1-M3 and M9 apply accepted decisions; rulings
  M4 (Paralyze caster effect) and M8 (`departure_text`) rule on contract gaps.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner answers of 2026-10-04 (1a: publish the local spell and NPC data as draft PRs
  per batch, no merge until this mapping is accepted; 2a: one architect mapping batch); the
  "Do architekta" sections of the spell handoff (#1622 comments 5977705698 and 5978988593) and of
  the NPC handoff (#1622 comment 5979309489)
- Builds on: `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` (S7, S23-S27, §9),
  `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` (parts A-D), the chain candidate (S23),
  CONDITIONS-0, CREATURE-AI-0 §8, FAMILIARS-0, PARTY-PVP-0, RUNE-USE-0 (§5.2, §9),
  SPELL-PRESENT-0, STANCE-0, WHEEL-0, WORLD-INTERACTION-0, EQUIP-0, NPC-0, NPC-BEHAVIOUR-0,
  QUEST-STATE-0, QUEST-GATE-0, TRAVEL-0, BANK-FEE-0, owner rule 5905825574 (Global parity), D479
- Amends, pending acceptance of this decision: TRAVEL-0 §4 (one optional route field, M8);
  the native-behaviours candidate D.5.3 test 4 (M4)
- Runtime, migration and production authority: NONE. Each child keeps its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Classification used in every ruling

Each handoff dependency is put in exactly one class, and each names its consumer seam (the code
or content reader that consumes it) and its owner (an existing child of an accepted decision):

| Class | Meaning | What the owner does |
|---|---|---|
| `ADAPTER_MISSING` | the mechanic is accepted and either built or allocated; only the reader of the source data is missing | the data lane maps the source rows to the accepted shape; no new contract |
| `CONTRACT_GAP` | no accepted shape represents the source behaviour | this decision rules, or names the decision that must |
| `RUNTIME_GAP` | the shape is accepted (S27) but its runtime is not built | the named child builds it; data waits, held |
| `SOURCE_CONFLICT_KEPT` | the sources disagree and the conflict is preserved | stays held with its diagnostic; S24 source order decides when evidence arrives |
| `EXCLUDED` | deliberately outside the active catalogue | never activated |

No handoff list creates a task by itself. A dependency without a row below stays with its
existing owner unchanged.

## 2. Spell schema extensions (M1)

The private schema `urn:oteryn:spell-authoring:source-complete:candidate:2` and its four
extensions are **not adopted** as a product schema. A second spell schema would split the S7
reader. The product reader stays `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`. Each extension maps to
the concrete shapes that S27 accepted from the native-behaviours candidate: a `native_behavior`
key, or an ordinary Ability, Effect or targeting field. The candidate's part and section names
(`wheel_of_destiny`, `monk_harmony_virtue`, `monk_p4_scripts`, `stance`, `party`, `familiar`,
`world_query`, `target_default`, `equipment_dependent`, `extra_presentation_only`,
`delayed_or_repeated`, `other` and the like) are family labels, never keys; the S7 reader
rejects them (`part_d_keys_without_a_runtime_stay_rejected`).

| Private extension | Accepted shape (candidate section) | Class |
|---|---|---|
| `state` | Wheel (A.1): roles B-D and the Avatars are plain Abilities (the Avatars a timed `condition` Effect, D.1.5), Divine Empowerment `owned_field_buff`, Divine Grenade `delayed_strike`, Executioner's Throw and Spiritual Outburst the S23 chain. Monk (A.2): `monk_focus`, spenders plain Abilities, the `harmony_role` field (S26). Monk scripts (A.3): Balanced Brawl `monster_ai_override`, Mass Spirit Mend a plain Ability with two `heal` Effects. Stances (C.4) and virtues (C.5): `stance_toggle`. Cancel Magic Shield (C.5): a `remove_condition` Effect. Delayed strikes (D.1): `delayed_strike` | `RUNTIME_GAP` per key; plain Abilities `ADAPTER_MISSING` |
| `equipment` | D.4: no key; the `player_expression` Formula with `inputs: skill` and `needs_weapon`, and the three shield-spell Ability additions | `RUNTIME_GAP` for the reader additions |
| `party-summon` | `party_buff` (C.3, built), `familiar_summon` (C.1), `acquire_summon` (C.2) | `ADAPTER_MISSING` for `party_buff`, `RUNTIME_GAP` for the rest |
| `world-control` | B.1: `monster_ai_override`, `vertical_move`, `locate_message`, `owned_field_buff`, `tile_item_operation`. B.2: `house_access`. B.3: `locate_message`, `vertical_move`, `creature_appearance`, `summon_named_creature` and the `targeting.parameter` extension. B.4: `random_item_grant`. B.5: `cast_restriction`. D.2: `tile_item_operation`. D.3: no key; `targeting.allowed_targets` and `affects.top_creature_only`. D.6: Challenge `monster_ai_override`, the bleeding spell a `condition` Effect with `top_creature_only`, Magic Wall and Wild Growth the `create_item` extension | `RUNTIME_GAP` per key or reader addition; Magic Wall and Wild Growth stay with the later decision RUNE-USE-0 §9 names |

- The data lane lowers each private model to the S7 shape the table names: `execution.native_behavior`
  with the candidate's concrete `key` and `parameters`, or the ordinary Ability, Effect and
  targeting fields. A model whose section has no concrete shape stays held. A private field that has no candidate parameter is reported
  as one `CONTRACT_GAP` row in the lane's publication PR, with the source evidence. This decision
  admits no new key and no new parameter.
- S7 still holds: today the reader implements only `party_buff`
  (`apps/game-server/src/spell/authoring.rs`); every other key, and every Ability or targeting
  field the candidate adds, is rejected until its runtime child lands it. A lowered model is therefore held, not activated. The handoff flags
  (`source_consumer_implemented=false`, `native_execution_qualified=false`,
  `runtime_activation=false`) clear only through the real reader and its tests, never by editing
  the index.

## 3. Blocked spell queues mapped to owners (M2)

| Queue (handoff) | Consumer seam | Owner | Class |
|---|---|---|---|
| Cast guards and caster restrictions (14 variants, Sweeping Takedown, Crystal Heal Friend) | `spell/cast.rs` admission, `spell/target.rs` | `cast_restriction` (B.5); `targeting.allowed_targets` (D.3); one spell-lane child per key or field (S27) | `RUNTIME_GAP` |
| Levitate and Magic Rope helpers | WORLD-INTERACTION-0 levitate and rope | that decision's child | `ADAPTER_MISSING` |
| Several Combat objects in one cast; Forked Glacier and Forked Thorns | `spell/chain.rs`, `spell/plan.rs` | S23 chain candidate | `ADAPTER_MISSING` where the chain shape covers it; otherwise one `CONTRACT_GAP` row |
| Scheduler and cancellation (delayed and repeated strikes) | the channel tick; `spell/plan.rs` | `delayed_strike` (D.1.5); `owned_field_buff` for Divine Empowerment | `RUNTIME_GAP`: no spell scheduler exists on main; it is the `delayed_strike` child's work, not a second runtime |
| Typed conditions | `ability/condition.rs` | CONDITIONS-0 COND-1, COND-CONTENT-1 | `ADAPTER_MISSING` |
| Speed and pacing | SPEED-1 (merged) | CONDITIONS-0 | `ADAPTER_MISSING`; D479 unchanged (M6) |
| Party spells (11) | `spell/party.rs` | `party_buff` (C.3); PARTY-PVP-0 PARTY-1 for party membership | `ADAPTER_MISSING` for `party_buff`; `RUNTIME_GAP` until PARTY-1 for membership reads |
| Summons (17) | the summon owner | CREATURE-AI-0 §8.2 SUMMON-1, SUMMON-WIRE-1; `acquire_summon` (C.2), `summon_named_creature` (B.3) | `RUNTIME_GAP` |
| Familiars | the familiar state row | FAMILIARS-0 FAMILIAR-1, FAMILIAR-CONTENT-1 (`familiar_summon`) | `RUNTIME_GAP` |
| Monk (25 variants, 3 callbacks, 6 formulas) | `spell/harmony.rs` | `monk_focus` (A.2), `monster_ai_override` (A.3), plain Abilities | `RUNTIME_GAP`; the 3 callbacks belong to the `monk_focus` child once, never a per-spell copy |
| Wheel spells (36) and Avatars | the ready reader; `wheel_unlock` | WHEEL-0 SPELL-WHEEL-GATE-1, W-1, W-FX-1; the A.1 shapes above; the Avatars are plain Abilities with an `outfit_binding` reader addition | `RUNTIME_GAP` |
| Stances (27) | the stance state | STANCE-0; `stance_toggle` (C.4) | `RUNTIME_GAP` |
| Effects and sounds | spell emission | SPELL-PRESENT-0 SPELL-PRESENT-1, PRESENT-CONTENT-1 | `ADAPTER_MISSING` |
| World and custom (P4) spells | the B.1-B.4, D.2 and D.6 keys above | the key's child | `RUNTIME_GAP` |
| Premium | the premium reader | PREMIUM-ACTIVATION (merged) | `ADAPTER_MISSING`; no second Premium runtime |

The CP allocates by mechanic (one child per key or decision child), never one task per spell.

## 4. Paralyze caster effect (M4, contract gap)

- **Source fact.** In the pinned Canary and Crystal sources, `luaCombatExecute` in the
  `VARIANT_NUMBER` branch sets the result to true, calls `doCombat` without reading its result,
  and returns true to Lua. The rune script shows the green caster effect when that call returns
  true. The caster effect therefore does not depend on whether the paralysis applied or on the
  health hooks; the `VARIANT_POSITION` and `VARIANT_STRING` branches assign their result
  differently.
- **Ruling.** The caster-tile effect is emitted when the cast is **admitted and executed**
  against the target, whatever the per-target outcome (immune, blocked, condition not applied).
  It is not emitted when the cast is refused before execution (no valid target, out of range, not
  in sight, refused by RUNE-USE-0 admission); a refused cast burns nothing (RUNE-USE-0 §5.2).
- **Amendment to D.5.3 test 4.** "A failed cast (target immune, out of range) gives no caster-tile
  effect" becomes two tests: a refused cast (out of range) gives no caster-tile effect; a cast on
  an immune target gives the caster-tile effect and the target's block effect.
- **Order kept.** The combat keeps `COMBAT_UNDEFINEDDAMAGE` (never replaced by `COMBAT_NONE`) and
  runs the health, block and change-health hooks before the condition is applied.
- **Evidence level.** Under S24 a donor source is a hypothesis; no official or wiki source
  describes the effect on an immune target. The ruling is `PARITY_PENDING` and changes only if a
  higher S24 source contradicts it. The Heal Friend caster effect (D.5.1) is not changed.
- **Owner.** The child that adds the D.5 `presentation.caster_effect_asset_binding` Effect field (no
  `native_behavior` key) and RUNE-CAST-1; Paralyze also needs
  SPEED-1 and COND-1. The R45 partials are the input; both full spells stay held until that child
  lands.

## 5. Monk formulas (M5)

The Monk formulas keep the source integer and float types, the clamp, the rounding and the
conversion order, and read the Harmony and Virtue state from A.2. A wiki curve never replaces a
source formula; a wiki value that disagrees is a `SOURCE_CONFLICT_KEPT` row resolved by the S24
order. The R35 programmes are the input of the A.2/A.3 child.

## 6. Exclusions and pacing (M6)

- The four `#example.lua` test spells (Test, test rune; both donors) are `EXCLUDED`.
- Sap Strength and Expose Weakness stay removed (S24 `removed`); their rows are reference only.
- D479 is unchanged: source pacing is the merged SPEED-1. No second speed, Premium or Wheel
  runtime is allocated.

## 7. NPC dependencies mapped to owners (M7)

| Handoff area | Consumer seam | Owner | Class |
|---|---|---|---|
| NPC actor, map placement, visibility, CHAT dispatch | the NPC runtime actor; `EntityKind` `Npc` | NPC-BEHAVIOUR-0 NPC-ACTOR-1, NPC-VIS-1 (started, D499), NPC-REBUILD-1; NPC-0 NPC-PLACE-1 | `RUNTIME_GAP`; the local placement proposal is input to NPC-PLACE-1, never a creature that pretends to be an NPC |
| Content and dialogue (`NpcDataCatalogue` resolver) | NPC-0 NPC-CONTENT-1, NPC-TALK-1 | NPC-0 | `ADAPTER_MISSING` |
| Quest tracks and progress | the QUEST-STATE-0 store | QUEST-PRED-1, QUEST-LOWER-1, QUEST-XP-1 | `ADAPTER_MISSING`; the local `tracks` amendment is aligned to the accepted store, with no second quest store and no storage-number aliases |
| Trade, travel, blessing, promotion, bank | the owning atomic transaction | NPC-TRADE-1, NPC-TRAVEL-1, BANK-FEE-0 GOLD-FEE-1b/2, the other domain owners | `ADAPTER_MISSING`; D498 holds stay; no migration or protocol lease is granted |
| Item and spell identities and appearance | the bundle admission | the item and spell packets of this batch | `ADAPTER_MISSING`; a name match is not a native binding |
| Spell acquisition at NPCs | the physical coin fee plan | NPC-0 services; GOLD-FEE-1b | `ADAPTER_MISSING`; no charm-only fee writer, no implied bank fallback |
| Captain Haba (32) and Summer Shirtalis (1) context holds | — | — | `SOURCE_CONFLICT_KEPT` |
| 8 blessing conflicts at 4 NPCs | — | — | `SOURCE_CONFLICT_KEPT`; no automatic 6→1 mapping |
| Postman discount on `Storage.Quest.ExampleQuest` | TRAVEL-0 §5 | TRAVEL-CONTENT-1 | `SOURCE_CONFLICT_KEPT`: the discount stays `PARITY_PENDING` with no invented gate; a missing `new frontier` entry is amount 0 |

## 8. Travel `departure_text` (M8, contract gap)

- **Ruling.** `ProjectV2TravelRoute` gains one optional field, `departure_text`: the NPC's line
  spoken when the travel commits. When it is absent, the vehicle's reply template line is used
  (TRAVEL-0 §4 "Kinds"). It is authored text: trimmed of leading and trailing whitespace,
  non-empty after trimming (an empty or whitespace-only value holds the route, as the ProjectV2
  dialogue validator does), at most 255 bytes of UTF-8 and with no control characters, and it has no effect on price, gate, refusal or arrival.
- **Owner.** TRAVEL-CONTENT-1 (the validator rule) and NPC-TRAVEL-1 (speaks it on the known
  commit only, never on an ambiguous one). No wire change: it uses the NPC-0 talk wire.

## 9. Publication of the local packets (M9)

Each local batch becomes one draft PR on its own branch, owned by the data lane and allocated by
the CP: the spell batches r59-r62 (and any of r28-r58 not on main), the NPC combined patch, the
travel data and the item candidates. No such PR merges before this decision is accepted. Each PR
lists, per model, its class from §1 and keeps its held state; it activates nothing.

## 10. Rejected options

- Adopting the private `source-complete` schema beside S7: two readers of one spell catalogue.
- One task per spell or per handoff row: S27 allocates by key.
- Emitting the Paralyze caster effect only when the paralysis applies: contradicts the only
  source of the behaviour.
- A wiki curve in place of the Monk source formulas: S24 order and the handoff evidence.
- A second quest store for the NPC tracks: QUEST-STATE-0 owns progress.
- Putting the departure line in the vehicle template only: routes of one vehicle say different
  lines in the source.

## 11. Decision test (`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`)

| Question | M4 Paralyze caster effect | M8 `departure_text` |
|---|---|---|
| Must decide now? | YES | YES |
| Blocked downstream work | the D.5 presentation child and RUNE-CAST-1 for Paralyze; the D.5.3 engine tests cannot be written against two contradictory rules; the R45 partials cannot be lowered | TRAVEL-CONTENT-1 lowering of the local travel data (the NPC handoff needs this field); the NPC-TRAVEL-1 commit reply |
| Harder later | changing the emission rule after RUNE-CAST-1 lands rewrites its tests and the presentation stream; no persistence or wire coupling | removing an authored field later is a content migration of every route that carries it; adding it later re-imports the routes; no persistence or wire coupling |
| Evidence that supersedes it | an official tibia.com source or the wiki (S24 order) describing the effect on an immune or blocked target; an owner in-game test (D.7) | a Global source showing the departure line is not per route; a TRAVEL-0 acceptance review that places the line elsewhere |
| Deliberately not decided | the effect for the `VARIANT_POSITION` and `VARIANT_STRING` branches and for other runes; the Heal Friend effect (D.5.1); the paralysis duration and speed values | translations, per-vehicle defaults, an arrival line, any wire field |

M1-M3 and M5-M7, M9 add no contract: they apply accepted decisions, so they need no separate
decision test.
