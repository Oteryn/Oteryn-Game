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

AUTHORING on draft #1942; remote predecessor
`c177231dee65188909049f0b26ce5cd12d35f4cd`; local authoring base
`b000e20fb0bc0f03ddd450141c6f975bb15ded45` merges main
`907412c53dec6f85a21c47a59be0f716cc7de4e6`; no freeze. Root is sole
publisher; no production deployment or DB reset.
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
path. The current workspace is a clean single-writer checkout; inherited
server/content files are ancestry from main, not work owned by this task. Preserve
them and stage only the explicit client/governance batch.

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
including the renderer API adjustments and the resulting World compiler-input
pin refresh. Locked Linux client tests, strict client and workspace Clippy,
Windows cross-target check/Clippy and the compiled 20-page
reachability test are the local evidence for this checkpoint. Native Windows
screenshot comparison and every explicitly pending engine/server consumer remain
open; do not describe the whole matrix as 1:1 complete.

## Continuation checkpoint — 2026-10-10 chat console

The received-chat projection now backs per-tab Copy and bounded, non-overwriting
local Save actions. Export names come only from the six fixed client tabs, private
files use owner-only Unix permissions, and a failed write removes its incomplete
file. Per-tab read-only mode blocks both the composer and send action. The inert
secondary-console control is disabled; participants, Clear and a second console
remain explicit gaps because their required projections/views do not exist.

The Windows-only HUD, panel, icon and reference-dialog modules are now included in
Linux test builds through `cfg(test)`; the application target therefore exercises
51 tests instead of silently omitting those modules. The `image` dev dependency
matches the existing pinned Windows dependency and does not change production
composition. Native Windows cross-check still requires the missing MinGW compiler
in this environment and remains a CI lane.

## Continuation checkpoint — 2026-10-10 chat hotkeys

The existing General Hotkeys inventory now exposes six additional executable
actions: local, next, previous, Help, channel list and close. Next/previous cycle
only through Local, rooms confirmed open by the current chat projection, and
Private. Help uses the accepted `OpenRoom(Help)` session intent when needed;
Close uses the matching typed room intent and returns to Local only after queueing
it. Missing chat capability sends nothing and remains visibly unavailable. Loot
chat, server-message filters and other action families remain disabled.

## Continuation checkpoint — 2026-10-10 panel hotkeys

The observed VIP List, Battle List, Skills, Quest Log and Prey General Hotkeys
now toggle the same real client-panel state as the HUD shortcut bar. They expose
only admitted session projections and keep unavailable values unknown. Unknown
panel identifiers and the unresolved Lenshelp destination remain non-executable;
persisting a binding still does not count as implementing those actions.

## Continuation checkpoint — 2026-10-10 local display hotkeys

Minimap Zoom In/Out now changes the same bounded three-level runtime zoom used
by the visible minimap controls. Toggle FPS/Lag Indicator now changes the real
FPS overlay preference, validates it through the finite settings catalogue and
persists it through the existing settings save path. Minimap scrolling/floor
actions and latency display remain disabled because no corresponding projection
or navigable minimap state exists yet.

## Continuation checkpoint — 2026-10-10 profile and settings hotkeys

Open Hotkeys now opens the compiled complete-preferences browser directly on
General Hotkeys. Next Hotkey Preset cycles only through existing profiles,
retains the outgoing profile's live action bars, activates the incoming action
bars and persists the same validated settings object. A single-profile client
remains unchanged rather than inventing another profile.

## Continuation checkpoint — 2026-10-10 battle target hotkeys

Attack First/Next/Previous Target now selects from the admitted visible-entity
projection and queues the existing typed attack command. Cycling follows the
server projection order, wraps deterministically and shares pointer targeting's
current Creature-only boundary; players, NPCs, objects, off-floor and off-view entities
are not guessed to be attackable. Exact reference battle-list sorting/filtering
remains a separate acceptance gap.

## Continuation checkpoint — 2026-10-10 stop-all hotkey

Stop All Actions now cancels the local click-walk goal, replaces bounded queued
movement/cast/attack inputs with one stop intent and sends the protocol's existing
`attack_target(None)` command. An already-sent step remains server-authoritative;
the client does not pretend it can rewind an in-flight outcome. The resulting
server disposition is exposed through the existing combat feedback path.

## Continuation checkpoint — 2026-10-10 minimap pan hotkeys

Minimap North/East/South/West now move the real loaded-map crop one tile per
press. The offset is bounded to the nearest zoom radius, clamped to loaded
texture bounds and keeps the authoritative player marker in view. Floor Up/Down
remain disabled because the client has only the admitted mapped floor; no blank
or fabricated layer is shown.

The current Linux worker has the `x86_64-pc-windows-gnu` Rust target but no
`x86_64-w64-mingw32-gcc`. A fresh locked cross-target check stops in the
`aws-lc-sys` C probe before reaching Oteryn's Windows shell. Linux locked tests,
strict all-target Clippy and compiled page coverage remain green; native Windows
CI is still required for the complete composition.

## Continuation checkpoint — 2026-10-11 main synchronization

Normal AUTHORING merge `b000e20f` brought main `907412c5` onto the draft without
rebasing or rewriting its predecessor. The two textual conflicts in
`REFERENCE-AUDIT.md` and `SETTINGS.md` were resolved by retaining main's privacy
sanitization together with the branch's newer fidelity evidence. CI evidence on
remote predecessor `c177231d` does not qualify any successor; each published
candidate still requires fresh exact-head Linux and Windows checks.

The targeted reference pass closed the former General Hotkeys capture gap without
applying a reference preference. Current AUTHORING work restores reference key
semantics in a bounded increment: movement bindings accept only an unmodified
physical key, an explicitly cleared profile movement binding cannot fall through
to legacy global movement, Escape is exclusive to Stop All, and F10 is available
to the admitted action-bar/hotkey router while remaining the pre-admission
settings shortcut. New action-bar defaults use F1–F12. These claims require the
batch's locked tests, strict Clippy and a new native Windows check before freeze.

## Continuation checkpoint — 2026-10-11 effects dependencies

Effects now has the captured combined Clouds & Indoor row and exact opacity labels.
Light gates retained Ambient/Level/combined values; opacity stays independent.
Only Ambient has a runtime consumer; other sliders remain disabled. The former
separate indoor value is loadable compatibility state, hidden from search.
