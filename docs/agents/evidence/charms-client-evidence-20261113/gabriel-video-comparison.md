# Gabriel: actual two-recording comparison

Both videos were published3May2026, verified in expanded descriptions. Selected
beginning, setup and near-end frames were actually viewed13November2026. This
supersedes the preceding packet's unreviewed-video lead status for these selected
portions; it is not a continuous31-minute replay or proc census.

| Hunt Analyser checkpoint | Low/Savage Blow [31:24](https://www.youtube.com/watch?v=vkxKYG6x42I&t=1884s) | Divine Wrath [31:13](https://www.youtube.com/watch?v=EqqDrky6xKM&t=1873s) |
|---|---:|---:|
| Session display | 00:30h | 00:30h |
| Raw XP gain | 3,146,490 | 3,205,018 |
| Damage | 4,266,685 | 4,275,984 |
| Supplies | 426,961 | 162,593 |
| Healing | 893,913 | 889,358 |
| Falcon Knights killed | 278 | 284 |
| Falcon Paladins killed | 193 | 197 |
| Loot (rounded display) | 1,024k | 1,088k |
| Balance | 597,540 | 926,365 |
| Ping at this frame | 58ms | 82ms |

Divine has58,528 more raw XP (+1.8601%),9,299 more damage (+0.2179%) and10
more kills (+2.1231%). Supplies are264,368 lower (-61.9185%). These are
approximate-duration hunt outcomes, not equal exposure or isolated charm-effect
estimates. Do not claim a universal winner or attribute supplies solely to charms.

## Setup and counter provenance

Both descriptions specify Virtue of Harmony, Sweeping TakedownII, AsceticT2,
Sanguine claws proficiency7 with sixth option Crit Dmg, and Animus Falcon
Knight+3.1%. AsceticT2 is not a verified charm stage. Divine's description says
**Divine Wrath + elemental charm**; the second elemental remains unidentified.
Exact charm stages and race assignments were not established in selected frames.

Blow0:04 shows level716, fist138, life leech50.75%, mana leech16.25%, critical
chance12%, extra damage72% and Sanguine claws in the proficiency window.
Divine0:03 also shows716/fist138; near its end the character is717. Both
descriptions say717, which does not override the beginning UI. Visible XP gain
rate is300%; raw XP is compared separately. Full equipment and potion prices
were not verified equal.

Divine0:03 begins with **prior-run** counters:36min, raw3,226,500, damage4,290,851,
supplies432,649. At0:37 it has reset:0min, raw0, damage0, supplies0, with loot2
and healing78 already recorded. Old counters are excluded. Blow0:04 also has
5min elapsed and supplies2,528 despite zero damage/raw XP: pre-hunt activity
is included. Near-end checkpoints therefore are not manufactured exact30min tests.

The Blow description says the author could use strong health potions but forgot.
The [full Discord conversation](https://discord.com/channels/1431612466818977814/1431635129121505280/1500411983982497792)
qualifies imperfect runs, potion choice,60versus110ms movement effects and a
bounty-talisman follow-up. Single58/82ms snapshots are not a full latency trace.

## Exact Oteryn comparison

Fresh main `3e168ce6afe374485da20ee0ac13d3749558e2f8`, unchanged during this
successor; source bytes are already retained by the preceding packet's receipts.
No Oteryn runtime or game test ran.

- `content/charms/charms-00000-00024.json:1`: Divine Wrath is holy damage,
  `min(5% creature maximum HP,2*attacker level)`, no armor reduction, but subject
  to resistances. Stage activation5/10/11%; Low Blow adds critical chance4/8/9
  percentage points; Savage Blow adds critical extra damage20/40/44 points.
  Unverified video stages cannot select one of these values.
- `crates/gameplay/progression/src/charm_effects.rs:398-420` gates trigger-
  chance stages by deterministic draw;445-456 computes the Divine percentage
  and cap. Assignment admission270-300 rejects duplicate categories per race:
  do not combine two major charms as simultaneously active on one target.
- `crates/gameplay/dispatch/src/gameplay_transport/attack/charm_combat.rs:199-225`
  adds bonuses to corresponding nonzero equipment skills;228-265 handles damage
  armor flags and target elemental response. Divine's source cap is1,432 at716
  or1,434 at717 before resistance transformation. This is source arithmetic,
  not a measured proc from the recordings.
- `crates/gameplay/combat-rules/src/resolution/equipped_offence.rs:93-100`
  returns `floor(damage*(10000+extra)/10000)` for a qualifying effective critical
  draw. Leech156-159 uses `min(real,floor((real*skill+5000)/10000))`.
- `crates/gameplay/dispatch/src/gameplay_transport/attack/equipped_offence.rs:75-97`
  supplies committed HP loss to leech. Aggregate healing includes other effects
  and cannot isolate the helper, AoE/overkill rounding or correlated rolls.

The two damage totals are almost equal, but do not validate individual formulas
without stage/assignment certainty, per-hit target HP/resistance and random-draw
exposure. No mechanic authority is promoted; unresolved interactions remain open.
