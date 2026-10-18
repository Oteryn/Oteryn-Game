# OTV2-20261013 — B1 supported settings transactions

```yaml
task_id: OTV2-20261013-client-settings-transactions
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Owner continuation, existing sole root writer, draft AUTHORING. B1 follows
apps/client/UI-EXECUTION-PLAN.md. Owned runtime: settings_ui.rs,
login_screen.rs and game_ui.rs; no server/protocol/account/trust/deploy writes.

## Batch and acceptance

Immediate language selection saves a current-only candidate before committing
current/draft language and the applied flag. Failure leaves active/draft/flag
unchanged and the toolbar restores its visible selection. Success does not
apply unrelated draft edits. Actual toolbar regression is Windows-only;
its zero-match Linux filter is not counted as a pass or native evidence.

A child-only LOCALAPPDATA fixture drives ordinary production widgets in PL/EN,
including normal scrolling, navigation, Cancel, Apply/reload, an actual staging
write failure, visible error and successful Ok retry. No process-global env
mutation or personal preference/clipboard access. Consumers exercised: real
redraw FPS and retained limit, held-key scheduling from changed delay, stable-
Context custom/Big/native cursor meshes, HUD health/Harmony placement, target
frame/highlight gates, receipt-time chat text and action-row viewport transform.
Unavailable cursor/HUD/timestamp children and a disabled delay arrow remain
inert. Profile mirrors are compared by effective consumers after canonical load,
not incorrectly treated as raw-cache parity defects.

## Exact-source qualification

Published sourcea65e7428fcbd92221997b87ffe22bc70451a7426 normally merges accepted
main376be56bcda0da6c1c014bc50429c521bec3dcdc after owned B1 commit4128953.
Post-merge locked all-target Linux client228/200/5+Session82, strict all-target
client+Session Clippy, workspace fmt, governance60 and diff checks pass.
Exact Rust1.95 MSVC client227/206/5+Session82, strict all-target Clippy/fmt and
release build pass. Both named routed-transactions and Windows-only toolbar
regressions execute and pass; the earlier zero-match Linux filter is not proof.

Release0.1.0+dev.ci.ga65e7428fcbd; executable SHA256
3d4de918e4aaa7271e775a76c84018f6a80cb98ae8bbdb1b17abfe38445a0213.
DX12 at125%DPI:20pages/78 independently rehashed captures/0invalid; manifest
7174e7b965670718f3bb991cd91b0649cfd1e3e32702158c8e49237239bc2078.
Private build C:\Users\barte\Downloads\oteryn-client-a65e7428-20261013-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-a65e7428-20261013.
All20 first-page captures were visually inspected from those exact artifacts:
legible content and stable visible footer; longer pages also have full automated
scroll/marker/footer/geometry/non-mutation checks. This does not assert every
captured pixel or every original interaction is independently compared.

New loginPID3008/window3346350 responds with exact path/hash and source-correct
U+2014 title. Existing public launcher/CA and explicit asset root are reused;
no automatic sign-in, account/trust/deploy/install/personal-preference mutation.
Previous37764/38236 were absent at the diagnostic read, not terminated by root;
their binaries, checkouts and earlier failed attempts remain preserved.
Native batchPID23212 exits0. Guarded recovery
/workspace/oteryn-client-publication-a65e7428.bundle SHA256
2b87a5f8745feb122e3dfa853487097903c9377d5e3ed0028b2f7116921763cb.

Compiled page/unit acceptance does not prove physical picker/clipboard/trackpad
parity, live gameplay or full1:1. Older exact fabf441a evidence is preserved,
not inherited by changed code. A documentation-only successor retains byte-
identical runtime evidence, not its predecessor's final-head CI/review status.

## Current external boundaries

Bounded same-a65 CI snapshot:13successful/4running/0failed; not all-green or
final qualification. Producer2041 merged at2026-10-13T16:14:38Z (mergea076a4ff,
sourcee4ac000c) and is included in acceptedmain376. This closes the source-merge
prerequisite, not deployment/switch/native gameplay. Canonical #1622 remains292
comments, with no operational answer to6149042654; subsequent6149333747 is an
unrelated Player Swarm comment. Dedicated QA/item, deployed node and authorized
ON/OFF/rollback/harness/retirement choices are still unsupplied.
LOOP-CLOCK-1 cap19/domain18 registration/pure clock is not live emission.
Do not advertise19 or call lighting unblocked. B2 remains operator/QA dependent;
root performs no account/deployment/restart/switch action. Keep the architect's
B2-before-later-activation sequence; read-only B3 source mapping is preparation.
