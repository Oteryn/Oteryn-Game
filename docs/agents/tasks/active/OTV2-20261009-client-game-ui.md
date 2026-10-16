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

### World-input companion repair — 2026-10-16 / AUTHORING

7d71a035 fixes character clipping/ZIP; CI supply-chain now passes116942718400.
World job116942718319 and the unchanged local release compiler both reject
stale9d8ccdae inputs after the Cargo.lock patch. Root missed that companion
check before freezing. Explicitly return to AUTHORING, refresh only inputs_digest
to the compiler-derived5ff3f898, then double-build the unchanged7429f13b bundle.
No World content, identity, start, validator or protection change. This is a
material CI repair, not a retrigger. Earlier qualification does not qualify
the successor. Owner has now confirmed the a534 Enter World click; selected
identity remains unbound. Test13344 closed normally by approved WM_CLOSE,
without force. The isolated QA browser again confirms both known characters.

### Launcher / dependency repair — 2026-10-16 / AUTHORING

a5342336 was published and qualified on Windows: client236/226/5 + Session88,
strict Clippy/fmt/release pass. Immutable executable SHA256
da4586db56ef0cbe2e0d1173d6a900b8009f1132b1867d3ea6942749d248d5ef.
DX12 audit validates20 pages/78 captures/0 hash mismatches; the PowerShell
wrapper subsequently failed on a null ExitCode. Manifest success is established,
but native audit exit0 is not claimed. These results do not qualify this successor.

Normal QA token exchange and owner-character read passed on a534. The native
capture showed only Testini, whereas the Platform operator read model has both
QA characters AVAILABLE. A standalone exact-egui0.35 probe and a failing actual
launcher/list regression establish the cause: fixed_size(width,0) clips the
scroll viewport to64px and hides the second row. This supersedes any suggestion
that the one-row capture proves API/projection loss. Public edge routes to the
real authenticated Platform handler; no static owner-character fixture was found.

PR1942 was explicitly returned from FREEZE_SHA to AUTHORING before edits. The
repair bounds only panel width and lets contents determine height. Shared actual
panel/list rendering tests check fully unclipped names and real second-row clicks
in both languages at900x620/1280x720: regression RED, then GREEN. Portable launcher
tests now run on Linux as well as Windows, without enabling a Linux native shell.

Exact-head CI job116934077632 rejects yanked zip9.0.1, used only by client options
archives. Upgrade upstream to9.0.4; Cargo changes only its version/checksum. Stored,
flat, bounded single-entry checks, duplicate/encryption/path refusal, atomic export
and import-to-draft behavior remain unchanged. Local cargo-deny passes all checks;
no advisory allow-list, trust or CI-policy relaxation. Full successor qualification
and fresh same-head CI/native evidence are still required.

A later a534 entry attempt reached ticket/Gateway/TLS and UnexpectedEof before
admission. Its operator/selected character is UNKNOWN: root did not click Enter
World. Injected input stopped; the owner was asked about concurrent interaction.
No ServerAccepted or correct-QA gameplay/chat/movement/logout acceptance follows.
No server/config/DB changes or forced client termination were performed.

### Playable continuation — 2026-10-16 / AUTHORING

e3ca05b1 was published and frozen for qualification, then explicitly returned to
AUTHORING after a verified profile-consumer defect: main's raw Ctrl+L/Q probe
bypassed profile changes while the observed Logout row remained unsupported.
The first regression run now fails2/4 for exactly that missing resolver action
and acceptance of extra modifiers. The repair connects client.logout through
the existing per-chat/profile resolver and native general-action consumer;
dialogs, inactive windows, wrong contexts/modifiers and OS repeats do not logout.
The raw utility probe is no longer used by the native router.

Prior e3ca Windows source qualification passed235/226/5+Session88, strict Clippy,
full fmt and release; executable SHA256
d9989e0567dec155d5498df3996b6362e93a2c591e638759407ec7ae82d9c178.
It does NOT qualify the successor. Git's initial CRLF checkout failed the raw
checksum/fmt preparation; preserved LF worktree bytes match published hashes
and pass. No policy/formatter weakening. Old39716 had already exited before
the owner-approved normal-close driver checked it; no WM_CLOSE/force was sent.
Read-only Platform projection currently confirms BOTH QA characters AVAILABLE;
do not infer that the earlier one-row native capture is current projection truth.

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
