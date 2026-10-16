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

## Authority and publication state

### Playable continuation — 2026-10-16 / AUTHORING

Owner resumed real gameplay work after the admitted Windows session showed blank
vitals/equipment and a chat refusal. Preserve these as distinct observations:
map/Play transition and live TCP were observed after Oracle deployed24fc332f
(includes domain2 fix4689d21f), but the actor of Enter World and the native
character identity were not conclusively bound. An isolated ordinary QA portal
sign-in confirms both Testini and Client Probe Redcde exist on that QA account;
the native selection capture showed only Testini. Do not repeat the earlier
claim that the missing HUD values prove every client consumer is broken.
Private receipt admission-20261016/acceptance-result.json has SHA256
e866c6298c5e5b97a5a86b83546edd692de7863837a5c13f64258a69eedee9f8.
No further injected input after the unexpectedly observed Play transition.

This batch normally merges accepted main24fc332f without rewriting this branch.
Resolve map/receipt/hotkey changes together with LOGOUT-WIRE-1, retaining
capabilities18 and21, paced movement, map USE and terminal/input fences. A committed
logout must rebuild the actual launcher UI, not leave an offline scene posing as
a live session. The logout chord wakes idle play, works with ordinary chat focus,
and stays blocked by dialogs/inactive windows. Unsupported logout does not kill
an older session; a refused exchange still publishes the projections it received.

Seed HUD projections synchronously from the admitted Session before scheduling
play; distinguish an empty admitted inventory/chat from a missing projection.
Add deterministic unpolled-task and terminal/logout-wake regressions. Log only
selected capability IDs and projection-presence booleans at the handoff; do not
log identities, items, resource values, chat, tokens, tickets or grants. This is
startup correctness and diagnostic evidence, not proof that Oracle supplies
vitals, initial items, equipment capability12 or a working CHAT runtime. New
Rune-use refusal variants get explicit non-success feedback without activating22.

Upstream accepted World digest7429f13b and its content remain unchanged; reconcile
only inputs_digest to the merged indexed inputs and repeat the double-build pin
check. Native qualification and real projection/movement/chat acceptance for this
successor remain PENDING. Preserve the currently open client until the owner
answers the batched session-close question; never terminate it blindly.

CI diagnosis at24fc332f: game-gate PASS; Agent governance job116886359002 fails
because six other active packets name merged PRs2003/2116/2016/2006/2045/2224.
Route their archive/lifecycle cleanup to the owning control plane; do not weaken
the validator or edit unallocated task records to hide the failure.

Execution driver correction: the first local routing packet was rejected for
empty optional tool fields and a non-Z timestamp; a merge command was mistakenly
started before checking that result. No further source edit followed until the
corrected packet validated ALLOW. Preserve that failed preflight as a tooling
error, not successful gate evidence. Subsequent execution requires ALLOW.

Draft PR #1942 remains in AUTHORING. Root is the sole branch publisher. Do not
freeze or qualify a candidate until all owned changes are committed, merged
normally with current `origin/main`, validated and reviewed as one exact SHA.
Before publication, fresh-read the remote branch head and use the guarded
publication-integrity helper with that exact predecessor and an external recovery
bundle. Never force, reset, rebase, amend or publish reference assets/private
account data. No production deployment, account mutation or database reset is in
scope.

The owner requires the Tibia Global client to be matched 1:1 and explicitly does
not count a control that merely serializes a value as implemented behavior.
Reference facts are retained in `apps/client/UI-REFERENCE-COVERAGE.md`,
`UI-FIDELITY.md`, `UI-REFERENCE-OBSERVATIONS.md` and
`UI-REFERENCE-SETTINGS.md`. Original captures remain private compatibility
evidence. Missing engine/server mechanisms must remain visibly unavailable and
must not be claimed as complete.

## Retained implementation history

The preceding settings, map, hotkey, receipt and qualification checkpoints are
preserved verbatim in [the retained task snapshot](../../evidence/OTV2-20261016-client-task-history.md).
Use UI-FIDELITY.md and the per-lane active records for exact historical sources,
Windows hashes and unresolved acceptance. Earlier native evidence never qualifies
this successor. Do not recreate completed settings pages or discard their behavior.

## Known completion gaps

- Original Help source and a live SaveFile dialog confirm ZIP export/import
  filters. AUTHORING replaces fixed-path JSON with native ZIP selection and
  atomic Oteryn-only export, with cancellation/error/round-trip regressions.
  ZIP contents remain unknown: the probe produced no archive and the original
  reference process disappeared; do not restart/sign in or copy global identity
  configuration. Schema-only metadata and private evidence are recorded in
  UI-REFERENCE-EXTRACTED.md. Continue offline/independently authored work.

- Spell/Object Custom Hotkeys need authoritative spell/inventory models and the
  observed picker compositions; the existing incomplete editors must not be
  enabled.
- Many General actions and panel interiors still lack genuine consumers or
  authoritative projections. Diagonal movement remains visible but disabled
  because the accepted movement wire has only four cardinal directions; the FPS
  toggle does not yet supply the label's lag indicator.
- Mouse preset/loot semantics, most Console and Game Window filters, graphics
  engine selection/AA visual parity, non-ambient effects, all audio families,
  Misc/Gameplay options and automatic screenshots remain pending.
- Action-bar production assignment sourcing remains absent; Clear/profile-remove
  confirmation semantics and edge-master behavior are not reference-proven.
- HUD condition projection/ordering, native visual comparison, populated module
  states and platform-specific graphics/audio choices remain open.
- Graphics Engine and Sound Device need real platform/runtime enumeration; their
  recorded static rows stay disabled and empty rather than hardcoding one probe
  machine. Factory-default state still lacks independent reference evidence.
- Secure remembered sign-in remains OFF/unwired. Actual sign-in stays system-
  browser PKCE. `protocol-oteryn`, multichannel WorldId/ChannelId separation and
  session-generation fencing remain invariant.

Presence, cardinality, saved intent, generated mockups and static reachability do
not establish 1:1 behavior. Continue in module-sized batches with shared validation.

## Oracle QA checkpoint — 2026-10-15

39ae passes native OAuth/Gateway/TLS; rejects unsolicited map domain2.
Next: owning server MAP producer repair; QA identity/E2E remains unproven.
Evidence/recovery:
../../evidence/OTV2-20261015-client-oracle-login.md.
