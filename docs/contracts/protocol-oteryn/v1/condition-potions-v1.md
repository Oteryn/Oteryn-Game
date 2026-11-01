# Condition potions through the existing item-use owner

This extension reuses `USE_INTENT` (command 2), `ITEM_USE_V1` (capability 15), the
carried item admission path, and the existing `Potion` audit cause (2). It adds
no command, capability, condition engine, HP/MP restoration or empty flask.
PR #2782 answers 6547429543 (Q4) and 6547466349 (Q6) authorize these four
condition consumers and the correction of their unreachable donor granters.

## Source rules

The sole donor is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data/scripts/actions/items/potions.lua:1-31,52-54,63,78-80,125-129,141-147`.
The source's `SKILL_MELEE` changes club, axe and sword only
(`src/creatures/combat/condition.cpp:912-917`). Promoted vocations use their
base vocation, as in the existing item requirements owner.

| Item identity | Allowed base vocations | Temporary effect | Achievement after 100 successful applications |
|---|---|---|---|
| `oteryn:item.tibia.i7439` | Knight | Club, axe, sword +5; shielding −10 | Berserker |
| `oteryn:item.tibia.i7440` | Sorcerer, Druid | Magic level +3 | Mastermind |
| `oteryn:item.tibia.i7443` | Paladin | Distance +5; shielding −10 | Sharpshooter |
| `oteryn:item.tibia.i49271` | Any, including no vocation | Fist +4 | Inner Peace |

Every effect lasts exactly 600,000 ms. The existing condition consumer resolves
live adjustments and expiry; durable base skills are unchanged. A condition
refused by that owner, including `KeptCurrent`, consumes nothing and counts
nothing. These four uses do not enter the separate health/mana Potion Addict
granter (`potions.lua:83-110`).

All four use sub-ID **0**, reproducing the donor's executable behavior. The
script references bare `JeanPierreMelee`, `JeanPierreMagicLevel` and
`JeanPierreDistance`; `lua_enums.cpp:38-43,479-483` exports only `AttrSubId_*`
names. The unchecked `lua_tonumber` conversion in
`lua_functions_loader.hpp:74-88`, called by
`condition_functions.cpp:188-196`, converts the missing values to zero.
Consequently the effects replace each other in the same condition slot.
The replacement copies all new skill adjustments in
`condition.cpp:552-592`; they do not stack. This implementation does not repair
those donor names or substitute the apparently intended enum values.

PR #2782 answer 6547886810 (Q9=A) also binds that zero slot to the existing
`Attributes` representation. Crystal's Swift Foot creates a 10-second
attribute debuff in the same slot (`data/scripts/spells/support/swift_foot.lua:1,35-43`).
An incoming potion replaces that shorter attribute condition; an incoming
Swift Foot attribute condition is kept out while the potion has more than
10 seconds remaining. The common `ConditionStore` owns this conflict,
dispel and immunity identity; typed reads, condition values and nonzero
skill sub-IDs stay unchanged. The source identity and remaining-duration
comparison are `creature.cpp:1409-1416` and `condition.cpp:436-447,552-592`.

## Commit and recovery boundary

The source record must exist in the independently pinned native item index and
policy. The item-use gate first qualifies the carried source, exact actor,
current session and vocation. The new module prepares the real
`OwnerCombatChange::PlayerConditions` with the existing `ConditionStore`, then
stages both physical and player-owner successors. The actual actor slots are
reserved before a SQL commit can occur.

The durable writer independently checks recovery, node, session generation,
character lease, scope generation and active content. Under the same item/root
locks it performs the existing one-unit burn, writes an immutable application
receipt and advances the appropriate existing achievement counter. Migration
0707 requires the ordinary item-use record and its application receipt in the
same physical transaction; both must name the exact source item, Potion cause
and burn shape. The receipt binds the prepared condition transition and its
original application/expiry times. An exact replay returns that receipt
without another burn, count or condition preparation.

For a known commit, runtime and spell-state guards remain held through the
existing checked, infallible owner installation. If cancellation or an
uncertain commit interrupts this phase, the existing spell lane retains the
complete prepared physical batch and player successor. Their reservation
blocks competing actor mutations. The production lane resolver waits for the
same database locks and either installs the committed original application or
releases the reservation after proving the receipt absent. It never repeats
consumption or refreshes the duration. Same-session connection advancement
requires the existing admission journal's pending-command reconnect proof;
a different lease, session or scope cannot acquire an old application.

The existing shared potion cooldown and item-use rate limit remain in force.
Their existing `PARITY_PENDING` cooldown value is not a new Crystal claim.

## Cross-session lifetime

The existing native condition owner starts fresh admission with no temporary
instances, except separately owned food restoration
(`foundation/condition.rs`, `ConditionStore::new`, COND-DUR-1). Crystal saves
timed `CONDITIONID_COMBAT` conditions at clean logout:
`condition_functions.cpp:52-64`, `condition.cpp:356-369`,
`iologindata_save_player.cpp:264-276`; it reloads them through
`iologindata_load_player.cpp:287-304` and `player.cpp:11565-11568`.
PR #2782 answer 6547749872 (Q8=A) explicitly preserves this native lifetime and
retained same-session reconciliation. **Left out dependency:** the
coordinator's **CLOSE-2** shared timed-condition persistence/restoration owner,
reserved migrations 0690–0699, supplies cross-session persistence. This extension does
not create its own saved-condition store. A receipt is never treated as
current authority for a new session.

## Executable coverage

- `condition_potions_tests.rs`: exact bonuses and the microsecond expiry
  boundary, source sub-ID collision, `KeptCurrent`, and time-overflow refusal.
- Mounted `postgres_tests/condition_potions.rs`: one real native-content,
  framed `USE_INTENT` control for each potion; live combat skill consumption;
  replay and vocation/stale-generation refusal without progress; definite
  rollback release; lost acknowledgement followed by receipt reconciliation
  with the original expiry; the 100th-use grant once; no Potion Addict count.
- The four mounted scenarios begin with a successful-use assertion. They fail
  when only the final item-use dispatch arm is disconnected, then qualify the
  connected consumer without a tests-only replacement path.
- `foundation/condition_attribute_slot_tests.rs`: the three Q9 regressions
  cover Swift Foot followed by a potion, the reverse shorter-condition
  refusal, and nonzero sub-ID independence, including shared dispel,
  suppression and post-Cleanse immunity.

The disconnected-arm PostgreSQL RED run on 2026-10-31 reached all four real
`USE_INTENT` assertions: each returned `NothingToUse` where `Committed` was
required (0 passed, 4 failed; exit 101). The integration record is
`red-potions-5.log`. After connecting the production arm, `green-potions-7.log`
records all four scenarios passing (4 passed, 0 failed) and distinct
`Q6 POSTGRES_CASE_COMPLETED` markers for Berserk, Bullseye, Mastermind and
Transcendence. Those complete scenarios include the live buff and expiry,
refusal and replay, definite rollback release, and uncertain-commit
reconciliation retaining the original expiry. The PR evidence records the
exact commands and integration environment.

The same three Q9 slot tests were executed before and after the common-owner
change: `red-slot-6.log` records 0 passed / 3 failed, and `green-slot-7.log`
records 3 passed / 0 failed. The RED assertions observed coexisting slots and
an accepted shorter condition; the GREEN run verifies the shared mutation,
dispel and immunity boundary without changing typed reads.

Test execution results belong in the task evidence and PR; this contract does
not claim that a pending run passed.
