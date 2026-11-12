# GAP-CL3: native movement, push and item keys

Original implementation base: `origin/main` = `c8b3890e601e6073713dcac2ea9ee271f4a257e8` (clean checkout before work).
Scope: client callers of existing server mechanics; no server rule or content changes.

## Re-verification on the original base

| Slice | Opened code on base | Gap |
|---|---|---|
| M01 | `apps/client/src/windows_shell/input.rs:35`, `:44`; `crates/protocol-oteryn/src/target_move.rs:10`; `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/target_move.rs:58` | Native shell only steps/clicks; Turn, Follow and Stop already have a server route, but no native caller. |
| M04 | `apps/client/src/windows_shell/input.rs:44`; `crates/client-ui/src/features/0082-creature-push/registrations/handlers.json:5`; `crates/protocol-oteryn/src/creature_push.rs:79` | Mouse press becomes a click, with no creature-drag caller; existing push encoder is usable. |
| I17a–d | `apps/client/src/features/0179-hotkey-equip/native.rs:3`; `apps/client/src/features/0179-hotkey-equip/input.rs:4`, `:22`; `crates/client-ui/src/features/0179-hotkey-equip/state.rs:3` | Raw command and result panel exist; native opener only sets a flag, with no item-to-key assignment. |

## Reference and parity

