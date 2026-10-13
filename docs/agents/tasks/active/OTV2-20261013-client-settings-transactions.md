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

Focused routed matrix, locked all-target Linux client228/200/5+Session82,
strict all-target client Clippy and workspace fmt pass; governance60/diff pass.
Exact-source Windows qualification follows; the toolbar case is Windows-only.
Compiled page/unit acceptance does not prove physical picker/clipboard/trackpad
parity, live gameplay or full1:1. Older exact fabf441a evidence is preserved,
not inherited by changed code. Preserve every native attempt/artifact.

## Current external boundaries

Bounded9196 CI read has no failures, but required jobs remain pending and one
governance run is cancelled. Producer2041 is open e4ac000c; no QA/switch/harness
setup was supplied to6149042654. Acceptedmainb5cdd82c adds LOOP-CLOCK-1 cap19/
domain18 registration/pure clock, not live emission. Do not advertise19 or call
lighting unblocked. B2 remains operator/QA dependent; no account/deploy action.
