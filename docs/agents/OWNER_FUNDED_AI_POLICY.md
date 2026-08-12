# Owner-funded AI policy

This document records the repository-owner instruction established on 2026-08-12 and implemented by the root `AGENTS.md`.

## Default

Owner-funded and owner-metered AI resources are deny-by-default for agents. Codex, OpenAI API, paid or quota-limited AI review services, and equivalent mechanisms may not be invoked when they would consume the owner's personal quota, credits, tokens, subscription limits, or metered allowance unless the owner explicitly authorizes that specific use.

The same rule applies to owner-supplied AI/model API keys, access tokens, session tokens, credentials, and secrets. Technical availability, an authenticated session, a connector, an environment variable, or prior permission is not standing authorization.

## Gate behavior

A mandatory review or validation gate is not waived by this restriction. Use a genuinely suitable non-owner-funded independent mechanism when available. If none can satisfy the gate, fail closed and report the blocker instead of consuming owner-funded AI or weakening the gate.

## Authority

The normative enforcement text is the highest-priority owner-funded AI section in the root `AGENTS.md`. A later explicit repository-owner instruction may authorize a bounded use or supersede this policy.
