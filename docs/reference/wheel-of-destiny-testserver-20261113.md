# Wheel of Destiny: Winter Update test-server audit, 2026-11-13

**Result: Oteryn does not yet implement the complete current test-server Wheel.**
This PR repairs three reproduced numeric defects. The eight-vessel layout, four new
Conviction Mastery perks, 20 additional supreme modifiers and Sorcerer elemental-state
integration remain gaps. Native combat, paid-operation and reconnect acceptance is incomplete.

Working PR: [Oteryn/Oteryn-Game#3181](https://github.com/Oteryn/Oteryn-Game/pull/3181).
Starting revision: `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`.
The target is the **13 November 2026 test build**, not older production or changes
announced for a later update.

## Evidence and limits

**FACT** identifies inspected code, an executed check or a reference observation.
**INFERENCE** needs the stated further check. **UNKNOWN** identifies missing evidence.
**RECOMMENDATION** is a proposed next step. Accepted Oteryn differences are separate.
Donor code does not establish current Global behavior.

| ID | Source | Established facts |
| --- | --- | --- |
| S1 | CipSoft, [Gems Polished Up!, 9 November](https://www.tibia.com/news/?id=9030&subtopic=newsarchive) | Eight vessels and quality order, four +125 Mastery Convictions, removal of gem domains/resonance, Workshop bonuses and new mod families |
| S2 | Test forum, [Gem Atelier post 60906](https://www.test.tibia.com/forum/?action=thread&postid=60906#post60906) | +5 stats per upgrade, maximum +15 per modifier; reshape prices; 30% resistance cap announced but not yet deployed at that initial post |
| S3 | Test forum, [post 61698, 13 November 13:54:33 CET](https://www.test.tibia.com/forum/?action=thread&postid=61698#post61698) | Further large changes planned, probably Monday 16 November; not evidence those changes were already live |
| S4 | [Official test planner with native preset](https://www.test.tibia.com/community/?subtopic=wheelofdestinyplanner&code=M1Y2CAAm8waQQnUkACJ6aBmOr_QUBRJuY_KgAA) | 700 allocated points, three vessels, Ascetic Conviction Mastery +125, Spiritual Outburst gem Mastery +225, resulting stages and gem stats |
| S5 | Installed official client `15.33.dd0955` | Workshop text and active QML binding, eligibility/cooldown text, static supreme table with IDs 0–113 |
| S6 | [Official summary post 39618238](https://www.tibia.com/forum/?action=thread&postid=39618238#post39618238) and test planner dropdowns | All 20 new modifier names; Grade IV values independently agree with the installed client table |

Derived evidence is retained in
[client-and-planner.json](wheel-of-destiny-testserver-20261113/client-and-planner.json),
[new-supreme-mods.json](wheel-of-destiny-testserver-20261113/new-supreme-mods.json),
[existing-mod-grades.json](wheel-of-destiny-testserver-20261113/existing-mod-grades.json) and
[official-sources.json](wheel-of-destiny-testserver-20261113/official-sources.json).
The modifier file records all four static grade values, hashes/table offsets and separate
Grade IV planner checks. The official-source file includes dated forum observations and
reproducible one-byte allocation exports. These files are evidence, not a runtime catalogue.

Client executable SHA-256:
`7f6555b19514abc34220a09f427732ea66ddfcd8d7052c733f0557d624baaf34`.
Translation file SHA-256:
`f627f7e51feaa25cc29f485efdcbe7d262206c3591c01bc980a8a6edd5016a62`.

The native Wheel was observed before the remote host disconnected. Character selection
then returned to an empty-password login form. The first source PNG was overwritten during
later capture and could not be recovered; it is not retained reproducible image evidence.
Login screenshots, credentials, binaries and graphics are excluded from Git. Obsolete
translation templates remain in the client, so text presence alone does not prove reachability.

The preset carries allocation and gem IDs, not the character's Workshop grades. The
planner comparison uses Grade IV: gem HP/Energy are 150/1.5%, whereas the initial native
panel showed 100/1.0%. Mana 150, Fire 1.5%, Life Drain 4.5%, Mastery 225 and Revelation
stages agree. No native combat-damage or controlled paid-upgrade measurement was completed.

## Findings

### W01 — Missing Workshop Health, Mana and Capacity

**FACT, repaired:** the UI advertised `5 * sum(zero_based_grades)`, but
`WheelRuntimeEffects::contribute_gems` did not add those stats. S2/S5 confirm +5 to each
stat for every upgrade I→II, II→III and III→IV, including without a socketed gem.

The repair contributes this once in the shared effect projector for the current grade
revision. Grade I adds zero; a fully enhanced modifier adds +15 to each stat. Existing
caller checks for current allocation, level, promotion and Premium remain. The projector
feeds vitals, spell facts, presentation and durable capacity; capacity's conversion to
hundredths remains in its existing owner.

The regression performs five real `AtelierState::commit(GradeUp)` transitions for each
vocation. Before the fix, the first upgrade returned `(0, 0, 0)` instead of `(5, 5, 5)`.
Afterwards every step through `(25, 25, 25)` passes without socketed gems. It also proves
that old-revision grades add zero and preserves the separate Grade IV point count.

### W02 — Gem Revelation Mastery applied twice

**FACT, repaired:** `wheel_effects/gems.rs::supreme_effect` doubled 150/165/180/225 points
to 300/330/360/450 to imitate a donor's two applications. The current official planner shows +225 at Grade IV for all 16 Mastery IDs; the
client/preset also shows Spiritual Outburst stage I. All four values are independently
qualified by the current client category-5 descriptor and grade-indexed integer table
at offset `0x1da99b0`; see `existing-mod-grades.json`. This remains client/reference
evidence, not a controlled native server damage measurement.

The repair contributes each Mastery modifier once. Tests cover all 16 existing Mastery
IDs at every grade. Before the fix, Grade I returned 300,000 thousandths rather than
150,000. A composed 700-point preset regression reproduced the actual stage error:
original code produced Spiritual Outburst II; the reference and repaired code produce I.
This is separate from the new +125 Conviction perks in W03.

### W03 — Wrong vessel count/order and outdated Convictions

**FACT, open:** current test has eight vessels, in this opening order:

| Slot, one-based | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Quality | Lesser | Regular | Greater | Lesser | Greater | Regular | Greater | Greater |

Oteryn has 12 slots, Lesser/Regular/Greater repeated four times. This is enforced in:

- `crates/gameplay/progression/src/atelier/vessels.rs`: qualities and open-slot limit;
- `crates/protocol-oteryn/src/atelier/v2.rs`: capability 483's vector/index bounds;
- `apps/game-server/migrations/2770_gem_vessel_slots.sql`: current state, receipts and guards;
- `crates/client-ui/src/features/0148-wheel-allocation/state.rs`: 12 selections;
- `docs/contracts/protocol-oteryn/v1/gem-atelier-v2-v1.md`: the twelve-slot contract.

S1 replaces four vessel nodes with +125 Revelation Mastery Convictions, one per domain,
and swaps their positions with middle-domain Convictions. Oteryn's catalogue retains the
12 former resonance Convictions, counted by the runtime as Gem Vessels, and lacks these
new Mastery Convictions. The official planner confirms the changed positions for all five vocations: one-based
3↔8, 18↔11, 19↔26 and 34↔29. New Masteries are at 8/11/26/29; all eight slices retain
100-point capacity. Monk slot identities were additionally checked with one-byte allocation
export deltas. Other vocations were checked at the same visible planner positions. A full
comparison of all remaining Devotion values is still required before catalogue admission.

A constant-only resize would break stored-state decoding, retained receipt chains and
wire compatibility. Historical committed requests for slots 8–11 also need replay before
new index bounds reject them. The bounded repair plan below covers these obligations.

### W04 — Monk mixed modifiers omit Mana

**FACT, repaired:** `basic_milli` returned zero Monk Mana for modifiers 33–36 while
retaining their resistance. The native preset and official planner show mod 33 supplying
150 Mana at Grade IV; the official planner confirms all four mixed Mana IDs at that
grade. Current client constructor links for IDs33–36 and their five-vocation grade
tables are retained in `existing-mod-grades.json`. Row4 is 100/110/120/150.
**INFERENCE, high confidence:** row4 is Monk because only that row matches the official
Monk Grade IV150; the other rows end at75/225/450/450. A separately named native
vocation-enum mapping was not extracted. No live server-stat delta is claimed.

The repair removes this donor override. Regression cases cover all four affected IDs at
all grades and retain resistance checks. Before the fix, mod 33 Grade I returned zero
rather than 100,000 thousandths. Other donor overrides remain verification targets (W07).

### W05 — Point removal permission is an accepted Oteryn difference

**ACCEPTED DIFFERENCE:** `wheel.rs::WheelRuleset::validate` records the owner's
2026-11-05 decision: removal/reset is permitted where logout's combat lock does not hold
the actor, including protection-zone tiles. The compatibility field `at_temple` carries
this permission. Global temple wording does not revoke that decision.

### W06 — Twenty supreme modifiers are absent

**FACT, open:** Oteryn's catalogue has 94 supreme modifiers, IDs 0–93. The installed
client has IDs 0–113. The 20 additions have matching names/Grade IV values in the official
test planner and are absent from Oteryn's reveal/reshape/grade pools.

| Vocation | IDs | Spell | Grade IV modifiers |
| --- | --- | --- | --- |
| Knight | 94, 95 | Shield Slam | +12% base damage; +18% critical extra damage |
| Knight | 96, 97 | Intense Wound Cleansing | +30% base healing; −20 s cooldown with +1% Momentum |
| Paladin | 98, 99 | Divine Barrage | +7.5% base damage; +18% critical extra damage |
| Paladin | 100, 101 | Ethereal Barrage | +7.5% base damage; +18% critical extra damage |
| Sorcerer | 102, 103 | Death Echo | +7.5% base damage; +18% critical extra damage |
| Sorcerer | 104, 105 | Lightning | +15% base damage; +22.5% critical extra damage |
| Druid | 106, 107 | Forked Glacier | +7.5% base damage; +18% critical extra damage |
| Druid | 108, 109 | Forked Thorns | +7.5% base damage; +18% critical extra damage |
| Monk | 110, 111 | Chained Penance | +7.5% base damage; +18% critical extra damage |
| Monk | 112, 113 | Thousand Fist Blows | +7.5% base damage; +18% critical extra damage |

Each row represents two modifiers, not one combined damage modifier. The cooldown
modifier's Momentum is an additional effect. The JSON preserves Grades I–IV and labels
their evidence. Planned Monday revisions must be checked again before admission.

### W07 — Remaining donor overrides and Grade IV domain points

**UNKNOWN:** current Global execution is not established for retained Monk Grade IV
Capacity, Flurry damage/critical mapping, Divine Caldera critical value, Heal Friend
cooldown and Combat Mastery thresholds. They are visible in `wheel_effects/gems.rs`;
imported descriptions alone are insufficient to repair them.

**UNKNOWN:** `contribute_gems` adds every fully upgraded modifier directly to each of four
Revelation domains, while Grade IV modifiers also increase the spendable point budget.
Existing tests intentionally assert this donor behavior. A controlled Global boundary
comparison must distinguish spendable points from automatic domain points before changing it.

### W08 — Sorcerer elemental state / Lord of Destruction integration is missing

**FACT, open:** `content/abilities/source-imports/unavailable-spells.jsonl` marks Master
of Flames, Master of Thunder and Master of Decay as `CONTRACT_GAP`. Their required typed
elemental-state, pending conversion and damage/critical modifiers are not admitted.
The Lord of Destruction numbers are projected from the Wheel catalogue without an
identified production numeric consumer.

`RegisteredSpellAccessOwner::read_current` in
`crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs` returns
Premium plus default access fields. The Wheel wrapper fills Wheel/learned facts, not
`AccessProjections.elemental_stance`. Native prepare in
`crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast/prepare.rs` requires
that field when a spell profile declares it. The native path then fails closed if it is
missing. This proves an integration gap; the active deployed spell-profile selection was
not inspected, so an actual live beam/wave refusal is **not** claimed.

## Coverage and execution boundaries

| Surface | Actual owners/consumers | Checked here / remaining work |
| --- | --- | --- |
| Eligibility, budget, connectivity, caps | `crates/gameplay/progression/src/wheel.rs` | Component tests executed; native level/Premium/promotion transitions remain |
| Shared stats and gems | `wheel_effects.rs`, `wheel_effects/gems.rs` | Three repairs, failing regressions, all-grade tests and composed planner fixture executed |
| Knight | `spell/cast/wheel.rs`, `spell/wheel_formula.rs`, `spell/magnitude_owner.rs` | Combat Mastery, Battle Instinct and Battle Healing consumers traced; live numeric qualification remains |
| Paladin | `attack/ranged.rs`, `attack/drain.rs`, native delayed owners | Ballistic/Positional effects, Grenade and Empowerment consumers traced; delayed/PZ runtime tests not executed here |
| Druid | `spell/magnitude_owner.rs`, `spell/native_combat.rs` | Grove, Healing Link/echo and Twin Bursts consumers traced; live threshold/batch validation remains |
| Sorcerer | `spell/magnitude_owner.rs`, `spell/wheel_formula.rs`, `item_spell_augments.rs` | Focus and Runic consumers exist; W08 blocks complete Revelation parity |
| Monk | `spell/cast/wheel.rs`, `owned_cast_facts.rs`, `party_spell_owner.rs`, native delayed owners | Ascetic, Sanctuary, Guiding and Outburst consumers traced; composed preset passes; party/harmony live cases remain |
| Gift of Life / Avatars | `spell/actor_conditions.rs`, `composed_admission/wheel_gift.rs`, `spell/native_combat.rs` | Trigger, cooldown persistence and condition consumers traced; existing receipt/relogin/expiry tests inspected, not executed |
| Vitals and spell facts | `composed_admission/wheel.rs`, `spell_access_facts.rs` | Current allocation/Premium/promotion gates and common projector traced; native reconnect/stale-session acceptance remains |
| Capacity | `crates/gameplay/durability/src/durability/character_wheel/capacity.rs` | Fenced read and conversion to hundredths traced; database/overweight transition tests not executed |
| Resistance | `wheel_effects/gems.rs::gem_value_milli`, `spell/cast/wheel.rs::after_wheel_resistance` | Real 30% cap and HP/mana-drain consumers; progression cap tests pass; official rollout/live cap measurement unresolved |
| Cooldown | `gameplay_transport/item_spell_augments.rs::cooldown` | Real cast path sums item/Wheel/gem reductions and applies 50% base floor; existing consumer test inspected, not executed |
| Reveal/dismantle/enhance/lock/reshape | Progression `atelier/`, durability `character_atelier/` | Component costs/pools/atomic refusals tested; paid writer inspected; PostgreSQL replay/race tests not run |
| Allocation/reset/receipts | Durability `character_wheel/` and `character_wheel.rs` | Fenced owners inspected; forward eight-slot migration qualification remains |
| Protocol/UI/presets | `protocol-oteryn/src/atelier/v2.rs`, client feature `0148-wheel-allocation/` | Twelve-slot mismatch and official preset import checked; new contract/UI/save/relog round trips remain |

Paths abbreviated to `spell/` belong to `crates/gameplay/spell/src/spell/`.
`attack/` and `gameplay_transport/` are under `crates/gameplay/dispatch/src/`;
`party_spell_owner.rs`, `spell_access_facts.rs`, `item_spell_augments.rs` and
`composed_admission/` are within its `gameplay_transport/` directory. The full producer and consumer names distinguish code
inspection from execution. The progression package suite does not run dispatch/PostgreSQL tests.

The historical `runtime_admitted: false` data flags do not mean the Wheel is absent.
Real runtime producers/consumers exist. Conversely, tests against the old catalogue do
not establish current Global parity.

Reshape prices match S2: position 1 costs 125,000 gold + one lesser fragment; position 2
costs 1,000,000 gold + two lesser fragments; position 3 costs 5,000,000 gold + three greater
fragments. Existing code excludes the old modifier and duplicate basic modifiers, keeps
gem identity and uses the paid writer. Proposed later retention of the old modifier is
not treated as deployed. The forum's Fixes & Changes entry 60937 was still empty when
read; it did not establish deployment of the announced 30% cap.

## Bounded repair plan

**RECOMMENDATION:** deliver the remaining update as a coordinated catalogue, persistence,
protocol and client change, with these gates:

1. Pin a fresh reference after the announced update; qualify all 36 Convictions per
   vocation, eight qualities and all 114 supreme IDs. Preserve the current snapshot.
2. Introduce one source-qualified runtime revision. The authoring schema assumes three
   resonance nodes per domain and 94 supreme mods. Use a coherent schema change or a
   versioned central overlay after original digest validation; stages, combat, UI,
   reveal/reshape and Workshop must consume the same catalogue.
3. Classify moved Convictions as a reset revision through the existing fenced reset
   owner. Explicitly migrate compatible paid gem/grade identities; a bare revision bump
   otherwise makes them inert and mutation-refused.
4. Add a forward migration, leaving merged migration 2770 intact. Preserve every gem,
   modifier, lock, grade, balance and paid receipt. Repack to 2 Lesser / 2 Regular /
   4 Greater slots; return surplus gems to inventory without dismantling them.
5. Preserve immutable historical four/twelve-cell receipts and raw request bindings.
   Normalize the prior after-vector for the first new write; support retries for old
   slots 8–11 before new bounds reject them. Do not reinterpret old occurrences.
6. Negotiate a version/capability or qualify an explicit coordinated cutover. Update
   server bounds, legacy projection, client selection, presets and contracts together.
7. Prove empty/sparse/full migration, legacy same-quality overflow, first new write,
   historical replay, repeated restart, old/new clients, current paid grades and
   stale/unauthorized/concurrent replacements. Verify one receipt and no extra fee.
8. Complete the Sorcerer elemental contract/owner and live five-vocation acceptance:
   damage/healing, thresholds, grades, paid operations, Premium expiry, reset and relog.

This PR intentionally delivers the three verified numeric repairs and the audit evidence.
It does not claim that a persisted twelve-slot build can safely use the new layout yet.

## Validation

Four negative observations were reproduced: missing Workshop stats, doubled Mastery,
missing Monk Mana and wrong Spiritual Outburst stage in the exact preset. The patched
progression suite passes 174 tests. Formatter, Clippy and governance results are recorded
in the task packet/PR for the candidate they actually checked. Independent review and
exact-head CI are reported separately. No PostgreSQL integration run, deployed server
acceptance or complete Global certification is claimed.
