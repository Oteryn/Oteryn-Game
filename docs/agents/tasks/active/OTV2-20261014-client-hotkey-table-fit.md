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
preferences_hotkey_pages.rs,preferences_browser.rs,settings_ui.rs and
windows_shell.rs with its own acceptance records.
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

## e4 table qualification and a distinct native modal-routing finding

Published e4d0be1710828fac1e8a834fdee57f9048426e03 passes exact MSVC1.95:
client229/224/5,Platform16,Session84,TCP2,strict all-target Clippy,fmt,release.
Owned40864 exits0/129.34s. DX12/125%DPI:20pages/78independently rehashed
lossless WebP captures/0invalid, full-scroll/geometry/marker/footer/nonmutation
guards and independent decoded dimensions pass. General/action/custom first
captures are visually inspected: populated rows align with headers and both
targets are visible. Empty custom has no populated native assignment proof.
Release0.1.0+dev.ci.ge4d0be171082,exe SHA256
6b7e2d0a6313ed9afeedc20bd4a3ecf4e00eb46dbd4f5d0840ed8921e17ac4b0;
manifest52a6d10ac63d81f938465a0cda343442d33f6ac3c2f59735ccef237240d0d28c.
Build/audit are Downloads/oteryn-client-e4d0be17-20261014-native and
Downloads/oteryn-settings-audit-e4d0be17-20261014. Guarded recovery bundle
SHA256dfc31cbf09af9e9f1a574031a17a09c6ebcae17aaa3852c81388910ed0df00dc.

Normal source-bound client18404/HWND14289660 uses a NEW private child profile
under Downloads/oteryn-binding-e4d0be17-20261014. Every screen is inspected
before the next input, avoiding the earlier stale-coordinate cascade. Clicking
the now-visible second key actually opens the correct Chat Off editor. Ordinary
OS SendInput F10 then hides ALL Options instead of becoming the editor candidate.
Capture01 modal SHA2561d1b3eae250827dcc05fe9df6ca44345bcc8120f727e085be0ceb3afbf61d54d;
capture02 Welcome SHA256cc6b75fbd2395b4d99fa66fe7567d27caee186abed903efdcf816f3bb73aff29.
This is a genuine production finding, not the rejected profile fixture. No
preference file,clipboard,credentials,browser or admission action. Controller33116
closes only own18404 normally exit0; native-binding-finding.json SHA256
fd8adaba914b451648519ad362ba8d5844b819165ab2b3e3471c7bd140b60c87.

Root cause: prelogin shell relies only on egui-winit consumed, which may be false
when an active key-capture/confirmation modal has no focused TextEdit. Route
F10 to the real existing inner-dialog owner before the global Options toggle;
do not drop the raw event or manufacture a captured key. A production Options
regression initially fails "F10 hid binding". PL/EN binding/profile/reset cases
must retain modal/draft/current, real editor F10 candidate, inner Escape and
ordinary-F10 recovery. This is an input-layer correction, not a table redesign,
new dependency, protocol or private-state change. Successor needs fresh checks
and native reproduction; e4's table proof does not qualify the input repair.
Repaired merged-source Linux client230/218/5, strict all-target Clippy,fmt,
governance validator+124tests pass. The guard reads the existing active nested
dialog state; no new stored state, fork or fabricated key. All six modal cases
also explicitly clear egui focus before routing, rather than assuming that the
consumed bit models ownership. Ordinary unconsumed F10 resumes after dismissal.

## Exact195 native repair acceptance

Published1954627d1db8c0c00aa0c041bbe1f39366709b28 passes fresh MSVC1.95:
client229/225/5,Platform16,Session84,TCP2,strict all-target Clippy,fmt,release.
Owned40532 exits0/131.15s; measured qualification130.73s. DX12/125%DPI:
20pages/78independently rehashed and dimension-checked captures/0invalid.
Unchanged full-scroll/geometry/marker/footer/nonmutation guards pass.
Release0.1.0+dev.ci.g1954627d1db8; executable SHA256
874620effe034b2ed3b278aff401a72e2f547eb1ee9ca6d7f5ce0c55db30ce75;
manifest3701bf6de71e77502f7b16dcaf71222940052b05cdfcdbdbddb793979fe4a824.
Private roots: Downloads/oteryn-client-1954627d-20261014-native and
Downloads/oteryn-settings-audit-1954627d-20261014. Linux230/218/5 and
native229/225/5 are different platform suites, not summed acceptance cases.

Normal source-bound client45684/HWND1903916 uses a NEW private child profile
under Downloads/oteryn-binding-1954627d-20261014. Actual second-key click
opens the Chat Off editor; modal capture01 is inspected before the next input.
Ordinary OS SendInput F10 now leaves Options and the editor open, paints F10
and the existing conflict warning. Capture02 SHA256
21de2a5fbd20139ea2567342f330e1a2c411f6f11ecb3e5f34b6cf30c8d3b624.
Escape closes only the inner editor; capture03 shows the parent and unchanged
blank second binding. Ordinary F10 then hides/reopens Options with the General
Chat Off draft intact; captures04/05 are inspected. No candidate is accepted,
no Apply occurs and the private active preferences file remains absent.
This fixes the previously retained e4 negative reproduction, not merely a saved
value. Native profile/reset F10 paths are not inferred: the six PL/EN actual-widget
component cases separately cover them. No physical keyboard, gameplay or E2E claim.

All6 PNGs independently hash/decode1125x775/0invalid. Controller8984 exits0/1.43s,
normally closes only own45684 exit0 without force. Final generated report:
native-binding-fixed-report.json SHA256
5a0ca6f15f627d9cd7649ee6c11b838f2c196cdf21029f1d912e182af2d6965e.
No clipboard, personal preferences, browser, credentials or admission action.
All failed/partial earlier fixtures remain retained and explicitly unqualified.

One fresh195 CI snapshot returns only the successful protected-base authority
audit38613585458/job116022856484; the targeted PR-triggered workflow query
returns no runs. This is NOT a green Merge gate/game-gate or closure proof.
Do not inherit690/ab111 CI, poll in a loop, manufacture a retrigger commit or
freeze/merge. Control plane needs exact final-head required CI/review at readiness.
Source repair is published; this evidence-only checkpoint stays locally committed
with recovery, rather than cancelling useful CI with a status-only source push.
Parent client task, B2/B3, audio bank and separate VK_PACKET route remain open.
