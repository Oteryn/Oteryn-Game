# OTV2 Full Content Population Programme

Canonical invocation:

```text
Oteryn: full content population
```

Compatibility invocation:

```text
Oteryn: full content census
```

The historical lifecycle ID and file path are retained for continuity. After the protected census/classification prerequisites are present, both invocations resolve to the **population-first** behaviour in this prompt.

## Profile and dispatch semantics

`OTV2_FULL_CONTENT_CENSUS_PROGRAMME` remains a **scoped dispatch alias for the canonical `OTV2_WORK_DELIVERY_COORDINATOR`**, not a second programme lead or mutating control plane.

On invocation:

1. fresh-read protected `main`;
2. resolve the current reusable `OTV2_WORK_DELIVERY_COORDINATOR` prompt and live control-plane allocation;
3. use the same profile identity, authority, custody, review and integration route as `Oteryn: work coordinator`;
4. apply this file only as the Full Content population scope delta.

Do not count this alias as a second active mutating control plane. If the canonical Work coordinator is current and authorized, coordinate this programme directly instead of bouncing routine scheduling back to #162 merely because the scoped alias was used.

This prompt grants no authority beyond the canonical Work coordinator.

## Current programme mode: POPULATION_FIRST

The source-discovery era is no longer the default execution mode once live protected `main` confirms the integrated census/classification/provenance prerequisites.

Treat protected G0-G3 results, the protected Item census, G4 source provenance, the typed source-identity carrier and the full-cardinality measurement as **inputs to implementation**. Do not restart them or create another analysis generation unless a concrete unresolved fact blocks a real canonical write in the current bounded batch.

Primary objective:

> turn already-discovered source entities into real Oteryn canonical identities, definitions, fields, relationships, tags, presentation bindings and later placements, using the existing WorldProject/v2 model and existing compilation/test paths.

Success is measured by product population, not by the number of new census/evidence artifacts.

## Population-first invariants

1. **No new census/crosswalk/revalidation-only slice by default.** A new analysis-only task/PR is allowed only when an exact unresolved fact blocks canonical population and cannot legally be resolved inside the same bounded family batch.
2. **A G4 family batch with at least one eligible record must produce a real canonical product delta.** Do not end the batch with only `MISSING_CANONICAL_IDENTITY`, `UNKNOWN` or another report if a safe canonical record or field can already be written.
3. **Partial canonical records are valid.** Identity completeness and semantic completeness are separate. Known exact fields may be promoted while unrelated fields remain `UNKNOWN` or `CONFLICT`.
4. **Second-source corroboration is helpful, not a universal admission gate.** If TibiaWiki BR supplies an exact source identity, supported family shape and non-conflicting structured facts sufficient for the bounded claim, absence of a second source does not by itself block canonical population.
5. **Do not use title-only matching to hijack an existing identity.** Existing Oteryn identities require the strongest available multi-signal match. For a genuinely missing non-Item identity, a new key may be created only under the existing canonical key conventions after exact source identity/family proof plus duplicate/collision checks.
6. **External IDs never become canonical Oteryn identity.** Preserve exact source key/revision/digest, namespace and verbatim external ID through the existing `ProjectV2SourceIdentityBinding`.
7. **Unknown evidence stays unknown without blocking siblings.** One unresolved field, relation or source must not stall independent eligible fields/entities in the same batch.
8. **Do not invent another schema/framework.** No `ProjectV3`, `ItemV2`, `CreatureV2`, generic registry service or new importer platform unless an exact current record proves the protected WorldProject/v2 model cannot represent a required product fact.
9. **Evidence is an output of product work, not a successor trigger.** Keep one batch end-to-end while the same writer/custody/execution surface can legally carry it.
10. **Prefer real content over process closure.** Lifecycle/archive cleanup must not consume the critical path while a path-disjoint population batch can legally continue.

## Protected foundation to reuse

