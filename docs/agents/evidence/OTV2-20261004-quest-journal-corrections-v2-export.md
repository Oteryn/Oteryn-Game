# Five chosen journal corrections and v2 Quest export

Local AUTHORING, HEAD `88d63a18b44123ce342008cf52bcdb427c909ec2`.
Six subagents prepared bounded candidates and independent review; Root alone
writes product files. No PR, commit, push, merge or server activation.

## Completed data changes

Five explicit `CHOSEN_OTERYN_APPROXIMATION` corrections cover three definitions:

- Spike: Upper, Middle and Lower each require four distinct successful task
  hand-ins. Repeated hand-ins do not advance the chosen region. Each task maps
  by its semantic name to the existing recipe stage, including Middle nests
  s7, mushrooms s8, charges s9 and kills s10. The original journal counters
  44302/44303/44308 and shared fame 44313 are not aliases for new Oteryn state.
  The existing chosen level-80 campaign and original Source level brackets
  remain explicitly different. The original 100-fame/outfit reward is not
  added to the selected finite campaign.
- Tibia Tales / Fox: both donor Budrik scripts write child Questline states
  1 and 2; Crystal's journal already selects the child, while Canary selects
  its parent table. Three chosen stages are inserted as s17–s19; the former
  completion becomes s20. Title-stage maps include the Fox subset, without
  changing its separate canonical reward-only definition or chest claim.
  Existing item139 acquisition and NPC item139 consumption/item875 grant
  stay distinct. No second reward intent is added.
- Grave Danger / Cobra: a fresh authenticated Scarlett death credited to an
  eligible player participant completes existing chosen s5 and advances to
  s6. Canary's actual damage-player death callback and ScarlettKilled credit
  support this choice. Prior kill history does not auto-complete a newly
  eligible stage. Monster encounter phase, Crystal access and Canary journal
  scalar remain distinct; no donor numeric ID becomes a Native storage slot.

The closed packet/schema and portable Source-span replay live at
`tools/content-schema/quest-authoring/samples/chosen-journal/`.
There are five correction records, three attachment envelopes and 57 checked
Source spans. Original donor journal disagreements remain retained as Source;
chosen adaptations resolve the selected data programme, not donor equality.
Missing or altered correction packets fail closed. Canonical authoring registers
all correction inputs and schemas, and exact packet/record digests.

## Accepted QuestState data import

