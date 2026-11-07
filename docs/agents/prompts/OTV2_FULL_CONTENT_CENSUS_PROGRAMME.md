# OTV2 Full Content Population Programme

Canonical invocation:

```text
Oteryn: full content population
```

Compatibility invocation:

```text
Oteryn: full content census
```

The historical lifecycle ID and file path are retained for continuity. Once the protected census/classification/provenance prerequisites are on live protected `main`, both invocations resolve to the **population-first** behaviour below.

## Profile and dispatch semantics

`OTV2_FULL_CONTENT_CENSUS_PROGRAMME` remains a **scoped dispatch alias for the canonical `OTV2_WORK_DELIVERY_COORDINATOR`**, not a second programme lead or mutating control plane. It grants no authority beyond that coordinator.

On invocation: fresh-read protected `main`; resolve the current `OTV2_WORK_DELIVERY_COORDINATOR` prompt and live control-plane allocation; use the same profile identity, authority, custody, review and integration route as `Oteryn: work coordinator`; apply this file only as the Full Content population scope delta.

The alias is not a second active mutating control plane. If the canonical Work coordinator is current and authorized, coordinate this programme directly instead of bouncing routine scheduling back to #162 because the scoped alias was used.

## Current programme mode: POPULATION_FIRST

Treat the protected G0-G3 results, protected Item census, G4 source provenance, typed source-identity carrier and full-cardinality measurement as inputs to implementation. Do not restart them or open another analysis generation unless a concrete unresolved fact blocks a real canonical write in the current bounded batch.

Objective: turn already-discovered source entities into real Oteryn canonical identities, definitions, fields, relationships, tags, presentation bindings and later placements, using the existing WorldProject/v2 model and compile/test paths. Success is product population, not the number of new census/evidence artifacts.

## Population-first invariants

1. **No new census/crosswalk/revalidation-only slice by default.** An analysis-only task is allowed only under the exception below.
2. **A G4 family batch with at least one eligible record must produce a real canonical product delta.** Do not end it with only `MISSING_CANONICAL_IDENTITY`, `UNKNOWN` or another report while a safe canonical record or field can be written.
3. **Partial canonical records are valid.** Identity completeness and semantic completeness are separate; exact fields are promoted while unrelated fields stay `UNKNOWN` or `CONFLICT`.
4. **Second-source corroboration is helpful, not a universal admission gate.** If TibiaWiki BR gives an exact source identity, supported family shape and non-conflicting structured facts sufficient for the bounded claim, a missing second source does not block population.
5. **No title-only matching to take over an existing identity.** Existing identities need the strongest available multi-signal match. A genuinely missing non-Item identity may get a new key under existing conventions after exact source identity/family proof and duplicate/collision checks.
6. **External IDs never become canonical Oteryn identity.** Preserve exact source key/revision/digest, namespace and verbatim external ID through `ProjectV2SourceIdentityBinding`.
7. **Unknown evidence stays unknown without blocking siblings.**
8. **No new schema/framework.** No `ProjectV3`, `ItemV2`, `CreatureV2`, generic registry service or importer platform unless an exact current record proves the protected WorldProject/v2 model cannot represent a required product fact.
9. **Evidence is an output of product work, not a successor trigger.** Keep one batch end-to-end while the same writer/custody/execution surface can legally carry it.
10. **Real content over process closure.** Lifecycle/archive cleanup must not take the critical path while a path-disjoint population batch can continue.

## Protected foundation to reuse

Fresh-read current protected state rather than trusting historical SHAs. Preserve these integrated results unless a protected successor supersedes them: G0 storage/family audit; G1 source discovery; G2 source overlap/deduplication; G3 family classification with explicit disposition of unresolved rows; protected wiki-first Item source lane; WorldProject/v2 family model; typed `ProjectV2SourceIdentityBinding`; wiki-first G4 source policy; existing Item identity/model/artifact lineage; full-cardinality WorldProject/v2 measurement.

Do not reopen a predecessor because later source observations drift; revalidate only the exact rows the current batch needs.

## Canonical storage contract

Use the existing logical storage model; do not create cosmetic per-family filesystem trees.

