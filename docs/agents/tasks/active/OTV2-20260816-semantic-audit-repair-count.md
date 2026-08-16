# OTV2-20260816-semantic-audit-repair-count

```yaml
task_id: OTV2-20260816-semantic-audit-repair-count
title: Semantic audit repair-count generalization
mode: TOOLING_GOVERNANCE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/architecture-semantic-audit-repair-count
owner: ARCHITECTURE_COORDINATOR
created_at: 2026-08-16
updated_at: 2026-08-16
owned_paths:
  - tools/architecture/semantic_contract_audit.py
  - docs/agents/tasks/active/OTV2-20260816-semantic-audit-repair-count.md
implementation_authority: TOOLING_ONLY
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
owner_funded_ai_authorized: false
```

## Goal

Repair the deterministic semantic auditor's incorrect assumption that every owner-overridden architecture gate must remain exactly at repair cycle 4. The project owner authorized continued C/D/E/F repair beyond the normal three-cycle ceiling; a real F finding now requires cycle 5.

The auditor remains fail closed: it parses the declared stable-gate repair count, requires it to be at least 4 for the current E/F profiles, requires durable owner-override evidence, preserves the no-Codex constraint, and rejects malformed/missing repair history.

## Acceptance criteria

- [x] no hard-coded equality to repair cycle 4;
- [x] declared cycle is parsed as a positive integer and must be `>= 4`;
- [x] owner override evidence remains mandatory;
- [x] no-Codex and merge-authority checks remain mandatory;
- [x] E profile and F profile semantic predicates are otherwise unchanged;
- [x] exact-head full-diff self-review is clean with zero material findings;
- [ ] final exact-head repository CI passes;
- [x] no Codex/owner-funded AI is used.

## Self-review

The final tooling delta is limited to `common(task)`: replace exact string equality to cycle 4 with fail-closed parsing of `repair_cycles_for_current_gate`, reject values below 4, and additionally require durable `repair_cycle_4_owner_override:` evidence. All E/F semantic predicates are unchanged. This is a general history-validation correction, not a task-specific cycle-5 bypass.

## Context checkpoint

```yaml
status: validating
completed:
  - generalize repair count parsing
  - preserve owner override and no-Codex requirements
  - full-diff self-review
validation_pending:
  - exact-head repository CI
next_action: VALIDATE_FINAL_HEAD_THEN_MERGE_AND_CLOSEOUT
```

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: TOOLING_ONLY`
