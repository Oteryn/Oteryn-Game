# OTV2-20261005-arch-progression-source-0

```yaml
task_id: OTV2-20261005-arch-progression-source-0
title: "ARCH-PROGRESSION-SOURCE-0: Character progression content source and PROGRESSION-OWNER-1 re-issue"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-progression-source-20261005
issue: 162
pr: 1803
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-progression-source-0.md
public_contracts: []
depends_on: []
blocks: [PROGRESSION-CONTENT-1, PROGRESSION-OWNER-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- CP D699. The content source: a Reference-derived finite experience table of 2000 levels
  (`CHARACTER_EXPERIENCE_TABLE_LEVELS`), formula-derived and checked against a tibia.com
  snapshot, plus death policy (1/1, floor), reward policy and declared differences files in
  `rulesets/character/{experience,death}/`, with content-addressed revisions (§1.1, §1.3).
- A World pins them through a new optional native gameplay section `progression`
  (`OTERYN_NATIVE_PROGRESSION/v1`) (§1.2).
- Admission pins nine revisions: six from the World pin, and profile, ruleset and content from
  the Character root. The binding is built per session in `AdmittedSession` and replaces
  `player_death_progression()`. A Character at root revision 1 is initialized eagerly at
  admission. Others are admitted with no binding (non-durable death), with no backfill (§1.3-§1.5).
- Packets: PROGRESSION-CONTENT-1 (content and pin, no `gameplay_transport/` change) and the
  re-issued PROGRESSION-OWNER-1 (composition, after #1798 and PROGRESSION-CONTENT-1) (§2).
- Flagged for owner acceptance (§1.6): A, the pin-schema change; B, revision irreversibility
  until a progression migration owner exists; C, no backfill.
- No code, registry or contract change.
- #1803 Codex round 1 (CP), on `fe664abf`:
  - 4180135223 (P1): a ruleset revision digests the canonical document with its own `revision`
    member removed, so it is not self-referential (§1.3).
  - 4180135231 (P1): the row stores no death policy revision, so `policy_revision` is the digest
    of the experience table and death policy revisions; a death-policy-only change is refused
    with no migration (§1.3, §1.6 B, §2.1, §2.2).
- #1803 Codex round 2 (CP), on `ccf9f61c`:
  - 4180330477 (P1): admission reads the row first; no row past root revision 1, or a
    mismatched row, admits the session with no binding, so death takes the non-durable respawn
    and never retries a write that cannot succeed (§1.5, §2.2).
  - 4180330482 (P2): the evidence digest is the raw bytes of exactly
    `docs/reference/experience-table-20261005/levels.csv`, with its format fixed (§1.3).
  - 4180330490 (P2): the revision tests mutate canonical content, and whitespace or key order
    alone changes nothing (§2.1).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
