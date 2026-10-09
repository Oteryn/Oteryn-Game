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
1ec067ff5900d535ef79c75f35c9f40519e17005. Not frozen or merged.
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
