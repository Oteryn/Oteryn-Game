# Prompting standard

A repository execution prompt is a **task-specific delta** on top of current root/nearest `AGENTS.md`, live GitHub task state and accepted contracts. For GPT-5.6 Sol, prefer the smallest prompt that preserves the intended outcome, authority boundaries, hard domain constraints and observable acceptance.

Do not copy repository-wide execution policy into every prompt merely for self-containment. Global GitHub-first, branch/concurrency, capability-discovery, Remote Desktop, AI-review, retry/continuation and merge rules are inherited from current governing instructions unless the task has a legitimate narrower rule.

## Minimal prompt contract

A substantial prompt needs only the task-specific information that materially changes execution:

1. **Role / outcome** — bounded role and one observable target.
2. **Authority / scope** — exact writable repository/path or allocation boundary and prohibited effects.
3. **Live locators** — Issue/task/PR/branch or other identifiers needed to refresh current truth; cached SHAs are locators, not authority.
4. **Hard constraints / dependencies** — Game/domain invariants, accepted contracts and prerequisites unique to this work.
5. **Acceptance / validation** — the observable evidence actually required for this task.
6. **Stop / handoff** — genuine owner/safety/authority blockers and the durable result or next action when execution cannot continue.

Omit a section when it has no task-specific content. Do not add a field, example, checklist or procedure solely because an older template contained it.

## Oteryn v2 domain invariants

Prompts must preserve native Rust, `protocol-oteryn` only, multichannel identities/ownership, server authority, session fencing and separate external repository authority unless an owner-approved ADR/task explicitly changes them.

Do not instruct an agent to infer missing bytes, schemas, assets, credentials, deployment state or external implementation. Require bounded discovery or record the exact blocker.

## Worker prompts

Worker prompts have exclusive paths/contracts, precise inputs/outputs, no coordinator authority, clear integration-ready state and focused acceptance. Do not assign overlapping public contract ownership to parallel workers.

A direct reusable worker alias grants no write authority by itself. The worker must resolve the live allocation/owner authority required by its lifecycle entry and task before mutation.

## Audit prompts

Auditors are read-only unless repair is separately authorized. They must challenge completeness, inspect exact source/live state and classify every finding with evidence and severity. Do not copy implementation-worker procedure into an auditor merely for symmetry.

## AI review inheritance

Reusable prompts inherit the current root `AGENTS.md` META-owned AI-review policy. They **must not** maintain a competing full copy of organization review routing.

A prompt may state a task-specific risk fact or identify which current policy trigger applies, but root/current META policy decides whether review is required, optional or not useful. Standing or historical local review prose cannot create candidate ownership, control-plane authority, merge authority or a required GitHub status.

Legacy `## Canonical Codex review routing` blocks may remain temporarily during migration, but new or materially updated prompts should omit them unless a deterministic consumer still requires that exact block. Any such retained block is compatibility text, not independent policy authority.

## Remote Desktop inheritance

Reusable prompts inherit Remote Desktop/Desktop Commander restrictions from current Game `AGENTS.md` and the canonical META execution-routing policy referenced there. Prompt-local policy must never broaden those rules.

A reusable prompt does **not** need to copy a `## Remote Desktop execution routing` section. During migration, an existing exact legacy canonical section may remain compatible; modified, duplicate, hidden/example-only or policy-broadening copies fail closed. Remote Desktop availability never grants authority and a DENY is not automatically a task blocker when a safe repository-native path remains.

The authoritative repository/supporting policy surfaces retain the detailed per-action routing contract and deterministic validation. Task prompts should reference/inherit that authority rather than repeat it.

## Prompt evaluation

Evaluate material prompt/harness changes against `docs/agents/PROMPT_EVAL_STANDARD.md`. Use balanced cases and ablation; keep a rule or example only when it protects a documented invariant or improves measured behaviour. Newer model families may require less scaffolding.

Do not score prompts by length alone: shorter is useful only when safety, task success and observable outcome are preserved or improved. Measure unnecessary owner questions, repeated reads/tool calls, context loaded versus used, and false blockers where practical.

## Retained prompt lifecycle

Every retained execution prompt under `docs/agents/prompts/*.md`, except the directory `README.md`, must have exactly one entry in `docs/agents/PROMPT_LIFECYCLE.json`.

The lifecycle registry supplies stable `prompt_id`, registry version, status, lifecycle owner, bounded scope and supersession semantics. Registry version `1.0` means first registration in that lifecycle registry; it does not retroactively claim that an older prompt document historically used that version.

`reusable` means only that the prompt may be resolved from current `main`. It does not grant a branch, owned paths or write authority; the prompt's live allocation/owner prerequisites still apply. `retired` means provenance-only: the file may remain in Git history or the prompt directory, but it must not be dispatched as executable work and must name its successor in the lifecycle registry.

A prompt is superseded only by an explicit registry entry. A completed task that previously used a reusable executor does not by itself retire the executor; a new invocation must still satisfy all current live-allocation and governance gates.
