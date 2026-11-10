# Client gaps: verified implementation scope (2026-11-10)

Source: the open rows of the Tibia client parity audit (`TIBIA-CLIENT-PARITY-2026-11-09`, 25 PARTIAL + 5 MISSING) and the
OTClient parity audit (`OTCLIENT-PARITY-2026-11-09`, 2 PARTIAL), checked against the code at `origin/main` `4f0b6c62a`.
The original report was a code-reading check only. A second trace at its exact
head `61c632f4db19a6f199c6cb448a90f5e1288dece7` found that three reported gaps
and the reset half of T187 already have production paths. The original list
therefore must not be used as eight missing implementations.
The owner subsequently requested implementation in PR #3091. Scope follows
the owner's rule: Crystal / current Tibia. Status below distinguishes source
tracing, changes under construction and executed validation.

## Verified disposition

| # | Row | Status | Verified path / remaining work |
|---|-----|--------|--------------------------------|
| 1 | T153, T154 | EXISTING; source trace verified | `support_delivery.rs` -> `5132-support_intake.mechanic` commands 6144/6145 -> `gameplay_transport/support_intake.rs` -> `durability/support_intake.rs::submit_support_evidence`, migration 2712. Consent, current-session checks, deduplication and commit-before-ACK exist. The PostgreSQL real-dispatch test is `support_intake_postgres_real_dispatch_consent_replay_and_fences`; it was not rerun during the trace. |
| 2 | T176 | EXISTING; source trace verified | `local-reference-preferences/status_order.rs` includes icons 1..=27, including `condition_status::icon_key(17)` = Mana Shield, plus Resting Area/Soul War/Rotten Blood. `play/resource_hud.rs::special_condition_rows` applies the independent HUD/status filters and shared order. Existing test: `t176_condition_matrix_persists_independent_columns_and_shared_group_order`. |
| 3 | T182 | EXISTING; source trace verified | `local-spell-action-bar/item_actions.rs::use_item` distinguishes crosshair `Cursor` from immediate `CursorPosition`, using the current map/battle selection. Existing client-path tests cover immediate use, hovered battle entity/generation and rune cooldown fallback. |
| 4 | T185 | IMPLEMENTED; broader validation pending | Native catalogue progress bar and count use the confirmed card's optional per-species first-defeat fact: 0/1 or 1/1. Missing data stays unknown, and another species/logout cannot inherit it. Extended `t185_echo_warden_progress_survives_real_play_queue`; focused actual-code decoder/UI check passed. |
| 5 | T187 | IMPLEMENTED; broader validation pending | Reset button 10 sends `Intent::Shuffle` / operation 2 to the durable owner; button 25 separately toggles visibility. The existing committed discovery notice now reaches the notification banner even with the map closed, deduplicated within the current session sequence namespace. Ordered replacement-snapshot markers preserve correctness across House/activity handoffs and mixed event batches without losing a queued banner. Manual Find emits the same notice only after the seventh POI commits. New client/real-dispatch regressions cover handoffs, reset, snapshot, receipt replay and expiry; final execution remains pending. |
| 6 | T188, T197 | BLOCKED: rule/source gap | Personal highest/unlocks exist, but there is no authoritative pity producer. `boss-difficulty-v1.md` explicitly excludes inferred pity parameters. See the bounded source finding below; do not display a synthetic zero, test percentage or kill count as a drop bonus. |
| 7 | T195 | IMPLEMENTED; broader validation pending | Every bounty reroll requires one existing Daily Reroll Token. The free timer is removed from production state, legacy JSON is tolerated, and the retained wire deadline is zero. UI shows the token cost. New/extended rule, client and real-dispatch regressions cover no-resource refusal and replay; they have not yet run. |
| 8 | O040 | IMPLEMENTED; broader validation pending | Normal House bed USE offers the sourced six-skill dialog through the existing sleep owner. The retained placement, revision and full session/House fence are checked again at the callback. Cancel/retry and unknown outcomes remain explicit. Opted-in clients receive a typed terminal ACK and end play before further I/O; legacy queries keep their original reply vocabulary. Codec/hub/UI checks pass; complete client and C10 PostgreSQL execution remain pending. |

T153/T154 must not be reopened solely because this report originally missed
their receiver. O040 now has a real gameplay rule source beyond MOTD/store;
the complete integrated execution still requires the checks listed above.

## Pity source finding

The official [Make Believe teaser](https://www.tibia.com/news/?id=8847&subtopic=newsarchive)
confirms a per-character, per-boss bonus that increases after an eligible victory
without Moonsilver gear and resets after a qualifying item. The official
[11 August fixes](https://www.tibia.com/news/?id=8925&subtopic=newsarchive)
also confirm the Phosphorus reset behavior. Neither located text specifies the
initial bonus, the increment formula or its dependence on difficulty.

At pinned Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`src/server/network/protocol/protocolgame.cpp::sendBossDifficultySelection`
serializes a bad-luck number supplied by its caller. The located caller in
`data/scripts/talkactions/god/boss_difficulty_test.lua` supplies a demonstration
value; it does not implement a combat/reward progression owner. A serializer
and a test constant are not evidence for a player's statistic.

The later [accepted W21 correction](https://github.com/Oteryn/Oteryn-Game/pull/2769#issuecomment-6549596550)
sets difficulty-based loot multipliers and special reward thresholds. Those
parameters do not define a per-failed-attempt pity increment or the rewards
that reset it; the difficulty multiplier must not be substituted for pity.

Required to close T188/T197: a sourced or explicitly accepted initial value,
per-boss/per-difficulty update rule, qualifying attempts and reward set, and
the exact reset event, then a durable
atomic reward-outcome producer and negotiated client projection. Until then the
tracker remains explicitly incomplete; no invented economy rule is introduced.

## Done on main (audit out of date)

T060, T168, T169, T170, T171, T172, T174, T175, T177, T178, T181, T184, T186, T189, T190, T193, T196, T199, T200, T201,
T202, O036.

Notes:
- T172: party members beyond 30 tiles stay on the list and only their HP and mana bars are hidden. This follows the manual
  ("HP/mana within 30 tiles").
- T184: the Soulseals window opens only with Ctrl+Alt+Shift+8; there is no button.
- Dead code: `apps/client/src/features/0195-weapon-proficiency/native_localized_1.rs` and `native/window_host.rs` are not
  referenced anywhere.