- `definitions/reference.json` — accepted executable/reference definitions owned by executable Reference families.
- `definitions/declarations.json` — typed declarative definitions and source-only authoring for families not executable through the Reference linker.
- `provenance/sources.json` — exact source records and accepted source-identity bindings; `provenance/imports.json` — import/candidate provenance where the pipeline requires it.
- `editor/author.json` — display names, aliases and **namespaced editor/catalogue tags**. Tags help browse, query and author; they never replace typed family fields or create gameplay semantics.
- `presentations/bindings.json` — Definition/Presentation binding; `assets/catalog.json` — immutable asset identity/digest catalogue. Keep `Definition -> Presentation -> Asset` separate from legacy/client/source IDs.
- `worlds/world.json` remains the semantic placement role. The full-cardinality measurement proved the semantics only through bounded sequential shards (the monolithic corpus hit an allocation limit), so do not block definition population on the final placement layout and do not invent ProjectV3. The minimum-sufficient placement layout/chunking decision belongs to the placement phase.

## Disposition of each source entity

Every source entity in a bounded family batch ends in one of:

- `MATCH` — bound to an existing canonical identity;
- `CREATE` — missing canonical identity/definition created under existing key conventions;
- `CONFLICT` — competing evidence prevents the exact bounded write;
- `AMBIGUOUS` — more than one credible target remains;
- `SOURCE_ONLY` — evidence retained, no reusable canonical entity;
- `RELATIONSHIP_ONLY` — contributes a relation to existing/created definitions;
- `PLACEMENT_ONLY` — contributes a world occurrence, not a definition;
- `EXCLUDED` — explicit programme exclusion.

`MISSING_CANONICAL_IDENTITY` is not a terminal success state. Convert it to `CREATE` whenever current evidence safely proves a new identity and the model can represent it.

## Family batch execution

Default flow: `select batch -> resolve/create identity -> write source binding -> write editor metadata/tags -> promote exact fields -> write exact relationships -> compile/test -> integrate`.

Split these substeps into separate tasks/PRs only for a real path-custody boundary, a distinct execution surface, a material architecture decision or an independently mandatory gate.

Batch size: start with 100-500 ordinary entities for homogeneous declarative families, smaller for high-relationship or executable families, and adjust from measured output/CI/review cost. Do not spend a task choosing the size; run a safe first batch immediately.

A completed mutating batch with eligible records reports: selected, matched, created, source_bindings_written, editor_entries_written, tags_written, fields_promoted, relationships_written, presentation_bindings_written, conflict, ambiguous, source_only, placement_only, blocked, compiled, tested, integrated. Canonical population, not evidence row count, is the progress vector.

### Identity rules

- Existing identities: use exact IDs/source bindings/aliases/structured fields/relationships and other strong signals; never remap on name similarity alone.
- New non-Item identity, only when all hold: exact source entity identity retained; family sufficiently established; no canonical key/source binding/alias collision resolves elsewhere; key follows the family convention; record carries source provenance; no stronger conflicting evidence. Do not wait for every gameplay field.
- Items: reuse the protected Item identity authority and Item model/artifact lineage; do not mint parallel Item identities because a wiki page is not yet resolved to it. Use `resolve -> verify -> continuity-if-needed -> promote -> compile/test`; partial field promotion is expected.

## Field promotion

Promote atomic facts independently, e.g. Creature identity + name + exact health with loot unknown; NPC identity + presentation/source binding with dialogue unknown; Quest identity + known prerequisite/reward relation with some steps unknown; Item identity + exact weight/armor with another stat in conflict. Do not require a complete entity. If an exact field does not fit the model, first prove the representation gap against the protected v2 model, then repair only that gap minimally.

## Tags, aliases and relationships

Write catalogue metadata in `editor/author.json` during population, not in a later cleanup. Aliases are non-authoritative lookup hints; tags are namespaced editor metadata and never gameplay authority; use a small stable vocabulary and reuse existing tags before minting synonyms; family is typed data and source/provider identity is provenance, neither only a tag. Do not create a separate tagging framework.

Write typed relationships as soon as both endpoints are safely identified/created (Creature -> Loot -> Item; NPC -> Dialogue/Service -> Item; Quest -> prerequisite/NPC/Interaction/rewards/Encounter; Item -> Ability/Interaction; WorldObject -> Interaction/Transition/Document; Area -> parent; Definition -> Presentation). Never embed a relationship as a freeform tag when a typed v2 relation exists.

## Source policy

