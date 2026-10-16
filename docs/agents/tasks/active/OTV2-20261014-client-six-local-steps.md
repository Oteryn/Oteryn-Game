# OTV2-20261014 — six local-client steps, without E2E

```yaml
task_id: OTV2-20261014-client-six-local-steps
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Owner requires all six planned local steps, not another premature stop at CI.
Parent client-game-ui; sole root writer, AUTHORING. Owned runtime: game_ui.rs,
preferences_hotkey_pages.rs, settings_ui.rs. No subagents, paid review trigger,
new dependency family, protocol/capability/identity/authority/persistence-format changes,
production/account/trust access, E2E or HUD/graphics redesign. The older excluded
point6 was HUD/graphics; this packet's step6 is reference/audio investigation.
Cargo patch/derived World input repair is also owned by this packet.

## 1 — finite source inventory and consumer gaps

apps/client/UI-LOCAL-OPTION-INDEX.md contains393 unique catalogue keys extracted
from unchanged settings_catalog.rs,27sections:20navigation families and7extra
compatibility/scaffold families. Balanced macro extraction retains keys and
declared consumers, no private reference values. This is NOT393 original visible
controls or393 completed functions. Native special hotkey/help pages override
generic Pending declarations; names-only rows and prepared panels are not live
functions. Native20-page semantic contract and UI-FIDELITY remain the consumer
evidence; the index does not replace them or turn serialization into completion.

Actionable gaps after removing already-qualified cases: unfocused binding
capture, ongoing IME/history ownership, routed profile-removal confirmation and
PL/EN staged-import failure/retry. Missing producer/rights/physical evidence stays
separate. Source observation starts from recovered15.33 QML and pinned upstream
53c3878a, not a repeated complete live reference sweep.

## 2 — profiles, bindings and focus

PROVEN: unfocused Escape was captured as Stop All, or dismissed another binding
editor; unfocused ordinary/F10 keys were also eligible. New12-case PL/EN/target/
key regression fails before repair. The editor now checks native focus before
reading queued keys. Recovery includes the real WindowFocused(false) event so
upstream key-down/repeat bookkeeping is not misrepresented. Focused capture
continues to use physical Q for logical A/Ctrl, and never Applies the draft.

Six routed PL/EN Remove/Cancel/Escape cases use the production Options window,
actual header/modal clicks and a profile with a different action-row layout.
Opening confirmation does not mutate; Cancel/Escape retain the entire draft;
confirmed removal selects the surviving profile and its actual bar preferences.
Inner dismissal never closes outer Options. Initial fixture compile errors were
fixed without changing production types/lints. Original removal was never invoked; exact
confirmation parity remains UNKNOWN, not closed by Oteryn tests.

## 3 — existing action bars, no invented assignment path

Twelve action_bar_ui regressions pass:9x50 row identity/geometry, global edge
locks, visibility, primary-only drag/drop, secondary menu, real assignment
availability and current item labels. Core tests additionally preserve locks,
explicit clears and hidden-row shortcuts. These assignments are component
fixtures, not proof of native gameplay or a currently reachable text picker.

PROVEN source gap: GameUi builds empty admission-local assignments; current
native assign_menu exposes Spell only with the authoritative supplied catalogue
and keeps Object/Multi-Action/Text/Passive/Hotkey disabled. AvailableSpell input
is empty in the native caller. Text is executable in Custom Hotkeys, NOT an
already-working native action-bar Text assignment. Do not bypass B2/B3 by
activating a new picker or count fixture assignments as the missing function.

## 4 — Unicode composer and upstream IME ownership

PROVEN: production GameUi/mailbox fixture initially replaces an active Japanese
preedit with history. The smallest Oteryn adapter observes public egui0.35
Preedit/Commit events: nonempty preedit owns editing; empty preedit/commit ends
it. Composition chords stay in ordinary upstream TextEdit input, not recall
segments. Ownership resets on lost/hidden/disabled composer focus. No vendor
fork, native IME replacement, fabricated received line or persistent IME state.

Eight PL/EN/same-frame/cross-frame/commit-order cases check no history replacement,
actual preedit bytes, committed Japanese/emoji presence, recall after commit
including commit→recall in one frame, scratch restoration and no extra enqueue.
Other Unicode selection/Copy/Cut/Paste/recall tests remain. This is component
ownership proof, NOT full original or physical OS-language/IME parity.

## 5 — existing local I/O and native isolation boundary

Ordinary child-process LOCALAPPDATA fixture now routes actual Help/Apply in PL
and EN. A directory at the exact PID-specific staging filename makes the real
save fail. Active bytes/current remain unchanged, staged import remains retryable,
error is visible and no Apply flag is emitted. After owned fixture cleanup,
actual Apply/save/reload succeeds and the source ZIP remains byte-identical.
Existing corrupt/ambiguous/encrypted/traversal/size/invalid/archive cases,
Cancel/export refusal, PNG/crop/collision and Compendium dispatch remain required.
No new serialization, original ZIP interoperability or automatic trigger claimed.

Physical picker/clipboard/trackpad/IME acceptance remains blocked by safe OS
isolation. Changing only LOCALAPPDATA in a child process does NOT isolate the
interactive Windows clipboard or all COM shell/MRU state. Current rfd0.17.2
source uses native COM dialogs and supplies no DONTADDTORECENT suppression in
our path. No owner clipboard is read/overwritten, original account/preferences
exported or physical dialog operated. An isolated Windows account/session or
explicit accepted operator fixture packet is needed before those interactions.

## 6 — source/REA and real audio dependencies

Existing original15.33.b21348 evidence, QML hashes,844-record sound catalogue,
pitch/volume ranges, sampled Vorbis48kHz/stereo and OTClient sounds.proto are
reused. Their resource metadata is not a legal Oteryn playback bank or recovered
native archive schema. Exact C++ defaults/soundEnabled/ZIP/minimap layouts remain
UNKNOWN; global original configuration contains identity data and is not copied.

Initial read-only Synology connector probes return429 and404/UNAVAILABLE.
An independently authorized, per-call-gated host-only Desktop Commander route
subsequently reaches the running otclient-track-a-kasmvnc container. Read-only
REA README/project metadata is available: Qt indexes cover Authentication,
GameClient and login/character controllers, not recovered Options/audio
controllers. Full scans previously timed out; none is rerun or paid for.
No process/account/configuration is restarted, copied or changed. The protected
Tibia.new directory returns Permission denied and is not traversed via escalation.

Version mismatch is now established. README's15.33.b21348 binary SHA256 is
065f961b29904f3231b5c82be2f7748d88c7b74f3d324e12b5d5dd50b14afead;
the current installed binary is
accfc073244a48dbe3a9848c26b1c62458b934c5395ef3917ac7613837fdd7f8,
with public package descriptor15.33.9d148f (not a proven runtime build number).
Four independently accessible cached software copies share SHA256
ed5469b9fa71349de688f719434d23875f76f28a3ebd08a36d30f7f6da0af6b8;
none matches the README. Old REA addresses must not be applied to either changed
binary. No targeted resource-extraction helper is found in the bounded project/
temporary helper search. Version-matched resources and credential-free original
archive fixtures remain missing; connector restoration is no longer the blocker.

Public upstream source investigation: crates.io API returns403; independent
authorized GitHub reads verify RustAudio/Rodio tagv0.23.0 at
c93a661fd667a755a251caf3c4060173d86b3e9b,MIT OR Apache-2.0,MSRV1.87,
CPAL dependency0.18.2 and Symphonia0.5.5. It fits the repository's1.94 toolchain
floor as a candidate. Prefer playback/Vorbis-only features, no default recording.
Do not pair it blindly with inspected CPAL main0.19.0 (5df8ae1803bd93e8638c16501ec32ad371b55c7c).
Rodio main685e05765c2aff8fbb51e32cda5b18428d09ffbe declares0.22.2/MSRV1.95;
moving main is not the newest compatible tagged release. No dependency added.

No playable OGG/WAV/MP3 bank or Rodio/CPAL consumer is found in current client/
Cargo closure. LICENSE-ASSETS and repository licensing separate reference use
from redistribution rights. Backend recommendation is now source-backed, but
actual playback,device recovery,five groups and battle/social events still need
a legally usable bank and real accepted producers. No fake beep/volume feature.

## Exit and remaining external inputs

One coherent input/transaction batch retains f8's exact Windows proof (PR body),
adds the inventory and runs scoped full tests/strict/fmt/governance, then one
exact-source MSVC/DX12 qualification. Keep failed regressions/fixture attempts.
Linux client230/214/5,strict all-target Clippy,fmt and governance60 pass. Published
71bbdf89d1ab5c913c95adced7cd35827a2b0cbd also passes exact MSVC1.95
client229/221/5+Session82,strict/fmt/release and DX12 at125%DPI:
20pages/78 independently rehashed captures/0invalid. Release
0.1.0+dev.ci.g71bbdf89d1ab, executable SHA256
67cc2539a4b5a747ccdb0c7df073a449398ab9ea24dec89e102cd8e9f3e7948c,
manifest SHA256
8d1fdc253feb2ac674c620be0ddb342348684ed8cd197f12d0f743c2232258ed.
Owned PC build is Downloads/oteryn-client-71bbdf89-20261014-native;
audit is Downloads/oteryn-settings-audit-71bbdf89-20261014. Helper31924 exits0
after134.73s. All new regressions execute; no autologin or owner-profile changes.
This is not physical OS interaction or whole-client parity proof.
About500–550handwritten lines is a
review warning, not a cap;393 generated inventory rows are separately identified.
No status-only publication or repeated unchanged heavy check. Source/native
proof is not whole-client1:1, final review/freeze/MQ or an E2E waiver.

## Supply-chain repair after real CI failure

71bb's actual Merge gate run38572338828 fails Rust supply chain job115881144764,
then validate/game-gate. GitHub logs and local cargo-deny independently identify
RUSTSEC-2026-0346/0347/0348/0349 in locked rustls0.23.45; all require>=0.23.46.
Protected main15b8f8bf still has0.23.45. Minimal upstream patch updates the exact
workspace requirement and Cargo-generated lock checksum to0.23.46, with no
ignored advisory, trust/wire change or dependency fork. cargo-deny then reports
advisories/bans/licenses/sources OK. Changed Cargo.lock requires deriving the
World input digest from the staged tree; bundle digest, entry and revision stay
unchanged. Old native qualification is not inherited by the successor.

First successor Linux compile fails from disk exhaustion, not an assertion;
cargo clean -p oteryn-client --profile dev removes only reproducible client
build caches (9.9GiB), preserving source, recovery bundles and PC evidence.
Retry disables incremental compilation. Linux client230/214/5,Platform16,
Session TCP2,strict all-target Clippy of client/Platform/TCP/server,fmt,
workspace architecture and governance60 pass. World double-build retains
25,089,907bytes and1b04cc35 bundle digest; staged input hash is e8d897f6.
Server TLS unit selection:8pass/1already-ignored; no E2E performed.
Required independent dependency/security
review goes to the active control plane; root does not emit an owner-funded trigger.

All safe local investigation/repair steps are executed; physical OS and version-
matched reference cells remain explicitly incomplete. Required external inputs
are isolated Windows fixture authority and credential-free reference samples.
Audio bank/event inputs and post-B2 bar activation remain separate later gates.
