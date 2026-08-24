# Prompting standard

A repository execution prompt must be self-contained, bounded and verifiable.

## Required sections

1. Role and exact task mode (`IMPLEMENT`, `AUDIT`, `CONTRACT`, `REPAIR`, `COORDINATE`).
2. Authorized repositories and explicit write allowlist.
3. Exact target outcome and observable acceptance.
4. Trusted source order and mandatory files/contracts.
5. Current known baseline labeled `PROVEN/DERIVED/UNKNOWN/CONFLICT`.
6. Owned paths/public contracts and excluded scope.
7. Dependencies, parallelism and cross-repository ordering.
8. Required implementation layers.
9. Validation ladder, audit, E2E and exact-head merge gate.
10. Stop conditions, budget and durable handover requirements.
11. Completion rule preventing plan-only or partial delivery.

## Oteryn v2 prompt invariants

Prompts must preserve native Rust, `protocol-oteryn` only, multichannel identities/ownership, server authority, session fencing and separate external repository authority unless an owner-approved ADR/task explicitly changes them.

Do not instruct an agent to infer missing bytes, schemas, assets, credentials, deployment state or external implementation. Require bounded discovery or record the exact blocker.

## Worker prompts

Worker prompts have exclusive paths/contracts, precise inputs/outputs, no coordinator authority, clear integration-ready state and focused acceptance. Do not assign overlapping public contract ownership to parallel workers.

## Audit prompts

Auditors are read-only unless repair is separately authorized. They must challenge completeness, inspect exact source/live state and classify every finding with evidence and severity.

## Retained prompt lifecycle

Every retained execution prompt under `docs/agents/prompts/*.md`, except the directory `README.md`, must have exactly one entry in `docs/agents/PROMPT_LIFECYCLE.json`.

The lifecycle registry supplies dispatch metadata that older prompt bodies do not consistently carry inline: stable `prompt_id`, lifecycle-registry `version`, `status`, lifecycle owner, bounded scope and supersession semantics. Registry version `1.0` means first registration in that lifecycle registry; it does not retroactively claim that an older prompt document historically used that version.

`reusable` means only that the prompt may be resolved from current `main`. It does not grant a branch, owned paths or write authority; the prompt's live allocation/owner prerequisites still apply. `retired` means provenance-only: the file may remain in Git history or the prompt directory, but it must not be dispatched as executable work and must name its successor in the lifecycle registry.

A prompt is superseded only by an explicit registry entry. A completed task that previously used a reusable executor does not by itself retire the executor; a new invocation must still satisfy all current live-allocation and governance gates.
