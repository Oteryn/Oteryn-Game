# OTV2-20261014 — hotkey table column fit

```yaml
task_id: OTV2-20261014-client-hotkey-table-fit
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Parent client-game-ui, sole root writer, AUTHORING. Owned runtime:
apps/client/src/preferences_hotkey_pages.rs and its own acceptance records.
No HUD/graphics redesign, protocol, capabilities, audio backend, dependency
fork, account or deployment change. Native gameplay/E2E stays excluded.

## One shared table defect

Normal qualified690e22ad client21440 at125%DPI provides private capture
C:/Users/barte/Downloads/oteryn-profile-690e22ad-20261014/00-general-profiles.png,
SHA256169e293236bf9b7ad4a5a01a97dc2b580c3cf1b5d5bc889f9d559b10e183844c.
First/Second headers use equal-third ui.columns while the data Grid uses natural
action-name widths. The first header is visibly displaced from its key cells;
the second key cell is partially scissored at the right edge. Custom Text may
contain much longer labels. General, action-bar and custom tables share the root.

Actual rendered egui regression initially fails in Polish at620logical pixels:
First header218.34 versus first key-column383.75. Native audit's page/marker
counts did not establish per-row column alignment. This is not a redesign request.
Use the same upstream Grid for headers/data and its supported max_col_width
wrapping, reserving the two existing120-point key targets and two gaps. Preserve
labels, profiles, availability, bindings, context routing and draft behavior.
Independent counterfactual keeps the shared header but removes only wrapping:
it fails specifically F11 cell clipped at width620. Restoring the bound passes.
Expanded24-case geometry regression covers PL/EN,two body widths,all three
tables and Chat On/Off, including long Unicode Custom Text. Clicking the second
key opens the actual correct-context general/custom editor without changing the
draft; native bar cells remain disabled and never open an editor. No activation
or gameplay execution is inferred from those UI fixtures.

## Native evidence boundaries and retained failed probes

The normal F10/focus/draft packet is fully retained in prelogin-focus.md.
Native profile Add + actual Apply creates only the owned private UTF8 settings:
two profiles,selected1,name Żółw QA, SHA256
0d8e1515f62751e45c488c12c2739130cb6d804967548ba66969db730b3260b9.
The native selector visibly shows that new name. This is profile editing proof,
not gameplay hotkey execution or original factory/removal parity.

First profile assertion wrongly reads UTF8 JSON with Windows PowerShell's
default ANSI decoding; explicit Encoding UTF8 proves the correct name. Retain
that failed fixture. Later fixed-coordinate Copy/Remove/binding attempts are
NOT acceptance: capture03/04/05 show Welcome and07 shows no binding modal.
A stale click reaches the unconfigured-portal message, with no browser,
credentials, account data or server request because child endpoints are unset.
The final stale click closes only own21440. PID/window fence refuses subsequent
input; readback confirms21440 no longer runs and private saved bytes are unchanged.
Do not claim Copy,Remove Escape or F10 capture from those filenames. Further
native drivers must assert visible Options/modal state after transitions rather
than assuming an owned window is still the intended screen.
Independent report hashes/decodes all9 retained PNGs1125x775/0invalid, but they
are explicitly partial/failure evidence, not9passed cases. Report SHA256:
8876c82fad25e4c57abdfabe72c8c2de44c3d1e2f0a8d2e8386cf1d8859588f7.
Normal-client F10 packet and its report8030baaf remain separately passed.

Linux changed client tests pass230/217/5, strict all-target Clippy and fmt pass;
governance validator+124tests pass. This is one shared production cause, not a
new issue/PR per table. All earlier native focus/failed profile evidence remains
preserved in this material batch. Fresh Windows qualification follows. Native
capture totals may change with legitimate upstream text wrapping; require all
20pages and unchanged full-scroll/nonmutation/marker/geometry/hash/dimension
guards, not the old source's incidental78capture count. No CI/validator is relaxed.

Checks and fresh changed-source Windows qualification follow the grouped repair.
Prior690 native/CI evidence remains source-bound and is not inherited. No
status-only source push, paid review trigger, freeze/MQ or1:1 claim.
