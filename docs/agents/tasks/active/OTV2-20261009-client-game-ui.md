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

Status: AUTHORING on draft PR #1942; published anchor 821c0854855708de729a106db65c19083ff3469e. Not frozen or merged.
Owner scope: complete client and necessary engine/protocol functionality together.
No production deployment or database reset. Single writer: root.

Client UI consumes actual session inventory, containers, actors and chat. Map
rendering and pointer picking share one viewport transform; modal/text input
suppresses movement. Unknown vitals remain unknown. Owned scope includes native
client UI/settings/auth candidates and renderer presentation. Existing source was
backed up on Windows. Docker ProgramData fixed for child process only; no resets.

Remaining completion scope: complete production consumers/content, audio,
captures, Linux GUI composition and secure remembered-login integration.
Prepared UI is not a claim that absent engine projections/mechanics work.
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

The earlier native-password proposal covers sign-in, server-selected MFA,
registration, recovery, authoritative characters and channel admission in the
same window. That proposal is not accepted; the later remembered-device scope
below supersedes it as the preferred candidate without implying owner acceptance
of a first-use browser exception.

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
directory returned one world. No database reset. The later local completion checkpoint records account/character creation and
admitted-session visual verification.

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

Remembered-device requirements; disabled service/vault candidate exists, integration pending:

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

## Local completion checkpoint — 2026-10-09

Owner clarified: prepare the complete client now even where engine mechanics will
arrive later. Engine work is not required to populate invented client values.
Scope now also includes client {action_bar,device_store,minimap,panel_catalog,
settings_catalog,client_panels,preferences_browser}.rs; world builtin atlas
packing correction; Cargo dependencies; semantic reference JSON and licensed
Cinzel font/OFL notice. Original proprietary graphics/binaries are not included.

28 panel shortcuts, specialised tab/field layouts, minimap of actually loaded
terrain, 12 session-owned action slots, 27 settings sections and 276 typed options, plus six reference-derived native
dialogs, are prepared. Working settings apply to existing runtime fields; future settings
retain validated user intent with an explicit pending consumer. Unknown server
values remain unknown. Reference RCC filename tree verified:1063 resources;
executable handler names are string evidence, not recovered source/layouts.

Local normal registration/login created a DPAPI-protected test credential on the
owner PC; Game operator bootstrap committed character Oteryn Tester. Normal
browser PKCE/consent and native selection completed actual Game admission.
No game database reset/raw character injection or password rotation.

Independent local engineering sweep found and repaired held-input suppression,
modified movement/action conflicts, action feedback hidden without chat, and
conflicting minimap controls. Linux duplicate vault deletion now cleans the exact
namespace while load/save fail closed on ambiguity. Remembered login remains
OFF and unwired: first-device/current login still uses browser PKCE; OS-vault
primitives alone do not implement rotation/crash/concurrency recovery.

Platform HTTP/core/MFA/cache/catalogue qualification:75 tests/737 assertions
and full PHPStan level10 PASS; platform-gate/runtime tests on published787028f8
also PASS. Deployment-DB races, live OS vault, concrete client lock/journal/HTTPS
bindings and contract/security approval remain outstanding. HTTP candidate OFF.
This engineering sweep is not formal frozen-head review.

Current Linux library tests112 PASS, including8 pure remembered-device adversarial
groups; candidate module is exposed but unwired. Native Windows111 library+4 application+5 launcher tests, strict Clippy and
MSVC release PASS. Archive timestamps were refreshed before rerun to prevent stale
Cargo artifacts; full native preferences footer screenshot verified. Normal browser PKCE and character
selection entered the live local world; screenshot verified actual loaded minimap.

Current UI successor: compact per-panel statistics/list layouts, real resource
values when supplied, minimap above vector shortcuts, equipment paper doll,
localised command feedback. Full preferences open directly under F10, with search
and their own Apply/Cancel/Defaults. Dedicated compositions now cover22 feature panels and20 Cyclopedia pages.
Missing server data stays unknown; this still does not prove full visual parity.

Drafts: Game1942 and Platform1484 are saved authoring candidates. No frozen-head
review, merge, production activation or Linux GUI parity is claimed. Font license
exception/notices qualified; coherent Rust1.95 protected workflow/pin migration is
proposal-only pending owning governance route. Jira mapping remains pending.

World pin-check PASS: two identical builds, unchanged payload f8b11ebc…; compiler-input pin refreshed for actual Cargo/toolchain inputs88be0150….

Observed UI repairs: saved shortcut order/editor, Drop Tracker→Cyclopedia Items
local navigation, separate offence/defence fields, title filters/search and Item
Summary Store Inbox/search/List/Grid. Real inventory data used; missing values
unknown. Settings navigation fills resized height; toolbar language propagates
to game. Full parity still lacks expanded action rows, Wheel/perk composition,
achievement controls and domain consumers. Rust1.94 CI rejects required1.95;
protected migration proposal exists, active gates unchanged.
