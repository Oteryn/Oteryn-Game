# GAP-CL5 — progression panels

Base: current `origin/main`, `c8b3890e601e6073713dcac2ea9ee271f4a257e8`.
The clean starting checkout matched that ref. No fetch, commit or push.
Scope: native client only; no migration, capability, command or message number allocated.
Existing capability/command pairs: Task Board 72/89; character statistics 199/388.
No Dawnport, OT-only features or Prey Hunting Tasks; no starting-equipment changes.

## Re-verification and outcome, in requested order

1. **P04 — client slice implemented.** On the base, `crates/client-ui/src/features/0072-task-board/mod.rs:25` only exposed a weekly request helper, while `:85` rendered only the legacy reply. `apps/client/src/features/0072-task-board/native.rs:121` sent legacy select/claim actions, and `view.rs:7` had no shell input caller. The full server actions already exist at `crates/gameplay/dispatch/src/gameplay_transport/task_board.rs:38`. Added full-result dispatch, strict decoding, weekly/delivery/bounty/list/shop rendering and controls, and Ctrl+Shift+T in the native shell. Creature choices retain server identities behind readable labels.
2. **P05 — client slice implemented.** Base native actions at `apps/client/src/features/0072-task-board/native.rs:121` contain upgrade but no purchase; server action 17 already maps to BuyTalisman at `crates/gameplay/dispatch/src/gameplay_transport/task_board.rs:63`. Purchase and four upgrades now use the full owner, pending gate and decoded refusal. The small P05 adapter is isolated in `crates/client-ui/src/features/0072-task-board/protocol/talisman.rs`.

### P08 — STOPPED (five-line boundary record)
1. Base `apps/client/src/features/0195-weapon-proficiency/native.rs:3` only changes an open flag.
2. `crates/protocol-oteryn/src/weapon_proficiency.rs:5-12` exposes mutation commands; `SelectPerkReply` at `:87` contains selections, not XP or a weapon catalogue.
3. A full XP/list client therefore needs a new server read contract; mutation replies cannot provide the requested facts.
4. Required design decision: paged query versus pushed track/catalogue snapshots, including catalogue identity and freshness semantics.
5. No partial UI, server contract or migration was introduced; continue with P19.

### P19 — STOPPED (five-line boundary record)
1. Base `crates/client-ui/src/features/0104-forge-creature/registrations/handlers.json:5` explicitly has no native state/view/input.
2. Marks exist in `crates/protocol-oteryn/src/appearance.rs:87`, but `crates/session/src/appearance_state.rs:103` resolves only Look to consumers.
3. A current dust read is absent: `crates/gameplay/durability/src/durability/item_forge_live/storage.rs:277` returns historical receipts, which become stale after creature dust credit.
4. Required design decision: authoritative current dust query/snapshot and its refresh path; history or a spending command is not a balance read.
5. No partial marker-only implementation or guessed dust balance was introduced; continue with S16b.

5. **S16b — client statistics slice implemented.** Base `crates/client-ui/src/features/0199-cyclopedia-character/protocol.rs:97` displayed a byte count without decoding; `apps/client/src/features/0199-cyclopedia-character/native.rs:3` only set open and queried badges. Added a real native General/Offence/Defence/Misc window, preserving badges, using existing GeneralStats and CombatStats schemas. Decode rejects truncated, trailing, wrong-tab and invalid knowledge-tag replies. Known zero and unknown remain distinct. Statistics stay in feature 0199; no vocation/Dawnport change.

### W06 + W11c — STOPPED (five-line boundary record)
1. W06 server production portal/reward paths exist at `crates/gameplay/dispatch/src/gameplay_transport/echo_warden.rs:17`, `loop_move.rs:281` and `echo_warden/rewards.rs:50`.
2. The existing focused PG scenario is `postgres_tests/echo_warden.rs:28`; it proves server rewards and custody, not a native client portal/reward test.
3. Rotten Blood state is read server-side at `endgame_entry.rs:64`, but no taint projection exists in protocol-oteryn or session.
4. Required design decision: a server-owned taint view contract and refresh semantics; the native Bestiary/Charms reward flow also needs app integration (`apps/client/src/cyclopedia.rs:5` is only a facade).
5. No server-PG evidence is relabelled as client completion; no invented taint, client fixture pretending to be a live encounter, or migration was added.

## Rule parity / accepted differences reused

