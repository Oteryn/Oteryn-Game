# GR-ENT-003 — first home-spawn deadline omits DailyBoost

**CONFIRMED, P2.** Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The lead and a separate client-support auditor independently traced the actual production callers and counterguards. Confirmation is static source/control-flow proof; no native, SQL or full Cargo reproduction was executed.

## Required and observed behavior

`docs/contracts/protocol-oteryn/v1/daily-boost-v1.md` requires the selected creature's half-delay to compose with Cyclopedia on every home-spawn scheduling operation, with a one-second minimum and no compounding on reload.

The ordinary gone-creature scan in `crates/gameplay/entities/src/carrier/spawn_realization.rs:654–716` instead obtains `spawn.definition.timing(cell)`, applies the scheduled-event rate and stores the resulting first deadline. Definition timing at 189–199 includes Cyclopedia but cannot inspect the runtime's daily choice. Installing that choice at 763–769 does not alter the authored definition or existing pending deadlines.

The DailyBoost reduction exists in the separate `spawn_timing` helper at 798–816. The actual driver in `crates/gameplay/entities/src/carrier.rs:1475–1545` scans first and stops when the stored deadline is still in the future. It calls the boosted helper only after that deadline has already arrived. The later call therefore cannot shorten the elapsed first wait. Blocked or skipped successors do use the helper, so this finding is limited to the initial ordinary gone-creature scheduling phase.

For an enabled selected home point with a 60-second authored delay, neutral event rate, no Cyclopedia improvement and no existing pending occurrence, ordinary death produces a first due time 60 seconds after gone detection instead of 30 seconds. Scan cadence, occupancy and warning steps can add their own delay; this example does not assert an exact final appearance time.

## Actual input path and counterguards

The serving composition starts `world_respawns.rs`, which initializes the durable daily selection before running the independent respawn job. Its selection loader uses the admitted current native pools, actual World/reset epoch, current owner and durable choice. Worlds without eligible pools are excluded from this case. `monster-ai/src/monster_owner_cycle/respawn_job.rs` forwards the monotonic owner time unchanged to `drive_spawn_respawns`; it supplies no compensating timing transform. Session cadence uses the same driver.

The world-projection path can separately cancel and immediately rearm a point after a world change. That operation is not the ordinary unchanged point's death path. Owner-generation fences, per-point uniqueness, checked deadline arithmetic and bounded scanning remain effective. Existing daily-boost tests exercise the helper directly, while the event-rate deadline test does not install the daily boost.

## Effect and limits

The selected creature misses its promised first-respawn speed benefit. There is no claim of item duplication, currency change, authority bypass, data loss or unbounded outage. A native spawn/death/deadline observation under the installed current selection remains useful runtime validation. A separate scheduled-event successor question is not included in this finding.
