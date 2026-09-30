# OTV2-20260929-achievement-owner-contract

```yaml
task_id: OTV2-20260929-achievement-owner-contract
title: ACHIEVEMENT - Achievement owner contract V1 with catalogue schema and validator (D126)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/achievement-owner-contract
issue: 162
lane_id: ACHIEVEMENT
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 50d75c6
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "achievement contract worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md
  - tools/content-schema/achievement-authoring/**
  - .github/workflows/achievement-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260929-achievement-owner-contract.md
public_contracts:
  - docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md
depends_on: [OTV2-20260929-tibiawiki-achievement-facts]   # evidence only; no file dependency
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner direction (2026-09-29, this session): allocate the Achievement owner (D126) now, with an Oteryn key and
source ids as provenance, secret achievements in the catalogue with a flag, and points taken from the wiki
inside the grade range. The contract candidate fixes the owner, the catalogue record, key identity and
compatibility (D49), source precedence for population, the grant path (request in the granter's fenced
transaction, consumed in the same transaction into a first-commit-wins fact) and points. The schema and
validator implement §2. `content/achievements/`, migrations and runtime are untouched.

## Architect choices for review

- Compatibility is key equality, because a key never changes meaning (§2.3).
- Grant requests are consumed in the same transaction (§3); asynchronous consumption is named as superseding
  evidence, not built.
- A grant for a key missing from the world's catalogue fails the granting transaction closed; content
  validation covers pinned granter revisions, and keys are never removed (§2.1, §3).
- A grant for a retired achievement is a no-op, so the granting transaction continues (§3).

## Owner decisions (2026-09-29, this session)

Resolutions of the 10 join anomalies (contract §2.2): Sculptor Apprentice, Smart Thinking and Sail Away! premium
true; Hell Rider 2 points (Canary and GuildStats); Taskaholic 7 points, provisional; The More the Merrier kept as
`retired` with 0 points; Achievement 563 left out. This added the optional `retired` field before review.

## Review round 1 (exact-head review of ebb0bd02, #1287 comment 5901554400: FIX)

1. MATERIAL_BLOCKER, identity bound to the name: the key is allocated once (`allocate_key`) and kept; the
   validator checks format and uniqueness only; a renamed record keeps its key (test).
2. HARDENING, retired grant aborting a pinned granter: a retired grant is a no-op; an unknown key still fails
   closed, content validation covers pinned granter revisions, and keys are never removed.
3. EVIDENCE_GAP, cited path only in #1286: #1286 merged; `main` merged into this branch.
4. EVIDENCE_GAP, tests not in CI: `.github/workflows/achievement-authoring-schema.yml` runs the validator, the
   tests (including slug parity with `ots_chests.py`, which is in its path filter) and ruff.

## Validation (local)

- `test_validate_achievements.py`: 7 tests pass (the key-slug test replaced by the rename test).
- Evidence fit: with the owner resolutions, 571 candidates from staticdata plus the #1286 wiki facts all validate,
  slugs unique; only `Achievement 563` stays out. 27 of 28 quest-sample refs bind by slug, the 28th explicitly.
- `ruff check`, `ruff format --check`, `validate_governance.py`, `validate_repository_policy.py`,
  `git diff --check`: pass.
- Review: exact-head independent review required (catalogue identity, account-fact grant path).
