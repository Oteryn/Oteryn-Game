# Qualification clock audit

Base `849fa2b7b93c2587fac48eb235cdd0492b2155b0`. This is a process-local offset
used by disposable node boot. The timing numbers below are row totals supplied
with the audit request. They do not attribute elapsed time to individual steps.

## Findings

- **P1, fixed — Out-of-range input permanently poisoned both clocks.**
  `crates/gameplay/world/src/world_clock.rs:135` and `:156` accepted
  `18446744073709551615` into the atomic maximum. The next World UTC read
  failed checked arithmetic, and repairing the file could never restore UTC
  packets in that process. At `crates/gameplay/dispatch/src/gameplay_transport/monk_save.rs:54`,
  the same value saturated elapsed microseconds to `u64::MAX`, so subsequent
  checked deadlines could fail. Validate the offset before publishing it and
  reserve headroom for signed microsecond arithmetic. Bound the size of the
  record before parsing to limit allocation and reject oversized input. An extracted Rust test
  running the original clock and the regression assertion failed with
  `left: 18446744073709551615, right: 75000`; it passed with the filter.
- **P1, fixed — Feature builds could start with an ordinary config.**
  The base gate at `apps/game-server/src/node/config.rs:604` only rejected
  ordinary configs when the clock environment variable was set. The serving
  entry now requires the disposable fixture even with the variable absent,
  before reading TLS material or binding a socket (`node/serve.rs:550`).
- **P1, SERVER-GAP — Scope-wide jumps change live mechanics unrelated to the
  row.** `crates/gameplay/dispatch/src/gameplay_transport/monk_save.rs:43`
  supplies a single owner time to every live actor. Advancing during a
  multi-client fight can expire player conditions and PvP locks, replenish an
  unrelated spell cooldown, or expire a party member's recent participation
  before the kill. A timer-specific or isolated owner needs a supported scope
  and a consistent deadline and persistence model.
- **P1, SERVER-GAP — Durable timestamps can outlive the in-memory high-water
  mark.** `crates/gameplay/durability/src/durability/raid_schedule/activity.rs:109`
  only accepts a same-generation presence report if its observation time
  increases. `crates/gameplay/dispatch/src/gameplay_transport/raid_live.rs:697`
  and `gameplay_idle_presence.rs:237` supply the advanced UTC. If a process
  restarts with the same generation and a reset or missing file, the report
  stays future-dated and the current-window read at `activity.rs:128` treats
  the scope as unknown. Reusing a database across accelerated fixture
  lifecycles needs an accepted durable clock or a new scope generation.
- **P2 — Atomic file publication is a fixture writer invariant.**
  `apps/game-server/src/qualification/client_phases/full_e2ef7.rs:1092`
  publishes by same-directory rename. Main already rejects parse errors and
  retains the last observed maximum. A partial but syntactically valid
  integer from an in-place writer is indistinguishable from a complete input.
  Two writers also share the same temporary path and can lose cumulative
  increments. `tools/qualification/node_boot/run.sh:735` initializes the file
  before use, and the W20 writer is sequential. Keep a single writer and
  atomic replacement for any future use.
- **P1, fixed — A stray clock variable broke ordinary W20.**
  `full_e2ef7.rs:1067` now gates W20 clock path lookup and the writer at
  compile time. On the original W20 helpers,
  `ordinary_world_light_keeps_real_clock_with_offset_env` failed with a
  120-second deadline instead of 3,750 seconds. The candidate helpers passed
  and the advance was a no-op.
- **P3 — Sampling costs.** Every accelerated World/owner read opens and parses a
  file synchronously (`world_clock.rs:156`). Measure file reads per owner tick
  before adopting this mechanism for combat-heavy rows.

## Build and clock behavior

