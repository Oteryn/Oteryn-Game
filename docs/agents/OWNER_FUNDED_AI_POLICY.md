# Owner-funded AI policy

This document records repository-owner instructions established on 2026-08-12 and refined on 2026-08-13 and 2026-08-27, implemented by the root `AGENTS.md`.

## Default

Owner-funded and owner-metered AI resources remain deny-by-default. Codex, OpenAI API, paid or quota-limited AI review services, and equivalent mechanisms may not consume the owner's personal quota, credits, tokens, subscription limits or metered allowance unless either:

1. the exact invocation is covered by the protected-main standing authorization in `docs/agents/CODEX_REVIEW_POLICY.json`; or
2. the owner explicitly authorizes that specific non-covered use.

Technical availability, an authenticated session, connector, environment variable, API key or earlier one-off permission is not authority outside those two cases.

## Standing authorization: independent Codex review

Issue #229 records the owner's standing authorization for the bounded review operations encoded in `docs/agents/CODEX_REVIEW_POLICY.json`. The authorization becomes effective only after that contract and the matching root governance are merged to protected `main`.

For an invocation that matches the policy exactly:

- `owner_confirmation_per_covered_run` is false;
- an allocated lane lead may request a fresh independent exact-head Codex review directly through the canonical GitHub pull request when a real supported native Codex review capability is proven available;
- the preferred native trigger is `@codex review`, optionally followed by bounded risk-specific review guidance;
- the owner is not the default message relay between the lane lead and the reviewer;
- the lane lead owns the candidate -> review -> repair -> fresh re-review loop until the applicable review gate passes or a real blocker is reached.

The standing authorization is limited to the operations and risk routing defined by the machine-readable policy. It grants no implementation/fix, tracked-file mutation, commit/push/merge, branch-protection, production/protected-environment, secret, live-data or cross-repository authority to the reviewer.

Every non-covered Codex/OpenAI/API invocation remains subject to exact per-invocation owner authorization.

## Independent review default

A genuinely independent reviewer must not have materially authored or materially modified the candidate it reviews. A Codex task/session that assisted implementation cannot count as the independent reviewer for that candidate; a fresh reviewer task/session is required.

The implementing/coordinating agent's self-review remains self-review and must never be relabeled as independent.

Codex is not required merely because any review is needed. Apply `CODEX_REVIEW_POLICY.json` mechanically: its high-risk classes require Codex independent technical review; an unvalidated or conflicting downgrade also fails closed to `CODEX_REQUIRED`. Only a mechanically proven ordinary-docs rule or independently validated authoritative downgrade metadata may select `CODEX_OPTIONAL` / `CODEX_NOT_REQUIRED_BY_THIS_POLICY`. Separate governance/lifecycle audit requirements remain separate.

A lane lead may self-tag a candidate only to increase review rigor. It may not make its own candidate optional or not-required. Any downgrade source role and source record must be proven from canonical authority under the machine-readable contract rather than accepted as self-declared text.

## GitHub review evidence

The pull request is the canonical review message bus when native GitHub Codex review capability is available. The request and result must be bound to the exact candidate head and durably visible on GitHub.

A material head change makes a prior qualifying Codex review historical. If the risk matrix still requires Codex, the lane lead freezes the new head and requests a fresh review. Technical findings are repaired by the owning lane lead within its existing allocation, not by the owner or control plane.

Successful Codex evidence can satisfy the independent technical-review gate only when all independence, exact-head, qualification and durable-evidence conditions in `CODEX_REVIEW_POLICY.json` are proven. Success means either an explicit exact-head PASS or the native exact-head no-suggestions signal accepted by that policy, with zero unresolved P0/P1 findings and zero unresolved required review threads. Green CI alone is not independent review.

## Capability and fallback

Authority and capability are separate. A standing authorization does not prove that a particular chat/session or repository has a supported Codex GitHub invocation mechanism.

Before requesting review, the lane lead must verify the actual capability exposed in that execution context. If unavailable, record the precise capability gap and follow the machine-readable fallback rule. Never claim a Codex review ran when it did not. Manual owner prompt relay is not the normal fallback.

If Codex is required by the current risk matrix and no permitted equivalent reviewer satisfies the exact repository gate, fail closed rather than weakening review requirements. If Codex is optional and unavailable, continue through the existing qualified independent-review path.

## Non-review Codex recommendation and handoff

For implementation, debugging, build/repository execution or any other owner-funded AI use not covered by `CODEX_REVIEW_POLICY.json`, the prior deny-by-default rule remains unchanged.

If an agent judges such a non-covered Codex use materially advantageous, it must first inform the owner, identify the exact task/PR/SHA and purpose, explain the expected advantage, provide a bounded prompt/handoff and wait for explicit authorization for that exact use.

Prior permission or the independent-review standing authorization is never standing permission for non-covered work.

## Gate behavior

A mandatory review or validation gate is never waived by quota/capability policy. Use the canonical standing-authorized Codex path when required and available, an explicitly permitted equivalent reviewer when the exact gate allows it, or fail closed with the exact blocker.

The uniquely active Work/Terra control plane mechanically validates risk-routing inputs, rejects any unvalidated low-risk/optional downgrade, and verifies required review presence and exact-head evidence. It does not decide ad hoc whether a technical finding is harmless and cannot waive `CODEX_REQUIRED` classifications.

## Authority

The normative enforcement text is the highest-priority owner-funded AI and autonomous Codex review sections in root `AGENTS.md`, together with protected-main `docs/agents/CODEX_REVIEW_POLICY.json`. A later explicit repository-owner instruction may narrow or supersede this standing authorization.
