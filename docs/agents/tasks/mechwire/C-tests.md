# Mechwire C rows: client-to-server test map (package C)

Every C row of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` that `CE.md` marks
BOTH-WIRED, mapped to the existing node-boot phase that drives the whole chain. Paths are
relative to `apps/game-server/src/qualification/client_phases/`.

All phases below use the same chain: `connect_world`/`connect_character` admit an
`oteryn_dev_client::DevClientSession`, which wraps the real client transport
`oteryn_session::Session` (`tools/dev-client/src/lib.rs`), against a booted node over its TLS
listener (`tools/qualification/node_boot/run.sh`). Commands go out through `Session`
(`step`, `chat`, `use_item`, `move_item`, `cast_spell`, `attack_target`, `set_fight_modes`,
`world_context_use`, `gated_command`); replies and pushes are applied by
`crates/session/src/domain_dispatch.rs` and the phase asserts on `Session` state
(`inventory()`, `world_spatial()`, `world_entities()`, `SessionEvent`s) or on the decoded
gated reply. "Relog" is a logout followed by a fresh admission that re-checks durable state.

`full` is the row's `<id>.full` scenario; `capture` is the `<id>.client` replay-capture packet
(raw payloads for the client reducers; several write only under `E2E_ALL_CAPTURE_DIR`).

| row | full scenario (client -> server -> client) | relog leg | capture packet | new test |
|---|---|---|---|---|
| C01 | full_row_c01.rs:55 (world.rs:713) | yes, `relog-xp-loot` full_row_c01.rs:263 | client_row_c01.rs:6 | none needed |
| C02 | full_row_c02.rs:67 (world.rs:706) | yes, fresh admission full_row_c02.rs:499 | client_row_c02.rs:97 | none needed |
| C03 | full_row_c03.rs:107 (world.rs:678) | yes, `refusal-relog` full_row_c03.rs:513 | client_row_c03.rs:91 | none needed |
| C04 | full_row_c04.rs:375 (world.rs:734) | yes, logout :348, fresh :353 | client_row_c04.rs:149 | none needed |
| C05 | full_row_c05.rs:21 (world.rs:727) | yes, `fresh-admission` full_row_c05.rs:19 | client_row_c05.rs:9 | none needed |
| C06 | full_row_c06.rs:29 (world.rs:699) | yes, fresh full_row_c06.rs:182 | client_row_c06.rs:5 | none needed |
| C07 | full_row_c07.rs:29 (world.rs:1203) | yes, `fresh-admission` full_row_c07.rs:143 | client_row_c07.rs:29 | none needed |
| C08 | full_row_c08.rs:48 (world.rs:1259) | yes, logout :239, fresh :255 | client_row_c08.rs:136 | none needed |
| C09 | full_row_c09.rs:82 (world.rs:692) | yes, `fresh-admission-regeneration` :426 | client_row_c09.rs:9 | none needed |
| C10 | full_row_c10.rs:73 (world.rs:1196) | yes, `fresh-admission-saved-rest` :218 | client_row_c10.rs:12 | none needed |
| C11 | full_row_c11.rs:82 (world.rs:748) | yes, `fresh-admission-monk` :165 | client_row_c11.rs:5 | none needed |
| C12 | full_row_c12.rs:92 (world.rs:685) | yes, `fresh-admission` :559 | client_row_c12.rs:7 | none needed |
| C13 | full_e2ef5.rs:2847 (`SUMMON_ACTIONS` :2812) | yes, `summon-logout-removal`, `familiar-logout-return`, `death-fresh-admission` | client_row_c13.rs:21 (observer applies the summon enter/leave deltas) | none needed |
| C14 | full_e2ef18.rs:70 | yes, `fresh-admission-persistence` full_e2ef18.rs:379 | client_row_c14.rs:7 | none needed |
| C15 | full_row_c15.rs:205 (world.rs:1252) | yes, logout :122, fresh :190 | client_row_c15.rs:191 | none needed |
| C16a | full_e2ef5.rs:933 (`BARRIER_ACTIONS` :914) | map-item expiry and traversal after the cast; no relog action | client_row_c16a.rs:294 | none needed |
| C16b | full_e2ef5.rs:932 (`PARALYZE_ACTIONS` :905) | yes, `fresh-admission-rune-custody` | client_row_c16b.rs:16 (target's 276 condition reply) | none needed |
| C16c | full_e2ef5.rs:2848 (`ACQUIRE_ACTIONS` :2826) | yes, `logout-removes-both`, `relog-rune-custody`, `death-fresh-admission` | client_row_c16c.rs:6 reports MISSING-CLIENT (capture gap, see below) | none needed for the chain |
| C16d | full_e2ef5.rs:4056 (`TILE_ITEM_ACTIONS` :4038) | yes, `chameleon-fresh-admission`, `removal-fresh-admission` | client_row_c16d.rs:5 | none needed |
| C17a | full_e2ef5.rs:64 (`HASTE_ACTIONS` :47) | yes, `haste-fresh-admission`, `strong-fresh-admission`, `expiry-fresh-admission` | client_row_c17a.rs:12 | none needed |
| C17b | full_row_c17b.rs:80 (world.rs:1189) | yes, `fresh-admission-condition` :351 | client_row_c17b.rs:11 | none needed |
| C17c | full_e2ef5.rs:65 (`ILLUSION_ACTIONS` :10) | yes, `fresh-admission-condition`, `fresh-admission-expiry` | client_row_c17c.rs:118 | none needed |
| C17d | full_e2ef5.rs:1773 (`CHALLENGE_ACTIONS` :1758) | yes, `fresh-admission-costs`, `fresh-admission-spell-access` | client_row_c17d.rs:10 | none needed |
| C18a | full_e2ef5.rs:1772 (`AIM_ACTIONS` :1749) | yes, `fresh-admission-costs`, `fresh-admission-spell-access` | client_row_c18a.rs:8 | none needed |
| C18b | full_row_c18b.rs:89 (world.rs:1182) | yes, `fresh-admission` :268 | client_row_c18b.rs:16 | none needed |
| C19a | full_e2ef6.rs:67 (`STARTER` :11) | yes, `relog-custody-no-second-grant`, `second-relog-no-second-grant` | client_row_c19a.rs:57 | none needed |
| C20a | full_e2ef6.rs:110 (`EXPERIENCE_LOSS` :1319) | yes, `unblessed-relog` | client_row_c20a.rs:7 | none needed |
| C20b | full_e2ef6.rs:124 (`ITEM_LOSS` :1332) | yes, `unprotected-relog-custody`, `protected-relog-custody` | client_row_c20b.rs:61 | none needed |
| C20c | full_e2ef6.rs:148 (`BLESSING_DEATH` :2047) | yes, `adventurer-relog`, `pve-relog`, `twist-relog` | client_row_c20c.rs:8 | none needed |
| C20d | full_e2ef6.rs:87 (`TEMPLE` :17) | yes, `relog-saved-temple-full-health` | client_row_c20d.rs:7 | none needed |

Counts: 30 BOTH-WIRED C rows; 30 mapped to an existing full client-to-server-to-client
scenario; 29 with a relog/fresh-admission leg (C16a proves expiry and traversal instead); 0 new
tests written, because no row lacked the chain.

## Not verified here

The node-boot phases need Docker, a TLS PostgreSQL 18.6 and the pinned Platform producer
(`tools/qualification/node_boot/run.sh`); this session has none of them, so no phase was run.
The map shows that each row has a phase that drives the chain, not that the phase is PASS on
the current head: `SCENARIO_MATRIX.md` keeps `<id>.full` FAIL until the whole row has evidence.
The matrix column "Existing scenario or NONE" predates these phases and still says NONE for
most C rows; it is not updated here.

## Gaps

- C16c capture: client_row_c16c.rs:6 reports `status=MISSING-CLIENT`; it records no raw
  Convince/Animate entity deltas for the replay corpus. The live chain is covered by the
  full scenario (full_e2ef5.rs:2848), so this is a replay-capture gap only.
- C19b (MISSING-BOTH in CE.md): full_row_c19b.rs:11 proves the vocationless kit and its relog,
  then ends FAIL `needed:vocation-change-route`: no player route from the town start reaches a
  Dawnport vocation door. Content gap, not a pass.
- C16a: no relog leg in `BARRIER_ACTIONS`; the walls are transient map items, so the durable
  part is rune custody, which C16b's `fresh-admission-rune-custody` covers for the same USE path.
