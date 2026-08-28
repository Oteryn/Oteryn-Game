# Game Remote Desktop Per-Action Adoption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task with TDD. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bind every reusable Oteryn Game agent prompt and canonical execution instruction to the merged META Remote Desktop per-action gate without forking META policy.

**Architecture:** Game references `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0` as the sole machine-readable routing authority. Root/operational instructions and every reusable prompt receive one concise self-contained `## Remote Desktop execution routing` section, while `tools/agents/validate_governance.py` derives the applicable prompt set from `PROMPT_LIFECYCLE.json` and deterministically rejects missing or contradictory routing semantics.

**Tech Stack:** Markdown, Python 3.12 standard library, GitHub Actions.

**Spec:** META design `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0:docs/superpowers/specs/2026-08-28-remote-desktop-per-action-enforcement-design.md`

## Global Constraints

- GitHub is authoritative; do not use Remote Desktop/Desktop Commander to implement, inspect, test or verify this adoption.
- Do not copy/fork the META machine-readable policy into Game.
- META exception reasons remain closed to `host_only_service`, `lan_or_hardware`, and `self_hosted_runner_diagnosis`.
- Out-of-band local connector/tool registration and argument-schema inspection may occur without invoking Remote Desktop.
- Every direct `Remote_Desktop_Commander.*` call requires a fresh valid host exception plus a positive exact per-action decision under canonical META semantics for the semantic host action and exact connector tool immediately before the call.
- `list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations are not capability-discovery exemptions.
- Unknown or undeclared connector tools fail closed; a prior ALLOW never authorizes a different tool/action.
- Remote Desktop is not a routine repository-test, Git-inspection, CI/log-polling or capability-probe fallback.
- A Remote Desktop DENY is not automatically a blocker; continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when capable.
- Do not claim connector/router physical enforcement until an actual connector/router hook exists.

---

### Task 1: Add RED provider-governance assertions

**Files:**
- Modify: `tools/agents/validate_governance.py`

**Interfaces:**
- Consumes: `PROMPT_LIFECYCLE.json`, `AGENTS.md`, `docs/agents/GITHUB_ONLY_EXECUTION.md`, `docs/agents/PROMPTING_STANDARD.md`, `docs/agents/PROMPT_EVAL_STANDARD.md`.
- Produces: `validate_remote_desktop_prompt_routing(registry: dict, errors: list[str]) -> None`.

- [ ] Add a function that discovers every lifecycle entry where `status == "reusable"` and `reusable is True` and requires the referenced prompt body to contain exactly one `## Remote Desktop execution routing` heading.
- [ ] Require each reusable prompt to contain all canonical markers: exact META SHA `e002fc7532188e73a0f495da3e20710541ed50e0`; `every direct \`Remote_Desktop_Commander.*\` invocation`; `positive per-action`; `list_devices`; `not automatically a blocker`; and language that Game must not broaden META exception reasons.
- [ ] Require root `AGENTS.md`, `GITHUB_ONLY_EXECUTION.md`, `PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md` to carry the corresponding canonical provider binding.
- [ ] Reject stale META routing references in root instructions and reject prompt text that explicitly authorizes Remote Desktop as a routine fallback for repository tests, Git inspection or CI/log polling.
- [ ] Call the new validator from `main()` immediately after `validate_prompt_lifecycle(...)`.
- [ ] Push only these RED assertions first and verify `Agent governance / validate` fails because current prompts/instructions do not satisfy the new contract.

---

### Task 2: Align canonical Game execution instructions

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/agents/GITHUB_ONLY_EXECUTION.md`
- Modify: `docs/agents/PROMPTING_STANDARD.md`
- Modify: `docs/agents/PROMPT_EVAL_STANDARD.md`

- [ ] Replace the stale META routing coordinate in `AGENTS.md` with `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0:ecosystem/agent-execution-routing-policy.json`.
- [ ] State in root instructions that out-of-band connector registration/schema discovery is not a direct connector call, but every direct `Remote_Desktop_Commander.*` invocation requires fresh host-exception context and positive exact per-action authorization immediately before the call.
- [ ] Explicitly prohibit `list_devices`, `who_am_i`, `ping`, `get_config` and equivalent read-only/metadata calls as ordinary capability probes.
- [ ] State that unknown/undeclared tools fail closed, previous ALLOW does not carry forward, existing exception reasons cannot be broadened, and DENY is not automatically a blocker.
- [ ] Add the same operational boundary to `GITHUB_ONLY_EXECUTION.md` without turning that document into a copy of META policy.
- [ ] Add a prompt-authoring invariant to `PROMPTING_STANDARD.md`: every reusable prompt must contain exactly one `## Remote Desktop execution routing` section with the canonical markers.
- [ ] Add the matching evaluation gate to `PROMPT_EVAL_STANDARD.md`.

---

### Task 3: Sweep all reusable prompt bodies

**Files:**
- Modify every lifecycle-reusable `docs/agents/prompts/*.md` body discovered from `PROMPT_LIFECYCLE.json` (43 prompts at admission main `7c2da078596a7d2e27c3066ff74ac69b8b7f9af6`).

**Canonical section to add exactly once to each reusable prompt:**

```markdown
## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. This prompt cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.
```

- [ ] Enumerate the reusable set from live branch `PROMPT_LIFECYCLE.json`; do not hard-code a historical prompt list in validator logic.
- [ ] Add the canonical section exactly once to every reusable prompt and do not change role authority, task scope, implementation semantics, Codex routing or production exclusions.
- [ ] If a prompt already mentions Remote Desktop, reconcile contradictory wording rather than retaining two competing rules.
- [ ] Do not add the section to retired prompts merely for provenance.

---

### Task 4: Prove GREEN and review the whole prompt sweep

**Files:** all files touched by Tasks 1-3 plus this plan/task packet.

- [ ] Verify exact-head `Agent governance / validate` PASS and confirm the workflow ran `python tools/agents/validate_governance.py` against the PR exact head.
- [ ] Verify repository-required Architecture semantic audit, Merge authority audit and Merge gate on the same final head where applicable.
- [ ] Inspect the full changed-file list and full diff. Confirm the reusable prompt count equals the lifecycle-derived count and every reusable prompt has exactly one canonical section.
- [ ] Confirm no runtime/Cargo/protocol/schema/resource/deployment/secret/runner-host/live-RDC path changed.
- [ ] Confirm the change references, but does not copy, META machine-readable policy.
- [ ] Confirm no tracked plan/task file attempts to record its own final SHA; final exact-head/review/check/merge evidence remains in GitHub.

---

### Task 5: Exact-head independent review and protected delivery

- [ ] Resolve the current `docs/agents/CODEX_REVIEW_POLICY.json` on the final branch head and mechanically classify review routing; broad reusable-prompt/governance changes must not be downgraded by author assertion.
- [ ] Freeze the final candidate only after required deterministic checks are green.
- [ ] Request one fresh independent exact-head Codex review through the canonical GitHub PR transport when policy requires it; consume findings without expanding scope and re-review after any material head change.
- [ ] Require zero unresolved blocking findings/required review threads and every exact-head merge gate required by current Game policy.
- [ ] Re-read current protected `main`; if it advanced, reconcile normally without discarding valid work and rerun invalidated exact-head evidence.
- [ ] Squash merge only with expected exact head and no bypass/force/admin override.
- [ ] Read back protected `main` after merge and verify root binding, prompt standard/eval gates, validator enforcement and representative reusable prompt sections. Runtime/E2E remains `NOT_APPLICABLE` for this governance-only adoption.
