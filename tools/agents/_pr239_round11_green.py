#!/usr/bin/env python3
from pathlib import Path
import re

validator_path = Path("tools/agents/validate_remote_desktop_prompt_routing.py")
text = validator_path.read_text(encoding="utf-8")

# 1) Authority is an equivalent blanket/default grant noun for direct connectors,
# connector actions, and named host operations. Extend only the existing bounded
# outside-policy grant vocabulary.
needle = "(?:approval|permission)"
count = text.count(needle)
if count < 8:
    raise SystemExit(f"unexpected approval/permission pattern count: {count}")
text = text.replace(needle, "(?:approval|permission|authority)")

# 2) Cover the two postposed automatic ping-authority orderings while preserving
# the existing ping-bounded lookahead.
ping_anchor = r'''        r"\bautomatically\s+(?:granted|given)\s+(?:approval|permission|authori[sz]ation|authority)\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?automatic\b|"'''
ping_replacement = r'''        r"\bautomatically\s+(?:granted|given)\s+(?:approval|permission|authori[sz]ation|authority)\b|"
        r"\bping\b.{0,80}\b(?:is\s+)?(?:granted|given)\s+(?:approval|permission|authori[sz]ation|authority)\s+automatically\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?(?:granted|given)\s+to\s+ping\s+automatically\b|"
        r"\b(?:approval|permission|authori[sz]ation|authority)\s+(?:is\s+)?automatic\b|"'''
if text.count(ping_anchor) != 1:
    raise SystemExit("expected one ping automatic anchor")
text = text.replace(ping_anchor, ping_replacement)

# 3) URL scan is already percent-decoded. Greedily allow decoded userinfo to
# contain @ or / and backtrack to the final @ immediately before the exact GitHub host.
userinfo = r"(?:[^\s@]+@)?"
if text.count(userinfo) != 2:
    raise SystemExit(f"expected two GitHub userinfo patterns, found {text.count(userinfo)}")
text = text.replace(userinfo, r"(?:[^\s]+@)?")
validator_path.write_text(text, encoding="utf-8")

# Refresh the active task packet so it no longer presents the obsolete 2026-08-28
# handoff as current authority.
task_path = Path("docs/agents/tasks/active/OTV2-20260828-remote-desktop-per-action-gate.md")
task = task_path.read_text(encoding="utf-8")
task = task.replace("base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6", "base_sha: external_pr_evidence")
task = re.sub(r"updated_at: .*", "updated_at: 2026-08-30T00:40:00+02:00", task, count=1)
task = task.replace("  - .github/workflows/rdc-prompt-sweep-once.yml\n", "")
task = task.replace("  - .github/workflows/rdc-final-p2-green-retry.yml\n", "")
owned_anchor = "  - tools/agents/test_validate_remote_desktop_prompt_routing.py\n"
if owned_anchor not in task:
    raise SystemExit("task owned-path anchor missing")
if "tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py" not in task.split("```", 2)[1]:
    task = task.replace(owned_anchor, owned_anchor + "  - tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py\n", 1)

start = task.find("## Continuation handoff — 2026-08-28")
if start < 0:
    raise SystemExit("stale continuation handoff section not found")
replacement = '''## Current continuation checkpoint

This checkpoint is intentionally SHA-neutral: exact head/base/check/review coordinates are immutable GitHub evidence and MUST be refreshed from live GitHub immediately before any mutation or merge decision. A commit cannot truthfully embed its own final SHA.

Current durable repository state represented by this task packet:

- the canonical META provider binding remains `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`;
- all **44** lifecycle-reusable Game prompts and the four canonical governance surfaces are covered by the retained provider validator;
- `tools/agents/test_validate_remote_desktop_prompt_routing.py` plus the durable `tools/agents/test_validate_remote_desktop_prompt_routing_codex_regressions.py` retain tests-first coverage for every repaired independent-review bypass class;
- no branch-only repair workflow, patch script or qualification marker is part of the intended final changed-file set;
- Remote Desktop/Desktop Commander was not used for this task and remains default-deny/exception-only;
- the task remains `validating` until one exact final head passes all four required Game workflows, receives one fresh independent clean Codex review, is guarded-squash-merged, and protected `main` readback succeeds.

### Required terminal continuation order

1. Refresh protected `main`, Issue #237, PR #239, exact PR head, changed filenames, unresolved review threads and exact-head workflow evidence from live GitHub.
2. Confirm the durable changed-file set contains no temporary helper/workflow/marker and the PR is `behind_by=0` against protected `main`; if main advanced, reconcile only by a normal merge-up, never reset/rebase/force.
3. Require `Agent governance`, `Architecture semantic audit`, `Merge authority audit` and the full `Merge gate` to PASS on the same exact final SHA. Agent governance must prove the complete base and durable Remote Desktop regression suites plus 44 reusable prompts + 4 canonical surfaces and repository policy.
4. Request exactly one fresh independent Codex review for that frozen exact SHA. Any material P0/P1/P2 reopens strict tests-first RED→GREEN repair and invalidates prior qualification.
5. With zero unresolved blocking threads, refresh head/main once more and squash-merge PR #239 using `expected_head_sha` equal to the reviewed frozen SHA; do not bypass protections.
6. Read back protected `main`, verify the returned squash SHA, canonical META binding, validator, representative reusable prompts and retained Agent-governance binding; then close Issue #237 as completed if its acceptance criteria are satisfied.

## Autonomous continuation prompt

> Continue autonomously and bring `Oteryn/Oteryn-Game#239` to a safe squash merge. GitHub live state is the sole source of truth. Do not use Remote Desktop/Desktop Commander. Preserve strict tests-first repair for any new P0/P1/P2. Never rebase/force. Freeze one exact final head, require all four Game workflows plus one fresh independent Codex review on that same SHA, then guarded squash merge with `expected_head_sha`, protected-main readback, and Issue #237 closeout. Never treat this checkpoint's prose as a substitute for live GitHub evidence.

## Context checkpoint

```yaml
last_progress: canonical provider binding, 44-prompt sweep, durable validator hardening and accumulated Codex regression coverage are implemented; live exact-head qualification remains authoritative
status: validating
branch: governance/remote-desktop-per-action-gate-237
head_sha: external_pr_evidence
pr: 239
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: pull_request
ci_checks_for_current_head: external_pr_evidence
ci_run_ids: external_pr_evidence
ci_job_ids: external_pr_evidence
runner_assignment_state: github_hosted
remote_desktop_used_for_this_task: false
owner_action_required: null
blocker: refresh live GitHub; require one exact final SHA with four required workflow PASS results, zero unresolved blocking threads, and one fresh independent clean Codex review
next_action: refresh live PR/main and durable changed-file scope; qualify one immutable exact head; request one fresh Codex review; if clean, guarded squash merge with expected-head guard; protected-main readback; close Issue #237
```
'''
task = task[:start] + replacement

# Fail closed if the obsolete authoritative checkpoint still survives.
for stale in (
    "three unresolved Codex P2",
    "temporary_helper_still_in_diff",
    "blocker: three unresolved",
    "exactly 43 lifecycle-derived reusable prompts",
):
    if stale in task:
        raise SystemExit(f"stale checkpoint text survived: {stale}")

task_path.write_text(task, encoding="utf-8")