Fresh-read current protected state rather than trusting historical SHAs, but preserve these integrated architectural results unless a protected successor supersedes them:

- G0 storage/family audit;
- G1 source discovery;
- G2 exact source overlap/deduplication;
- G3 family classification and explicit disposition of unresolved rows;
- protected wiki-first Item source lane;
- WorldProject/v2 family model;
- typed `ProjectV2SourceIdentityBinding`;
- wiki-first G4 source policy;
- existing Item identity/model/artifact lineage;
- full-cardinality WorldProject/v2 measurement evidence.

Do not reopen a protected predecessor merely because later current-source observations drift. Revalidate only the exact affected rows needed by the current population batch.

## Canonical storage contract

Use the existing logical storage model. Do not create cosmetic per-family filesystem trees merely to make the repository look catalog-like.

### Definitions

- `definitions/reference.json` — accepted executable/reference definitions already owned by executable Reference families;
- `definitions/declarations.json` — typed declarative definitions and source-only authoring for families not executable through the Reference linker.

### Source identity and provenance

- `provenance/sources.json` — exact source records and accepted source-identity bindings;
- `provenance/imports.json` — import/candidate provenance where the existing pipeline requires it.

### Editor catalogue metadata

- `editor/author.json` — display names, aliases and **namespaced editor/catalogue tags**.

Tags help browse, query and author content. They must not replace typed family fields or silently create gameplay semantics.

### Presentation and assets

- `presentations/bindings.json` — Definition/Presentation binding layer;
- `assets/catalog.json` — immutable asset identity/digest catalogue.

Keep:

`Definition -> Presentation -> Asset`

separate from legacy/client/source IDs.

### World placements

- `worlds/world.json` remains the semantic placement role.

The protected full-cardinality measurement proved the current semantics across the target cardinality only through bounded sequential shards; the monolithic full corpus hit an allocation limit. Therefore **do not block definition population on final world-placement physical layout**, and do not invent ProjectV3. A minimum-sufficient production placement layout/chunking decision belongs to the placement phase after definitions/relationships are populated far enough to consume it.

## Canonical population decision for each source entity

For every bounded family batch, every source entity must end in one of these product-oriented dispositions:

- `MATCH` — bind to an existing canonical Oteryn identity;
- `CREATE` — create a missing canonical Oteryn identity/definition under existing key conventions;
- `CONFLICT` — competing evidence prevents the exact bounded write;
- `AMBIGUOUS` — more than one credible target remains;
- `SOURCE_ONLY` — evidence is retained but does not define a reusable canonical entity;
- `RELATIONSHIP_ONLY` — source row contributes a relation to existing/created definitions;
- `PLACEMENT_ONLY` — source row contributes a world occurrence rather than a new definition;
- `EXCLUDED` — explicit programme exclusion.

`MISSING_CANONICAL_IDENTITY` is not a terminal success state by itself. Convert it to `CREATE` whenever current evidence safely proves a new identity and the existing canonical model can represent it.

## Family batch execution

Default bounded flow:

`select batch -> resolve/create identity -> write source binding -> write editor metadata/tags -> promote exact fields -> write exact relationships -> compile/test -> integrate`

Do not split these logical substeps into separate tasks/PRs unless there is a real path-custody boundary, distinct required execution surface, material architecture decision or independently mandatory gate.

### Batch sizing

Prefer batches large enough to create visible catalogue progress but small enough to review and qualify deterministically.

Default starting point:

- 100-500 ordinary entities for homogeneous declarative families;
- smaller batches for high-relationship or executable families;
- adjust only from measured output/CI/review cost.

Do not spend a separate task choosing an exact batch size when a safe bounded first batch can run immediately.

### Required batch output

When eligible records exist, a completed mutating family batch should report at least:

```text
selected
matched
created
source_bindings_written
editor_entries_written
tags_written
fields_promoted
relationships_written
presentation_bindings_written
conflict
ambiguous
source_only
placement_only
blocked
compiled
tested
integrated
```

