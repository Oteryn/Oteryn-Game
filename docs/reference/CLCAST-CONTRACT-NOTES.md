# CLCAST contract notes

## CLCAST S17a — native caster control

Existing command 502 and capability 290 are reused; no migration, command,
capability, mechanic ordinal or registry number is allocated (lane +30;
reserved migrations 1130–1139 remain unused).
Canary `src/creatures/players/livestream/livestream.cpp:237,249,260,281`
provides explicit start/settings/stop and access rotation evidence. The native
Cast settings form (Ctrl+Alt+Shift+S) sends bounded delay/cap/password-required
and saved access revision through the existing fenced command dispatcher.
Platform retains account preference/password storage; Game still verifies the
saved settings before every start/update. The client never sends a password.
Confirmed revisions drive stop/update and disconnect clears the native panel.

## CLCAST S17b — account-only Watch client

Allocation against main 9668e0a04: the next free PublicMedia kind is 4;
CLCAST lane offset +30 allocates nested media kind **34** for Camera{floor}.
It travels inside the existing family 16 / Message kind 21 and carries only
-15..0, with no world position or identity. Existing capability 289 is reused;
no migration, command ID, capability ID or mechanic ordinal is allocated.
Canary `src/creatures/players/livestream/livestream.cpp:486,541` and
`src/server/network/protocol/protocolgame.cpp:8477,12379`, plus Crystal
`src/server/network/protocol/protocolgame.cpp:7626,7846,8043`, ground the
map/creature/effect/public-speech view and observer-only input allowlist.
The explicit display floor completes the public map projection for native
layer placement. All camera/map chunks follow the existing consent and delay
queue; the client commits a map only at its final chunk and clears on slate,
terminal transport failure or epoch reset.

The native `oteryn-client --watch` entry consumes a one-use account observer
ticket through OTERYN_WATCH_ADDRESS, OTERYN_WATCH_TLS_NAME,
OTERYN_WATCH_ROOT_CERT and OTERYN_WATCH_TICKET. TLS/ALPN uses the existing
adapter. It never invokes character selection/admission or initializes private
panels. Platform directory, account authentication, ticket issuance and stored
preferences remain the accepted handoff; no Platform files are changed.

CLCAST S17a character setting completion: Message kind **52** is
ConsentQuery on existing command 503 / capability 292: next free outer Message
kind 22 on main 9668e0a04 + lane offset 30. It has no fields and is forbidden in
observer envelopes. The existing session/character-fenced preference owner reads
and writes consent. Native Read/Allow shown/Hide me controls show confirmed state
and use its revision; unknown consent cannot be optimistically enabled. This is
the accepted S17 consent-default-off difference from Canary
`src/creatures/players/livestream/livestream.cpp:486`, recorded in
`docs/architecture/design/S17-CAST-DESIGN.md` section 6. No Platform write or
migration is added. These controls let bystanders opt in independently; the caster's explicit Start already supplies the caster's own consent in the existing projection rule.

## CLCAST S17c — native live room and moderation

Reuse capabilities 290/291, commands 504/505 and state domain 103; allocate no
new numbers or migrations. Canary
`src/creatures/players/livestream/livestream.cpp:388,396,431,441,674,692,724,812`
provides opaque viewer names/presence, isolated live room delivery and
mute/kick/ban evidence. Accepted S17 differences retain account-scoped moderation
and durable owner-account bans in Game; the client sends only the opaque handle
received in that caster's room, a confirmed cast revision and fresh operation
key. It never sends a viewer account ID or routes cast text to gameplay chat.
Domain 103 now has a strict native session decoder and revisioned event. The
playing mailbox presents it immediately, including D=300, while viewer copies
continue through the existing delay queue. Ctrl+Alt+Shift+H opens the native
reply/mute/unmute/kick/ban form. Disconnect clears room history and pending UI.

CLCAST projection allocations extend the reserved outer Message sequence after
52: **53 Ready{epoch,chat,delay}**, **54 CasterRoster{revision,subjects}**, and
**55 ControlQuery**. Domain 103's next free delta type is 2; +30 allocates
**32 CAST_CASTER_ROSTER_V1**, bounded to 1024 bytes and 50 sorted, unique opaque
handles. No command/capability/migration is added. Ready is an admission/clear
policy message, containing selected chat permission and chosen delay but no
picture, speech, viewer count or absolute identity. It is emitted under the
existing egress barrier on epoch/consent clears; content still follows D.
ControlQuery uses command 502 and lets native settings refresh a stopped or
externally revoked cast without guessing its revision.

Canary `src/creatures/players/livestream/livestream.cpp:388,396,431,541,724`
grounds viewer presence, removal, opcode selection and the caster room. The
accepted S17 typed-projection difference uses a structured owner-only roster
rather than inferring membership from spoofable user text. It refreshes on
change and at the existing two-second renewal cadence for resync, keeping
silent viewers selectable after chat history rolls over. Roster frames cannot
enter an observer envelope. Moderator targets come only from that roster.

