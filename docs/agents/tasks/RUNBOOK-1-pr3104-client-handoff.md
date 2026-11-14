# PR3104 client handoff — stopped by owner

Task: RUNBOOK-1 / [#3169](https://github.com/Oteryn/Oteryn-Game/issues/3169).
Delivery: draft [#3104](https://github.com/Oteryn/Oteryn-Game/pull/3104).
Status: paused on the owner's explicit save-and-stop instruction, 2026-11-13.
Do not resume implementation, desktop tests, polling or subagent work without a new owner request.

## Saved implementation

The validated product head is `0c4ffa63a323c861e1b4ae7caa53ca80853fba71`.
This handoff is a documentation-only successor; it does not change that product code.

- The shared Bevy frontend retains the existing Rust admission, session, command,
  scene, audio and preference owners. Run instructions remain in
  [apps/client/BEVY.md](../../../apps/client/BEVY.md).
- Market purchases now use a named, server-provided sell offer, bounded quantity,
  explicit price/total confirmation and the original offer UUID. Exact snapshot,
  session, pending and modal checks occur under the owner lock. Duplicate/stale
  consent is rejected; funds, items and books are never updated optimistically.
- Container headers now expose bulk transfer to another real open container.
  Both displayed container snapshots are bound atomically; the retained dispatcher
  preserves item order and stops on the first refusal.
- The opt-in `--ui-audit` exposes `play_observation` with the current generation,
  map position and last actual step reply: reply count, command ID, result server
  sequence, `accepted` and `moved`. Only a successfully applied `PlayEvent::Stepped`
  creates a receipt. Local input/facing and unrelated pushes cannot create one;
  a fresh play view resets it.

## Qualification of the product head

- 1,100 client library tests and 6 launcher tests passed.
- 7 runtime world-item integration tests passed.
- Formatting, strict Bevy/default-client Clippy, architecture and Windows GNU
  cross-compilation passed.
- Native Windows MSVC build passed. Its binary SHA256 is
  `23D00AF7A84A7BE3C752032C4A203BADCDD314567CA73E9D25B16D8C28DE3D82`.
- The purchase, container and step-observation caller regressions were genuinely
  red before implementation/repair and green afterward. No assertions were weakened.

These results do not establish live admission, native macOS execution, visual
parity, performance under load, or completed CI/formal PR review. Earlier Linux
GPU/offline OS-input evidence belongs to the earlier checkpoint described in the PR.

## Actual live-test state at stop

The Windows host opened Sign in and submitted the connection form through guarded
OS input, starting ordinary browser OAuth. Browser inspection stopped without
entering credentials, and the attempt returned offline. Character selection,
an admitted server world and an accepted movement step remain **unqualified**.
No `walked.attest` was written.

The unfinished automation issue is browser accessibility calibration: Chrome's
address-bar value omits the scheme. Its document `ValuePattern` exposes an absolute
URL and matched the approved test origin, but the actual path was not `/login`.
Do not guess an origin, assume that path, or log OAuth URLs/state/tokens.
The managed `LegacyIAccessiblePattern` type was unavailable in Windows PowerShell.
No browser settings, firewall or WSL network configuration were changed.

The owned native host was stopped; no unrelated app/browser process was stopped.
All subagent work was completed or interrupted. The coordinator was asked in
PR3104 to clean up the held RUNBOOK-1 stack without treating the run as a successful
walk. The acceptance issue remains unfinished.

## Resume only after a new owner instruction

1. Obtain fresh coordinator READY/profile information through PR3104; previous
   private profiles are ephemeral. Check Windows-side gateway/game reachability
   and native certificate, world-root and UI-asset paths before launching.
2. Use ordinary browser authentication and the real native character/channel picker.
   Keep credentials runtime-only in the supplied private profile; never commit them.
3. Observe a new same-generation step receipt with both `accepted` and `moved`
   before recording a walk attestation. A keypress, facing change or screenshot
   alone is insufficient.
4. Continue functional gaps before visual polish, following the owner's priority.
   Depot/stash portable callers and other specialized windows remain unfinished.

Private operator scripts, build logs, screenshots and original/reference UI assets
remain outside Git. The workspace handoff/artifact index is retained separately at
`/workspace/artifacts/client-bevy-integration/NEXT_BATCH_HANDOFF.md`.
Do not add those private files or proprietary images/fonts when resuming.
