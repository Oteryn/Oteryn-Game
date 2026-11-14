# Global reference: Leech and passive spell critical implementation

Task: `OTV2-20261113-charms-leech-evidence` (coordination alias `CHARMS-LEECH-EVIDENCE-20261113`).
Base: `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`.
Scope: numerical Leech and Low Blow / Savage Blow on direct native and ordinary spells.
This is a bounded implementation slice, **not a claim that all six researched mechanics are complete**.

## Reference packet and chosen policy

[Research PR #3177](https://github.com/Oteryn/Oteryn-Game/pull/3177), evidence head
[`97685d43a194010559a3a92821089067dbb147a3`](https://github.com/Oteryn/Oteryn-Game/blob/97685d43a194010559a3a92821089067dbb147a3/docs/reference/mechanics/tibia-charms-leech-20261113-evidence-rounds.md)
contains the complete four research rounds and their corrections. It is research, not a declaration
of current-runtime parity. The accepted project reference date remains **2026-09-27**.

| Rule | Evidence / classification | Implementation |
|---|---|---|
| Ordinary area Leech distributes `f(N)=(N+9)/(10*N)` to each target | [Wiki Formulae, revision 1205374](https://tibia.fandom.com/index.php?title=Formulae&oldid=1205374); [Tibia Analysers Jan 2026](https://discord.com/channels/1431612466818977814/1431633212995670078/1462060286453485578). `DERIVED`, well-supported working model | Shared integer helper, one ceiling per target, then sum |
| Four 679-damage falcon hits give 117 HP each | [Observed log, 2026-01-24](https://discord.com/channels/1431612466818977814/1431633212995670078/1464603474837635124). 53% and 52.75% both fit; separate Monk heal is not Leech | Regression verifies 468 HP, without adding the unrelated 145 HP passive heal |
| Overkill is included; Damage Prey is not | Wiki revision above and research R1/R4. `DERIVED`, not a new Global experiment | Capture admitted damage before Prey and before HP clipping; retain critical/mitigation already resolved by existing owner |
| Minor Leech bonus applies additively only to the assigned race | [tom, 2025-07-03](https://discord.com/channels/836592213327609957/836593610811768833/1390345009844457595); [Alytreta, historical test](https://www.youtube.com/watch?v=MxiCFl3OT_8); current stages from existing pinned catalogue | Project passive per-race rates; preserve existing rule that a minor augments an existing leech skill |
| Beam centre and combined sides have different groups | [Porfz, 2026-07-15, 16:23](https://www.youtube.com/watch?v=YKA_VqNX1eI&t=983s); [Sabat Sep 2026](https://discord.com/channels/836592213327609957/1405864725367095317/1548629834958110781). `DERIVED` model, not three independently proven columns | Use actual `side_percent` classification from qualified native plan; retain 2/6 grouping, not an inferred damage ratio |
| Qualified zero-damage / immune targets can reduce other targets' Leech | [Historical test report](https://www.reddit.com/r/TibiaMMO/comments/ji85yi/how_is_voids_call_calculated_for_aoe/); [Cereyon](https://discord.com/channels/836592213327609957/836593610811768833/1415696735207952507). `DERIVED`, exact eligibility remains version-sensitive | Preserve immediate qualified area count before `lower_native` omits zero-damage effects; zero targets themselves restore nothing |
| Low Blow produces a common additional critical result for eligible targets | [CipSoft news 5268](https://www.tibia.com/news/?id=5268&subtopic=newsarchive), [Liamas 2026](https://www.tibia.com/forum/?action=thread&postid=39596416#post39596416). `PROVEN` for official description, not internal RNG implementation | Cache one numerical critical draw for the cast; compare against each target's own threshold. No bonus leakage to unassigned targets |

The preserved `damage` upper cap for pathological rates above 100%, existing PvP eligibility,
Talisman/elite multiplier policy and delayed-effect policy are not newly qualified Global rules.
No new content stages, client flags, database schema or authorisation grants are introduced.

## Code and owner path

- `combat-rules/src/resolution/equipped_offence.rs`: exact `u128` ratio and ceiling;
  zero skill/damage/target count returns zero; per-contribution damage bound is retained.
- `dispatch/.../attack/drain.rs` and `attack/equipped_offence.rs`: the current auto-attack
  retains a second damage basis excluding only Damage Prey. An applied receipt is still required.
- `dispatch/.../spell_leech_projection.rs`: one private projection from the qualified cast;
  per-target damage, rates and target count are retained across an in-memory retry. Creature
  identity comes from the current runtime; passive charm state comes from the bound character.
- `spell/src/spell/native_combat.rs` and `magnitude_owner.rs`: qualified immediate group sizes,
  shared critical draw and per-creature passive critical thresholds/extra damage.
- `dispatch/.../native_combat_cast.rs`: capture before Prey, preserve existing Talisman/elite
  scaling, and feed gains into the existing joined physical/player owner install.
- `actor_spell_commit.rs`: no extra heal after the commit; the original fenced replacement
  incorporates both pools and analyser reports only the actual HP gain.
- Native/ordinary direct spell preparation receives the existing content-owned charm catalogue;
  healing and non-creature critical targets do not acquire creature charm bonuses.

**Recovery boundary:** no change to the persisted source JSON or to deferred parent binding format.
The prepared cast retains its original numerical projection. Existing physical receipt replay,
player-state revision comparisons, current lease/session checks and atomic install remain in force.
A stale player proof must install neither damage nor regeneration. An unapplied/replayed hit
must not grant Leech a second time. Max HP/mana clips the credited resource, not the damage basis.

## Regression evidence

The following failures were reproduced before their production fixes:

1. `leech_global_nonzero_fraction_rounds_up`: `24 * 2%` returned `0`, expected `1`.
2. `global_spell_leech_uses_pre_overkill_damage_and_does_not_replay`: 100 calculated damage,
   victim at 20 HP, 16% leech credited 3 HP instead of 16. The fixed actual owner credits 16;
   a duplicate batch does not credit again.
3. `global_low_blow_aoe_shares_one_roll_but_not_the_race_bonus`: the real magnitude owner
   ignored the supplied eligible-target bonus. Expected two 14250 hits and one 9500 hit;
   the original path returned three 9500 hits. The fixed path keeps the correct result
   even when the unassigned target is evaluated first.

4. `global_native_area_crit_is_shared_without_beam_mastery`: the actual native Energy Wave planner
   did not request a shared area critical. The corrected planner does, also for unenhanced
   Energy Beam, Great Energy Beam and Strong Ice Wave, without changing chain/healing policy.

Additional tests cover the old 3-target HP/MP log, 4 falcons, ordinary 16%/17.6% per-race rates,
zero/maximum arithmetic, the 852-vs-538 beam example, qualified zero damage, delayed-hit exclusion,
resource caps, stale-player rollback, retained projections, the real character/race assignment
owner, Savage Blow extra damage and no replacement after a shared critical roll.

For the beam example, **18% is an explicit modelling assumption** from the research context;
its regression is not a claim that the test character's complete equipment was independently read.
Exact intermediate rounding (`ceil(ceil(D*L)*f)` versus a single final ceiling) remains uncertain;
this change chooses the directly documented single ceiling rather than fitting one small sample.

Validation command output and exact frozen SHA are recorded on the implementation PR. Local
PostgreSQL-route tests without `OTERYN_TEST_POSTGRES_ADMIN_URL` do not establish database coverage.
The real PostgreSQL harness is required before merge; no local result is represented as that run.

## Six-topic audit and remaining boundary

| Topic | Observed code / decision | Remaining work or evidence |
|---|---|---|
| Leech | This slice fixes direct-hit damage basis, ordinary areas, qualified beam groups, rounding and minor per-race rates | No universal claim for every delayed/DoT/PvP/summon interaction; precise intermediate rounding remains a research question |
| Low Blow | Passive bonus now reaches actual native/ordinary direct-spell magnitude owner with shared outcome | Dedicated diamond-arrow off-target execution must be audited separately; this PR does not add a new ranged-area engine |
| Parry | Existing `attack/charm_combat::damage_amount` distinguishes armor and resistance ignoring; incoming hooks retain their existing base | Real Mana Drain and complete modern mitigation ordering remain `UNKNOWN`; no invented conversion is added |
| Dodge + minor | Existing actual Dodge/condition path is preserved | A blocked/zero-damage contact needs its own replay-safe admitted-contact path; “poof” must not be equated with Dodge. Not implemented by this slice |
| Carnage | Existing auto-attack consumer handles cardinal radius, capped damage, last-hit event and no charm recursion | Native spell post-kill integration and full secondary kill-credit/loot/XP need their own owner-level tests; no full completion claimed |
| Cleanse | Existing condition owner keeps distinct immunities and rejects replay; Hex is already owned by separate PR #3176 | Same-type timer refresh and summon-edge cases remain uncertain; no competing edits to #3176 |

**Conclusion:** this is an executable, reviewable correction of the confirmed numerical and passive
spell-hook gaps, not a rewrite of all charm owners or a declaration of 100% Global parity. Do not
promote the mechanics ledger, merge, deploy or mark the entire six-mechanic task complete based
solely on this slice.

## Existing PostgreSQL regression alignment

The two equipment-spell cases in `postgres_tests/parameter_spell.rs` now use the
source-bound Demon rather than Rat and assert that the hit is nonlethal. They retain
the exact existing 100% health/mana-leech equations and duplicate-replay assertion.
This prevents the old fixture from accidentally asserting HP-capped Leech after the
overkill correction. New actual-owner regressions separately require full overkill
recovery. No fixture rewrites native content, bypasses identity checks or weakens an
assertion. These database cases still require the real PostgreSQL CI run.
