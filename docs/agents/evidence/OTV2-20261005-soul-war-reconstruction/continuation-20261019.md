# Soul War continuation and four-subagent review — 2026-10-19

Task: `OTV2-20261005-soul-war-reconstruction`; PR: `Oteryn/Oteryn-Game#1867`.

## Candidate boundary

- Starting remote head: `3a4f8bf24b0e5cb537a17cd8b0bf9d325d157cc8`.
- Integrated protected main: `575bcf7c94201fa80a6838e7fc8303d90038dc30`.
- Merge base: `e89e605bd72ba91a8905309d7aedf4faeeb71b6d`.
- One lead owns the canonical task branch. The four diagnostic subagents did not mutate files, publish, merge, run tests or activate runtime.
- Their initial filesystem commands were denied by their read-only execution policy. The same four threads then reviewed bounded source packets verified by the lead against the exact checkout. No sandbox or execution policy was weakened.
- Complete reports, thread identities and verified primary-donor source hashes are retained in `four-subagent-review-20261019.json` beside this document. These diagnostic reports are not exact-final-candidate CI or a new visual video audit.

## Findings and disposition

| Finding | Classification | Disposition |
| --- | --- | --- |
| Latest Quest CI failed the committed QuestState provenance comparison. | PROVEN / current-gate blocker | Regenerate QuestState from the integrated inputs, not by substituting digest strings. Final checks are recorded on the exact PR head. |
| The donor capture still referred to earlier Quest input witnesses. | PROVEN / derived-data dependency | Rebuild the complete capture pipeline from the pinned corpus after the canonical index is finalized; preserve the corpus-wide donor pins. |
| Routing CI failed on a blank line at the end of the archived task. | PROVEN / current-gate blocker | Remove the extra end-of-file blank line and run `git diff --check`. |
| Blue-to-Purple evidence pointed to the Green-to-Blue rule. | PROVEN / evidence error | Correct the JSON pointer to rule 5 and add a destination-to-rule regression. |
| Hatred defense and outgoing-damage evidence shared only the outgoing destination. | PROVEN / evidence error | Split the verified source ranges between the player-hit defense rule and outgoing empowerment rule. |
| Raid timer evidence claimed the 5-second player kick was represented by an Encounter timer. | PROVEN / evidence error | Limit mapped timer constants to 20 and 120 seconds; keep the existing 5-second player-removal hard hold. |
| The readback said Greed had a 45-second window. | PROVEN / evidence error | Correct it to the source's 34-second sphere-phase timeout; this is not a claim of a 34-second vulnerability window. |
| Accepted values, final-stage identities and all hard holds lacked focused mutation/regression checks. | PROVEN / bounded qualification repair | Add checks for accepted Mirror Image/taint values, any-order/final gate, final boss/reward identities, all 22 pre-existing unresolved source ranges, and corrected manifest destinations. |
| Quest CI did not explicitly run the Soul War Encounter generator and tests. | PROVEN / qualification gap | Add both to the existing Quest check runner, preserving main's cheap-check-first order and all existing checks. |
| An unused adapter retained a placeholder digest and obsolete packet shape. | PROVEN / obsolete code | Remove only `quest_soul_war_reconstruction.py` after confirming no consumers. Keep the active `quest_soul_war_followup.py` and its provenance fences. |
| Optional structured shrine-ID/cadence fields were proposed by a diagnostic worker. | HARDENING / deferred | Existing accepted IDs, exact 3000 ms evidence, donor-defect dispositions and runtime holds remain unchanged. This repair does not redesign the digest-bound reconstruction payload or claim those held mechanics are executable. |
| Completion-import summary used fixed totals. | DERIVED / no present defect | The actual generated receipt still reports 310 quests, 3217 tracks and 5150 transitions; no unrelated reporting refactor is included. |

## Primary source verification

