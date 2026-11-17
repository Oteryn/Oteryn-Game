# SRVWLD M17 — source NPC UTC calendar

Coordinator assignment: annual NPC date window plus current-year projection;
First Dragon and Hot Cuisine. Migration reservation is **2190–2199** under
the coordinator's M17 packet. No migration or public number was allocated.

## Source evidence

- Tamoril, Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  <https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tamoril.lua>:
  lines 45–63 implement inclusive January 14–February 12 admission before
  dialogue; lines 65–108 retain topic order and unconditional counter resets.
  Full source SHA-256: `d5d55a74c2bb9ade413c23442924b46ee6e5b05c9daf73ed8259e67f04576969`.
  The producer reconstructs from the existing retained source, verifies this
  hash and runs the existing closed Lua lowering; no Lua is executed at runtime.
- Jean Pierre, same pinned Crystal revision,
  <https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/jean_pierre.lua>:
  lines 154–157 compare the saved bonus year with the current calendar year;
  line 687 records the current year on first menu completion.
  Downloaded source SHA-256: `dd9e5a92688d4563e5bb5590f4d436f35db87b9adec435dfe543f3ea967518bf`,
  matching the existing Hot Cuisine programme's source custody.
- UTC follows the already accepted Hot Cuisine source calendar contract
  (`batch3_a/capture_hot_cuisine.py`, owner answer #6589517205 point 10).

## Date-window component evidence

Red: the two valid window tests failed on unknown `calendar_window` before
implementation. Green: inclusive boundaries/inverse, year wrap, leap day and
invalid dates pass. Source acceptance/reset coverage checks all five original
writes and preserves the three unconditional resets on repeated acceptance.

Real PostgreSQL 18.6, `cargo test -p oteryn-game-server --test
character_authority_postgres srvwld_npc_calendar_ -- --nocapture`: two cases
passed, SQL-derived UTC boundary/reset plans in **5.553295506 s** and the real
NPC owner/current clock/stale-fence case in **5.589545407 s**. The owner case
submits an eligible historical preview, then verifies the actual transaction
clock controls admission and a stale lease cannot mutate state. No fence,
transaction timeout or source counter is weakened.

## Current-year projection and quest binding

`NpcRuleInput::utc_calendar()` exposes the owner's UTC year, month and day.
Tamoril's production NPC programme uses the annual window on every source
branch and the inverse window for the outside-event response. The Hot Cuisine
calendar projection uses the same conversion; its planner rejects a supplied
current year or month that differs from the owner's epoch before granting a
bonus. Existing cooking dispatch remains the separately assigned M42 scope.

Red: `hot_cuisine_rejects_forged_current_year_before_bonus_grant` accepted a
forged next-year calendar before the check. Green: it rejects that calendar
and grants the correct 2026 bonus with the real UTC projection. Five calendar
unit cases cover inclusive boundaries, recurrence, wrap, leap day, negative
epochs, invalid dates and source reset semantics.

Three real PostgreSQL cases passed together after adding year coverage:
`srvwld_npc_calendar_postgres_hot_cuisine_current_year_projection` in
**7.0198388 s**, `srvwld_npc_calendar_postgres_utc_boundaries_and_source_resets`
in **7.061914609 s**, and
`srvwld_npc_calendar_postgres_current_clock_and_stale_fence` in
**7.064251071 s**. These use PostgreSQL UTC timestamps and the actual durable
NPC commit owner; no SQL migration or new protocol capability is required.

The code is published as owned-branch head `48ac55e7b` and was integrated by
the coordinator into qmech as `c9ced657a`.


## Owned-branch verification after live-eligibility integration

On `48ac55e7b`, the three PostgreSQL cases above passed again with real
execution durations **8.705502818 s** (boundaries/reset), **8.753381567 s**
(current clock/stale fence), and **8.765736751 s** (Hot Cuisine year).
The five calendar unit tests, forged-year regression, exact runtime programme
activation/default test, and published source-programme hash test passed.
The complete quest-authoring checks and unit-tests runner (107 modules)
passed, including all 28 quest-tree tests. Commands:

- `python3 tools/content-schema/quest-authoring/run_checks.py --part checks`
- `python3 tools/content-schema/quest-authoring/run_checks.py --part unit-tests --jobs 4`
- `cargo test -p oteryn-game-server --test character_authority_postgres srvwld_npc_calendar_ -- --nocapture`
- `cargo clippy -p oteryn-gameplay-quests --all-targets -- -D warnings`

The native content-tree validator also passed (34,111 items, 252 mounts).
Server integration-target clippy with `-D warnings` passed in **3m 56s**,
and `cargo fmt --all --check` passed. The final coordinator merge retained
the single original forged-year regression after the additive merge driver
had duplicated the identical test; no assertion or case was removed.

After merging coordinator qmech `955b12a58`, all **338** quests library tests
passed in **26.93 s**, and quests all-target clippy passed in **9.31 s**.
The incoming M37 teleport test initializer needed the live-eligibility
`premium_current: false` field; this compile fix changes no assertions.
`cargo fmt --all --check` passed on the merged tree.
