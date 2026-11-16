# Mechwire package P: row -> client-to-server test map

Scope: rows of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` starting with P that
`PW.md` marks BOTH-WIRED, P23* excluded (14 rows). Tree: branch
`claude/happy-davinci-ijz3wy-mechwire-p` on origin/main 60e40aae99.

All tests below are node-boot qualification phases (`apps/game-server/src/qualification/client_phases`,
`#[cfg(test)] mod qualification` in the game-server lib, run by `journeys` in `world.rs` under
`e2e_row_selected`). They drive the real client transport `oteryn_session::Session` through
`tools/dev-client` (`DevClientSession`) against the booted node over TCP+TLS; PostgreSQL is the
node's durable store, and "relog" is a normal logout plus a fresh admission. The dispatch
PostgreSQL cases (`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`) drive the
server owner directly with no client code, so none of them counts as a client-to-server test
here; they stay the server-side proof (for example `rest_status.rs`, `s10_blessings.rs`,
`forge_creature.rs`, `imbuement.rs`).

Column "client layer" names what decodes/reduces the reply on the client side:
`client-ui` = production panel builder + reducer; `client-runtime` = typed client adapter
(send + decode); `session+protocol` = `Session::gated_command` plus the protocol-oteryn decoder
that the client crates re-export.

"Verified how": `read` = traced by reading the phase against the PW.md links; `compiled` =
`cargo test -p oteryn-game-server --lib --no-run` built it on this branch (`cargo test -p oteryn-game-server --lib qualification`: 304 passed, 0 failed, 3 ignored pre-existing). No node-boot run was
possible in this session (no node-boot world bundle/pin and Platform descriptor here), so no row
below is claimed as passing at run time.

| row | test (file:line) | new/existing | client layer | relog | verified how | gap |
|---|---|---|---|---|---|---|
| P03 Prey | `client_phases/full_row_p03.rs:80` (`journey`; query, list reroll, select, bonus reroll, out-of-range refusal, hunt, checkpoint) | existing | client-runtime `prey` | yes | read, compiled | none in the chain |
| P04 Task Board | `client_phases/full_row_p04.rs:109` (`run`; query, locked-start refusal, start weekly, deliver, bounty open/select/reroll/claim, shop, talisman, reset) | existing | client-runtime `task_board` | yes | read, compiled | Soulseal fights (6120) have no Bevy entry (PW.md Client gaps) |
| P06 Wheel of Destiny | `client_phases/full_row_p06.rs:190` (query, partial/full save, stale-revision refusal, revelation, reset, vitals) | existing | client-runtime `wheel_allocation` | yes (twice) | read, compiled | revelation needs >=250 domain points; the phase reports FAIL with the observed level when the node-boot character lacks them |
| P08 Weapon proficiency | `client_phases/full_row_p08.rs:116` (browse, kill credit, select perk, modify/rank-up/orb/reshape/clear, stale refusal) | existing | client-runtime `server_mechanic` + session+protocol | yes | read, compiled | 379/380/392-395 have no Bevy entry (PW.md Client gaps); the test still sends them through the session |
| P09 Daily reward wall | `client_phases/full_row_p09.rs:126` (unopened query, refused claim, shrine use, claim, idempotent claim) | existing | client-runtime `daily_reward`, `prey` | yes | read, compiled | calendar gaps, monthly jokers and timed boost need real reset epochs; reported FAIL by the phase, never manufactured |
| P10 Resting-area bonuses | `client_phases/full_row_p10.rs:151` (`journey`; client-ui `rest_status::request` + `RestStatusPanel::finish`, area vs tile check, owner bonus rule, malformed refusal clears the panel, cross the PZ boundary and back, relog) | **new** | client-ui `rest_status` | yes | compiled | none in the chain; `client_row_p10.rs` stays as the capture phase. Runs in the existing P10 batch index 165 (`world.rs`, `P10.full` after `P10.client`) so no row index moved |
| P11 Achievements | `client_phases/full_row_p11.rs:72` (page query, local malformed refusal, item-use granter, repeat, notice delta) | existing | client-runtime `account_achievements` + session notice decoder | yes | read, compiled | the phase states the row stays FAIL: quest/event granters beyond the starter item use are not reachable in a run |
| P12b Badges | `client_phases/full_row_p12b.rs:117` (badges request vs durable `game_account_badges`) | existing | session+protocol | yes | read, compiled | client-ui `cyclopedia_character::apply_result` not driven; 387 visibility toggle is native only (PW.md); an account meeting no badge rule reports the missing precondition |
| P12c Cyclopedia character, deaths/PvP kills | `client_phases/full_row_p12c.rs:138` (QuerySelf, paged query, refused QueryVisible, death, relog) | existing | session+protocol (`character_info`) | yes | read, compiled | client-ui `cyclopedia_character::apply_result` not driven; needs the preceding E2EF7 PvP fight |
| P13 Loyalty | `client_phases/full_row_p13.rs:109` (query, malformed refusal, hunt, relog) | existing | session+protocol (`loyalty_skills`) | yes | read, compiled | apps/client loyalty reducer is not reachable from game-server; points do not advance in a run (phase reports MISSING-SERVER: the Platform earning writer has no player command) |
| P16 Blessings | `client_phases/full_e2ef6.rs:2639` (`blessing_services`, row `P16`: decline, seven purchases, duplicate refusal, relog, Henricus) and `full_e2ef6.rs:2586` (`blessing_death`) | existing | session+protocol (StoreSummary tab 6 via `full_e2ef6.rs:1391`) | yes | read, compiled | client-ui `request_blessings`/`apply_result` is driven only by `full_row_g29.rs:75` (no purchase there) |
| P18 Exaltation Forge | `client_phases/full_row_p18.rs:177` (history, fusion/transfer/convergence, resource conversions, history, relog) | existing | session+protocol (`item_forge`) | yes | read, compiled | client-ui `0152-item-forge` reducer not driven; resource-starved operations end in the explicit refusal branch |
| P19 Influenced/fiendish | `client_phases/full_e2ef7.rs:6624` (`forge_journey`, row `P19`, actions `full_e2ef7.rs:6148`; marker read from `Session::visible_appearances().forge_stacks` in `forge_hunt` `full_e2ef7.rs:6375`) | existing | session appearance decoder | yes (`fresh-admission-slivers`/`-dust`) | read, compiled | client-ui `forge_creature::replace` not driven; marked creatures appear only after the 330 s spawn sweep |
| P21 Imbuements | `client_phases/full_e2ef7.rs:4696` (`imbuement_journey`, row `P21`; apply, slot-occupied refusal, duration, fresh admission) | existing | client-runtime `imbue` | yes | read, compiled | none in the chain |

Counts: 14 rows; 13 existing, 1 new (P10), 0 rows without a client-to-server test.
Rows where the production client-ui reducer is not yet driven (session/protocol decode only):
P12b, P12c, P13, P16, P18, P19 - follow-up candidates, not missing wiring.
Content/precondition gaps that keep a row from PASS at run time (reported by the phases, not
passed): P06 revelation points, P09 reset epochs, P11 granters, P12b qualifying account facts,
P13 loyalty earning, P19 dust acquisition time.
