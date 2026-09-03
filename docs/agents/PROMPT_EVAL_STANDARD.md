# Prompt evaluation standard

Evaluate prompts before reuse. Prompt quality is behavioural quality, not adherence to a long template.

## Gates

- **Authority:** exact writable repositories/paths and protected/live exclusions are unambiguous.
- **Resolution:** the task can be located from current repository/GitHub state without relying on chat history.
- **Ownership:** paths/contracts do not overlap ambiguously.
- **Architecture:** accepted ADRs and product boundaries are preserved.
- **Completeness:** the observable outcome and all materially required product layers are clear.
- **Evidence:** live-state/source order and exact-head requirements are sufficient for the task.
- **Validation:** focused/component/integration/E2E/audit/CI expectations are proportional and executable.
- **Autonomy:** the agent continues useful authorized work but has real bounded stop conditions.
- **Handover:** durable state can reconstruct one concrete next action when work cannot continue.
- **Safety:** secrets, production, assets, destructive data and cross-repository operations remain protected.
- **AI review inheritance:** current root `AGENTS.md` / META policy remains the sole review-routing authority; a prompt may classify task-specific risk but must not recreate a competing global review controller.
- **Remote Desktop inheritance:** current Game `AGENTS.md` plus canonical META execution-routing policy remains authoritative; a prompt may omit duplicated routing prose, but any prompt-local routing text must not weaken, bypass or falsely claim physical enforcement of that authority.

## Lean-prompt evaluation

Prefer one statement of each rule at the highest appropriate authority level. A reusable prompt should contain only the domain/task delta that materially affects its execution.

When comparing a current prompt with a lean candidate, evaluate at least:

- task success and safety outcome;
- unnecessary owner questions or approval requests;
- repeated policy reads/tool calls;
- context loaded versus actually used;
- false blockers and premature stops;
- missing domain constraints or acceptance criteria;
- whether a removed rule was already supplied by governing instructions or machine enforcement.

Use ablation: remove one class of duplicated scaffold, run the same representative cases, and retain the scaffold only when it provides measurable value or protects a clearly documented invariant. Newer models may need less scaffolding.

Do not treat shorter text as success by itself. Do not preserve a duplicated block merely because an older validator expected it; update the validator when the inherited authority remains fail-closed and regression evidence proves the lean contract.

## AI review policy

External review decisions are inherited from current root `AGENTS.md`. Legacy `## Canonical Codex review routing` sections are compatibility text only while they remain in older prompts; their absence is not an evaluation failure when the prompt inherits current root policy and has no task-specific review delta.

A prompt-local review statement fails evaluation if it broadens authority, creates a required status outside current policy, grants reviewer mutation authority, or requires unnecessary per-run owner relay contrary to current policy.

## Remote Desktop policy

A clean reusable prompt may contain **zero** `## Remote Desktop execution routing` sections and inherit current root/META policy. An existing exact legacy canonical block may remain during migration. Modified/duplicate/example-only legacy blocks or prompt-local text that authorizes routine Remote Desktop use, exempts direct connector calls, broadens META exception reasons or claims unverified physical enforcement fail closed.

The detailed per-action contract remains on the authoritative repository/supporting policy surfaces and in deterministic validators. Reusable prompts should not copy/fork the META machine-readable policy.

## Verdicts

- `PASS` — executable without material ambiguity and without unnecessary duplicated scaffold.
- `PASS_WITH_NOTES` — safe, with minor non-blocking simplification or clarity improvements identified.
- `FAIL` — authority, ownership, architecture, acceptance, safety, validation or stop conditions are materially ambiguous or a prompt attempts to redefine inherited policy.

Record concrete defects; do not score prompts by length, checklist count or confidence of tone.
