# OTV2-20261003-prof-test-key

```yaml
task_id: OTV2-20261003-prof-test-key
title: "PROF-TEST-KEY-1: synthetic Item keys in proficiency codec fixtures"
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-test-key-20261003
pr: 1648
base_sha: d75ba6d
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
merge_commit: "squash merge of #1648"
owner: claude-code-session-019fDXTqUqbneB5LCpYj4iFB
control_plane: session_013KJX6mv8LQveCKKXYgAX94
decision: "D320 1b"
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - apps/game-server/src/durability/character_proficiency.rs
  - docs/agents/tasks/archive/OTV2-20261003-prof-test-key.md
```

## Outcome

`main` failed the G4 step "No Item key reference is retired or dangling (A12 §5)" with
`DANGLING_KEY:apps/game-server/src/durability/character_proficiency.rs:oteryn:item.tibia.i1`.
That key was introduced by #1629.
The codec fixtures now use `oteryn:item.synthetic.i{n}`, the synthetic `apps/` test form that
`tools/content-census/item_key_references.py` documents as allowed. The invalid-definition case
keeps a wrong-family Item key. The pinned command-binding SHA-256 vector was re-derived
independently from the v1 framing and equals #1607 commit adc0fa4a. The guard and `content/**`
are unchanged.

#1607 also touches this file. The CP said it is held and has no active writer, so whichever PR
merges second resolves the conflict.

## Validation

- `cargo test -p oteryn-game-server --lib durability::character_proficiency`: 20 passed.
- `cargo fmt --all -- --check`, `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean.
- `item_key_references.py`: this file contributes no errors. The 14 `RETIRED_KEY` errors that
  remain belong to D320 1a.
- Review: CP-triggered, pending at freeze.