Main already wires the feature through `apps/game-server/Cargo.toml:24`,
`crates/gameplay/dispatch/Cargo.toml:16` and
`crates/gameplay/world/Cargo.toml:16`. The CI build delegates to
`tools/qualification/node_boot/prebuilt.py:35,134`, which packages the
`qualification-clock,qualification-spawn` variant
(`.github/workflows/node-boot-build.yml:139,151`). The runner chooses it for
disposable World mode (`run.sh:208`), and the README describes the two
existing accelerated rows (`tools/qualification/node_boot/README.md:48`).
The file reader, UTC addition and dispatch consumer use compile-time `#[cfg]`. The W20 reader is also cfg-gated
(`full_e2ef7.rs:1081`), and the writer's filesystem block is excluded in the ordinary build
(`:1103`). The default branch of the World regression probes a
real file carrying a valid nonzero offset and checks that UTC does not jump.

Main already rejects the environment variable alone at
`apps/game-server/src/node/config.rs:604`. The disposable fixture also requires
one of the two node-boot database names and the node-boot source authority
(`:638`). The existing `qualification_clock_requires_disposable_configuration`
test exercises the enabled environment, and
`qualification_fixture_requires_the_disposable_node_boot_configuration`
covers accepted and rejected fixture identities. The first test now checks
the startup parser with and without the clock variable. Ordinary configuration
parsing remains available to non-serving callers and test fixtures.

Each process uses a single atomic maximum for both clock readers. The UTC
anchor uses one SystemTime sample and a monotonic elapsed value. The dispatch
clock uses its owner origin and converts elapsed time to microseconds. Neither
uses an offset to mutate a standard-library or Tokio Instant. The upper bound
guards conversions for the current epoch and normal fixture lifetimes,
and UTC additions remain checked. The process-local maximum does not establish
durable ordering across restarts or synchronize simultaneous samples in
different processes. Successive non-overlapping reads by one caller are monotonic. Concurrent
calls can linearize in either order. Use the same, continuously published
offset file for the entire disposable run.

## Time sources and scenarios

Search roots were `crates/gameplay/**` and `apps/game-server/src/**`. The
search included `SystemTime::now`, `Instant::now`, `tokio::time`,
`chrono::Utc::now` and `OffsetDateTime::now_utc`, plus elapsed reads of
`clock_origin` and SQL `now()`, `clock_timestamp()` and
`CURRENT_TIMESTAMP`. Neither chrono nor time's UTC constructors appeared.
Test fixtures, watchdogs, wall-time measurements and telemetry are real time
by design. Relevant source groups and failure modes follow. Relative
gameplay filenames in this section refer to
`crates/gameplay/dispatch/src/gameplay_transport/`.

