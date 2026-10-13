# DEATH-WIRE-1

```yaml
task_id: DEATH-WIRE-1
title: "DEATH-WIRE-1: first player death defects closed; DEATH-3 waived (D1022)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death-wire-1-20261013
issue: 1622
governing_issue: 1622
pr: 2028
base_sha: 0db5c53b
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of #2028"
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md
  - docs/agents/tasks/archive/DEATH-WIRE-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Read first: DEATH-0, DEATH-2/2b, DEATH-RESTART-SAFETY-1 (#2004) and ACCEPT-CHAT-ATTACK-DEATH-0
(#1994). The 2026-10-12 review of the first player death found two defects: DUR-03 item loss
(DEATH-3) and restart safety. Restart safety is already fixed on `main` by #2004 and is not
redone. D1022 (owner, 2a) fixed the order: restart-safety fix, then acceptance with a DEATH-3
waiver. The waiver means no item loss, amulet consumption, corpse or lost-item set is wired:
`character_death.rs` still rejects them (`23514`), and a lost backpack cannot be split before
container expansion. Wiring them would change the death receipt contract (D1022 forbids), so this
task records the resolution and the waiver in the decision and leaves DEATH-3 open (§5).

## Acceptance criteria

- [x] The decision names the #2004 fix and the D1022 waiver; no rule, wire or code change.
- [ ] Independent re-review, then the owner acceptance (outside this worker).

## Validation

`python3 tools/agents/validate_governance.py`, `python3 -m unittest discover -s tools/agents/tests` (60 passed), `git diff --check`: pass. Documentation only.

## Self-review

Whole-diff read; the wording matches §4.4 "Delivery gap" and the #2004 record. No open finding.
