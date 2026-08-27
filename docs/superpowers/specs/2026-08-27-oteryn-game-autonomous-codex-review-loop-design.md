# Oteryn Game Autonomous Codex Review Loop Design

## Status

Owner-approved governance direction for Issue #229. This design removes the owner from routine review message routing while keeping owner-only product/authority decisions and all existing repository safety gates intact.

Admission: protected `main@4b6656f688868aa2fb59c18392c2f859f1c5a1c7`. Live GitHub always outranks this snapshot.

## Goal

Allow an allocated lane lead to own the complete candidate-review-repair-re-review loop:

```text
lane lead freezes exact candidate
  -> lane lead requests native GitHub Codex review when policy requires it
  -> Codex posts durable exact-head findings on the PR
  -> lane lead repairs findings inside the existing allocation
  -> new head invalidates prior review
  -> lane lead requests a fresh independent Codex review
  -> PASS + all other gates
  -> READY_FOR_INTEGRATION
```

The owner must not be required to copy a prompt into a second chat or approve every covered Codex review invocation.

## Authority model

The standing authorization is deliberately narrow. It applies only after this governance package merges to protected `main` and only to operations enumerated by `docs/agents/CODEX_REVIEW_POLICY.json`.

Covered operations are independent read-only exact-head review/audit and non-mutating test/reproduce/fuzz/static-analysis used to support that review. It grants no implementation/fix, tracked repository mutation, commit/push/merge, production/protected-environment, secret, live-data or cross-repository authority.

All other owner-funded Codex/OpenAI/API uses remain deny-by-default and require the existing explicit per-invocation owner authorization.

## GitHub as the review message bus

The canonical review transport is the pull request because it provides durable target identity, exact head, timestamps, findings and re-review history. When the repository has a real supported native Codex GitHub review capability, the lane lead should use that mechanism; the preferred trigger is `@codex review` with bounded risk-specific guidance.

The agent must verify capability before claiming invocation. If the integration is unavailable or not configured, it records the exact capability gap and follows the fallback rule in the machine-readable policy. It must never pretend a Codex task ran.

## Independence

A review counts as independent only when the reviewer task/session did not materially author or materially modify the candidate under review. A Codex task used for implementation cannot become the independent reviewer for the same candidate. A fresh Codex review task is required.

Every material candidate-head change invalidates prior Codex qualification. The new head receives a fresh review when its risk class requires Codex.

## Deterministic risk routing

The uniquely active control plane and the lane lead do not decide ad hoc whether Codex is desirable. They mechanically apply `CODEX_REVIEW_POLICY.json`.

Codex is required by this policy for high-risk classes including durable persistence/schema/migrations, concurrency/races/contention, authentication/session/reconnect/fencing/trust, protocol/wire/stable identity/security, and durable item/value/economy custody.

Codex is optional for complex refactors, broad-code-awareness cases and test/fuzz-intensive changes. Ordinary docs-only and low-risk path-local implementation are not made Codex-dependent by this policy.

## Responsibilities

### Lane lead

The allocated lane lead owns candidate freeze, Codex review request, repair of findings inside allocation, fresh re-review after head movement and final evidence handoff. It does not ask the owner to relay review prompts.

### Codex reviewer

The Codex reviewer is independent and read-only with respect to tracked repository state. It may execute non-mutating validation needed to review the candidate. It posts durable findings/verdict to the PR and does not implement fixes or merge.

### Control plane

Work or Terra, whichever is uniquely active, mechanically verifies risk classification, required review presence, exact head, review independence and PASS status. Terra has zero technical discretion and never adjudicates whether a technical finding is acceptable.

### Work auditor

The Work auditor remains the governance/lifecycle auditor. Codex technical review does not replace an explicitly required separate governance audit.

## Failure handling

If required Codex capability is unavailable, the lane records the exact missing capability. If current repository policy accepts an equivalent qualified independent technical reviewer for that exact gate, that fallback may be used. Otherwise the lane waits fail-closed. Manual owner prompt relay is not the normal fallback.

If Codex leaves findings, the lane remains active and repairs them. The control plane does not bounce the owner between agents.

## Compatibility

This design does not change the single-active-control-plane rule, lane allocations, shared-path serialization, architecture escalation, production authority, runtime architecture or existing independent-review requirements. It specializes how a required technical reviewer is invoked and authorized.

## Success criteria

- root governance contains the standing-authorization exception;
- the owner-funded AI policy distinguishes covered standing review from other metered AI uses;
- one machine-readable risk-routing policy is canonical;
- lane leads are globally required by root instructions to own review/repair/re-review loops;
- Work/Terra can apply the matrix deterministically without technical judgment;
- no covered Codex reviewer can implement, merge or mutate production;
- exact-head and reviewer-independence requirements are explicit;
- governance validation and genuinely independent exact-head review pass before merge.
