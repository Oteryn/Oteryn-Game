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

AUTHORING on draft #1942. Published anchor:
72b0ecf958bc79bf03c677b654c7019867ca37ae. Not frozen or merged.
Root is sole publisher. No new worker delegation in this increment.
Owner scope: complete native client/UI/settings plus necessary Platform/engine
work. Preserve missing server values as unknown; prepare future controls without
claiming their absent mechanics work. No production deployment, database reset,
password rotation, merge or third-party proprietary asset redistribution.

## Current scope and evidence

Owner requested faithful UI and every observed option/function. The explicit
acceptance matrix is apps/client/UI-FIDELITY.md. Counts are not fidelity proof.
This increment replaces generic Console, Effects, Sound, Battle Sounds, UI Sounds
and Shortcuts pages with dedicated compositions. Shortcuts use displayed/available
lists with add/remove/order editing inside the settings draft. Live panel registry
now refreshes after Apply without overwriting unsaved manager changes.
Private source captures establish composition, not redistributed assets or
invented defaults. Audio/effect selections remain bounded pending intent.

Prior published 487cb644: native 124 library +26 application +5 launcher tests,
strict all-target MSVC Clippy and release pass. Linux library125 tests pass.
Current increment: native124 library +29 application +5 launcher tests
(158 total), strict all-target MSVC Clippy and release PASS. Native private
capture verifies two-list composition; painted arrow controls replace
unsupported font glyphs. Scope format/whitespace, clean publication governance,
repository policy and59 governance regression tests pass.
Source transfer uses SHA-256 baseline/staged checks, exact file backups and
refreshed timestamps. Use only the own executable HWND for capture and guarded
foreground clicks. Windows checkout:
C:/Users/barte/Downloads/oteryn-client-main-20261007.

Nine action rows/108 physical chords, per-row locks, assignment clearing and
modal/held-key/session fencing are implemented. Edge master toggles preserve raw
row selections and legacy migration preserves locks/chords. Full settings fit
32KiB bound. Original client assets and private account data are not published.
28 shortcuts,22 prepared panel views,20 Cyclopedia subpages,six dialogs and27
settings sections exist, but many arrangements and consumers remain incomplete.
Server HP/MP/inventory/chat absence stays unknown. Wheel topology is unverified;
no fabricated allocations, progression or records. Gem/Fragment private captures
establish their group compositions only. User wants missing engine mechanisms
completed later; accepted authority/projection contracts still govern that work.

## Display/input authoring batch

Dedicated Controls, Interface, Graphics, Game Window and Gameplay compositions
replace generic fields. Rotation has separate Ctrl/Shift/Alt modifier intent,
not a required letter key. Whole-stack Ctrl intent, link COPY warning and three
expiry destinations have distinct bounded bool keys; legacy intent remains valid
and is not silently repurposed. Delay range and target-dropdown membership are
still unverified. No input/clipboard/expiry consumer is falsely asserted.
Graphics consumes existing fullscreen/VSync/FPS/window fields. Unlimited toggle
restores the previous draft frame limit; FPS range follows existing30..360 bound.
Current FPS stays unknown because diagnostics are not supplied to preferences.
Existing JSON schema/read compatibility is preserved. Compact encoding saves
all new choices inside the unchanged32KiB bound; maximal escaped-text roundtrip
regression covers actual save/load. Native124 library+30 application+5 launcher
tests(159 total), strict all-target MSVC Clippy/release PASS. Private actual Windows Graphics/Shortcuts captures verify current layouts;
clean publication governance/policy and59 regression tests PASS.

## HUD/capture/confirmation increment

Dedicated HUD groups own/other, Harmony position radios, arc controls, six directly
visible condition rows with independent HUD/bar switches, and status-bar flags.
Other conditions/order and undiscovered arc-size choices remain unverified.
Screenshots has17 separate triggers/two columns, three capture flags and an
unavailable folder action. No capture pipeline or retention is falsely enabled.
Misc has seven confirmation/session choices, without storing credentials or
activating remembered sign-in. Existing legacy intent keys remain accepted.
Movement editor wraps vertically; ASCII physical arrow-key names replace missing
font glyphs. New actual click/clip regression checks all four direction choices
in PL/EN and WASD preset validation. Native124 library+31 application+5
launcher tests(160 total), strict all-target MSVC Clippy and release PASS.
All configurable preferences fit both existing count/byte bounds and32KiB
compact save with maximal escaped text; save/load roundtrip PASS. Clean
publication governance/policy and59 regression tests PASS. Native private HUD
capture verified own/other groups and exposed excess empty headings/row height.
Follow-up removes empty headings and scopes compact spacing to HUD. Native160
tests, strict all-target MSVC Clippy and release PASS on the compact source.

## Related PRs and authority

