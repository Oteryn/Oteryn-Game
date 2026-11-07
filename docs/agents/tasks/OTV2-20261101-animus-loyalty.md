# Animus Mastery and Loyalty completion

Task: `OTV2-20261101-animus-loyalty`
Status: AUTHORING
Branch: `chatgpt/animus-loyalty`
Base: `38641fecd4558a4a14e6b15b3874110ffa2dff19`

## Accepted scope

Complete W08 Animus Mastery production integration and native presentation,
and P13 Loyalty titles and native skills display. Keep the two mechanics in
separate commits. Deliver a draft PR; no merge, review request or heavy-tests
label is authorized by this task.

Use the existing Soulpit and Animus durable owners, the current account
Loyalty projection, and the current native client architecture. Keep all
character access session-generation fenced. Platform owns Loyalty point
earning.

## Reference evidence

Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.

- Soulpit survivors receive mastery at
  `data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106`.
- Duplicate species insertion and XP multiplier:
  `src/creatures/players/animus_mastery/animus_mastery.cpp:33-61`.
- Loyalty titles and bonus brackets:
  `data/libs/functions/player.lua:819-865`.
- Native skills fields:
  `src/server/network/protocol/protocolgame.cpp:9380-9402`.

Fresh inspection of the base finds an existing Soulpit completion transaction
in `crates/gameplay/durability/src/durability/soulpit/progress.rs` that already
calls the Animus owner. The remaining production activity bridge must be
reconciled with its owner; no parallel award implementation is planned.

PR #2793 is unmerged at allocation. P12 character-info files are outside the
writable scope until that dependency lands. A dedicated Loyalty skills view
can reuse the existing durable build and account projection without a new
writer.

## Allocated numbers

The task reserves migrations 0748-0751, capabilities 274-275, commands
488-491, mechanic ordinals and offer orders 628-631, resource entries
1688-1691. Reuse Animus capability 168 and command 232.
Verify reservations against current main and open PRs before use and push.

## Verification

Pending implementation: focused rule and codec tests, real-dispatch
PostgreSQL cases, native client state and action tests, formatting,
changed-package clippy, architecture/registry checks, and diff review.
No test result is claimed by this initial task record.

## Questions

The draft PR conversation is the decision channel. Questions use one
`QUESTION:` comment each; accepted answers use `ANSWER:`. Read current
comments before subsequent implementation steps and final delivery.
