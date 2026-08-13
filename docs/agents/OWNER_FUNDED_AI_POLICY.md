# Owner-funded AI policy

This document records repository-owner instructions established on 2026-08-12 and refined on 2026-08-13, implemented by the root `AGENTS.md`.

## Default

Owner-funded and owner-metered AI resources are deny-by-default for agents. Codex, OpenAI API, paid or quota-limited AI review services, and equivalent mechanisms may not be invoked when they would consume the owner's personal quota, credits, tokens, subscription limits, or metered allowance unless the owner explicitly authorizes that specific use.

The same rule applies to owner-supplied AI/model API keys, access tokens, session tokens, credentials, and secrets. Technical availability, an authenticated session, a connector, an environment variable, or prior permission is not standing authorization.

## Independent review default

When independent review or audit is required, prefer a fresh second agent/session that did not implement or materially author the change, provided that reviewer is qualified for the exact task. A fresh independent agent/session is sufficient independent-review evidence when it verifies the exact final SHA and governing requirements independently; Codex is not required merely because the gate requires a second reviewer.

The implementing/coordinating agent's self-review remains self-review and must never be relabeled as independent.

## Codex recommendation and prompt handoff

Codex remains an optional specialist tool rather than the default reviewer.

If the coordinating agent judges that Codex would be materially more efficient or effective for a particular implementation, review or audit—for example because of unusually broad code awareness, adversarial code review, complex refactoring/migration, security-sensitive implementation, or test/fuzz execution—the agent must **inform the owner before any Codex invocation**.

That notification must:

1. identify the exact task/PR/SHA and the work Codex would perform;
2. explain the material advantage expected from Codex over the available non-owner-funded mechanism;
3. provide a ready-to-run, bounded Codex prompt that the owner can approve or use directly;
4. preserve all repository safety, scope, audit-only/implementation and exact-head constraints applicable to the task.

Providing the recommendation or prompt does not itself authorize Codex. The agent must wait for explicit owner authorization for that exact use. Prior permission is never standing permission.

## Gate behavior

A mandatory review or validation gate is not waived by this restriction. Use a genuinely suitable non-owner-funded independent mechanism when available. If none can satisfy the gate, fail closed and report the blocker instead of consuming owner-funded AI or weakening the gate.

If Codex would materially improve the task but is not mandatory, the agent may recommend it and provide the prompt while continuing any independent work that does not require owner-funded AI. The agent must not represent a declined or unauthorized Codex recommendation as a blocker unless no permitted mechanism can satisfy a mandatory gate.

## Authority

The normative enforcement text is the highest-priority owner-funded AI and independent-review sections in the root `AGENTS.md`. A later explicit repository-owner instruction may authorize a bounded use or supersede this policy.