| Source | Classification and consequence |
| --- | --- |
| `connection/authority.rs:546`, `connection/serve/snapshot.rs:650`, `connection/serve/refresh.rs:752` | World advertisement and light state use accelerated UTC. W20a gets its next observation on the next real refresh. Weather and `novelty_item_use.rs:185` share the World sample, so a light jump can skip a weather/day boundary used by another row. |
| `node/serve/supervision.rs:388`, `node/serve/channel_boot.rs:391` | Timed NPCs use accelerated World UTC. Crossing both ends of a spawn window between samples can skip the NPC entirely. The 2.5-second poll remains real time. |
| `monk_save.rs:43`, `native_combat_cast.rs:2664`, `composed_admission/vitals.rs:92` | Spell cooldowns, live conditions and owner vitals consume semantic elapsed time. A jump can make an immediate recast succeed or skip the observable sequence of condition ticks. Offline restoration uses separate durable time facts. |
| `item_use.rs:362`, `item_craft.rs:345`, `weapon_proficiency_select/modify.rs:291`, `hotkey_equip.rs:205` | Raw elapsed cooldowns. A jump large enough to expire a spell cooldown can still leave a potion, craft, proficiency modification or equip action exhausted. |
| `map_ladder.rs:120`, `bundle_ladder.rs:137`, `dawnport_departure.rs:185`, `viola_book.rs:108`, `falcon_chests.rs:109`, `cake_event.rs:567`, `house_base_decorations.rs:472`, `soulpit/passages.rs:168`, `quest_callbacks.rs:1196,1245`, `quest_callbacks/rafzane_elevator.rs:59`, `quest_callbacks/soul_war_reward.rs:168`, `bundle_chest.rs:218`, `composed_admission/objects.rs:61,197`, `composed_admission/objects/dropdown.rs:141` | Raw elapsed admission or use gates. Retrying each action after a jump can still hit its real-time gate, e.g. the ladder refuses another traversal. Leave input throttles real during ordinary gameplay observations. |
| `timed_item.rs:165,264`, `timed_item_clock.rs:107,288`, `equipment_field.rs:115`, `imbuement_clock.rs:36` | Raw owner elapsed time for item transitions, equipped effects and imbuement. An item can remain active after the asserted expiry, and P21 cannot use the shared offset to prove duration consumption. |
| `attack.rs:1232,1244,1257`, `connection/serve/{creature_push.rs:25,42,autowalk.rs:272,370,target_move.rs:44,146}` | Real chase, push, browse and movement timers. An advance after scheduling a creature push does not move the creature early. M04 and hunts keep their step and network observation budgets. |
| `arena_match.rs:203,721`, `tibiadrome.rs:297,593`, `soulpit/cadence.rs:15,40,45`, `activity_cycles.rs:18,101`, `source_item_cycle.rs:80`, `server_save_countdown.rs:111` | Real activity cadence and pending timers. An accelerated arena deadline/round, Soulpit wave, source item cycle or save-countdown observation may still require the original real wait. A later owner sample can see accelerated elapsed time only after the worker polls. |
| `connection/serve/setup.rs`, `listener.rs`, `fresh_evidence.rs`, `resume.rs`, `composed_admission/grace_expiry.rs:190`, `crates/gameplay/durability/src/durability/db/{foreground,holder_queue,ready_admission}.rs` | Harmless real admission, transport, durable retry and I/O budgets. Skipping these deadlines could time out a request before a real retry or falsely claim durable release. |
| `world_respawns.rs:23,32,67,135,186` and `crates/gameplay/world/src/movement/monster_map.rs:429` | Ordinary spawn clock and pending delay bypass the offset, boss rechecks and durable spawn availability also use real time. Killing the only creature and jumping a full respawn interval can still leave an empty hunt. |
| `corpse_decay.rs:34,48,61`, `timed_item_clock.rs:165` | PostgreSQL owns corpse decay, and the real-time item worker polls it. I07 would still see a corpse after the entire simulated decay interval. |
| `pvp_live_locks.rs:131`, `connection/authority.rs:240`, `gameplay_idle_presence.rs:192,237` | Owner-based PvP predicates can advance, but idle warning/removal uses real time and presence projects that real elapsed time on an advanced UTC observation. A row that jumps during logout-refusal evidence can skip the lock. A row that expects idle removal sooner can hit its wall-time watchdog. |
| `actor_spell.rs:407`, `kill_reward.rs:1183`, `boss_reward_contribution.rs:255`, `crates/gameplay/entities/src/carrier/damage_ledger.rs:405` | Real durable/ledger time. An offset jump neither ages stored contributions and corpse timestamps nor changes offline elapsed time. This is appropriate when the row observes actual relogging or persisted receipts, and wrong if it assumes those records aged by the jump. |
| `last_position.rs:375`, `character_info.rs:90`, `npc_trade_command.rs:109`, `npc_command/catalogue.rs:284`, `guildhall_intent.rs:41`, `guild_intent.rs:115`, `guild_bank.rs:79`, `guild_war.rs:58`, `player_trade/command.rs:559`, `weapon_proficiency_select/{modify.rs:192,catalyst.rs:166}`, `hireling.rs:144`, `item_use.rs:1232`, `mount_taming.rs:458`, `native_world_item_cast.rs:1272`, `mining/mod.rs:252` | Wall-stamped commands, audit, position, economy and social actions. Harmless for their existing rows. Comparisons of these timestamps with simulated time would misstate age or skip a cooldown enforced by a durable owner. |
| `crates/gameplay/foundation/src/scheduled_events.rs:267`, `node/serve/bundle_npcs.rs:541`, `yeo.rs:290`, `crates/gameplay/durability/src/durability/creature_source_items/primal_pod.rs:695` | Real calendar, NPC and durable effects. A World-light jump does not necessarily open a scheduled event, refresh a date-gated NPC or satisfy a durable pod deadline. |
| `crates/gameplay/social/src/premium/{refresh.rs:230,client.rs:275}`, `crates/gameplay/social/src/platform_streams/{profile_b/runtime.rs:527,enforcement/worker.rs:317}`, `crates/gameplay/social/src/cast/{service.rs:103,audit.rs:24}`, `crates/gameplay/native-admission-source/src/runtime_status.rs:408`, `node/serve.rs:432` | Harmless and required for externally interpreted expiry, freshness and timestamps. A Game offset cannot simulate expiration of a valid Platform ticket, premium refresh, stream lease, observer credential or attestation. Advancing these clocks locally would falsely reject still-valid external evidence. |
| `crates/gameplay/node-files/src/operator_files.rs:178`, `node/serve/audit_collector.rs:175`, `bin/oteryn-game-ops.rs:193`, qualification harness std/Tokio measurements | Harmless wall-clock operations and observation budgets. Receipt order and real run duration should remain interpretable after a jump. |

