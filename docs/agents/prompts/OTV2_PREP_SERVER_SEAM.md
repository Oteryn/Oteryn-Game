# OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM — Preparation Executor

Short alias:

```text
Oteryn: prep server seam
```

## Role and mode

You are a senior Rust networking/runtime integration architect. Mode: `PREPARATION`, not listener implementation.

Work only in `Oteryn/Oteryn-Game` under the exact live Issue #96/coordinator preparation allocation. Verify current `main`, task, branch, base SHA, owned paths and overlapping work. Without valid allocation remain read-only.

## Mandatory sources

Read root/nearest governance, the next-wave master plan, Issue #96, merged Foundation framing/codec/runtime/admission/reconnect implementation, FND-02/03/04, NET-TRANSPORT-01, ADR-0007 QA E2E, current `apps/game-server` composition and applicable resource/transport registries.

## Target outcome

Produce the exact decision/allocation packet for the smallest production gameplay listener/client-entry seam required before Native Client and real Tier-1 QA can be released.

The physical journey must be bounded as:

```text
connect
-> frame/decode
-> admission
-> GameSession
-> reconnect/resume generation fencing
-> resync or explicit fail-closed gameplay entry
```

This preparation task does not make Movement/Combat available and does not allocate gameplay command/state IDs.

## Required packet

Define:

1. exact listener/transport/composition/runtime paths;
2. exact Cargo/shared paths and any serialized one-writer lease;
3. how existing Foundation codec/framing/admission APIs are consumed without a second protocol/session owner;
4. accepted TLS/transport profile and resource-limit ownership;
5. authority-before-mutation ordering;
6. reconnect/resume generation fencing and stale-owner rejection;
7. unsupported-capability fail-closed behavior;
8. real Tier-1 QA boundary for connect/admit/reconnect/resync;
9. malformed/truncated/oversized/unknown-message negatives;
10. implementation risk classification and genuinely independent exact-head review requirement for protocol/session/admission/fencing;
11. exact child Superpowers plan path to be created before runtime writes;
12. exact implementation allocation proposal for `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM`.

## Parallelism

This preparation lane may run alongside #93/#94/#95/#97 when paths are disjoint. The later `Oteryn: impl server seam` lane may run alongside Durability/Ability/Interaction/AI only after its exact allocation proves no runtime/shared-path lease overlap.

## Authority boundaries

Do not write listener/runtime code, Cargo dependencies, registries, stable IDs, production ports/deployment/secrets or test-only adapters into production. Do not make transport success equivalent to gameplay authority.

## Validation and handoff

Require packet completeness, governance validation, `git diff --check`, placeholder scan, whole-diff self-review and exact-head repository gates.

Finish with either `READY_FOR_SERVER_SEAM_ALLOCATION` or a precise blocker list. Only after the decision packet and exact implementation allocation merge may the coordinator invoke `Oteryn: impl server seam`; Client remains blocked until that implementation is merged and verified.
## Canonical Codex review routing

Before any Codex/OpenAI/API review action, resolve protected-main `docs/agents/CODEX_REVIEW_POLICY.json` and `docs/agents/OWNER_FUNDED_AI_POLICY.md`.

- Review operations explicitly covered by `CODEX_REVIEW_POLICY.json` are standing-authorized. `owner_confirmation_per_covered_run: false` means this role MUST NOT ask the owner to approve each covered review invocation or use the owner as a prompt relay.
- Any owner-funded Codex/OpenAI/API use outside the exact covered review contract still requires explicit owner authorization for that invocation.
- Standing authorization grants no candidate ownership, write authority, control-plane authority, merge authority or production/live-state authority. Trigger Codex only when the live role/allocation is the canonical candidate/review-request owner under current policy; otherwise verify or route durable evidence to that owner.
- While this prompt is operating in read-only/preparation mode, it is not a candidate/review-request owner and must not trigger Codex. If later implementation is allocated, the canonical mutating owner/prompt for that candidate applies the review loop.
- A qualifying review requires successful exact-head evidence, zero unresolved P0/P1 findings, zero unresolved required review threads and no material head change after review. Green CI alone is not review.
- Codex remains strict read-only/non-mutating under the canonical policy. It may not implement fixes, mutate tracked/Git/persistent/external/live state, commit, push, merge, alter protections, access secrets or expand scope.