Crystal: `zimbadev/crystalserver`, branch `main`, pin `201189f871d23597b82caf88fd611454a2ddefb8`.
Canary: `opentibiabr/canary`, branch `main`, pin `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Both pins were verified as ancestors of their repository's `main` using GitHub compare on 2026-11-05.
Donor paths below are relative to those pins, evidence class `OTS_HYPOTHESIS_ONLY`.
Current Tibia cross-check: owner-captured manual notes, `CIPSOFT_OFFICIAL`, capture 2026-09-28 (`docs/reference/tibia-manual/README.md:8`).

| Rule | Donor file:line (Crystal; Canary) | Current Tibia cross-check / client implementation |
|---|---|---|
| Turn without stepping | `src/game/game.cpp:6480`; `:7187` | `docs/reference/tibia-manual/controls.md:26`; Ctrl+arrows send existing command 81. Facing changes after accepted acknowledgement, without relocating. |
| Follow an actor without attacking | `src/game/game.cpp:6311`; `:7033` | `controls.md:28`; select a friendly player, then Ctrl+F. Exact selected generation must still be visible. Player selection does not queue an attack. |
| Stop walking/following | `src/game/game.cpp:4024`; `:4567` | `controls.md:28`, `interface.md:107`; Escape cancels local walk/queued movement and sends Stop through command 81 when selected. |
| Drag a creature/player to a destination | `src/game/game.cpp:1620`, `:1640`; `:1776`, `:1796` | `controls.md:42`; press captures the target, release on another tile sends command 98. Focus/capture loss cancels; changed generation/position/floor cancels. No predicted movement or drag attack. |
| Assign an item definition to an equip key | `src/game/game.cpp:3703`, `:3746`; `:4159` | `interface.md:254`, `controls_communication.md:17`; Ctrl+Shift+E on the rebased client opens real native assignment controls. Inventory/equipment/open-container definitions populate the item list; assign/clear F1–F12. |
| Toggle worn item into managed storage | `src/game/game.cpp:3753`; `:4209` | `interface.md:254`; repeating the assigned key sends the same definition through command 348; server owns the toggle and destination. |
| Weapon/shield/ammunition exchange | `src/game/game.cpp:3756`, `:3777`, `:3783`; `:4212` | Same assigned-key path; existing server displacement and quiver routing stay authoritative. |
| Jewellery delay, action exhaustion and fear | `src/game/game.cpp:3712`, `:3716`, `:3720`, `:3738`, `:3796`; `:4168`, `:4172`, `:4176`, `:4194` | One equipment command remains pending until decoded server success/refusal. Repeat notifications cannot bypass it; the client invents no cooldown duration or equipment success. |

The bounded native controls use Ctrl+F for the selected actor and twelve assignable function keys; this does not add a general hotkey preset editor or reproduce Tibia's entire context-menu/action-bar layout. Bindings live for the admitted client session. The structural client-ui migration descriptors for 64/82 remain unchanged; actual callers are the app feature adapters, as with existing paced movement.

## Wiring and evidence

M01 and M04 adapters are isolated under `apps/client/src/features/0064-target-move/` and `0082-creature-push/`; item assignment stays under `0179-hotkey-equip/` in app/UI. Shared edits mount the state, connect native input, publish selected capabilities/inventory, and reduce command replies. Existing command/session generation gating remains authoritative.

No new capability bit, command ID or protocol message. Reused capability/command pairs: **64/81**, **82/98**, **179/348**; capability **180** retains its existing carried-item semantics. Migrations: **NONE**. No pins/goldens regenerated. Owner exclusions and Newhaven equipment content are untouched.

Red-first logs are retained outside git at `/tmp/gap-cl3-{m01,m04,i17}-red.log`: native caller/assignment tests fail to compile because their production entry points are missing; the corresponding green runs execute the added tests. Focused cases exercise native input to the session command channel and decoded result reducers, including stale selections, drag cancellation, player versus creature behavior, pending equipment lock and toggle payloads. Existing PostgreSQL route cases independently exercise durable server semantics.

The Win32 assignment window cannot be compiled or exercised in this Linux-only environment; no Windows Rust target is installed. Native input/reducer paths are platform-neutral and exercised on Linux. This is local component/command-path evidence, not a claim of a proven full native end-to-end journey.

## Original local validation (before client-package rebase)

- `cargo fmt --all --check` and explicit rustfmt checks of every changed generated-feature source: pass. Checking *all* feature sources additionally exposes existing formatting drift in untouched Prey/Wheel/etc.; those files are preserved.
- `cargo clippy --locked -p oteryn-client -p oteryn-client-ui -p oteryn-session -p oteryn-protocol-oteryn --all-targets -- -D warnings`: pass (existing dependency warnings remain outside the changed crates).
- `cargo test --locked` for those four packages: 1,001 tests pass, including six added regressions; no failures/ignored cases. Component test durations: app library 12.62 s, protocol 0.51 s, UI 0.00 s; session duration is in the retained final log.
- Focused dispatch nextest filter only: 15/15 pass in 271.783 s; 12 real PostgreSQL cases plus three coalescing unit cases, 2,309 unrelated cases excluded. Connection separately proved PostgreSQL 18.6 with fsync and synchronous_commit on. Movement 116.410 s; push 164.886 s; equip/toggle 97.929 s; fear refusal 97.928 s; burst cooldown 60.590 s; seven swap/quiver cases 48.076–69.693 s. No new PostgreSQL case is needed for these client-only changes.
- `oteryn-architecture-check workspace .`: pass. The publish helper and unchanged input enum were extracted to respect the existing size ratchets; no baseline was relaxed.
- Self-review under `oteryn-review`: checked native callers, capability/session gating, inventory publication, strict reply matching, cancellation and source parity. The drag/autowalk regression first failed on queued movement (`/tmp/gap-cl3-m04-walk-red.log`), then passed after cancellation was added. No unresolved blocking finding; Win32 execution remains unverified as stated above.


## Client-package rebase

Uncommitted GAP-CL3 changes were reapplied onto `claude/happy-davinci-ijz3wy-clpkg4`, exact head `92e32018ea3da9d5d39ffb029eb64a837a21c11e` (main `a9db6a76` plus CL1/CL2). No commit or push was made. The original patch and complete changed files were backed up outside git before changing the checkout.

- Retained main/CL1/CL2's `play/input.rs`, `play/tick.rs`, `play/stepper.rs` and session publisher; the superseded CL3 module extractions are omitted.
- Preserved server TargetWalk, native item dragging, minimap and secondary context clicks, use-with targeting, settings routing and Shift/cursor spell targeting. Creature gestures suppress only their own primary clicks; actor capture prevents a simultaneous item drag underneath. Drag sends Stop before Push when server target movement is selected.
- Preserved main's Ctrl+Shift+E opener, numeric Equip submission and its native tests while adding item/key assignment. Available definitions include inventory, equipment and all currently open container views.
- Retained death/respawn fences: native controls refuse blocked world input and queued CL3 movement commands are dropped with the other world actions. New overlap regressions cover ground/cursor clicks and death queue disposal; existing CL1/CL2 tests remain intact.
- Base 64 client registration and Walk adapters are preserved; the earlier structural-descriptor note applies to the original base only. No new IDs, migrations, pins or goldens.

Rebased validation (2026-11-05):

- Fmt, explicit changed-feature rustfmt, changed-crate Clippy `--all-targets -- -D warnings`, architecture workspace check and diff whitespace check pass.
- The four changed crates pass 1,409 tests: client 302, client-ui 257, protocol 694, session 156; no failures or ignored cases. Client library 287 tests / 34.63 s; protocol 1.57 s; session 21.13 s. The final native button-state adjustment was followed by another complete client test run, also passing.
- Focused nextest: 15/15 pass in 267.226 s (12 PostgreSQL cases and three coalescing unit cases; 2,445 unrelated cases excluded). PostgreSQL 18.6 connection and enabled fsync/synchronous_commit were separately verified.
- PostgreSQL durations: movement 106.695 s; push 182.504 s; equip/toggle 90.996 s; fear refusal 90.997 s; burst cooldown 82.607 s; ammo-to-quiver 75.651 s; ammo-without-quiver 78.627 s; full-backpack refusal 49.363 s; full-quiver refusal 49.158 s; one-hander swap 51.665 s; shield swap 51.757 s; two-hander swap 51.190 s.
- Win32 window execution remains unverified on this Linux-only toolchain. No tools or targets were installed, and nothing was committed or pushed.
