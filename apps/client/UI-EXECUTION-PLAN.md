# Native client fidelity execution plan

Owner direction (2026-10-09): retain inspected Tibia controls, panel organization
and observed behaviors, with a modern Oteryn visual identity. The accepted visual concept uses opaque charcoal,
warm bronze/brass, ivory text, spacious rows and restrained serif headings. Legacy textures,
pixel-art chrome and rough procedural shortcut sketches are not the visual target. Draft #1942 remains
AUTHORING. This plan is the execution order, not a completion declaration.

## Current continuation — 2026-10-13

Owner task: `OTV2-20261009-client-game-ui`; branch
`feat/client-completion-20261009`; draft [PR1942](https://github.com/Oteryn/Oteryn-Game/pull/1942).
One root writer/publisher. This update plans work; it does not allocate server,
Platform or operator work or authorize deployments/account mutations.

Starting documentation head: `82efd3df7f65e12eea8d22df6283a42a76e8f8df`;
qualified runtime: `fabf441a1a45a747bad9f82c1a0b994b517d8e95`.
Linux client228/199/5 and exact Windows227/204/5+Session82, strict checks/release
and DX1220pages/78captures/0invalid pass. Full hashes, retained failures and private
paths are in [UI-FIDELITY.md](UI-FIDELITY.md). Page/component acceptance is not
all-control behavior, native gameplay, original1:1 or release qualification.
MAP/PACE and the five-defect supported-profile batch are retained, not redone.

Bounded live read: PR head matches82efd3df, still draft AUTHORING. CI has no
failed checks but is not complete:20successful/1skipped/7running and one
cancelled governance run; mandatory same-head gates must actually succeed.
Accepted main is `b4774770a6c7671cea2049a9d6210d1e6865773e`.
Server2041 remains open/unmerged at `e4ac000cabc760f693847d8d9b80a0ae17d87dd6`.
These are snapshots to refresh before the relevant mutation/qualification.

## Acceptance rule

For every screen record its private reference capture, visible controls, dropdown
members, conditional/enabled states, geometry, actual client consumer and engine
requirements. Unknown details remain explicitly unverified. A screen passes only
when its compiled native capture and interactions satisfy both visual and behavior
criteria. Serialization tests and panel/control counts do not establish parity.

Visual criteria: coherent Oteryn charcoal/bronze surfaces, legible typography, licensed
consistent icons, hierarchy, group/row order, proportions, frames,
icons, table columns, spacing, scrolling and default dialog reachability.
Behavior criteria: selection, dependencies, add/remove/reorder, conflict checks,
Apply/Cancel/reset, failed save, restart, modal focus and actual effect in game.
A prepared control without its consumer is not a completed function.

## Execution order and batch size

Next: B0 and B1. B2 is the first production-shaped gameplay milestone; it starts
only when its operator/QA prerequisites are supplied. While waiting, continue
client-only B1 and targeted reference/projection mapping for later batches.
After B2, prioritize accepted consumers that unblock real play; graphics, sound
and remaining modules follow their actual dependencies, not a cosmetic checklist.

One module sweep produces one findings list; fix up to about five findings or
500 hand-written changed lines per reviewable batch. Split larger modules at
real boundaries. Read-only workers can map source/decisions and audit the patch
in parallel; root integrates and publishes. Do not repeatedly compile/push each
small finding. One shared local check set and exact-source native qualification
per runtime batch; documentation-only updates need no duplicate Windows build.

| Batch | Work and entry condition | Observable exit condition |
| --- | --- | --- |
| B0 — CI/baseline | Finish mandatory checks; collect actual failures together. Reconcile accepted main normally before final freeze, inspecting impact/pins. | No missing, red or cancelled-only required check on the chosen head; all affected regressions pass. No weakened gates, forced updates or inherited successor CI. |
| B1 — supported settings transactions | Client-only routed-widget tests for delay, resource/Harmony display, target marking, action-row visibility, timestamps, cursors and graphics limits; controlled native interaction. | Draft survives navigation; Cancel discards; Apply changes the actual consumer and reload; failed save preserves active state/file. Focus, resize, release and disabled rows behave correctly. |
| B2 — login/map/pacing E2E | Merged/qualified2041, live operator-confirmed WorldBundle, dedicated seeded QA, temporary ON/OFF and approved harness. | Exact Windows build completes the full sequence below,3/3 passes within max5 total recorded attempts, including failures. Simulation/page capture is not this proof. |
| B3 — input/actions/inventory | Recover exact mouse/modifier/context precedence. Reuse admitted walk/USE/attack and existing inventory/container projections. Activate spell/object/bar assignment only with accepted authoritative models/commands. | Supported presets produce the right real command; modal/focus safety holds; all9x50 rows have reachable genuine assignments, execution, locks/clear and received count/cooldown behavior where supplied. No guessed diagonal/loot/spell semantics. |
| B4 — rendering/presentation | Real platform engine capabilities and accepted actor-visual, light/environment, effect and condition projections; independently authored/licensed assets. | Graphics settings alter actual rendered pixels; picking/HUD/capture agree. Layer separation, clouds, opacity, actor visuals, contextual cursors and all35 conditions react to received state. Compare named original evidence. |
| B5 — audio | Mature upstream device/decoder/mixer plus a legally usable playable bank. Local UI events can be client-owned; battle/world/social sound requires authoritative events. | Actual playback, five volumes, nested filters/master dependencies, retained children and device loss/recovery work. An enabled slider or invented combat sound does not qualify a family. |
| B6 — console/game window | Finish locally supported output/focus/copy behavior; consume accepted participant/private/secondary/filter and text/combat/effect data when available. | Real received channels/lines obey filters and selection; clipboard transfers controlled Unicode data; timestamps/clear/save remain non-fabricating and bounded; tab/reconnect/focus behavior is correct. |
| B7 — miscellaneous/gameplay/screenshots/help | Accepted purchase/container/inspection/chase/loot commands, automatic-event projection, verified help destinations and original ZIP/minimap schemas. | Confirmations protect real operations; each of17 automatic triggers produces the correct non-overwriting capture; private compatibility fixtures round-trip. Oteryn-only ZIP is not original interoperability. Remembered sign-in waits for Platform policy. |
| B8 — panels/NPC | First native gameplay proof, then integrate existing accepted producers before requesting new contracts; NPC read-only consumer is distinct from trade. Group docked panels, content/progression and activities separately. | All28 shortcut destinations and discovered nested dialogs display authoritative content and real permitted actions, with loading/empty/error/restricted/reconnect states. No fabricated commerce/rewards/statistics. |
| B9 — final qualification | Completed applicable behavior/visual evidence and dependencies; choose one final candidate. | AUTHORING→FREEZE_SHA→required independent reviews/CI→protected MQ. Windows main-channel artifact plus required live ON/OFF proof and CP/operator handoff; PR/PC build alone is not a release. |

### B1: the next implementation packet

Use existing `settings_semantic_contract.rs`, `preferences_browser.rs`,
`settings_ui.rs` and `settings.rs`; touch the real consumer only for a proved gap.
First identify missing accepted test cases, rather than refixing finished hotkeys.
Cover PL/EN, dependencies/retained values, Apply/Cancel/reload/failure and focus
across the supported page families. Batch discovered defects once.

Native picker/clipboard/slider/trackpad/cursor checks need private fixtures and
an agreed safe isolation of preferences/output. Do not inspect or overwrite the
owner's existing clipboard/profile as a shortcut. Use independently owned build
directories and preserve the dirty checkout/installation/starters. Document an
interaction as pending when only component tests or page markers are available.

### B2: required native sequence

Architect criteria: [6146789689](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6146789689).
PKCE→character selection→ticket/Gateway→admission18→18x14 received map→at least30
receipt-paced steps with zero unsolicited TOO_EARLY and measured intervals from
wait through wait+20ms→deliberately early TOO_EARLY→typed USE→local chat echo→wall
BLOCKED/resync/reset→reconnect same QA character without duplicate commands→OFF
legacy repeat. Keep3/3 same-build passes, max5 total visible attempts and every
failure. Private logs/captures only; public summaries contain no credentials.
Fresh admission/reset does not prove live resume or mid-session snapshot
replacement: current client lacks those paths. Resolve the owning acceptance/
contract before claiming their implementation; do not silently substitute reconnect.

### B8: reuse the accepted NPC producer

NPC read-only integration is missing client work, not a request for new protocol
IDs: accepted producera7a0b374 (#2035 after2030) is already an ancestor. Existing
`docs/contracts/protocol-oteryn/v1/npc_service_v1.proto` and ARCH-NPC-PACKETS/NPC-0
allocate capability3, talk command7 and whole-replacement domains7/8. Session
still lacks their stores/events/typed talk route; native GameState/UI lacks the
conversation/reply/read-only offers consumer. Re-export existing codecs, enforce
bounded revisions/identity/generation and test empty admission/close/range-loss,
correlation and malformed data. Do not assume each successful talk claims a delta.
Advertise3 only after complete routing, then qualify against the live bundle.
Command8 remains rejected in the current server transport; merged2040 durability
primitives alone do not authorize or implement BUY/SELL/purchase controls.

## Coverage of the nine settings lanes

| Lane | Main batch and remaining dependency |
| --- | --- |
| Graphics | B1/B4: physical interaction, engine capability and original image/gamma comparison |
| Effects | B4: real floor/light/environment and spell-effect compositing |
| Audio | B5: playback bank/backend and real event families |
| Controls/General Hotkeys | B1/B3: native input, exact presets/loot and unsupported action consumers |
| Action Bars/Custom Hotkeys | B3: authoritative learned spell/object selection, executable assignment, cooldown/count |
| Interface/HUD | B1/B4: contextual cursor, safe link warning, loot/expiry and condition data |
| Console/Game Window | B1/B6: OS clipboard, accepted filtering/participants/private/secondary and effect data |
| Misc/Gameplay | B3/B7: actual commands/confirmations; secure remembered sign-in needs Platform policy |
| Screenshots/Help | B1/B7: physical pickers,17 event triggers, destinations and original-compatible formats |

## Concrete handoff needs, not new allocations

Already visible to the coordinator in [6149042654](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6149042654):

1. Server owner/operator: qualify/merge2041; confirm the deployed node/WorldBundle;
   grant a testing/preprod `OTERYN_STEP_WAIT_RECEIPTS=1` window and restore-OFF/
   restart proof. Root does not flip switches or deploy.
2. CP/operator: dedicated QA account/character, seeded USE item, at least six
   walkable tiles and a wall; private credentials/access route, not a personal
   character or public secret.
3. CP: approve forced-early/resync harness hooks and exact native Tier2 execution
   packet. Requirements written in a comment are not supplied setup/authority.
4. Owner/architect: explicit legacy retirement after qualification (recommended)
   versus an accepted server-build gate; preserve old binaries until decided.
5. Domain owners: identify the existing accepted producer/contract and supplying
   revision for each missing projection/command above, plus legally usable sound/
   artwork and verified help/format evidence. No blanket invented client protocol.

The last canonical read supplies none of1–4. New replies are verified at their
exact links before consuming them. This plan does not edit STATE/Jira, allocate
other writers, authorize spending or promise unattended work/monitoring.

## Evidence and delivery discipline

Use [UI-REFERENCE-COVERAGE.md](UI-REFERENCE-COVERAGE.md) before a new probe;
preserve [UI-REFERENCE-OBSERVATIONS.md](UI-REFERENCE-OBSERVATIONS.md).
Consult the pinned [OTClient source map](UI-REFERENCE-OTCLIENT.md), then
[recovered original resources](UI-REFERENCE-EXTRACTED.md), then only a named
unresolved live/REA question. Reuse mature upstream code where licensed and
sufficient; upstream stubs and recovered original metadata are not runtime proof.
No proprietary code/art/audio/account files enter the public product.

For each control record reference/version, choices/default provenance,
dependencies, actual consumer/producer, tests and exact native evidence or named
blocker. Distinguish observed, implemented, component-qualified and E2E-qualified.
Keep UI-FIDELITY as the screen matrix; update it once per coherent batch.

Runtime batch exit: changed-path local tests/strict Clippy/fmt, relevant security/
pin/governance gates, exact-source Windows build and all20-page native audit plus
targeted real interactions. Preserve failed attempts and hash artifacts. A
docs-only successor can retain byte-identical runtime evidence, never final-head
CI/review qualification. Linux GUI parity requires its own execution evidence.

Save through the existing sole guarded publisher with a fresh expected remote
head and recovery bundle; stop on unexpected movement. Review all known findings
in one repair batch after returning to AUTHORING; never weaken gates. Final
merge/release/deploy/retirement follow the owning authority, not this plan.

## Live-reference correction after visual prototype

The modern five-switch overview is Quick settings only, not the full HUD screen.
The owner rejected replacing the reference's dense functional grouping with
oversized cards. Keep own/other HUD, arc controls, condition matrix and status
controls together; retain independent docked game panels, compact list/detail
views and their per-panel controls. The charcoal/bronze theme is presentation,
not permission to reduce functions or disperse related controls.
Current native styling is still a draft, not accepted visual fidelity.
