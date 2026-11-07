# C15 hunting stamina (owner Q4, 2026-10-29)

The accepted rule is Global under `docs/architecture/waves/WAVE_A_PLAN.md:122`.
The donor code below is `OTS_HYPOTHESIS_ONLY`, adopted by that owner decision.
Inspected local pinned sources before implementation:

- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data/events/scripts/player.lua:99-135`: zero stamina returns without advancing
  the deadline; `currentTime - NextUseStaminaTime <= 0` does nothing. An overdue
  deadline of at most 60 seconds spends one minute and sets `now + 60`; more
  than 60 seconds overdue spends two (floored at zero) and sets `now + 120`.
  These are whole seconds (`os.time()`), and the gap is measured from the
  **next-use deadline**, not the previous kill. `data/scripts/creaturescripts/player/login.lua:169`
  initializes the deadline to login time; `logout.lua:6-7` clears it.
- Crystal `data/events/scripts/player.lua:484-507`: only non-player target XP
  reaches `useStamina`, before reading the XP multiplier. No kill XP means no
  stamina consumption, including idle time in a protection zone.
- Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`,
  `data/events/scripts/player.lua:98-134,532-555`: the same deadline, one/two
  minute cadence, target guard and modifier ordering.
- Crystal `data/libs/functions/player.lua:434-446`: Premium above 2340 minutes
  gives 1.5x, 1..840 gives 0.5x, otherwise 1x. `src/creatures/players/player.cpp:6612-6617`
  rejects experience when stamina is already zero. Existing XP arithmetic stays.
- Crystal `data/scripts/creaturescripts/player/regenerate_stamina.lua:9-29`:
  21-day elapsed cap, 600-second delay, 180 seconds per normal minute and
  360 seconds per minute above 2340, capped at 2520. The 180/360 defaults are
  `src/config/configmanager.cpp:408-409`. Existing offline recovery stays.

Baseline `96a066772` activity drain: `crates/gameplay/characters/src/stamina.rs:29-48`
and `crates/gameplay/dispatch/src/gameplay_transport/hunting_stamina.rs:70-90`.
The outgoing melee signal is `attack/drain.rs:636`; incoming/other damage
signals also call `hunting_activity`. Prey time uses that same activity signal;
keep its existing timing independent of the new experience stamina clock.

No stored state shape or protocol change is required; no migration is needed.

Regression tests:

- `experience_stamina_first_kill_then_thirty_seconds_then_sixty_one_seconds`
- `experience_stamina_deadline_edges_zero_floor_and_replay`
- `hunting_stamina_no_kill_in_protection_zone_never_drains`
- `hunting_stamina_postgres_kill_experience_idle_checkpoint_and_reload`

The first cadence regression failed on the original clock (0 spent instead of 1).
The real PostgreSQL regression failed on the original damage clock (837 minutes
instead of the unchanged 840 before any kill). The PostgreSQL case retains the
kill for actual receipt reconciliation, then checkpoints and reloads the debit;
an idle checkpoint remains `NotApplicable`. The existing receipt replay,
conflicting debit, resting gain and stale fence assertions remain in the case.

Implementation paths:

- `crates/gameplay/characters/src/stamina.rs`: experience deadline arithmetic;
  retain the former activity arithmetic separately for Prey.
- `crates/gameplay/dispatch/src/gameplay_transport/hunting_stamina.rs`: preview
  and publish the kill debit, preserving reconnect and checkpoint behavior.
- `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/first_entry.rs`:
  initialize the deadline at admission.
- `crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs`:
  use the proposed stamina for XP and publish after successful commit or
  receipt reconciliation, for every eligible recipient.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/hunting_stamina.rs`:
  real damage, idle, kill, receipt replay and durable checkpoint regression.
