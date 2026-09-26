# Work Package

## Outcome

Produce one executable, rollback-safe Oteryn agent orchestration canary that compiles from source, validates its OpenSpec contract and can route one read-only diagnostic worker without altering existing merge authority.

## Inputs

- Issue `#591`.
- Candidate branch `agent/otv2-agentic-openspec-pilot-01`.
- gh-aw `v0.88.7`.
- OpenSpec `1.13.0`.
- Existing Oteryn Game governance and exact-head checks.

## Owned scope

- Project-local OpenSpec config/schema/change for this pilot.
- Two gh-aw source workflows and their compiler-generated lock files.
- One deterministic qualification workflow.
- Pilot task record and generated-workflow gitattribute.

## Excluded scope

No runtime gameplay change, production mutation, existing workflow weakening, protection/ruleset mutation, required-check change, secret creation, external-repository write or autonomous merge/integration.

## Validation

1. `openspec schema validate oteryn-agent-flow` using pinned OpenSpec.
2. `openspec validate agentic-orchestration-pilot --strict --no-interactive`.
3. Compile both gh-aw sources with pinned gh-aw and validate compiler output.
4. Existing repository governance/CI on the exact PR head.
5. One runtime canary: router dispatches the allowlisted worker and worker produces only staged handoff output; otherwise record the exact unavailable capability.

## Handoff

After deterministic validation and behavior evidence are stable, hand the exact candidate to one independent deep control-plane review. Do not integrate from this work package.