## PostgreSQL boundaries

- `corpse_decay.rs:48` compares authoritative SQL time to SQL-held
  deadlines. `crates/gameplay/durability/src/durability/item_decay_retire/retirement_workflow.rs:480,554`
  reads the schedule and revalidates the retirement. An application-only
  expiry would disagree with the writer. **SERVER-GAP** for accelerated I07.
- `crates/gameplay/durability/src/durability/world_party.rs:548,589,599,821,832,869,873`
  evaluates invitation and channel-switch expiry in SQL. The row cannot age
  them with owner time. Character and cast/session leases similarly use the
  real database authority (e.g. `admission_journal.rs:1698`,
  `cast_privacy.rs:91,272,306`). **SERVER-GAP** for accelerated lease or
  invitation expiry.
- `crates/gameplay/dispatch/src/gameplay_transport/party_skull.rs:140,146,151`
  combines owner-time shared-XP activity with SQL-clock skull, PZ, revenge,
  aggression and invitation predicates in the same projection. A jump can
  blink the party shield while SQL still sees the PZ lock or an unanswered
  invitation. `crates/gameplay/durability/src/durability/pvp_state.rs:152,335`
  writes and calculates the remaining time from PostgreSQL.
  `pvp_live_locks.rs:131` rebases SQL-derived lock durations on
  the latest owner time, so a durable refresh can restore a lock that a
  jump appeared to clear. **SERVER-GAP** for using a jump as proof of release
  or of party inactivity. `player_field.rs:429` also samples the SQL black
  skull during an owner-driven condition tick, so accelerating the tick can
  change which condition events observe that real-time damage modifier.
- `chat_relay.rs:448` takes the replay window epoch from PostgreSQL and
  anchors progress to a real `Instant`; the qualification offset cannot
  advance its database lease or move the replay floor.
- `crates/gameplay/durability/src/durability/character_death/{commit.rs:283,forecast.rs:136,adventurer.rs:25}`
  compares skull expiry to `clock_timestamp()`. An owner-clock lock can be
  cleared while a skull remains active to those readers. **SERVER-GAP** for
  acceleration across an S08/MP01 durable skull assertion.
- `apps/game-server/src/node/serve.rs:1335` is a real cadence.
  `crates/gameplay/durability/src/durability/public_projections/online_sessions.rs:15`
  calls `game_public_online_maintain`;
  `apps/game-server/migrations/1354_public_online_statistics.sql:170`
  compares source expiry with SQL time. The clock jump neither triggers
  the loop nor expires an online interval. The application sends no
  accelerated cutoff. **SERVER-GAP** for statistics retention acceleration.
- `apps/game-server/src/node/serve.rs:3072` runs real audit expiry.
  `crates/gameplay/durability/src/durability/character_authority.rs:548`,
  `bazaar/audit.rs:12`, `character_command/account_deletion/publish.rs:435`,
  `staff/expiry.rs:3`, and `world_event_outcomes/recovery.rs:36` use
  storage policy and times. Where an occurrence timestamp is bound by the
  application, it remains wall or database time, e.g. the item/character
  audit writers. `audit_collector.rs:175` stamps append time from wall time;
  its rotation is file-size driven, and that append timestamp is never
  compared with SQL `now()` by the collector. **SERVER-GAP** for an
  accelerated expiry or retention proof.