The primary progress vector is canonical population, not evidence row count.

## Identity creation and matching

### Existing identities

Use exact IDs/source bindings/aliases/structured fields/relationships and other strong signals. Never remap an existing identity on name similarity alone.

### New non-Item identities

A missing canonical non-Item identity may be created when all are true:

- exact source entity identity is retained;
- family is sufficiently established for the bounded definition;
- no existing canonical key/source binding/alias collision resolves to another target;
- the chosen key follows the existing family/key convention;
- the new record carries source provenance;
- no conflicting stronger evidence exists.

Do not wait for every gameplay field to be known.

### Items

Reuse the protected Item identity authority and existing Item model/artifact lineage. Do not mint duplicate parallel Item identities merely because a wiki page is not yet resolved to the protected Item identity space.

For Item batches use the existing bounded flow:

`resolve -> verify -> continuity-if-needed -> promote -> compile/test`.

Partial field promotion is expected.

## Field promotion

Promote atomic facts independently.

Examples of eligible partial population:

- Creature identity + name + exact health while loot remains unknown;
- NPC identity + presentation/source binding while dialogue remains unknown;
- Quest identity + known prerequisite/reward relationship while some steps remain unknown;
- Item identity + exact weight/armor while another stat remains conflict.

Do not require a "complete entity" before writing exact supported fields.

When an exact field does not fit the current model, first prove the representation gap against the protected v2 model. Repair only that gap with minimum sufficient change.

## Tags and aliases

Create useful catalogue metadata in `editor/author.json` during population instead of deferring all discoverability to a later cleanup pass.

Rules:

- aliases remain non-authoritative lookup hints;
- tags are namespaced/editor metadata;
- tags never determine gameplay authority;
- use a small stable vocabulary and reuse existing tags before minting synonyms;
- family is typed data, not merely a tag;
- source/provider identity is provenance, not merely a tag.

Do not create a separate tagging framework.

## Relationships

Write typed relationships as soon as both endpoints are safely identified/created.

Examples:

- Creature -> Loot -> Item;
- NPC -> Dialogue / Service -> Item;
- Quest -> prerequisite / NPC / Interaction / rewards / Encounter;
- Item -> Ability / Interaction;
- WorldObject -> Interaction / Transition / Document;
- Area -> parent;
- Definition -> Presentation.

Do not embed relationship facts as freeform tags when a typed v2 relation already exists.

## Source policy

### Primary working source

**TibiaWiki BR is the primary bulk working source** for current static/semi-static identities and structured fields.

### Second structured source

Use a maintained second source when available and genuinely independent. Its absence, HTTP denial or licensing-preflight failure does not block an otherwise safe ordinary canonical identity/field.

Do not create repeated second-source probes for unchanged access failures. Cache the blocker fingerprint and continue population work.

### Official public evidence

Use public CipSoft/Tibia evidence when it directly resolves an exact atomic conflict.

Authenticated Global Tibia/Cyclopedia browsing remains a future verification layer, not a denominator or normal G4 blocker.

### OTS/donors

Use Canary/Crystal/other OTS sources actively for implementation/mechanics/map/quest/spawn hypotheses and missing detail discovery, but not as automatic current static truth.

`CURRENT STRUCTURED REFERENCE DATA > DONOR IMPLEMENTATION` for the same static field unless stronger direct evidence proves otherwise.

## Analysis-only exception

An analysis-only task is permitted only when all are true:

1. a named current population batch is blocked;
2. the blocker is exact and bounded;
3. no safe sibling entity/field in that batch can progress around it;
4. the needed evidence cannot be gathered inside the same writer/custody/execution surface;
5. the task has a direct consumer population batch and terminal condition.

The analysis output must name the product delta it unlocks.

Do not create a generic "next census", "next crosswalk", "next revalidation" or "next evidence" generation without this linkage.

## G3 UNKNOWN closure consumption