| Client rule | Source / accepted owning contract |
| --- | --- |
| Weekly start, delivery, bounty selection/claim/reroll, list operations and task-shop purchase | Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, branch main, `src/server/network/protocol/protocolgame.cpp:3437-3588` (OTS_HYPOTHESIS_ONLY); mapped by the existing accepted `docs/contracts/protocol-oteryn/v1/task_board_v1.md:13-55`. No server rule changes. |
| Delivery count includes carried/stash/depot and can exceed required | Same Crystal file `:6210-6222`; existing Task Board contract `:75-81`. UI displays the recorded server value. |
| Purchase and four talisman paths, costs and bonuses | Existing accepted Task Board contract `:44-55`; `docs/reference/NEWS-HIST-TALISMAN-1.md:15-32` records the accepted Canary profile versus Crystal differences. UI introduces no progression formula or purchase price. |
| General statistics and combat views | Same Crystal protocol file `:4372`, `:5016`, `:5197`, `:5265`; existing `docs/contracts/protocol-oteryn/v1/cyclopedia-character-v1.md` and `crates/protocol-oteryn/src/cyclopedia_character/combat.rs:1-6`. |
| Unknown observations remain unknown; saved HP/MP are labelled as saved; ratios display percent from ppm | Existing accepted native schema: `cyclopedia_character.rs:308-319`, `cyclopedia_character/combat.rs:1-6`. No inference of current HP/MP or missing combat facts. |
| Native window shortcuts and pending/refusal handling | Oteryn presentation choices using existing feature/window/session conventions; no Tibia gameplay rule or donor wire framing changed. |

## Verification

Red evidence: full-board play-loop test timed out before result routing; purchase test failed to compile before its adapter existed; statistics test failed on the old byte-count display. Logs retained outside Git under `/tmp/gap-cl5-*.log`.
Final host checks: `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-client -p oteryn-client-ui --all-targets -- -D warnings`; `cargo test --locked -p oteryn-client -p oteryn-client-ui`. All passed: 214 tests, zero failures (121 client library, 5+3 client integration, 83 UI library, 2 UI registration tests). `git diff --check` passed.
Focused PG command: `cargo nextest run --locked -p oteryn-gameplay-dispatch --all-features -E 'test(task_board_real_dispatch_weekly_bounty_custody_reset_and_replay) | test(echo_warden_all_damagers_first_species_replay_and_real_scroll_custody_postgres)' --nocapture`. This selects exactly two existing cases, not the full suite; the environment has a nonempty admin route and startup proved PostgreSQL 18.6 connectivity. Both executed and passed: Echo 89.333 s, Task Board 131.807 s; total execution 221.145 s. Both emitted POSTGRES_CASE_COMPLETED markers. No new PG case was needed for client-only code. Linux has only `x86_64-unknown-linux-gnu`; Windows compilation and interactive GUI testing cannot run here without installing a target, which the task forbids.
The broader Cyclopedia registration for command 388 remains marked unsupported because its non-statistics tabs still lack paging/target controls; this package implements the requested four statistics views only. Registrations, pins and goldens were not changed.


## Client-package rebase

Follow-up base: `92e32018ea3da9d5d39ffb029eb64a837a21c11e` on `claude/happy-davinci-ijz3wy-clpkg4` (main a9db6a76 plus CL1/CL2). The original uncommitted binary patch was retained outside Git before fast-forwarding and applying it with three-way resolution; no commit or push.
The complete upstream Cyclopedia native window, all ten tabs, paging/target controls, immutable requested-tab result fence, strict reader, and stamina hours/minutes display are retained. CL5 reuses that reader instead of keeping a duplicate statistics decoder; it adds explicit last-saved labels for HP/MP. Its regression cases now issue an outstanding request for each reply and assert the upstream statistic labels and unavailable/zero distinction.
Task Board retains upstream Ctrl+O as well as CL5 Ctrl+Shift+T. Its full result display includes upstream feedback in the existing full panel rather than an overlapping second edit control. Duplicate input handlers and module registrations introduced by the additive merge were removed. The Ctrl+O regression first verifies the full-board opening, then explicitly runs the unchanged legacy query/refusal assertions. A legacy reply clears the prior full snapshot so that the compatibility reader presents the latest response. All CL1/CL2 routes in the session loop and native shell were retained.
The earlier verification above describes the original base; fresh validation on the new composition is recorded with this follow-up delivery.

Fresh host validation on the rebased composition passed: `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-client -p oteryn-client-ui --all-targets -- -D warnings`, and `cargo test --locked -p oteryn-client -p oteryn-client-ui` (558 tests passed, zero failures). The focused legacy/full-board compatibility regression also passed separately. PostgreSQL rerun results are recorded in the follow-up delivery.
