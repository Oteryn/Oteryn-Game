> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #686 is merged on protected main as `cb7ade54d5622cb75a6a059e6c8517eca4a2a45b`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw3-ability-effect-formula-project-types-504

```yaml
task_id: OTV2-20260919-content-world-cw3-ability-effect-formula-project-types-504
title: CW3 Ability to Effect to Formula structural Reference closure
mode: BUILD
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw3-ability-effect-formula-project-types-504
issue: 162
pr: null
base_sha: 0ac1093817064eee4e50a54be3bf0ddfc90c7caf
head_sha: null
owner: Oteryn: content world build
created_at: 2026-09-19T22:51:35Z
updated_at: 2026-09-19T22:56:40Z
owned_paths:
  - apps/game-server/src/content/reference_playable.rs
  - apps/game-server/src/content/project.rs
  - apps/game-server/tests/content_reference_playable.rs
  - apps/game-server/tests/content_world_project.rs
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw3-ability-effect-formula-project-types-504.md
public_contracts: []
production_authority: NONE
```

## Outcome

Extend the existing Reference definition graph and canonical project snapshot with
one structural `Ability -> Effect -> Formula` closure. Ability records retain an
authored ordered list of exact typed Effect references, including repeated
references. Effect records retain their existing Damage/Heal family and add one
exact typed Formula reference. Formula is an opaque authored identity and
reference endpoint.

The existing `link_reference_playable` graph validates both reference edges.
This change introduces no second model, linker, parser or artifact carrier.

## Authority boundaries

- Ability and Formula definitions are server-only. Formula has no expression,
  arithmetic, RNG, rounding, target, timing, cost or cooldown semantics.
- A client-safe Effect projects only its existing Damage/Heal family. The
  Formula reference and Formula/Ability definitions do not enter client
  authority.
- Ability effect-list order and repeats are preserved as authored structure.
  They do not establish execution order, multi-hit behavior or composition
  policy.
- Generic Ability, Effect and Formula definitions fail closed; each family must
  use its typed shape.
- Proof records use independent project-owned keys. Protected CW2-B4 candidate
  source identifiers remain import evidence with `UNKNOWN`/`PENDING`/`BLOCKED`
  dispositions and zero native bindings.

## Project representation

The strict project record schema adds typed Ability, Effect and Formula variants.
Ability and Formula omit a client-projection field and lower as server-only.
Formula accepts only its identity, so an expression or arbitrary payload is an
unknown-field error. Project metadata remains separate and cannot redirect the
typed edges.

The accepted snapshot's strict duplicate-member, unknown-field, locator,
digest, Content Lock, pre-allocation resource and reimport checks remain in
place. Filesystem capture and atomic publication remain deferred.

## Evidence limits

The original accepted six-document/four-record proof corpus and all of its exact
resource-boundary tests remain unchanged. The additional structural proof corpus
also has six documents and nine selected Reference records. Its largest document
is 2,429 bytes and its aggregate size is 6,842 bytes. The focused test derives
these dimensions from canonical emitted documents, accepts equality and rejects
one byte or one record beyond the measured boundary. These remain non-production evidence
limits, not full-world maxima.

## Acceptance criteria

- [x] Only the five allocated paths are changed.
- [x] Ability retains ordered, repeated exact Effect references through
  definition permutation and canonical project round trip.
- [x] Effect retains its family and exact Formula reference; Formula is an
  opaque typed endpoint.
- [x] Wrong-family, missing and stale revisions fail on both reference edges.
- [x] Generic Ability/Effect/Formula bypasses fail.
- [x] Ability and Formula cannot gain client authority; client-safe Effect
  projection contains only the Effect family.
- [x] Formula unknown payload fields fail strict decoding.
- [x] Metadata changes cannot select or redirect either structural edge.
- [x] Protected B4 candidates create no native definitions or bindings.
- [ ] Published immutable head passes focused tests and the hosted exact-head
  Rust and repository gates.

## Validation

Passed in the allocated checkout with the isolated official Rust 1.94 toolchain:

- focused locked `content_reference_playable` tests: 31 passed;
- focused locked `content_world_project` tests: 19 passed;
- strict `oteryn-game-server` all-target Clippy;
- workspace formatter check;
- governance and repository-policy validators;
- diff check and exact five-path custody inspection.

The published immutable head must also pass the hosted locked workspace build,
strict workspace Clippy, workspace tests and aggregate game-gate.

## Deferred claims

This structural closure does not define Formula evaluation or any gameplay
execution semantics. It does not promote the protected B4 candidates, prove a
real imported family playable, establish Reference parity, complete a compiled
artifact journey or authorize B5/B6. It does not implement filesystem capture
or atomic project publication.

## Handoff

The parent control plane owns independent review and governed integration. This
worker does not merge or trigger external review.
