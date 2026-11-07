# CLHUD ADD-0066 — native step and use refusals

The 2026-11-07 audit row ADD-0066 identifies missing complete native presentation
of step/use refusal details. At base `9668e0a04`, both production paths already
exist; this change adds the missing combined session-driver/native-HUD regression.
No protocol IDs, capability bits, mechanic ordinals or migrations are allocated.

Donor evidence: Canary `src/utils/tools.cpp:1392-1415,1601` maps ordinary return
values to player-facing text; `data/scripts/actions/doors/key_door.lua:32` provides
the locked-door refusal. The existing capability-66 codec preserves those eight
reference messages. No client guess about distance, carrying capacity, floor or
locks replaces the authoritative refusal.

Production paths retained:

- Step: `crates/session/src/session_exchange.rs` decodes negotiated detail and
  clears it for the next result; `apps/client/src/play/session_loop.rs` publishes
  `session.refusal_detail()` after the actual step. The mailbox/tick flow renders
  the selected message through `PlayView::combat_status` and the native HUD.
- Use: `crates/client-runtime/src/features/0011-world-interaction/protocol.rs`
  splits the negotiated suffix, validates the use disposition and returns its
  reference message. The native `world_context_run` writes feedback, and the
  native tick/HUD shows it. A subsequent completed use replaces the refusal.

Regression `clhud_step_and_use_driver_present_every_detail_and_clear_success`
drives the production `run_session` with a scripted `Stepper`, exercising all
eight messages through both step and context-use dispatch into the native HUD;
successful results remove prior refusal text. It complements the existing real
session wire regression `refusal_details_survive_step_and_use_exchange`, decoder
test `use_flow_presents_every_negotiated_refusal_detail` and native status test
`every_server_detail_reaches_the_native_status_without_rewording`.

The added regression covers existing behavior and is not a baseline-red bug fix.
It passed in the full native app suite (345 library cases, 17.16 s). All app/UI
targets passed, as did changed-crate check, all-target Clippy with `-D warnings`,
formatting and architecture checks. Full command timings are in
`CLHUD-CONTRACT-NOTES.md`.
