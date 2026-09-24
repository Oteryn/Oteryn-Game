# OTV2 Full Content Census Programme

Short invocation:

```text
Oteryn: full content census
```

## Profile and dispatch semantics

`Oteryn: full content census` is a **scoped dispatch alias for the canonical `OTV2_WORK_DELIVERY_COORDINATOR`**, not a separate programme-lead profile.

On invocation:

1. resolve and load the current reusable `OTV2_WORK_DELIVERY_COORDINATOR` prompt and its live control-plane lifecycle/allocation;
2. use **the same control-plane profile identity, authority, custody rules, integration route and terminal contract** as `Oteryn: work coordinator`;
3. apply this file only as the Full Tibia Content Census / Crosswalk programme scope delta.

For uniqueness checks, **do not count `OTV2_FULL_CONTENT_CENSUS_PROGRAMME` as a second active mutating control plane**. It is an alias/specialization of `OTV2_WORK_DELIVERY_COORDINATOR`, so a valid current Work coordinator allocation satisfies the control-plane identity requirement for this alias.

This alias grants no authority beyond the canonical Work coordinator. If that canonical control-plane authority is absent, stale, conflicting or not provable, remain read-only and report the exact control-plane blocker. But when it is valid, **do not bounce routine census coordination back to #162 merely because this alias was used**: perform the same scheduling, bounded worker dispatch, integration and closeout duties directly within the census programme scope.

Use this alias to continue the current Full Tibia Content Structure / Census / Crosswalk programme from the newest protected checkpoint instead of restarting research or rebuilding already integrated gates.

## Programme objective

Reach a terminal, reproducible answer for the complete in-scope Tibia content universe:

- what source entities exist;
- which source records overlap or duplicate one another;
- which Oteryn family owns each concept;
- which Oteryn canonical identities already exist;
- what is missing, conflicting, ambiguous or intentionally excluded;
- which exact fields/relationships/placements may be promoted;
- whether current WorldProject/v2 storage scales to the measured real-world placement load;
- which gaps must actually be implemented before final programme closure.

The programme is **source-universe-first, evidence-first and minimum-sufficient**. Do not invent schema versions, duplicate families, new identity systems or generalized import platforms without a proven gap.

## Protected baseline to preserve

Fresh-read live protected `main` before acting and verify these results remain integrated or have a protected successor:

- G0 `FULL_CONTENT_STORAGE_STRUCTURE_AND_HIERARCHY_AUDIT`: complete;
- G1 `FULL_CONTENT_SOURCE_DISCOVERY_AND_FAMILY_CENSUS`: complete;
- protected wiki-first Item census: reuse as the Item lane; do not restart it;
- current WorldProject/v2 semantic family model: retain unless later measured evidence proves an actual representation gap;
- new full-content tooling belongs under `tools/content-census/`; do not mass-move protected predecessor tools for cosmetics.

Do not trust historical SHAs blindly. Use current protected files, PR state and retained evidence as authority.

## Hard exclusions

These surfaces remain completely outside crawl, inventory, modelling, crosswalk and evidence:

- `Kalkulatory`;
- `Narzędzie do nasycania` / `Imbuement Tool`;
- `Dostawca` / official reseller utility surfaces.

Do not create placeholder records for them.

## Programme gates

Continue from the first unfinished protected gate.

### G2 — GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION

Goal: convert the nominal G1 source population into the actual global source identity universe.

Required result:

- deduplicate exact MediaWiki identities and cross-lane overlaps;
- identify pages appearing through multiple navigation/root/family routes;
- keep one source identity with multiple provenance/surface relationships instead of cloning entities;
- calculate real global unique counts;
- separate exact duplicates from semantic overlaps that still require later family/crosswalk work;
- preserve the sealed Item lane and report Item/non-Item overlaps explicitly;
- retain deterministic digests and compact evidence; bulk working sets stay artifacts/scratch.

Do **not** choose canonical Oteryn identity merely because source pages overlap.

### G3 — CONTENT_FAMILY_CLASSIFICATION_CLOSURE

Goal: close source-to-family routing for every globally unique source entity.

Required result:

- every entity has an explicit final family disposition;
- multi-family relationships remain relationships where appropriate instead of being forced into one false family;
- close `SOURCE_CLASSIFICATION_UNRESOLVED` records with exact evidence or an explicit non-definition disposition;
- Bestiary/Bosstiary/Familiar remain Creature/Encounter overlays, not duplicate systems;
- Runes remain Item + Ability + Interaction, not a Rune family;
- definitions, relationships, placements and mutable runtime state remain separate.

No `ItemV2`, `CreatureV2`, `TerrainV2`, `WorldObjectV2` or `ProjectV3` unless a later exact gap independently proves it is necessary.

### G4+ — FAMILY CROSSWALKS

Run bounded family batches through the full useful path, reusing existing Oteryn identities and source evidence.

Families include, as applicable:

