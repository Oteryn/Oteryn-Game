# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Status: implementation in progress; private local source, not frozen or merged.
Owner scope: complete client and necessary engine/protocol functionality together.
No production deployment or database reset. Single writer: root.

First implementation batch consumes authoritative session projections for inventory,
open container, visible actors and chat; sends bounded chat intents through Session;
fits the map between panels with a shared rendering/picking transform; persists
inventory/battle/chat visibility with backwards-compatible defaults; separates text
and modal input from movement. Vitals are shown only when supplied by the session.

Owned paths: apps/client/src/{game_ui,layout,play,settings,settings_ui,windows_shell,
main,lib}.rs and crates/renderer/src/windows.rs. Existing private login/settings
implementation remains intact. The Windows source copy was compared against the
cloud checkout; differences in the six replaced existing files are only this batch.
Original Windows source backed up before replacement.

Validation: Linux client library tests 81 passed; strict library/test Clippy passed;
Windows GNU cross-check passed. Native Windows tests passed (81 library + 2 application); release build passed.
Client launched on owner PC via the existing local launcher. Login window
verified; portal unavailable because Docker Desktop engine was stopped.
Docker launch inherited a missing ProgramData variable from the remote shell.
Restored the canonical CommonApplicationData path for the child process only.
Docker now running; existing Game/Platform/gateway/database containers resumed
automatically. No factory reset, DB reset, or test-password rotation.
Native Windows strict Clippy also passed.
No native runtime claim before build and UI inspection.

Remaining completion scope: item names/icons and actions, full actor rendering/HUD,
minimap, skills/quests/social/cyclopedia panels, action bars and hotkey profiles,
chat filters/notifications/accessibility, audio, captures, Linux GUI composition.
Add required owning engine projections and commands, including missing mechanics;
current server capability availability is a sequencing constraint, not a scope cut.
Draft reference audit PR #1941 contains documentation only, not this product source.

Portal and public directory HTTP 200. Directory initially empty: the Game
container restarted only its sleep entrypoint, not the local server process.
Reviewed existing restart-game-ui.sh: scoped local node authorization/assignment
replacement, no database reset, account mutation or factory reset. Restarting
the existing local node in Ubuntu-24.04 with preserved character data.

Local restart was blocked by expired one-day qualification PKI (2026-10-09
07:53 UTC). Renewed 13 local certificates for 30 days, preserving existing keys,
subjects, SAN/EKU and local issuer boundaries; validated chains before replacing
23 matching public-certificate copies. Backups retained in local runtime tree.
Reloaded PostgreSQL and nginx certificates; restarted only the local gateway.
No DB/schema/account write, password rotation or production trust change.

## Owner-requested in-client authentication — proposed change, not accepted contract

Owner requests the entire sign-in-to-world flow in the native client window,
including account and character creation. Cross-repository Platform changes are
explicitly authorized. Preserve the existing browser PKCE implementation until
its replacement has an owning contract and independent security review.

Current constraint: Platform OTCLIENT_GAME_AUTH_CONTRACT requires the external
system browser and rejects embedded credential webviews. Threat model GA-006
and ADR-0009 must be reconciled with any accepted replacement. This packet does
not amend those contracts or claim an embedded webview is compliant.

Proposed first-party native screens, sharing the existing egui window:

1. Sign in: email, masked password, reveal control, sign-in action, registration
   and recovery navigation; explicit loading, cancellation and retry states.
2. MFA: server-selected supported challenge, expired/rejected challenge feedback
   and recovery-code alternative when permitted by Platform policy.
3. Registration: email, password and confirmation, server policy feedback;
   verification state when required. No success before authoritative acceptance.
4. Recovery: neutral acknowledgement preventing account enumeration; completion
   depends on possession of the required recovery proof.
5. Characters: authoritative list, world availability, native character creation
   and its pending/committed/rejected result; no legacy Canary substitution.
6. Channel selection and admission: current world/channel directory, progress,
   cancellation, unavailable-world feedback, then the admitted live session.

Platform remains the credential, MFA and security-generation authority. A new
bounded native authentication contract must define challenge lifetime, attempt
binding, replay handling, rate limits, uniform failures, revocation and token
scopes. Do not introduce an OAuth password grant or a confidential client secret
in the distributed binary. Passwords and recovery codes remain transient and
must never enter logs, settings or crash diagnostics. Persistent sign-in needs
OS credential storage and defined token rotation; remember-email is separate.
Game admission still uses the existing account ownership and session-generation
fences and canonical WorldId/ChannelId boundaries.

Acceptance requires real local account creation, valid/invalid credentials,
configured MFA challenges, cancellation/stale response handling, rate limits,
credential redaction, authoritative native character creation and successful
world admission without opening a browser. Current implementation does not yet
meet that acceptance. Linux must use the same protocol and state transitions.

Runtime recovery completed: renewed client development trust certificate,
advanced the local descriptor revision for changed certificate facts, and
replaced assignment through fenced ops. Native node reached READY and public
directory returned one world. No database reset. Account/character creation and
admitted-session visual verification remain outstanding.

## Clarification: avoid recurring browser redirects without weakening security

Owner clarified that repeated web redirection is the usability problem and
security must not be lowered. Preferred proposal is first-use external PKCE
authentication and a remembered-device session for later client launches.
This supersedes native password forms as the proposed default; it does not
claim that the owner accepted a first-use browser exception to the earlier
entirely in-window requirement. Native password entry remains an alternative
requiring a distinct reviewed contract, not a shortcut to remember-login.

PROVEN: current Platform first-release contract forbids refresh-token use and
persistence. Current client discards refresh_token and expires_in on exchange;
Platform consumes access and associated refresh credentials upon ticket issue.
Current default access/refresh lifetimes are 5/10 minutes. Keeping the existing
refresh token in a file or removing ticket-time revocation is not a valid fix.

Required remembered-device extension, proposed and not implemented:

- Explicit opt-in on trusted devices; shared-computer use retains interactive
  authentication. No passwords in the client or its persistent preferences.
- A narrowly scoped remembered-device credential held in the OS credential
  vault, bound to the Platform issuer and public-client identity. No plaintext
  fallback if the Linux/Windows vault is absent or locked.
- Server-authoritative absolute and inactivity expiry; refresh rotation with
  durable family lineage and replay detection, serialized concurrent use and
  defined crash recovery. Client clocks never extend server validity.
- Revalidate current account/security generation and revocation at refresh;
  no generation rebinding that makes a stale credential current again.
- Per-device and all-device revocation, logout forgetting local credentials,
  and a fresh interactive challenge when required by current security policy.
  A network outage must not grant admission or delete a valid stored session.
- Continue issuing short-lived, single-use gameplay tickets/grants. Gateway
  and Game never receive the remembered-device credential or OAuth refresh.
- Independent security review and negative tests for stale generation,
  revoked device, replay, concurrent refresh, expiry and unavailable vault.

UNKNOWN: current OAuth library refresh rotation semantics have not been
qualified here; no application-owned family reuse-detection implementation was
found by the focused read-only audit. A stored reusable credential introduces
additional theft exposure compared with memory-only bootstrap credentials.
The design must document and control that risk; do not promise identical risk,
indefinite sign-in, or claim that the OS vault protects a compromised account.
