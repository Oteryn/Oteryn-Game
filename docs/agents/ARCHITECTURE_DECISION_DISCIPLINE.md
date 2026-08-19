# Oteryn v2 Architecture Decision Discipline

- Status: Owner-accepted architecture-working policy
- Date: 2026-08-07
- Applies to: architecture analysis, ADRs, contracts, decision backlogs, continuation agents and architecture PR review in `Oteryn/Oteryn-Game`
- Does not authorize: runtime implementation, production changes or external-repository writes

## Purpose

Keep Oteryn-v2 architecture rigorous without allowing the architecture programme itself to become the product. The project should freeze a decision when downstream work genuinely requires it, preserve safe extension points for later systems, and reach evidence-producing vertical slices as early as safety permits.

## Mandatory decision test

Every material architecture proposal must answer these questions before acceptance:

1. **Must decide now?** `YES` or `NO`.
2. **What concrete downstream work is blocked?** Name the gate, contract, implementation package or product proof that cannot proceed safely without this decision.
3. **What becomes harder or impossible later?** Name irreversible coupling, migration cost, compatibility burden or operational constraint created by the decision.
4. **What evidence would justify superseding it?** Name measurements, product evidence, failure evidence, security findings or changed requirements that would make reopening the decision rational.
5. **What is deliberately not decided?** Preserve explicit extension points and avoid accidental scope capture.

If `Must decide now? = NO`, register the subject in the appropriate backlog/horizon and do not freeze technology, schema, topology or formula merely for completeness.

## Required analysis shape

For an important decision provide:

- **Problem** — the exact problem being solved.
- **Constraints** — accepted invariants and external limits.
- **Options** — only realistic alternatives.
- **Trade-offs** — material benefits and costs.
- **Risks** — technical, security, gameplay, player-experience and operational risks.
- **Recommendation** — the preferred option and why.
- **Future impact** — migration, compatibility and extension consequences.
- **Decision timing** — the mandatory decision test above.

Do not manufacture many variants when two real choices exist.

## Vertical-slice bias

The default policy is:

```text
freeze what blocks the next safe proof
register what matters later
measure the real system
then refine
```

A distant expansion system must not block the first native vertical slice unless its omission would force an unsafe public contract, corrupt durable state, invalidate security boundaries or create an expensive irreversible migration.

Broad architecture discovery may continue in parallel, but it must not silently become implementation authority.

## Technology discipline

Technology is selected only for a named Oteryn problem. Prefer explicit contracts, strong typing, bounded inputs, schema validation, idempotency, ownership, fault isolation, deterministic tests and reproducible evidence.

Do not select a framework, broker, datastore, serializer, orchestrator or scripting runtime because it is fashionable or because another OTS uses it. Where a choice is benchmark-sensitive, keep the invariant and defer the product/library choice to a bounded spike with acceptance criteria.

## Player and producer review

Every major architecture decision must consider both:

- player-visible latency, responsiveness, fairness, reconnect/recovery, exploitability, progress safety and feature flexibility;
- implementation cost, maintenance cost, rollout/rollback, migration cost, operational burden and time-to-first-evidence.

A technically elegant design that materially delays product proof without protecting an important invariant requires explicit justification.

## Evidence and supersession

Accepted decisions are not immutable dogma. Supersession requires named evidence and an explicit newer ADR/contract that states exactly what changes and what remains binding.

Historical ADRs and evidence stay preserved. Progress/status wording may be superseded by a later current-status source without rewriting historical evidence.

## Protocol and E2E guardrails

Protocol contracts must use independent evidence strong enough to detect common-mode client/server bugs. As applicable, require:

- canonical byte-level golden fixtures;
- malformed and adversarial fixture corpora;
- property-based parser/encoder tests;
- fuzzing of externally controlled parsers and decoders;
- cross-version fixture validation;
- explicit resource ceilings and failure categories.

Sharing the production schema/codec between a headless test client and the native client is useful but cannot be the only oracle for wire correctness.

## Event-schema guardrail

Game Intelligence uses a shared semantic event foundation, not one unbounded nullable mega-event.

`ANL-01` should prefer:

```text
small common envelope
+ strongly typed/versioned event-family payload
```

Required common correlation and durability fields belong in the envelope. Domain-specific fields belong in typed payload contracts with family-specific required/optional rules.

## PR safety

Architecture review classifies open PRs as `KEEP`, `FIX`, `REBASE`, `SUPERSEDED`, `CLOSE` or `NEEDS_DECISION`.

Do not close a repairable PR merely because it is old or CI is red. Automatic close is appropriate only with concrete evidence of duplicate, obsolete or superseded work and only when repository/owner authority permits it.

A failed required gate means `FIX`/`BLOCKED`, not "merge anyway".