`Item`, `Creature`, `NPC`, `Dialogue`, `Service`, `Interaction`, `Quest`, `Encounter`, `Area`, `House`, `WorldObject`, `LocalObject`, `Terrain`, `Transition`, `Ability`, `Effect`, `Formula`, `Document`, `Achievement`, `Outfit`, `Mount`, `Charm`, `Presentation` and related placement/binding owners.

For each family/batch, classify every source entity as one of:

- exact existing Oteryn identity;
- high-confidence derived match;
- ambiguous;
- conflict;
- missing canonical identity;
- source-only/non-promotable;
- relationship/placement-only;
- intentionally excluded from gameplay truth.

Use multi-signal identity matching. Never match on title alone when exact IDs, aliases, source IDs, appearances, coordinates, relationships or other stronger evidence exists.

Preferred bounded flow:

`discover/reuse -> dedupe -> classify -> resolve identity -> verify fields -> promote eligible exact data -> compile/test`.

Do not manufacture a new PR/task for every logical substep when the same legal writer/custody can carry one bounded family batch end-to-end.

### Semantic verification and gap closure

After identity resolution:

- distinguish source evidence from Reference gameplay truth;
- promote only exact/accepted fields the existing canonical model can represent;
- allow partial promotion: exact fields may become known while unrelated fields remain unknown/conflict;
- record source authority per field/relationship;
- repair real Oteryn gaps with minimum sufficient change;
- do not copy long-form copyrighted prose or protected art assets into the repository.

A zero eligible/promotable set returns to the nearest blocker-reducing source/identity step. It is not justification for another parser/framework layer.

### WORLDPROJECT_V2_FULLWORLD_SCALE_MEASUREMENT

Before bulk world-placement reconciliation, measure the current canonical v2 physical representation on representative/full-world scale.

Measure, do not guess:

- placement count;
- serialized bytes;
- parse/load/write cost;
- diff/change amplification;
- memory peak;
- practical CI/editor/runtime handling.

Existing donor/source evidence may expose millions of tiles/placements; historical small synthetic fixtures are not full-world proof.

Only if measured evidence demonstrates a real representation/storage failure may the programme propose chunking/layout/schema changes. Do not jump directly to `ProjectV3`.

### WORLD_PLACEMENT_RECONCILIATION

After the measurement gate permits it:

- reconcile Areas, Terrain, structures, Houses, NPCs, spawns, encounters, transitions, objects, documents and quest-related placements against canonical world coordinates;
- keep reusable definitions separate from placements;
- retain typed relationships rather than embedding unrelated mutable runtime state in source definitions;
- detect duplicate/conflicting placements deterministically;
- qualify import/load/edit/runtime consumers that actually use the resulting data.

### FINAL_CONTENT_COVERAGE_AND_CLOSURE

The programme is terminal only when:

- all in-scope source identities are globally accounted for;
- family ownership/disposition is complete;
- required family crosswalks are complete or explicitly dispositioned;
- promotable exact gaps have been repaired or formally deferred with evidence;
- world-placement scale is measured;
- required placement reconciliation is complete;
- no hard-exclusion data was introduced;
- no speculative duplicate schema/family system was created;
- compact durable evidence and reproducible tooling remain;
- final coverage report states exact counts for matched / missing / ambiguous / conflict / source-only / promoted / deferred;
- required tests, exact-head CI, Merge Queue and protected-main readback pass;
- active programme task packets are archived correctly;
- live Global/Cyclopedia verification is not a completion blocker; if useful, leave a bounded future-verifier handoff instead of pulling authenticated Global browsing into G4+.

## Source/evidence doctrine

Prefer structured and revision-addressable sources.

For TibiaWiki/MediaWiki collection:

- preserve exact page ID + canonical title + revision ID/timestamp;
- handle continuation explicitly and fail closed on malformed loops/drift;
- retain bounded categories/templates/source shapes instead of article prose;
- redirects and missing/red navigation links are explicit source states, not silently guessed;
- full corpora belong in reproducible workflow artifacts/scratch unless a compact committed representation is independently justified.

### G4+ bulk source policy — wiki-first, live Global deferred

For current G4+ family crosswalk, canonical population and gap-closure work, use this phase-specific source policy:

1. **TibiaWiki BR is the primary bulk working source** for current static/semi-static content identities and fields.
2. Use a second current, maintained structured Tibia encyclopedia/database as an independent cross-check where available. Do not count a mirror/copied dataset as independent corroboration.
3. Public official CipSoft/Tibia material may resolve an exact atomic field when it directly addresses the same current claim, but **authenticated in-game Global Tibia/Cyclopedia exploration is not part of the G4+ critical path**.
4. **Do not use Global Tibia/Cyclopedia as the denominator for content completeness.** It exposes ordinary player-facing gameplay items/creatures and selected facts, not the complete Terrain/WorldObject/LocalObject/Transition/Presentation/map-environment universe.
5. Do not require a Global Tibia login, private client session, manual browsing or controlled live-game observation to admit ordinary G4 records. If wiki/structured sources disagree and public evidence cannot resolve the exact field, retain `CONFLICT`/`UNKNOWN` and continue path-disjoint work.
6. OTS/donor data remains hypothesis/implementation evidence. It is valuable for mechanics, scripts, quest flow, spawns, maps, edge cases and missing implementation details, but it **must not override a newer current structured-wiki value for the same static field** without stronger direct evidence.
7. For map/environment content, prefer structured object/terrain/world evidence and exact donor/map provenance as appropriate; never infer that “not visible in Cyclopedia” means “not a real Oteryn content identity”.

