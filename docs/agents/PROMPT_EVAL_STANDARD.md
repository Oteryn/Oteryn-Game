# Prompt evaluation standard

Evaluate prompts before reuse.

## Gates

- **Authority:** exact writable repositories and protected/live exclusions are explicit.
- **Resolution:** task can be located from repository state without relying on chat.
- **Ownership:** paths/contracts do not overlap ambiguously.
- **Architecture:** accepted ADRs and product boundaries are preserved.
- **Completeness:** observable outcome and all required layers are named.
- **Evidence:** source order, truth labels and exact-head requirements are explicit.
- **Validation:** focused/component/integration/E2E/audit/CI expectations are proportional and executable.
- **Autonomy:** agent continues through lifecycle but has real bounded stop conditions.
- **Handover:** durable checkpoint fields and one next action are required.
- **Safety:** secrets, production, assets, destructive data and cross-repository operations are protected.
- **Codex review routing:** reusable prompts defer to `CODEX_REVIEW_POLICY.json`; covered review never requires per-run owner relay, non-covered owner-funded AI remains exact-owner-authorized, and standing authorization does not create candidate/control-plane/auditor authority.
- **Remote Desktop routing:** every reusable prompt contains exactly one `## Remote Desktop execution routing` section bound to `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`; out-of-band local connector/tool registration and argument-schema inspection is distinct from every direct `Remote_Desktop_Commander.*` invocation, which requires a fresh valid host-exception context and positive per-action authorization for the exact semantic host action and exact connector tool.

## Remote Desktop execution routing

A reusable prompt fails evaluation if `list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations or another direct connector call can be treated as ordinary capability discovery. Unknown or undeclared tools must fail closed, a prior ALLOW must not authorize a different action/tool, and Game cannot broaden META exception reasons. Remote Desktop must not become a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: the prompt must continue useful authorized work through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when possible.

The evaluated prompt may restate the routing boundary for self-containment, but must not copy/fork META machine-readable policy or claim connector/router physical enforcement without a verified transport hook.

## Verdicts

- `PASS` — executable without material ambiguity.
- `PASS_WITH_NOTES` — safe, minor non-blocking improvements identified.
- `FAIL` — authority, ownership, architecture, acceptance, validation or stop conditions are materially ambiguous.

Record concrete defects; do not score prompts by length or confidence of tone.
