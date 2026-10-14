# OTV2-20261014 — local chat/history and supported Options focus

```yaml
task_id: OTV2-20261014-client-local-input
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

Owner explicitly requested the local settings/chat/Windows packet while B2
is blocked. Parent OTV2-20261009-client-game-ui, sole root writer, AUTHORING.
Owned runtime: game_ui.rs, play.rs, preferences_browser.rs, settings_ui.rs.
No subagents, paid providers, dependencies, HUD/graphics redesign, protocol,
capability, server, account, personal clipboard/profile or trust changes.
Local14d2b5ca preserves exact6f Windows proof; this runtime batch publishes it
together rather than issuing a checkpoint-only push.

## Source-backed behavior and batch

CIPSOFT_OFFICIAL manual §controls_communication/4.2.1, owner capture2026-09-28:
docs/reference/tibia-manual/controls_communication.md, clean-text SHA256
27283933d754ce23886ecf30aa445d2b5d01e28d5c307c89c8ae27b651c503b9,
establishes Shift+Up/Down sent-line recall. Original capacity, duplicate policy
and persistence remain unknown. The64-line session-local bound is Oteryn's
memory safeguard, not a recovered factory value. Nothing persists across logout.

Record trimmed text only after the existing bounded PlayLink mailbox accepts it;
this includes shared composer/custom-text submission, not an invented server
echo or successful server disposition. Recall changes the focused enabled
composer only, never sends, switches tab/recipient or fabricates received chat.
Returning past newest restores the exact unsent scratch. Real editing or manual
custom-text placement ends browsing; recalled Unicode caret collapses at end.

Unmodified egui0.35 handles ordinary editing/selection/clipboard/IME input.
A bounded Oteryn adapter processes input segments around exact recall chords
in their original order, using the same editor/rectangle and invisible
intermediate paints before its final visible text. No fork, substitute text
engine or OS clipboard implementation. Existing native configured hotkeys retain
precedence; claimed chords do not reach egui/history. Full IME/reference parity
and private-peer tabs are not claimed from this packet.

Two proven existing-path defects are repaired in the same input packet:

- Options consumed queued Escape even when the native window lost focus,
  discarding its draft. The routed PL/EN test fails before the focus guard;
  after repair, controls/console/screenshots drafts survive unfocused Escape
  and tiny-window resize, page Reset affects only supported page values,
  unavailable sound intent survives and Cancel restores unchanged current.
- PlayLink.send_chat accepted intent after its command receiver closed. A
  closed-receiver test fails before repair. It now latches the existing terminal
  class and refuses without reserving chat. Busy/closed refusal retains composer
  draft/history for retry; no dropped message is recorded as accepted.

## Consumer regressions and qualification boundaries

Actual GameUi.show fixtures use real play_channel/run_session/mailbox routing,
scripted Stepper with no successful echo, received empty ChatLog and actual
Send clicks in PL/EN. Tests cover oldest/newest boundaries, Unicode painted
text/caret, preserved scratch, mixed Text-before-Up versus Up-before-Text,
edited recalls, hidden/read-only/modal/window focus, private-recipient/output
focus, Chat Off, plain/extra-modifier arrows,64/65 eviction and invalid/refused
submissions. A bypass of the actual recall consumer fails its Unicode regression;
restored consumer passes. The fixture's first CharIndex compile error, incorrect
setting-key fixture and forbidden panic callbacks are corrected without weakening
tests/lints. This is component/routing proof, not server Sent or real-network E2E.

Existing routed settings transactions/persistence failure and Windows-only toolbar
tests remain required. Rust1.95 locked all-target Linux client230/209/5+Session82,
strict all-target client/Session Clippy, fmt, governance60 and diff pass.
Exact-source MSVC release/audit follows publication before native qualification.
Native20-page captures establish
reachability/viewport/integrity, not every control's original physical behavior.
Do not inspect/overwrite the owner's clipboard/profile or enable unsupported
families to pretend those acceptance gaps are closed.

The batch is about600 handwritten lines, mostly consumer fixtures. That is the
repository's review warning, not a cap. Keep the one owner/input consumer/test
packet together rather than split per file or rerun CI for individual findings.

## Reference and external gates retained

Help QML ZIP filters/native delegation do not establish entry names, archive or
minimap schema. The previous export failed without producing a fixture and the
original process disappeared; cause unknown. Do not restart/sign in, copy global
configuration/identity data or invent interoperability from192 setting keys.
Existing own Oteryn ZIP remains explicitly own-format. No new REA provider or
original-client probe is needed for this documented local input contract.

Parent6f217f16 has green game-gate/CI at continuation; a new head needs its own
checks. Dedicated QA/character/item, deployed bundle node, temporary ON/OFF/
rollback/harness and retirement decision remain CP/operator gates. This packet
neither supplies nor bypasses B2 or its3/3/max5 live attempts. NPC/new object or
spell assignments/audio/event-trigger activation remain behind their actual
dependencies. Existing binaries/qualified artifacts and all failures are retained.

## Published99a8 native proof and real CI source repair

99a8a3851dda92966bcee9e306a338a8085053b3 guarded publication/readback passes;
recovery /workspace/oteryn-client-publication-99a8a385.bundle SHA256
18e4f330c0f3bf6b8d25a3188fd94cd2bbcf866b813f867f7c3b50fba6a9d680.
Exact Rust1.95 MSVC client229/215/5+Session82, strict all-target Clippy, fmt and
release pass. Five history tests, focus-loss Reset/Cancel, closed-chat receiver,
prior item-menu and Windows-only toolbar tests execute/pass. DX12 at125%DPI:
20pages/78 rehashed captures/0invalid. Release0.1.0+dev.ci.g99a8a3851dda;
executable8e48deb23e3d3e3185cd2d0635f96f445d4dda3f8f909ff3d9cdec784e33931e;
manifest26c62c859cbccf17033870372bf9464424ae08484b8c4f18781e1d5e4bdc8b75.
Build C:\Users\barte\Downloads\oteryn-client-99a8a385-20261014-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-99a8a385-20261014.
QualificationPID40872 exits0 after134.82seconds. Separate launcher finishes0;
source-bound loginPID31460/window4522326 responds with exact path/hash/U+2014
title, no automatic sign-in or old-process termination.

CI99a8 has only one protected-base success, not ordinary queued CI: live PR
is mergeable=false/dirty. REST's base field still reports71c014d4; fresh Git
fetch proves accepted mainaf85035f5864eb4a437dfe4b6a78584c3ea81182.
Actual merge-tree identifies one conflict, content/world/pins/oteryn.json.
Normal merge imports accepted New Haven/data/decision work, not root-authored
server implementation or live deployment. Main's placement fingerprint changes
world.rs; no HUD redesign or reset/rebase/force of the client branch.

Preserve main's World digest1b04cc35a65bcced64dbc933fcb5ac9adad426a369149bb8c170b1aa80d04406,
entry/identity/revision/nonproduction flag. Release pin-check rejects copied
main inputs9a2b5616 and derives the merged-index inputs
883be88aebae8c7d74bf58123f3b8d28ed2b95d21d9eea8f88839180032cd455.
After the one-field repair, actual double-build validates identical25,089,907
bytes matching main's digest. No invented fingerprint or weakened gate.
Post-merge Linux230/209/5+Session82, strict/fmt and governance60 pass; new
native/CI qualification follows.99a8 proof remains bound to its exact source,
not inherited by the changed placement fingerprint.

## Exact post-merge native checkpoint

Publishedc53099c9170bbc3c5e4630a795b90e8ee575b224 normally composes99a8 and
accepted mainaf85035f. Source-only pin repair above restores real mergeability:
fresh PR MERGEABLE, ordinary CI10success/7running/0failed, game-gate pending.
No queue/release/final-review/all-green claim. This evidence-only successor is
saved locally until useful running CI finishes; do not cancel it for a checkpoint.

Exact c530 Rust1.95 MSVC229/215/5+Session82, strict Clippy/fmt/release pass;
all named history/focus/closed-receiver/earlier menu/toolbar tests execute/pass.
DX12 at125%DPI validates20pages/78 rehashed captures/0invalid. Release
0.1.0+dev.ci.gc53099c9170b; executable SHA256
4fd3e1b44171bacd68d9ddc46da46719a9c61c63dfbce4918e8bbe67b6a0d056;
manifestedc8624c1f84cab9cda342df40d313d2d37dc9d99fe609e96d3da6f988a57c5c.
Build C:\Users\barte\Downloads\oteryn-client-c53099c9-20261014-native;
audit C:\Users\barte\Downloads\oteryn-settings-audit-c53099c9-20261014.
QualificationPID19372 exits0 after127.86seconds. Separate launcher exits0;
independent fence verifies loginPID26640/window4393676 responding with exact
path/hash/U+2014 title and no automatic sign-in. Older sources/checkouts/binaries/
windows are preserved, not terminated/relabelled or installed over the owner.
Guarded c530 publication/readback passes; recovery
/workspace/oteryn-client-publication-c53099c9.bundle SHA256
58390826a469374e56547880d271f36ff14548a54a7f99882fdbfffcea6a2ab1.

Local/native component tests and compiled page capture are complete for this
packet, not every physical OS interaction or full original1:1. No personal
clipboard/profile/picker operation, new server activation, deployment or live
QA action is performed. B2/main-channel/final protected integration remain open.

## Local input follow-up — retained focus ownership

Owner requested continuing available work without B2 setup. Fresh PR readback
still has remote c53099c9, draft AUTHORING/MERGEABLE,32 successful checks and
one genuinely skipped check. The local cfa72fca Windows evidence is preserved
and included with this actual repair, not published as a status-only checkpoint.

PROVEN: a new production GameUi.show/mailbox regression fails before repair:
Enter followed by Shift+Up in one input frame sends the recalled historical
line rather than the draft Enter committed. The ordered adapter checked focus
only at frame entry. Up then Enter must still send the intentionally recalled
line. Recheck upstream editor focus after each ordinary input segment before
recall, preserving the existing egui editor and command path. Four PL/EN/order
cases verify exact admitted ChatIntent text,one enqueue and an emptied draft;
no successful server echo is synthesized.

Twelve PL/EN/order clipboard-event cases exercise actual Unicode selection,
Copy/Cut/Paste before and after recall, single-line CR/LF normalization, final
paint, scratch recovery and no extra mailbox submission/received line. Copy
emits exactly the selected text once; recall collapses the cursor, so a later
Copy/Cut does not use an obsolete selection. Paste after recall edits that line
and ends browsing. Seven history regressions pass, including prior boundaries,
refusal and visibility gates. These inspect egui platform commands, not the
owner's OS clipboard; physical OS transfer,IME,pickers and trackpad parity
remain UNKNOWN. Existing picker dispatch/import/export/error transactions are
retained, not relabelled as physical native acceptance.

Scoped Linux230/211/5,strict all-target client Clippy,fmt and governance60 pass.
Exact-source MSVC qualification follows publication. No dependencies,protocol,capabilities,preferences,
server/operator changes,paid review or HUD/graphics redesign. Dedicated B2
QA/node/ON-OFF/harness/retirement prerequisites remain unsupplied.