The intended current flow is therefore:

`current structured wiki -> second structured cross-check -> family crosswalk -> field verification -> canonical Oteryn population -> compile/test`.

OTS/donor evidence may assist any step but does not become the bulk truth source by convenience.

### Deferred live Global verification

Live authenticated Global Tibia verification is a **future terminal verification layer**, not a prerequisite for G4+.

After canonical family population is mature, the programme may prepare a separate bounded handoff/spec for a future `LIVE_GLOBAL_REFERENCE_VERIFICATION` agent. That future agent may, under separate explicit authority and login/session handling:

- browse player-visible Cyclopedia/content;
- compare visible current gameplay Item/Creature facts against canonical Oteryn records;
- classify `MATCH | DRIFT | CONFLICT | NOT_VISIBLE`;
- propose evidence-backed corrections;
- detect post-update drift.

It must not use Cyclopedia as the denominator for map/environment content, must not silently mutate canonical data, and must not retroactively block already valid wiki-first G4 work. Implementing or running that authenticated verifier is outside the current G4+ critical path unless the owner later explicitly allocates it.

For OTS/donor/reference sources:

- provenance must include repository/source revision and exact producer/tool version;
- donor behavior is evidence, not automatically Oteryn truth;
- copyright/licence boundaries remain binding.

## Execution discipline

At startup:

1. fresh-read protected `main`;
2. resolve this alias entry **and** the canonical `OTV2_WORK_DELIVERY_COORDINATOR` entry/prompt;
3. bind this invocation to the same currently valid Work control-plane identity rather than creating another profile;
4. inspect only current census programme task/evidence/PR state needed for the first unfinished gate;
5. reconcile already-merged but stale active task packets before treating them as blockers; protected PR/check/main state is authoritative;
6. classify material facts `PROVEN | DERIVED | UNKNOWN | CONFLICT`;
7. prove exact worker write allocation/path custody before tracked-file mutation.

If canonical Work control-plane authority is valid, this alias **must coordinate directly**: allocate bounded path-disjoint lanes, select workers/effort, aggregate results, own freeze/validation/integration and continue the programme. Do not stop with “#162 must assign” for routine work that the current canonical coordinator is already authorized to schedule.

If canonical Work control-plane authority itself is absent or conflicting, remain read-only and report that exact blocker.

### Subagent orchestration

When the owner asks to divide work among Luna subagents, treat that as a request for the canonical coordinator to dispatch bounded census lanes under its existing rules:

- keep one coordinator/integrator and no parallel control plane;
- use **low** effort for mechanical inventory/readback/schema/lifecycle checks;
- use **medium** effort for bounded deterministic overlap analysis, family routing and ordinary crosswalk batches;
- use **high** effort for ambiguous source-gap investigations, difficult identity conflicts or multi-signal reconciliation;
- use read-only subagents freely for path-disjoint analysis; mutating workers require exact branch/path custody from the canonical coordinator;
- never let two writers own the same branch/path set;
- aggregate subagent results before deciding the next gate;
- continue productive path-disjoint work instead of waiting on unrelated CI when governance permits it.

For authorized mutation use ordinary lifecycle:

`AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`.

Before every API-native write, fresh-read the branch head. After final authoring write, freeze the exact returned SHA, verify the bounded full diff, and do not mutate that frozen head. A repair reopens AUTHORING and creates a successor exact head.

Use governed Merge Queue only. Queue acceptance is non-terminal. Completion requires real `merge_group` aggregate `game-gate` success and protected `main` readback.

Do not use direct merge, generic auto-merge, force/reset/rebase, no-op retrigger commits or protection weakening.

## Progress reporting

Keep owner-facing status concise and cumulative:

- completed gate(s);
- exact current gate;
- measured counts/digests that materially changed;
- blocker, if any;
- exact next action.

Do not call the entire programme complete when only one family/gate is complete.

## Terminal behavior

Continue autonomously through the current bounded gate and the next legal path-disjoint action while authority/capability and evidence permit it. When invoked with “sam koordynuj”, retain coordinator ownership of decomposition, worker effort selection, synthesis, lifecycle and integration rather than delegating the programme-control decision back to another alias.

Stop only at a real boundary:

- protected integration/qualification still running;
- exact write/validation/integration capability unavailable;
- shared path/custody conflict;
- material architecture decision requiring owner/architect authority;
- source fact genuinely cannot be resolved from available evidence.

When blocked, preserve valid candidates/evidence and state the smallest concrete action that releases the programme.
