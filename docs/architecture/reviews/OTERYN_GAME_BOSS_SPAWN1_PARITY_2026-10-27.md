# BOSS-SPAWN-1 parity (2026-10-27)

Authority: BOSS-RAID-0 §5; NIGHT-W1 decisions 22–24. Source revision:
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. No runtime protocol or boss-room admission is added.

## Baseline audit

The starting checkout and local `origin/main` were both
`ed774e0589c70fd59fc793a59fb62e60f90dfb21`. Migration 0246 already supplied
World/Channel clocks. `claim_respawn_clock_in_transaction` had no production caller;
`read_clock` was called only by that uncomposed primitive and tests. `plan_respawn_death`,
`plan_reset`, and `plan_reset_death` had test callers only. The bundle compiler routed
bosses away from ordinary spawn realization. Ordinary activation and respawn remain separate.

## Rules

| Rule | Canary / accepted reference | Oteryn | Provenance |
|---|---|---|---|
| Classification | Bosstiary or reward boss | Compiler creature facts use either marker; a policy on an ordinary or encounter-bound creature is rejected | BOSS-RAID-0 §5; project.rs |
| Content policy | Respawn interval or reset chance | One externally tagged `boss_policy`: positive ordered seconds, or positive exact chance ratio ≤1; floats, extra fields and multiple tags fail | BOSS-RAID-0 §5; source loader tests |
| Legacy placements | Old static boss placements were already excluded | The 59 old boss points without qualified scheduling policy are preserved in generated held-bosses.json, outside active spawn shards. An active boss entry without policy now fails compilation | Existing SPAWN-CONTENT-1 routing |
| Scope | World and Channel independently | Composite PostgreSQL primary key; draw seed includes both separately | Migration 0246; native clock PG test |
| Due claim | Realize only a passed deadline | DB time after authority locks; whole-row CAS; install only after confirmed commit | BOSS-RAID-0 §5 |
| Restart alive | Lost fight state is discarded | Higher committed ownership generation claims the old ALIVE clock once; runtime installation uses maximum health | BOSS-RAID-0 §5; D52 |
| Death | One live-generation descendant | Clock CAS is in the corpse receipt transaction; rollback undoes both; receipt replay never advances twice | DUR-03 A4; real-dispatch PG tests |
| Respawn | New cycle, seeded interval | `plan_respawn_death` sets cycle+1 and portable due time; boss actors are absent from ordinary respawn sources | BOSS-RAID-0 §5 |
| Reset draw | Once per clock and real reset epoch | Reads/locks active reset pointer and activation time; no synthetic reset when uninitialized; epoch zero is supported | WorldReset owner; BOSS-RAID-0 §5 |
| Reset death | Inactive until next epoch | `plan_reset_death`; same-epoch activation remains inactive | BOSS-RAID-0 §5 |
| Timers | Future deadlines survive owner changes | DB deadline retained; monotonic local timer suppresses repeated DB claims before due; DB rechecks deadline on wake | BOSS-RAID-0 §5 |
| Commit uncertainty | No claim snapshot grants actor authority | Claim/installation failure retires the owner; serving ends rather than retrying an uncertain claim as a new boss | Existing clock API contract |
| First respawn clock | Section 5 does not specify initialization | Proposed: first activation plus deterministic cycle-zero interval; inserted once. Owner clarification requested | Explicit implementation assumption; pending owner answer |
| Binary compatibility | Existing ordinary spawn encoding | Ordinary bytes unchanged; zero ordinary delay tags a boss policy; runtime compatibility raised to 3 | World bundle runtime version gate |

## First batch

Coordinates below are original source `(x,y,z)`; runtime floor is `-z`.
Every frequency below is **provisional, not verified against Global**. For script bosses the
exact `targetChancePerDay` becomes one reset draw. This deliberately does not reproduce Canary's
intra-day chance accumulation, player-count eligibility, broadcasts, supporting waves, or raid
lifetime. Zevelon's XML scheduler is converted to an explicitly provisional uniform range with
the same nominal interval; its margin is a source scheduler parameter, not evidence of a Global
minimum respawn. No kill-history tables or player data are copied.

| Boss | Source position | Canary frequency | Oteryn policy | Provenance and derivation |
|---|---|---|---|---|
| Arachir The Ancient One | (32964, 32400, 12) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/arachir_the_ancient_one.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/arachir_the_ancient_one.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Diblis The Fair | (32009, 32795, 10) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/diblis_the_fair.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/diblis_the_fair.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Furyosa | (33281, 31804, 15) | targetChancePerDay = 1/100 | at_reset 1/100 | canary-pin: [data-otservbr-global/scripts/raids/bosses/furyosa.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/furyosa.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Hirintror | (32101, 31167, 9) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/hirintror.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/hirintror.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Sir Valorcrest | (33264, 31768, 10) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/sir_valorcrest.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/sir_valorcrest.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| The Old Widow | (32776, 32296, 7) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/the_old_widow.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/the_old_widow.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| The Pale Count | (32969, 32420, 15) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/the_pale_count.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/the_pale_count.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Weakened Shlorg | (33164, 31716, 9) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/weakened_shlorg.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/weakened_shlorg.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| White Pale | (33264, 31875, 11) | targetChancePerDay = 1/50 | at_reset 1/50 | canary-pin: [data-otservbr-global/scripts/raids/bosses/white_pale.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/raids/bosses/white_pale.lua); targetChancePerDay converted exactly to one at_reset chance; intra-day checks, raid waves and minimum-player gate are not implemented by this spawn policy. |
| Zevelon Duskbringer | (32754, 31578, 11) | interval2 1440 minutes; margin 90 minutes | respawn [5400, 167400] seconds | canary-pin: [data-otservbr-global/raids/farmine/zevelon_duskbringer.xml](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/raids/farmine/zevelon_duskbringer.xml); Provisional uniform range: min=margin(90)*60=5400; max=2*interval2(1440)*60-min=167400; mean=86400 seconds. This is not the Canary random raid scheduler. |

The JSON declarations retain each source path, immutable revision, derivation and provisional
status. Demodras and Grand Mother Foulscale are not in this first batch: their frequency and
placement were not inferred from names or unsourced folklore.


## Qualification

All 10 new points are admitted by the compiler's final parity pass. Active source inputs have
54,667 sources and 87,795 points; the compiled family has 51,098 sources and 82,697 points.
The 59 legacy boss points are retained outside active shards, with their source facts and reason.
The generated World bundle is 30,924,998 bytes, digest
`9ba81732e85a40d3b99bdb80e94fac8b7ae191adcc0f14d18c69d0ff7760927b`.
Focused PostgreSQL cases cover confirmed claims, full-health owner recovery, independent channels,
reset draws/INACTIVE, transactional death/replay and occupied placement. Normal pre-transaction
holder contention is deferred without retiring the owner. Exact commands/results are retained in
`BOSSSPAWN-FINAL.md`.