Product draft #1942 stays AUTHORING, not merge-ready. Root is sole publisher;
fresh-read live head, validate owned paths in clean /workspace/publish-client,
then guarded publication with exact expected predecessor and recovery bundle.
No force/reset/rebase. Preserve unrelated server/login-local diagnostics and two
older untracked task records. Govern only this task's own record.
CI #1943 frozen4b535efc and #1944 frozeneba78bf have clean required review and
remain unmerged/untouched. Owner approved their preparation and funded required
review under standing policy; no authority to merge. CI still targets Rust1.94
until those fixes integrate; client requires1.95. Platform #1484 head787028f
passes75tests737assertions/PHPStan10/Pint. Remembered native sign-in primitives
are OFF/unwired; first/repeated login uses accepted system-browser PKCE. No
credential webview or insecure password persistence. One earlier admission
failure succeeded on normal retry; its cause is undiagnosed by UI work.

## Next acceptance

Complete native qualification/captures, publish current page batch and continue
remaining advanced arrangements and panels per UI-FIDELITY.md. Required checks:
client all-target native tests/strict Clippy/release, scoped formatting/whitespace,
clean publication governance and applicable policy checks. Save concrete evidence
and remaining consumer gaps; never describe this draft as the finished client.

## Owner-directed execution plan, 2026-10-09

Owner requested a concrete plan and execution after rejecting catalogue-based
completion claims. apps/client/UI-EXECUTION-PLAN.md defines screen-by-screen
visual and behavior acceptance across preferences, hotkeys, panels and login.
First target: native options shell/HUD. Reference pages no longer waste body
space on global search/footer implementation notes; Oteryn account access moves
to separate navigation and Reset retains draft-only behavior. HUD arcs form two
columns and condition switches align in full-width columns. Native161 tests
(124lib+32app+5launcher), strict all-target MSVC Clippy/release PASS. New full-dialog
PL/EN regression proves lower status controls/Reset/OK/Apply/Cancel visibility
and independent HUD/bar condition clicks. Native Windows capture compared: fixed full-width HUD/bar columns, all six
condition rows and both lower flags/footer visible. Account/network access is
sticky below category scroll; PL/EN regression clicks it and verifies cell
alignment across differently sized condition labels. Private own-client capture:
/workspace/artifacts/oteryn-client-hud-qualified-native-20261009.png.
Original NAS session confirmed active; private research uses exact client window,
never Apply/Reset or character commands. Additional unknowns remain unverified.

## Own-resource renderer authoring checkpoint

Published plan/options-shell head49e2a6ee. Private original client.en.qm research
verified Small/Default/Large arc sizes and own-HUD bars/arcs dependencies. The
new actor_hud module reads authoritative vitals, projects onto the same scene
viewport and draws only explicitly selected HP/MP. Missing/max-zero data draw
no fabricated resource. Native164tests/strictMSVCClippy/release PASS; Linux
headless3geometrytests PASS. Actual live-session capture remains pending.
Names/marks/Harmony/conditions and remaining HUD table/order are unfinished.
Do not infer full HUD behavior from this bounded renderer checkpoint.

## Owner-requested Astra pass

Owner explicitly requested Astra. Astra specialist edited only HUD composition:
shared own/other frame/divider, indentation, master-dependent child editing with
retained values, arc dependencies and framed condition columns with working
six-flag aggregate switches. Two regression tests cover disabled-click
preservation/re-enable and bounded aggregate edits. Native166tests (124lib +
37app +5launcher), strictalltargetMSVCClippy/release PASS. Linux26headless
composition/behavior tests PASS. Scoped formatting and whitespace PASS.
Actual in-world visual acceptance remains pending; mouse position changed under
automation and its guard withheld input. No original-client changes in this pass.
No condition reorder/icons, new condition membership or consumer completion
claim. Root sole publisher, product draft stays AUTHORING. Model leading this
chat cannot be switched by the assistant; user must use the app model selector.

## Modern Oteryn visual direction

Owner clarified: reference functionality/controls and panel organization, modern
Oteryn presentation rather than legacy chrome. Shared chrome now uses slate
surfaces, restrained gold selection, rounded controls and larger text; procedural
gray noise/bevels removed. All 28 shortcut symbols use pinned Lucide 0.468.0
assets with upstream license, source SVG hashes and lossless high-DPI atlas.
License is embedded and readable in Help. Empty equipment silhouettes remain
explicitly empty-slot symbols, not item art. Actual item rendering and modules
are unfinished. No new gameplay behavior or full-reference-parity claim.
Native Windows167tests (124lib+38app+5launcher), strict Clippy and release PASS.
Linux26composition tests PASS; repository governance/policy PASS. Actual Windows
Options capture checked at default 900x620 logical dimensions: footer reachable,
new chrome visible. In-world and Linux visual acceptance remain pending.
Private evidence: artifacts/oteryn-modern-options-native-20261009.png.
Draft1942 remains AUTHORING; no merge, production or database changes.