A fresh live coordination read identified the already merged QUEST-LOWER-1
(PR #1727, commit `864c6b6ab3fbbdd7b3e5ea07e432d20014964f2a`). Its exact
Python lowerer and six tests are adopted without changing the tool or Rust.
`content/quests/missions/quest-state.json` and its population marker are now
generated locally from the final canonical definitions. The accepted current-main
loader embeds this file at build time; the older local base lacks that Rust loader.

The data contains 96 definitions with Source progress (1328 tracks, 3258
transitions, 2243 NPC requested_by bindings). Computed effects and inexact guards
keep 387 transitions NOT_SUPPORTED. Single-track completion is lowered for six
quests through four completion transitions; 52 multi-track and 38 no-mission
completion cases retain explicit limitations. The other 256 definitions have no
Source progress in this input and are not given invented tracks. Chosen recipe
stages and new operational supplements remain data for subsequent bindings; this
accepted generator consumes the original Source progress.

Regeneration and all six original tests pass; both generated files match the
independently replayed receipt byte-for-byte. Actual Rust loader execution and
server activation were not run. This accepted QuestState key rule differs from
generic v2 ProductionKey; the Barbarian Test naming restriction below applies
only to that secondary overlay, not this QuestState import route.

## Secondary v2 import artifact and precise limits

`quest_v2_export.py` emits the existing ordinary server-v2 declaration document
format, not an installed world package. It reads all 352 canonical definitions:
351 declarations, 309 chosen authoring profiles and 42 declarations without
chosen profiles. Chosen level/premium/repeatability become existing optional
fields; unknown or out-of-range values are omitted. Selected stages, daily-cycle
parameters and reward/prerequisite intents remain explicitly non-executable Text
observations. No unresolved free-text name is converted to a typed production
Item, Achievement, Encounter or Quest reference.

One definition, `oteryn:quest.barbarian_test`, is retained in the export report
as an identity hold: the existing server ProductionKey validator rejects a
`test` segment. No alias or identity mutation was invented. Every included
profile targets an included declaration. Canonical and recipe pointers carry
separate record digests; deterministic `--check` performs no writes.

The overlay is in `samples/v2-export/{declarations,report}.json`. Qualification
is Source-backed closed-shape validation against actual Rust types and key
rules. The Rust parser and a merged project package were not run. The overlay
exceeds the native-entry first-slice 64KB document limit; that limit was not
weakened. Existing world declarations, manifest and lock are untouched.

## Source access

All Quest facts reused existing pinned Canary and Crystal corpus/AST/wiki
captures. The new replay reads only selected blobs from the portable corpus
archive; it does not download or extract a second full corpus. Cached Fandom
and BR revisions are corroboration, with their historical capture limits.
No new donor Quest/wiki fetch or Remote Desktop action occurred in this batch.
Live coordination and merged PR state were read through the ordinary GitHub
connector; the accepted lowerer, tests and loader source were read through normal
public GitHub HTTPS at the exact merged commit. Pins and hashes are retained in
`OTV2-20261004-quest-state-import-receipt.json`.

## Coordinator / architect follow-up

This is a local follow-up request, not an architectural decision:

1. Use existing QUEST-LOWER-1 / QUEST-PRED-1 / QUEST-XP-1 and reconcile this
   local data with current main before assigning integration. Those tasks have
   merged (#1727/#1723/#1724); do not create duplicate mechanism workers. Route
   remaining chosen-stage/gate/trigger bindings to the existing QUEST-CONTENT-2
   owner. Separately qualify any complete world-v2 overlay merge/budget; the
   secondary overlay cannot replace the current declarations.
2. If integrating the secondary generic-v2 catalogue, decide how the legitimate
   Barbarian Test identity should pass its ProductionKey:
   an approved validator policy adjustment or an explicit canonical alias with
   all references migrated. Preserve the current identity until that decision.
3. Assign Native bindings for chosen stage events, Item/Achievement/Encounter
   references, NPC reward delivery and the authoritative quest reducer/writer.
   These are execution/integration tasks, not absent donor files. Findings on
   local base88d63 are not assertions that current main lacks these mechanisms.

The previous coordination request is at
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5973711588
(the original #162 is closed). The completed local data and precise remaining
integration request were posted after final validation at
https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5979811358.
This updates the prior request; it does not claim a new assignment or decision.

## Verification

Focused new tests: 21 PASS (six chosen correction tests, fifteen exporter tests).
Canonical tree, binding, rollout, v2 export and full donor packet regeneration
pass. Independent final readback passes: all 352 Source cores, prior donor
attachments, reward intents and 25 pinned compiler inputs are unchanged; exact
finite overlay replay matches the complete current tree. Every exported parent,
recipe and packet digest is verified. The first full run exercised 910 tests and
found one stale schema-extension test expectation. It now checks both optional
extensions explicitly while retaining every original constraint. The repaired
38-test focus passes, including the six accepted lowerer tests. The complete
updated `run_checks.py` finishes with exit 0: 916 unit tests pass (three unchanged
skips), 262 schema cases pass, and all generated-content/source checks pass,
including the accepted QuestState import and secondary v2 export. The separate
RewardClaim suites pass (13 and six tests), as do governance, repository policy
and `git diff --check`. Readiness remains 42 definition_ready,
242 waiting_data and 68 waiting_native_bindings; it is not a count of playable
Quests. No full-Quest Source/runtime admission is claimed.
