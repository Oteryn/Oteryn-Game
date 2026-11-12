# CLHUD C11 — Battle Instinct panel state

Base main: `9668e0a04e1a631ebfc8752772102a1fd98ce8fa`.

The existing server projection, negotiated decoder, automatic HUD, native status
window and PostgreSQL case are retained. The native status panel now distinguishes
Wheel eligibility without a current census bonus from an active bonus, matching
the existing HUD: inactive, eligible with zero melee/shielding, active with a
nonzero bonus. Displayed bonus values remain the server values.

Donor evidence: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`src/creatures/players/wheel/player_wheel.cpp:2981-3004,3397-3418` and
`src/creatures/players/player.cpp:7426,7431` describe the two-second physical
census and conditional Battle Instinct bonuses. This retains the accepted
interpretation recorded in `NX-7-server-client-status.md`; it introduces no
formula or eligibility rule. Classification: `OTS_HYPOTHESIS_ONLY`.

No new numbers or migrations: existing capability 278 and command 494 are reused.

Regression: `battle_instinct_panel_distinguishes_inactive_eligible_and_active`
drives request, wire encoding, strict result decoding and the text consumed by the
native status window for all three states and exact melee/shielding values.
It failed before the production fix because the eligible state rendered Active.
Fresh validation: UI library 269 passed (0.02 s), native app library 343 passed
(25.47 s), with all additional targets passing. The new regression failed before
the wording fix. UI `cargo check --locked` passed (18.52 s wall), and all-target
Clippy with `-D warnings` passed (12.80 s wall). The retained composed PostgreSQL
case `battle_instinct_status_postgres_composed_query_and_stale_lease` executed
against PostgreSQL and passed in 56.79 s, printing its completion marker.
Execution logs remain outside the repository under `/tmp/clhud-*`.
