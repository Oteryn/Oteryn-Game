# OTV2-20261013 — authoritative client step receipts

```yaml
task_id: OTV2-20261013-step-pace-client-1
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

## Authority and scope

Continuation of the owner's existing client task, one root writer, PR1942 in
AUTHORING. Architect6146789689 supplies receipt criteria; CP6146799516 withdrew
its competing allocation. Accepted producer2034 is on main59f9fb12. Producer2041
is still open at4f4fbadc and emission remains OFF. No protocol IDs/codecs,
game-server, deployments, accounts or trust stores are written here.

Owned implementation: crates/session/src/lib.rs, apps/client/src/input.rs and
play.rs, plus minimal existing windows_shell.rs wake-up integration. No new
modules/dependencies. Parent task/UI-FIDELITY retain map/native evidence.

## Current authoring

StepReceiptOutcome carries command/generation-bound wait and monotonic receipt
time, stamped before awaiting the claimed spatial delta. It wraps the existing
StepOutcome, preserving public legacy literals and server/tools fixtures.
Native receipt-bearing TOO_EARLY returns to the deadline gate immediately;
receiptless retries retain the existing bounded doubling backoff.
Raw golden MOVED100/max111150, TOO_EARLY37 and legacy tests use withheld-delta
handshakes without sleeps. Existing malformed tests still fail closed.

Native dispatch consumes the typed receipt; the mailbox checks its command and
channel generation before setting a monotonic deadline under the reservation
lock. Actual channel dispatch runs10,000 jittered simulated outcomes: zero early
sends, lateness at most20ms, no bursts or input consumption before due. The timed
run_session bridge explicitly rejects the legacy methods, proving real routing.
Receipt TOO_EARLY preserves the goal/latest single buffered intent; a drained
Stop still cancels retries. Own-actor quads interpolate for the MOVED duration,
not a speed guess. Corrections/floor changes and session end reset presentation.
Native idle Notify wakes before any first byte; started frames finish intact.
Tests cover stored permits, withheld frame bodies and the following aligned frame.
Full Linux client227/196/5+Session82, strict all-target Clippy/fmt pass. Native
MSVC/DX12 and same-head CI are next, not inherited from map7335 qualification.
No gameplay/emission qualification is claimed. Fresh admission/reconnect and
fatal resync create/reset the actual
channel; live resume/mid-session snapshot replacement are not implemented and
must not be reported as proven by those fresh-admission cases.

## Native E2E dependency

Needs merged2041, operator-authorized temporary ON/OFF with rollback, dedicated
QA account/character/USE item and approved harness hooks. Preserve every attempt;
only private logs/captures, public summaries. No automatic login or personal
account mutation. Page audits alone never qualify deployment/rollout.
