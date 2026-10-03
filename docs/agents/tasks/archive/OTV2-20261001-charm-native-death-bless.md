# OTV2-20261001-charm-native-death-bless

```yaml
task_id: OTV2-20261001-charm-native-death-bless
title: Bless residual reduction in the native PvE death calculation
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-native-death-bless-20261001
pr: 1487
issue: 162
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; passives source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/domain/death.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-death-bless.md
public_contracts: []
depends_on: [CHARM-0, D58, D68]
blocks: []
external_repositories: []
```

## Bounded implementation

The existing PvE death calculation accepts the current Bless residual reduction6/9/12%,
after ordinary blessings/promotion, with one final floor before the held-experience cap.
Existing APIs and input structs retain their behavior through explicit zero-context wrappers;
item-loss selection, item RNG, blessing consumption and amulet handling are unchanged.
No alternate death calculator, source redistribution, schema, protocol or activation change.

The original full nine-row Charm task remains IMPLEMENTING in PR1479/#162. This single-PR
child does not close Bless gameplay: the durable caller requires an owner-issued player
lethal receipt and current assigned killer-race facts. Existing creature bites retain their
accepted floor of1; a creature death receipt cannot substitute for player-lethal provenance.
AuthorityInvariant×ConsumerBoundary×MutationOperator is NOT_APPLICABLE to this pure arithmetic
delta: no session/lease grant, fenced persistence mutation or current authority is reconstructed.

## Validation and integration

RED: all three regressions failed with independently specified wrong values99,834/507,278/713,100.
GREEN: all14 native domain death tests PASS, including item-loss independence and cap/floor order.
Workspace rustfmt, diff check and strict server all-target Clippy PASS. Existing fixed D58
oracles exercise zero-context compatibility. No PostgreSQL/client/gameplay E2E claim.
Root preserves the local Git candidate identity with a normal guarded push and final remote
readback; final-head independent review and CI are recorded in the external FREEZE_SHA packet.
Protected integration is control-plane owned. The archive reaches main only if PR1487 merges;
a commit cannot contain its own final SHA.
