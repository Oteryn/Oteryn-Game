# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

AUTHORING on draft #1942, predecessor 580b31689837b2e2dd78ffad76f0571de96e6113.
Root is sole publisher; no freeze, merge, production deployment or DB reset.
Owner permits client/Platform/engine work; preserve unrelated modifications.
Do not publish proprietary reference assets or private account data.

## Current owner direction and correction

Owner rejected cosmetic patches to incomplete existing forms. Inspect the original
Tibia Global interface, dropdowns, scroll extents and nested panels BEFORE building
replacement UI. Modern Oteryn appearance must retain complete interaction groups.
Previous complete-HUD wording was wrong: live scrolling reveals 35 conditions;
our catalogue and table expose six. The original first bottom action bar ends at
1.50; our rows have 12 slots. Hotkey profile auto-switch is missing in our source.
Evidence and remaining audit scope: apps/client/UI-REFERENCE-OBSERVATIONS.md.
apps/client/UI-FIDELITY.md now marks the HUD partial, not complete.
This increment changes documentation only; no additional cosmetic client edits.

## Existing implementation and verification scope

580b3168 improves HUD legibility and fixes footer reachability. Native Windows
169 tests (124 library/40 app/5 launcher), strict MSVC Clippy/release and Linux28
composition tests passed for that source. Those results do not establish option
coverage, original behavior, visual parity or complete gameplay. Native screenshot:
/workspace/artifacts/oteryn-full-hud-native-20261009.png.
Quick settings are supplementary; complete pages must remain reachable.
Nine action rows/108 chords, shortcut draft add/remove/order and Apply/Cancel work.
Own-resource HUD consumes real session vitals; unknown/invalid maxima draw no fake
resources. Names/marks/conditions/order and many panel consumers remain incomplete.
28 shortcuts,22 prepared panel views,20 Cyclopedia pages and six dialogs are
scaffolding counts, not completion. Audio/effects/capture consumers remain open.
Modern charcoal/bronze chrome and pinned Lucide icons do not establish fidelity.

## Security, dependencies and retained boundaries

Actual first/repeated sign-in remains system-browser PKCE. Remembered native
sign-in primitives are OFF/unwired. Never persist account passwords. Platform
PR#1484 is separate. CI fixes #1943/#1944 remain frozen/unmerged; the main Rust
1.94/client1.95 mismatch is not repaired by this draft. protocol-oteryn,
WorldId/ChannelId separation and generation fencing remain invariant.
Original resources are private compatibility references only. Missing engine
mechanics do not justify omitting planned controls, but cannot be claimed working.

## Runtime and next steps

Windows checkout: C:/Users/barte/Downloads/oteryn-client-main-20261007.
Native file transfer remains SHA-256 guarded with exact backups and scoped HWND
interaction. Do not print private launch scripts or credentials. Synology reference:
otclient-track-a-kasmvnc, DISPLAY=:1, window27262999; verify live title/geometry.
Private fresh captures are NAS /tmp/oteryn-hud-conditions-*.png and hotkey captures
listed in UI-REFERENCE-OBSERVATIONS.md. No Apply/Reset/reference account changes.

Finish the complete original interaction inventory: remaining scroll extents,
choices, hotkey modes/profiles/custom editors, plus-menu panels, context menus,
nested modules and visible/disabled dependencies. Record observed facts separately
from untested behavior. Then replace incomplete compositions using the complete
inventory and a consistent Oteryn component system; verify compiled native views
and actual actions. Preserve working runtime code without treating its forms as
the target design. Publish bounded increments through the guarded single-writer
path in /workspace/publish-client. Root workspace contains unrelated server/login
changes and task files; do not stage or alter them. Use isolated publication
checkout for governance checks because unrelated task docs fail its current gate.

## Continuation checkpoint — 2026-10-10

The current branch supersedes the historical 6-condition/12-slot/missing-auto-switch
implementation notes above without changing their audit provenance. It now has the
35 observed HUD rows with ordering, nine 50-slot action rows, captured two-column
hotkey editing in distinct Chat On/Off contexts with conflict blocking,
character-name preset auto-switch, and real
consumers for movement, Options, fullscreen, action-row visibility and text custom
actions. Unsupported general actions and spell/object assignments are disabled;
persisted intent is not completion credit. Help options export/import/reset are
real local operations, while support destinations and minimap import/export stay
disabled because no verified target/format exists.

Rust remains pinned to repository MSRV 1.94. The incompatible client-only Rust
1.95 / egui 0.36 change was replaced with egui 0.35 and its matching wgpu 29,
including the renderer API adjustments. Locked Linux client tests, strict client
and workspace Clippy, Windows cross-target check/Clippy and the compiled 20-page
reachability test are the local evidence for this checkpoint. Native Windows
screenshot comparison and every explicitly pending engine/server consumer remain
open; do not describe the whole matrix as 1:1 complete.
