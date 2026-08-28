# Oteryn v2 Architecture Review Refinements — 2026-08-07

- Status: Owner-accepted review reconciliation
- Date: 2026-08-07
- Base evidence: `blakinio/Oteryn-v2@10392eb89d11de2ea260c82587b4b1ef22ddd7e6`
- Applies to: interpretation of current architecture status and future architecture packages
- Does not authorize: runtime implementation, production changes or external-repository writes

## Purpose

Record the owner-approved results of the whole-foundation architecture review without rewriting historical evidence or reopening sound accepted decisions.

This document is a reconciliation/working overlay. Dedicated ADRs remain semantic authority for their domains. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` remains authority for live gate progression.

## Review verdict

The accepted Oteryn-v2 foundation is retained. No whole-foundation redesign is required.

The strongest accepted directions remain:

- native Rust client/server and one independent `protocol-oteryn`;
- multichannel-first worlds with one logical authoritative writer per channel;
- explicit typed identity, generation and fencing boundaries;
- Platform control-plane separation from gameplay authority;
- native world/content model and bounded legacy conversion;
- PostgreSQL for native game persistence with explicit ownership boundaries;
- Game Intelligence separation of observability, telemetry and durable audit;
- three-tier E2E evidence;
- no production Canary fallback/translation;
- measured capacity and recovery instead of guessed limits;
- reference/evolved worlds over one engine/client/protocol;
- fail-closed pre-native-protocol client transition.

## Accepted refinement 1 — Platform database migration is conditional

ADR-0013 supersedes only the ADR-0004 requirement that Oteryn Platform must eventually migrate to PostgreSQL.

PostgreSQL remains accepted for native game persistence. Platform database technology becomes independently owned and requires a separate evidence-backed Platform decision before migration.

## Accepted refinement 2 — current foundation status

As of the reviewed base:

- FND-01 and VSL-02 are complete;
- FND-ID-01 is accepted and merged;
- issue #86 is already closed/completed;
- `FND-02 — protocol-oteryn v1` is the next ordered foundation architecture gate;
- FND-03 and FND-04 remain separate later foundation gates;
- no native protocol/runtime/admission implementation is authorized by this reconciliation.

Any older progress-only sentence saying "complete #86 first" or "FND-ID-01 is next" is historical/stale progression text. It must not override `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` or later exact merged evidence.

Do not mass-rewrite immutable historical task/evidence records solely to modernize old timestamps. Current coordination documents should absorb this reconciliation when next edited for their owning purpose.

## Accepted refinement 3 — analytics identifier ownership

The merged `FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md` is authoritative over older broad candidate catalogues.

Foundation identifiers do not retroactively absorb the complete analytics/durability identity space. In particular:

- `CommandId` belongs to FND-02;
- runtime-local handles belong to FND-03;
- admission/session/lease mechanics belong to FND-04;
- event/operation/transaction/correlation/causation/pseudonymous analytics identities belong to `ANL-*`/`DUR-*` as appropriate.

Older ADR-0006 wording that appears to assign the full later event/audit identity catalogue to FND-ID is interpreted as historical pre-final-contract planning, not a command to reopen FND-ID.

## Accepted refinement 4 — event schema shape

The ADR-0006 common event foundation should not become one giant nullable structure.

`ANL-01` should prefer:

```text
minimal versioned common envelope
+ strongly typed/versioned event-family payloads
```

The envelope carries cross-cutting identity/correlation/version/durability/privacy information required across families. Family-specific semantic fields stay in typed payload contracts with explicit required/optional rules.

This refinement changes no datastore, broker or serialization choice.

## Accepted refinement 5 — independent protocol evidence

The shared Tier-1 headless client may reuse production protocol schemas/codecs, but shared code cannot be the only proof of wire correctness.

`FND-02` and later QA implementation should include independent evidence appropriate to the chosen encoding:

- canonical byte-level golden fixtures;
- malformed/adversarial fixture corpus;
- property-based round-trip and invariant tests;
- fuzzing for externally controlled decoders/parsers;
- cross-version compatibility fixtures;
- explicit resource limits and stable failure classes.

This does not require a second production protocol implementation.

## Accepted refinement 6 — architecture timing and vertical-slice discipline

Every material decision must answer:

```text
Must decide now? YES/NO
What concrete downstream work is blocked?
What becomes harder or impossible later?
What evidence would justify superseding the decision?
What remains deliberately undecided?
```

The programme should decide what blocks the next safe proof and register distant systems without freezing them prematurely.

The current intended progression remains toward a minimal real native vertical slice after the required foundation/durability contracts, rather than completing every MMO expansion architecture before gameplay proof exists.

## Product analysis in parallel

`GAME-VISION-01` should continue in parallel with FND-02 when it does not redefine foundation protocol/identity/runtime/persistence boundaries.

It should eventually make parity measurable by naming the reference baseline, first-launch profile strategy, intentional differences and explicit product pillars/anti-pillars before broad gameplay/content production.

## Open-PR hygiene observed during this review

At the time this package began, PR #91 was a task-closeout PR with a correct narrow two-path archive scope and a mergeable branch, but its required `Agent governance` workflow failed in the PR metadata-validation phase while Dependency Review and CodeQL passed.

Classification: `FIX`.

It must not be merged until its own required governance gate succeeds. It is not superseded or obsolete and should not be closed merely because that check is red.
