# Oteryn Game Autonomous Codex Review Loop Design

## Status

Owner-approved governance direction for Issue #229. This design removes the owner from routine review message routing while keeping owner-only product/authority decisions and all existing repository safety gates intact.

Admission: protected `main@4b6656f688868aa2fb59c18392c2f859f1c5a1c7`. Live GitHub always outranks this snapshot.

## Goal

Allow an allocated lane lead to own the complete candidate-review-repair-re-review loop:

```text
lane lead freezes exact candidate
  -> lane lead requests native GitHub Codex review when validated policy requires it
  -> Codex posts durable exact-head findings on the PR
  -> lane lead repairs findings inside the existing allocation
  -> new head invalidates prior review
  -> lane lead requests a fresh independent Codex review
  -> successful exact-head review + zero blocking findings + all other gates
  -> READY_FOR_INTEGRATION
```

The owner must not be required to copy a prompt into a second chat or approve every covered Codex review invocation after the standing authorization is canonical.

## Authority model

The standing authorization is deliberately narrow. It applies only after this governance package merges to protected `main` and only to operations enumerated by `docs/agents/CODEX_REVIEW_POLICY.json`.

Covered operations are independent read-only exact-head review/audit and non-mutating test/reproduce/fuzz/static-analysis used to support that review. It grants no implementation/fix, tracked repository mutation, commit/push/merge, production/protected-environment, secret, live-data or cross-repository authority.

All other owner-funded Codex/OpenAI/API uses remain deny-by-default and require the existing explicit per-invocation owner authorization.

The policy is itself normative authority and therefore must be registered in `docs/agents/GOVERNANCE_CONTRACT.json` and validated by `tools/agents/validate_governance.py`; green governance CI must fail if the policy is absent, malformed or materially weakens its bounded authority/routing/gate invariants.

## GitHub as the review message bus

The canonical review transport is the pull request because it provides durable target identity, exact head, timestamps, findings and re-review history. When the repository has a real supported native Codex GitHub review capability, the lane lead should use that mechanism; the preferred trigger is `@codex review` with bounded risk-specific guidance.

The agent must verify capability before claiming invocation. If the integration is unavailable or not configured, it records the exact capability gap and follows the fallback rule in the machine-readable policy. It must never pretend a Codex task ran.

## Independence

A review counts as independent only when the reviewer task/session did not materially author or materially modify the candidate under review. A Codex task used for implementation cannot become the independent reviewer for the same candidate. A fresh Codex review task is required.

Every material candidate-head change invalidates prior Codex qualification. The new head receives a fresh review when its validated risk route requires Codex.

## Deterministic risk routing

Neither the control plane nor the lane lead may decide ad hoc that a required review can be downgraded.

The machine policy defines precedence:

```text
CODEX_REQUIRED
  > CODEX_OPTIONAL
  > CODEX_NOT_REQUIRED_BY_THIS_POLICY
```

Lane-lead self-tags may only increase review rigor. They cannot make the lead's own candidate OPTIONAL or NOT_REQUIRED. Such a downgrade requires one of the exact canonical sources admitted by the policy (owner decision, Sol Supervising Architect classification, canonical risk contract) with independently provable source role/reference recorded before candidate freeze, or an explicit mechanical changed-path rule. Unvalidated or conflicting classification fails closed to `CODEX_REQUIRED`.

Codex is required for high-risk classes including durable persistence/schema/migrations, concurrency/races/contention, authentication/session/reconnect/fencing/trust, protocol/wire/stable identity/security, and durable item/value/economy custody.

The ordinary-docs mechanical exception is deliberately narrow and excludes `docs/agents/**`, `docs/architecture/**` and `docs/contracts/**`. Low-risk local implementation and optional routing require validated authoritative downgrade metadata; worker self-assertion is insufficient.

## Review success semantics

The presence of any Codex comment is not a PASS.

A Codex review may satisfy the independent technical-review gate only when the policy proves all required conditions, including:

- fresh non-authoring reviewer task/session;
- exact final head;
- correct risk qualification;
- durable GitHub evidence;
- successful review evidence bound to that exact head (explicit PASS or the native no-suggestions signal accepted by the policy);
- zero unresolved P0/P1 findings on that head;
- zero unresolved required review threads;
- no material head change after the qualifying review.

Green CI alone never substitutes for review.

## Responsibilities

### Lane lead

The allocated lane lead owns candidate freeze, Codex review request, repair of findings inside allocation, fresh re-review after head movement and final evidence handoff. It does not ask the owner to relay review prompts after standing authorization becomes canonical. It may increase its own review rigor but cannot self-authorize a downgrade.

### Codex reviewer

The Codex reviewer is independent and read-only with respect to tracked repository state. It may execute non-mutating validation needed to review the candidate. It posts durable findings/verdict to the PR and does not implement fixes or merge.

### Control plane

Work or Terra, whichever is uniquely active, mechanically validates the allowed risk-routing inputs, rejects any unvalidated downgrade, verifies required review presence/exact head/independence/success, and applies the resulting gate. Terra has zero technical discretion: it does not invent risk tags or decide whether a technical finding is acceptable.

### Work auditor

The Work auditor remains the governance/lifecycle auditor. Codex technical review does not replace an explicitly required separate governance audit.

## Failure handling

If required Codex capability is unavailable, the lane records the exact missing capability. If current repository policy accepts an equivalent qualified independent technical reviewer for that exact gate, that fallback may be used. Otherwise the lane waits fail-closed. Manual owner prompt relay is not the normal fallback after the standing authorization is canonical.

If Codex leaves findings, the lane remains active and repairs them. The control plane does not bounce the owner between agents.

## Compatibility

This design does not change the single-active-control-plane rule, lane allocations, shared-path serialization, architecture escalation, production authority, runtime architecture or existing independent-review requirements. It specializes how a required technical reviewer is selected, invoked, authorized and qualified.

## Success criteria

- root governance contains the standing-authorization exception;
- the owner-funded AI policy distinguishes covered standing review from other metered AI uses;
- one machine-readable risk-routing policy is canonical;
- that policy is registered as a required governance document and validated fail-closed for authority/routing/independence/prohibition/gate invariants;
- lane leads are globally required by root instructions to own review/repair/re-review loops;
- lane leads cannot downgrade their own review requirement;
- Work/Terra can validate and apply the matrix deterministically without technical judgment;
- no covered Codex reviewer can implement, merge or mutate production;
- exact-head reviewer independence, explicit review success and zero blocking finding/thread requirements are machine-readable;
- governance validation and genuinely independent exact-head review pass before merge.