The lead verified the retained corpus archive and manifest digests, then each selected `crystal-summer` raw source against both SHA-256 and Git blob SHA-1 at `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.

- `data-global/monster/quests/soul_war/goshnars_greed.lua`, lines 138–188: line 166 sets the 34000 ms sphere-phase timeout.
- `data-global/lib/quests/soul_war.lua`, lines 259–261: spawn 20 s, survival 120 s, separate player kick 5 s; lines 588–600: Blue returns to Purple after 7000 ms.
- `data-global/scripts/quests/soul_war/soul_war_mechanics.lua`, lines 647–653 and 655–662: distinct defense and outgoing-primary-damage branches.
- `data-global/monster/quests/soul_war/soul_sphere.lua`, lines 90–124: one westward step per 3000 ms and a boss-contact heal; this remains unrepresentable in Encounter v1 and held.

## Preserved product and integration boundaries

The canonical Quest reconstruction/recipe payload is unchanged. The subsequent primary-source audit below corrects Encounter behavior and evidence where the accepted vocabulary can express the exact bounded repair. Five independent boss flags remain the intended semantics, while the chosen count-five projection retains its explicit uniqueness limitation. Mirror Image stays 70% / 7.5% / 1 HP, the accepted taint teleport remains unchanged, Rotten Wasteland requires four distinct shrine IDs, and the invalid donor `Pos` coordinate remains unguessed. All 22 pre-existing unresolved topics remain hard holds; incorrect line ranges were corrected against the primary donor. The final packet contains 45 unresolved rows, including newly exposed timing, clock, captured-position and source-defect gaps. The explicit old-to-new source-range proof is `hard-hold-source-corrections-20261019.json`. No canonical Encounter admission, Native activation, new Quest engine, reward delivery, production action or protected-main merge is part of this repair.

Main's `--reference-engine` option and relational source-progress assertions are preserved alongside the PR's LF normalization and Soul War population counts. Conflicting compact JSON is regenerated through the owning tools, not selected wholesale as a final merge resolution.

## Qualification record

The local qualification environment is Linux Python 3.12.15, with the same declared `jsonschema==4.26.0` and `referencing==0.37.0` requirements as the protected Quest workflow. Checks use actual generators, schemas and tests; no validator or runtime hold is bypassed. Exact final candidate SHA, observed local exits and GitHub workflow results belong in the PR's final freeze/readback comments. A committed task record or this report does not itself assert green CI, playable runtime, or merge completion.

## Primary-source audit and final bounded repairs

The same four subagent threads performed a second source-based sweep, this time against the complete exact pinned donor files used by their assigned Encounter bundles. Reports are retained alongside the first diagnostic round in the review JSON. The lead verified each raw donor file against the corpus manifest SHA-256 and Git blob SHA-1. The new regression checks every manifest's source identity and line bounds directly against that archive; no network or donor code execution is required.

This sweep found wrong source offsets, including Malice lifecycle citations that pointed to white-tile/loot code and a Purple Megalomania reference beyond the source's end. These are corrected without closing the underlying mechanic holds. Global clocks, shared counters, delayed captured positions, disappearance cleanup and the Cruelty shared timestamp gate are now explicitly distinguished from nominal instance-local draft timers. Hatred's pre-decrement/reversal and primary-only damage details remain held, not falsely described as exact forward-timer parity.

Two bounded behavior repairs use existing accepted vocabulary:

- **Greed:** delayed Weak/Strong/Powerful Soul actions require the original triggering creature to remain in the arena (`in_anchor` with `triggering`, CW2-1). Weak and Strong spawn a fresh successor at the source's fixed position and remove the original, rather than transforming it. An early-killed Powerful Soul no longer buffs the boss. Its source's separate 10-second Weak Soul replacement obligation remains explicitly unresolved instead of being replaced with an incorrect immediate transform. Shared source-type counter and max-health/reflection semantics remain held.
- **Megalomania:** mastered Aspects do not count or schedule the authored return; entering Blue first teleports the boss to the source center. The real `MegalomaniaDeath` event is associated with Green/Blue rather than invented `SoulWarMegalomaniaDeath` coverage, and the authored outcome is not emitted in Purple. Capturing the boss position at Aspect death (rather than reading it five seconds later), the Green fallback and the Necromantic Focus callback/identifier defect remain unresolved.

**Soul Cage timing is not silently repaired or waived.** Exact Crystal lines 979–1004 resolve a cage by name when the callback fires, so replacement targeting exists in the donor itself. No accepted Oteryn exception selecting an original-instance policy was established. A previous PR agent comment mentioned `D778` / `SOUL-WAR-TIMING-1`, but no corresponding accepted decision/task was established by the bounded read; the comment is not treated as closure. The manifest now holds overlapping-cycle/original-instance versus donor name-wide semantics explicitly. Runtime admission remains blocked.

The seven other Encounter behavior files and every catalog remain unchanged; their evidence ranges/statuses are corrected as required. These bundles contain partial draft cores plus explicit holds, not nine fully faithful or executable encounters. A passing authoring suite does not turn them into playable content.