- **TibiaWiki BR is the primary bulk working source** for current static/semi-static identities and structured fields.
- A maintained, genuinely independent second structured source is used when available; its absence, HTTP denial or licensing-preflight failure does not block an otherwise safe identity/field. Cache the blocker fingerprint and do not re-probe an unchanged access failure.
- Public CipSoft/Tibia evidence resolves exact atomic conflicts. Authenticated Global Tibia/Cyclopedia browsing is a future verification layer, not a denominator or normal G4 blocker.
- Canary/Crystal/other OTS donors inform mechanics/map/quest/spawn hypotheses and missing detail, not automatic current static truth: `CURRENT STRUCTURED REFERENCE DATA > DONOR IMPLEMENTATION` for the same static field unless stronger direct evidence proves otherwise.

## Analysis-only exception

Allowed only when all hold: a named current population batch is blocked; the blocker is exact and bounded; no safe sibling entity/field can progress around it; the evidence cannot be gathered inside the same writer/custody/execution surface; the task has a direct consumer batch and terminal condition. The output names the product delta it unlocks. No generic "next census", "next crosswalk", "next revalidation" or "next evidence" generation without this linkage.

## G3 UNKNOWN closure consumption

The historical G3 UNKNOWN cohort is not a reason to stop. Consume the protected closure dispositions: definition-family eligible rows enter the family population batch; relationship-only rows feed relationship batches; source-only rows stay evidence; placement-only rows wait for placement reconciliation; genuinely unresolved rows remain bounded blockers. Do not force every UNKNOWN row into a reusable definition.

## Full-world placement phase

Definition and relationship population may run before the final placement layout is chosen. When placement reconciliation begins: consume the full-cardinality measurement; select the minimum-sufficient bounded physical layout/chunking compatible with v2 semantics (no monolithic 24.5M-placement in-memory document if the measured path fails); keep definition identity independent of placement identity; qualify real consumers for load/edit/runtime; propose a schema-major change only if measurement proves v2 cannot support the layout.

## Hard exclusions

Keep out of crawl, catalogue population and placeholders: `Kalkulatory`; `Narzędzie do nasycania` / `Imbuement Tool`; `Dostawca` / reseller utility surfaces.

## Subagent orchestration

When the owner asks for Luna/Sol subagents, the canonical Work coordinator stays the sole coordinator/integrator. Parallelize path-disjoint work: low effort for mechanical source-row grouping, metadata/tag normalization and lifecycle readback; medium for ordinary family identity resolution and homogeneous batches; high for ambiguous identity conflicts, difficult relationships and representation-gap proof. Read-only subagents are free for preparation; one mutating writer per owned path/branch; aggregate results into product batches, not one PR per subagent.

Do not stop with "#162 must assign" when the current canonical coordinator already has valid authority to allocate the bounded work.

## Execution lifecycle

Authorized mutation follows `AUTHORING -> FREEZE_SHA -> VALIDATE -> MQ` and the merge route in root `AGENTS.md` and the bound META policy; this file adds nothing to them. Queue admission is non-terminal: completion needs real `merge_group` aggregate `game-gate` success and protected-main readback.

## Population scoreboard

Lead owner-facing status with cumulative product counts: canonical identities matched and created; source bindings written; fields promoted; relationships written; editor aliases/tags written; presentations/assets bound; placements reconciled; unresolved conflicts/ambiguities; exact current blocker; next mutating batch. Evidence/census counts are secondary unless they change one of those numbers.

## Completion

Complete only when: all in-scope source entities have a final product disposition; eligible missing identities are created and existing ones matched without duplicate reminting; exact promotable fields are populated or explicitly blocked/deferred; required typed relationships, presentation/asset bindings and catalogue aliases/tags are populated; source identities remain recoverable; required world placements are reconciled on a measured viable layout; compile/load/runtime consumers are qualified; exclusions remain absent; final coverage reports canonical product counts, not only source counts; required CI/review/Merge Queue/protected-main closeout is done.

## Terminal behaviour

Continue autonomously through the current batch and the next legal path-disjoint population work while authority/capability permits. Stop only for a real boundary: no valid control-plane/write authority; path/custody conflict; required validation/integration capability unavailable; material architecture decision; or an exact source fact that blocks the bounded write with no sibling able to proceed. When blocked, preserve the valid candidate and state the smallest action that releases **product population**, not a generic request for more research.
