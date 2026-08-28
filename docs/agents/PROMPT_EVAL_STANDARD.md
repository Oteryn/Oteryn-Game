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

## Verdicts

- `PASS` — executable without material ambiguity.
- `PASS_WITH_NOTES` — safe, minor non-blocking improvements identified.
- `FAIL` — authority, ownership, architecture, acceptance, validation or stop conditions are materially ambiguous.

Record concrete defects; do not score prompts by length or confidence of tone.
