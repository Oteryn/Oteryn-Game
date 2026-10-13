# OTERYN-20261013-npc-cmd-1a

```yaml
task_id: NPC-CMD-1a
title: "NPC-CMD-1a: NPC_TALK_INTENT (command 7) and domain 7 NPC_CONVERSATION on bundle Worlds"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-cmd-1-20261013
pr: 2030
base_sha: null
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01GVsJCTNfwAsCUr72htmidv
control_plane: "#1622 (plan comment 6139230459)"
governing_issue: 1622
issue: 1622
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_command.rs
  - apps/game-server/src/gameplay_transport/npc_command_tests.rs
  - apps/game-server/src/gameplay_transport/npc_talk.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/node/serve/bundle_npcs.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/agents/tasks/active/OTERYN-20261013-npc-cmd-1a.md
public_contracts: []
external_repositories: []
```

## Outcome

On a bundle World with a loaded NPC catalogue, capability 3 `NPC_SERVICE_V1` is offered and
command 7 is served: a talk intent within 4 tiles is answered by the shared `npc_talk::talk` rule
and the reply goes to the speaker as a whole domain 7 delta after the command result. Domain 8 is
a closed window; command 8 is `REJECTED` until NPC-CMD-1b. No wire change.

## Architecture and source of truth

- PROVEN: NPC-0 section 4 (commands 7/8, domains 7/8, `NPC0-RL-04`, `RL-07`); NPC-TALK-1 rule.
- DERIVED: offer follows the capability 18 precedent (bundle-World-only set, registry `offered`
  stays false with an updated `offer_gate`).

## Excluded scope

NPC-CMD-1b: trade (domain 8 window, command 8, migration, `NpcTradeCause`, BUY/SELL transaction).

## Acceptance criteria

- [x] Greeting, topic, farewell, NoConversation, NotInRange, Stale, RateLimited, Rejected
  (`npc_command_tests`).

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib gameplay_transport`: pass (515 passed).
- `cargo test --locked -p oteryn-game-server --lib npc`: pass.
- `python3 tools/agents/validate_governance.py`: OK (22 documents, 9 lanes).

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy; full Codex review on the PR.

## PR and closeout

- changed-file review: within owned paths