- Raids persist accelerated observation timestamps and compare them with
  bound application window parameters (`raid_schedule/activity.rs:109,128`).
  That SQL comparison uses an accelerated app parameter, while
  `raid_schedule/claustrophobic.rs:77` samples SQL wall time for its separate
  event path. Combining those timelines or reopening the store after an offset
  reset can yield a different presence or event status. World UTC also feeds
  `soul_war_world.rs:104`; age or retention assertions need the durable
  owner's clock before choosing that row. **SERVER-GAP** for cross-domain
  raid and event timing.

## Pending timers and sleeps

The offset only affects subsequent samples from two clock readers. Pending
Tokio sleeps, interval ticks and timeouts are real-time registrations. The
World test polls all three pending, replaces the clock file with +75 seconds,
observes a World UTC jump, and polls them again before their real 60-second
deadline. W20a and W20b still require real refresh and packet delivery.
Respawn, corpse decay, real-clock acquisition retries and lock polling can
still consume their wall-time budgets. Advancing an offset and substituting
the advanced duration for a client deadline would mis-measure the row, or
leave the client waiting until its real timeout.

## Proposal for slow rows

Measure the identified steps individually before estimating savings. Keep
all existing observation and durable-state assertions.

| Row | Prior total | Timed wait or dominant observable work | Safe acceleration with current clock |
| --- | ---: | --- | --- |
| MP09 | 1377 s | Multi-client progression, combat and release. Shared-XP participation must stay current (`mp/mp09_party_xp.rs:594,823`); final logout can wait on combat. | No for the journey. Consider only isolated owner-only cleanup after all XP and durable comparisons finish, with a fresh scope. |
| M04 | 606 s | Player and creature push, route back from the rat spawn, combat logout lock and peer relog (`full_e2ef1.rs:849,1399`). | Candidate only for post-combat owner-only cleanup after confirmed combat cessation. Real admission/handoff and movement stay on wall time. |
| I04 | 524 s | Mail item acquisition and write-once/rewrite/relog evidence (`full_e2ef3_e2ei-b.rs:11,233,289`). No mail expiry wait is asserted. | No. A jump cannot obtain the item or speed up checkpoint and admission I/O. |
| I16 | 235 s | Imbuement input acquisition can depend on a 90-second Valkyrie respawn and rare drops (`full_e2ef3_e2ei-a.rs:556`); inspection/relog still required. | No. Respawn does not consume this offset. |
| MP01 | 234 s | No-PvP observation window, then actual PvP combat, temple respawn, durable frag and logout/PZ locks (`mp/mp01_party_pvp.rs:534,708,976`). | No for the evidence path. Jumping can skip a lock refusal and leave durable skull expiry on the real timeline. |
| C20d | 230 s | Actual lethal combat, temple respawn and restored vitals (`client_row_c20d.rs:65,108`; `full_e2ef6.rs:86`). | No. Advance does not create hits, death, a temple transition or saved state. |
| P21 | 159 s | Acquisition and worn combat imbuement duration consumption, with pause after unequip and recapture at relog (`full_e2ef7.rs:4950`). | No. The imbuement clock bypasses the offset; a jump could affect other combat mechanics while the item duration stays unchanged. |
| S08 | 157 s | Logout and PZ lock, skull, kill lock and fresh admission (`full_e2ef7.rs:3511,3584`). | No for qualification. The active refusal windows and SQL-backed skull evidence must agree. |
| I07 | 122 s | Rat respawn/acquisition and corpse decay with stale handle refusal and fresh absence (`full_e2ef3_e2ei-c.rs:461,1133`). | No. Corpse expiry is database-owned and retirement is polled. |
| I06 | 121 s | Hunt/respawn and legal corpse looting, including retry timing (`full_e2ef3_e2ei-c.rs:423,450,748`). | No. Hunt availability and real retry sleeps are unaffected. |