## CLCAST composed native qualification

Canary `src/creatures/players/livestream/livestream.cpp:249,486,541` grounds
current-caster settings, recurring public picture capture and observer-only
inputs. A successfully fenced Capture refreshes the existing five-second
caster heartbeat, just as Refresh already does. The one-second capture permit,
privacy lease, revocation barrier and observer reservation deadlines are
unchanged. Shared-holder gating reads use the existing bounded NODEBOOT-1
command retry; a cast mutation is never retried by that adapter.

Observed real-dispatch PostgreSQL cases (each creates and uses a real database):

- `s17a_client_start_stop_dispatch_commits_audit_and_rejects_stale_revision`:
  5.612 s in the clean serial run; committed start/stop audits and stale stop refusal.
- `s17b_native_observer_map_creatures_effects_and_public_speech_dispatch`:
  50.007 s; native picture receives the actual door appearance and creatures,
  public speech from accepted gameplay Say and emitted effects. The viewer
  creates no character root and cannot move the caster.
- `s17c_native_live_chat_mute_kick_and_persistent_ban_dispatch`: 48.727 s
  in the clean serial run; D=300 caster text remains live while viewer content stays
  delayed, and reply/mute/unmute/kick/ban use the real command dispatcher.
  A fresh one-use account ticket cannot bypass the committed account ban.

The non-PostgreSQL six-package test run passed 1,736 cases. A subsequent focused
cast run passed 55 cases, including the native settings, consent, room, viewer
rendering and strict wire consumers. PostgreSQL admission/content qualification
is CPU-intensive, so final database cases run serially after content tools.
Platform preference/password storage, account directory and ticket issuance
remain the existing handoff; no Platform or migration files changed.

All three cases passed together from clean code head `8a71047cf40ed969f37bad44b209426d6ed0571a`: 3 passed, 0 failed, 105.45 s total. No content compiler or other validation job ran during this PostgreSQL proof.

Final Linux gates on the same source: `cargo fmt --all --check` 18.845 s;
seven-package `cargo check --all-features` 147.433 s; seven-package
`cargo clippy --all-targets --all-features -- -D warnings` 337.453 s;
10 focused cast transport unit tests passed. Pin refresh (723.600 s), pin check
(612.714 s) and workspace architecture check (506.159 s) passed; generated pins
needed no content changes. These wall times include compilation/queue time.
Native draw/use tests run on the Linux host; Windows HWND execution was not run.
The Watch entry retains the existing CLIENT-INSTALLER-0 lifetime guard before
loading assets, independently of character bootstrap.

Independent read-only review of `20f8161d6275f6e6736d2e23552e5344f188e4d5`
found no P0/P1 issue and one P2 viewport origin defect. Watch now derives its
origin and dimensions from the protocol window constants, preserving the full
eastern column and southern row. The corner regression failed on the previous
origin and passes with all four corners, an edge creature and an edge effect;
all nine final native cast tests pass (5.442 s wall). No wire or server rule changed.

Post-review viewport repair gates passed: final fmt check 21.135 s, client
`cargo check --all-features` 0.375 s, client all-target/all-feature clippy with
`-D warnings` 25.489 s, and workspace architecture check 3.754 s.
After workspace restoration, code head
`4b5838f4aa68350deb7303eb1099df4f5921ec71` was verified unchanged locally and
on the assigned remote branch. All nine focused native cast tests passed again
(0.609 s wall), and workspace architecture validation passed again (4.184 s).
This verification adds no implementation, allocation or migration changes.


## S17 integration with main 1ae39c7fb

Merged QENGINE2/QBASE through a merge commit. Canonical main migrations
0999–1008 remain; CLCAST adds no migrations (1130–1139 remain reserved).
Main's SRVWLD notes are unchanged. Its incoming root notes are preserved
byte-for-byte at `docs/reference/QENGINE2-CONTRACT-NOTES.md`; no root notes remain.

The serial real-PG run exposed an idle caster fixture while the viewer was
awaiting media: S17b rejected Stop and S17c reported an unusable viewer.
The fixture now services caster probes and roster deltas concurrently with
viewer waits, using the existing cancellation-safe FrameReader. All assertions
and deadlines remain intact. Donor: Canary `livestream.cpp:486,541` (independent
playing and viewer connections). The rerun passed all three real-dispatch cases:
S17a 7.035 s, S17b 51.344 s, S17c 65.904 s (125.69 s total test execution).

Native regeneration passed (226.738 s); World content/imports have no lane
differences from main. Release pin-refresh passed (95.989 s), pin-check passed
(67.461 s), and workspace architecture passed (4.247 s). The focused cast run
passed 56 cases, all six wire contract cases passed, and the two touched
session capability/route closure cases passed. Eight changed-crate clippy
checks with all targets/features and `-D warnings` passed (852.971 s wall,
including build-lock wait). Final fmt passed (25.238 s).

Final `cargo check --workspace --all-targets` passed (941.958 s wall, including
build-lock wait). Duplicate-migration check passed against the staged merge tree.
