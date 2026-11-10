# Client gaps still open (2026-11-10)

Source: the open rows of the Tibia client parity audit (`TIBIA-CLIENT-PARITY-2026-11-09`, 25 PARTIAL + 5 MISSING) and the
OTClient parity audit (`OTCLIENT-PARITY-2026-11-09`, 2 PARTIAL), checked against the code at `origin/main` `4f0b6c62a`.
This was a code-reading check only; no tests or client runs were done.
Of 32 open rows, 22 are already done on main. The 10 rows below (8 gaps: T153+T154 and T188+T197 are one gap each) are still to do.
Scope follows the owner's rule: Crystal / current Tibia.

## To do

| # | Row | Gap | What to build | Where it is today |
|---|-----|-----|---------------|-------------------|
| 1 | T153, T154 | Crash report and survey answers are sent but nothing on the server receives them | A server handler for the support delivery command (crash report with consent, survey answers), its persistence, and a test through the client path | `apps/client/src/support_delivery.rs:23-36`, `apps/client/src/play/session_loop.rs:54-61`; no handler under `crates/` or `apps/game-server` |
| 2 | T176 | Special Conditions grouping | Add Mana Shield (and the other grouped conditions) to the special-condition key list so the two checkboxes, the shared order and the grouping cover them | `apps/client/src/features/local-reference-preferences/status_order.rs:63-66`, `condition_matrix_controls.rs:61,125` |
| 3 | T182 | Use at cursor position | Cursor mode must use the object or rune at the cursor position at once, without waiting for a target; check the battle-list and run cases | `apps/client/src/features/local-spell-action-bar/item_actions.rs:37-70` |
| 4 | T185 | Echo Raids: Echo Warden progress | A progress display (bar and count) for Echo Warden like Animus Mastery, not only the text "complete/incomplete"; server data if missing | `apps/client/src/features/0067-bestiary-charm-progression/input/catalogue.rs:180-184` |
| 5 | T187 | Cyclopedia map: POI reset and discovery banner | A real POI reset (server command; today "Reset POIs" only hides them in the client) and a discovery banner instead of text feedback; the speed tooltip 100-195 is done | `apps/client/src/features/0057-cyclopedia-map/view.rs:154`, `protocol.rs:118`; `crates/client-ui/src/features/0057-cyclopedia-map/view.rs:96-103` |
| 6 | T188, T197 | Boss Difficulty and Archfoe: pity tracker | The bad-luck (pity) tracker: server figures in the reply, then the client display in the 0225 window and the Archfoe browser; personal highest and unlocks are done | `crates/protocol-oteryn/src/boss_difficulty.rs:165` ("no pity figures"), `crates/client-ui/src/features/0225-boss-difficulty/view.rs:18-33`, `crates/client-ui/src/features/0117-boss-encounter/view/browser.rs:9-27` |
| 7 | T195 | Free reroll on the Bounty Board | Remove the free timed reroll (action 11, `FREE_REROLL_SECONDS`) from the Bounty Board; rerolls come only from the Reroll Token given by Daily, which is done | `crates/gameplay/quests/src/quest/task_board_full.rs:236-237,574-589`, `apps/client/src/features/0072-task-board/native.rs:167,249` |
| 8 | O040 | Server modal dialogs | A general, rules-driven source of server modal dialogs with choices; today only the hireling store and the MOTD notice (no choices) send them. `CLIENT-GAPS-2026-11-08` marks this DONE, which is wrong | `crates/gameplay/dispatch/src/gameplay_transport/server_modal.rs:43-59`, `server_modal/store.rs:61`, `apps/client/src/features/0450-server-modal/view/native.rs:107` |

Corrections to `CLIENT-GAPS-2026-11-08`: T153, T154 and O040 are marked DONE there but are still open (rows 1 and 8).

## Done on main (audit out of date)

T060, T168, T169, T170, T171, T172, T174, T175, T177, T178, T181, T184, T186, T189, T190, T193, T196, T199, T200, T201,
T202, O036.

Notes:
- T172: party members beyond 30 tiles stay on the list and only their HP and mana bars are hidden. This follows the manual
  ("HP/mana within 30 tiles").
- T184: the Soulseals window opens only with Ctrl+Alt+Shift+8; there is no button.
- Dead code: `apps/client/src/features/0195-weapon-proficiency/native_localized_1.rs` and `native/window_host.rs` are not
  referenced anywhere.