The historical G3 UNKNOWN cohort is not a reason to stop population.

Consume the protected closure dispositions:

- definition-family eligible rows enter the relevant family population batch;
- relationship-only rows feed relationship batches;
- source-only rows stay evidence;
- placement-only rows wait for placement reconciliation;
- genuinely unresolved rows remain bounded blockers.

Do not force every historical UNKNOWN row into a reusable definition.

## Full-world placement phase

Definition and relationship population may run before final full-world placement layout selection.

When placement reconciliation begins:

1. consume the protected full-cardinality measurement;
2. select the minimum-sufficient bounded physical layout/chunking compatible with existing v2 semantics;
3. do not use one monolithic 24.5M-placement in-memory document if the measured path fails;
4. preserve stable definition identity independently of occurrence/placement identity;
5. qualify real consumers for load/edit/runtime use;
6. propose a schema-major change only if measured evidence proves v2 semantics cannot support the required layout.

## Hard exclusions

Keep completely outside crawl, catalogue population and placeholders:

- `Kalkulatory`;
- `Narzędzie do nasycania` / `Imbuement Tool`;
- `Dostawca` / reseller utility surfaces.

## Subagent orchestration

When the owner asks to use Luna/Sol subagents, the canonical Work coordinator remains the sole coordinator/integrator.

Parallelize path-disjoint work:

- **low effort** — mechanical source-row grouping, metadata/tag normalization, exact lifecycle/readback;
- **medium effort** — ordinary family identity resolution and homogeneous population batches;
- **high effort** — ambiguous identity conflicts, difficult relationships, representation-gap proof;
- use read-only subagents freely for preparation;
- one mutating writer per owned path/branch;
- aggregate results into product batches rather than creating one PR per subagent.

Do not stop with "#162 must assign" when the current canonical coordinator already has valid authority to allocate the bounded work.

## Execution lifecycle

Authorized mutation uses:

`AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ`

Before every API-native write, fresh-read the branch head. After final authoring write, freeze the exact returned SHA, verify the complete bounded diff and owned paths, and do not mutate the frozen head. A repair returns explicitly to AUTHORING and creates a successor candidate requiring fresh qualification.

Use governed Merge Queue only under the current bound META routing. Queue admission is non-terminal; completion requires real `merge_group` aggregate `game-gate` success and protected-main readback.

No direct merge, generic auto-merge, force/reset/rebase, no-op retrigger or protection weakening.

## Population scoreboard

Owner-facing status should lead with cumulative product counts:

- canonical identities matched;
- canonical identities created;
- source bindings written;
- fields promoted;
- relationships written;
- editor aliases/tags written;
- presentations/assets bound;
- placements reconciled;
- unresolved conflicts/ambiguities;
- exact current blocker;
- next mutating batch.

Evidence/census counts are secondary unless they materially change one of those product numbers.

## Completion

The programme is complete only when:

- all in-scope source entities have a final product disposition;
- eligible missing canonical identities are created;
- existing identities are matched without duplicate reminting;
- exact promotable fields are populated or explicitly blocked/deferred;
- required typed relationships are populated;
- source identities remain recoverable;
- catalogue aliases/tags are present where useful;
- required presentation/asset bindings are populated;
- required world placements are reconciled using a measured viable physical layout;
- compile/load/runtime consumers are qualified for the resulting data;
- exclusions remain absent;
- final coverage reports canonical product counts, not only source counts;
- required CI/review/Merge Queue/protected-main closeout is complete.

## Terminal behaviour

Continue autonomously through the current population batch and the next legal path-disjoint population work while authority/capability permits.

Stop only for a real boundary:

- no valid control-plane/write authority;
- path/custody conflict;
- required validation/integration capability unavailable;
- material architecture decision;
- an exact source fact blocks the bounded write and no sibling population can proceed.

When blocked, preserve the valid candidate and state the smallest action that releases **product population**, not a generic request for more research.
