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
