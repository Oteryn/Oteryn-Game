# SRVWLD M17 — source NPC UTC calendar

Coordinator assignment: annual NPC date window plus current-year projection;
First Dragon and Hot Cuisine. Migration reservation is **2190–2199** under
the coordinator's M17 packet. No migration or public number allocated yet.

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
transaction timeout or source counter is weakened. Current-year integration
is still in progress; M17 is not yet marked DONE.
