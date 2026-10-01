# NPC corrective admission R5

R5 applies proof-bound corrections to existing native declarations and preserves unresolved source facts separately. It follows the owner’s request to complete the NPC audit using subagents and alternative public sources. PR #1358 is integrated; R4 custody remains historical and unchanged.

## Concrete content changes

- Correct eight base travel fares; retain 195 plain routes. Hold Bluebear→Carlin and Seahorse→Venore because they mutate quest state, and Seahorse→Oramond because its Citizen access predicate is absent from the native DTO. All source routes, 171 conditional discounts and 34 postal quote contexts remain evidence.
- Hold 457 unsafe trade tuples: unmapped fluid/count variants, 57 unresolved exercise/wrap variants and 284 offers guarded by quest callbacks. Five gated merchants retain their Service identities with empty executable offers. Prices, Item references and callback source predicates are retained without invented native guards or fluid mappings.
- Correct five existing speech values across Sam, Frodo and Xodet. Existing triggers, children, matching order, flags and event hooks are preserved. Thirteen transcript alias proposals remain source-only.
- Hold Hagor’s presentation (feet=1156 outside native palette 0–132) and A Sleeping Dragon’s absent appearance object 168. No replacement appearance is guessed. Authoring and promotion validators enforce the native palette bound.
- Attach source quest associations to 275 NPCs using hash-bound locators. All 1,442 source transitions across 54 source quests resolve to NPC identities. 311/314 source Item occurrences resolve to 154 native Item targets with XML/loader/alias/appearance proof. The three remaining chest appearance references stay held. These fields do not define or execute native Game Quests.

`native-repairs.json` contains 433 exact before/after guards: 295 NPCs, 135 Services and three Dialogues. The native materializer verifies its digest and rejects identity/family drift or mutations outside the corrective boundary before canonical serialization. The original R9 admission inputs and provenance remain unchanged. The project revision becomes `g4-npc-source-repairs-r10`; derived content is regenerated from the resulting canonical package.

## Alternative source recovery and coverage

Public GitHub access supplied 226 digest-verified captures from `s2ward/tibia` at `8824eb38872a1174b0f0c923e08719e981f32ecc`. This is also its latest head; derivative mirrors are not independent confirmation. 95 of 159 new identity proposals have named literal speech; 64 do not. Six existing identities acquire first literal observations, while 170 still have no primary capture. There are 1,686 named literal speech observations with line/byte offsets and SHA-256/Git blob proof. John the Carpenter and Ghost Captain filename/speaker contamination stays held. Prompts in an observed conversation do not establish native matchers or quest guards.

The 159 identity proposals and two missing native candidate identities stay explicitly incomplete; none is silently counted as a playable import. `dialogue-qualification-summary.json` records the 902 nonempty source programs and their outstanding matching, branch, guard and action requirements; full roots remain in R4. Global completeness and NPC runtime eligibility are false. The supplied wiki/TibiaSecrets URLs remain evidence locators; proxy-denied reads are not treated as successful captures.

There are also 78 plain offers with unresolved price disagreement among captured sources. D15 preserves their source price until qualified evidence settles it; `price-parity-pending.json` maps every pending source tuple to its current native offer and keeps the dissenting prices. These are not reported as confirmed Global prices.

## Files and reproduction

`manifest.json` binds every retained payload. `corrective-baseline.json` preserves only the touched original declarations, enabling offline drift detection and idempotent reconstruction. `corrective-plan.json` binds the six corrective inputs; its operations are limited to NPC/Dialogue/Service data. `inventory-repaired.json`, `trade-r4-overlay-repairs.json`, `transport-repaired.json`, `static-dialogue-repairs.json`, `quest-stage.json` and `recovered-dialogue-associations.json` preserve lane evidence and explicit holds. Capture indexes contain public immutable URLs and digests; original third-party raw assets are not redistributed.

Run `npc_corrective_overlay.py` with the retained baseline and six inputs to reconstruct the packet, then the existing `materialize_content_world_project_v2` example to construct native canonical documents. Native parsing, reference validation, byte-identical canonical roundtrip and successor-tree roundtrip are required before publication. Focused validation results are recorded in `validation.json`; CI remains the PR’s live exact-head state. No queue, merge, runtime switch or production authority is granted by this packet.
