# OTV2-20261031-soulpit-echo-warden

```yaml
task_id: OTV2-20261031-soulpit-echo-warden
title: Soulpit encounters and Echo Warden parity
type: mechanic
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/soulpit
pr: null
base_sha: a52549e3b9230c7486a24df66f4e8ec59d1f96cb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: soulpit-lead
created_at: 2026-10-31T12:00:00Z
updated_at: 2026-10-31T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/domain/src/domain/soulpit.rs
  - crates/gameplay/domain/src/domain/echo_warden.rs
  - crates/gameplay/dispatch/src/gameplay_transport/soulpit
  - crates/gameplay/dispatch/src/gameplay_transport/echo_warden
  - crates/gameplay/durability/src/durability/soulpit
  - crates/gameplay/durability/src/durability/charm_state
  - apps/game-server/migrations/0652-0655
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implement W07 Soulpit and the confirmed Global W06 Echo Warden mechanic through
the existing native rules, dispatch, custody, session fences and charm owners.
The initial commit opens the requested draft PR and its coordinator question
channel; it does not claim any playable implementation or passing Rust tests.

## Architecture and source of truth

- PROVEN: starting main is `a52549e3b9230c7486a24df66f4e8ec59d1f96cb`.
- PROVEN: the task pins Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- PROVEN: official Echo Raids announcement: https://www.tibia.com/news/?id=8834&subtopic=newsarchive.
- PROVEN: bound organization policy is version 3.1.0 at
  `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- UNKNOWN: the shared encounter handoff sufficient for Soulpit; inspect the
  accepted instance owners and ask the coordinator if a new handoff is needed.
- Separate mechanical rules from runtime authority. WorldId and ChannelId
  remain distinct; mutations require live session-generation evidence.

## High-risk authority/recovery qualification

Applicable: item consume/mint, encounter transitions and charm rewards.
Required negative cases: mismatched character/world/channel/session, stale
generation, replay with different intent, duplicate transition, interrupted
fight, concurrent same-species reward and terminal replay. Each refusal must
preserve prior custody and balances. The production runtime, not client input,
supplies creature deaths, random draws and encounter outcomes.

## Acceptance criteria

- [ ] Four Soulpit waves, stack modifiers, entry/exit and terminal paths.
- [ ] Durable fenced core, prism and exalted-core custody with replay receipts.
- [ ] Echo death trigger, delayed portal, raid and first-species charm reward.
- [ ] Existing server consumers read the committed encounter state.
- [ ] Focused rule tests and real dispatch PostgreSQL transition cases.
- [ ] Required formatting, clippy, registry and architecture checks.
- [ ] Exact command results and open decisions recorded in the draft PR.

## Reference parity

| Behaviour | Reference | Oteryn | Test |
|---|---|---|---|
| Soulpit waves and stacks | Crystal lib/others/soulpit.lua and quests/soulpit/soulpit_fight.lua | Pending | Pending |
| Core/prism custody | Crystal actions/soulpit and quests/soulpit scripts | Pending | Pending |
| Echo raid | Crystal lib/core/echo_warden.lua and custom/echo_warden scripts | Pending | Pending |
| Species charm reward | Crystal src/io/iobestiary.cpp:422-478 | Existing charm owner extension pending | Pending |

## Excluded scope

Animus Mastery W08 implementation, unrelated boss mechanics, deployment,
production activation, merge, review requests, heavy-tests labels, vendor,
workflows, repository protection and the shared re-audit document.

## Implementation / findings

AUTHORING. Three workers own disjoint Soulpit rules, Echo rules and durable
storage; the lead owns dispatch/protocol integration and all publication.
Migrations 0652-0655, capabilities 236-237, commands 452-455 and mechanic
ordinals/offer orders 592-595 are reserved by the task and must be checked
against main and open PRs before use.

## Validation

Pending. Local cloud network proxy is unavailable and Rust is not installed.
The authorized PC has Rust and PostgreSQL; use an isolated verification
checkout and retain exact results. Do not count route-less PostgreSQL cases
as database execution.

## PR and closeout

One draft PR against main. Questions use individual `QUESTION:` comments;
read `ANSWER:` comments before each new step. No merge or reviewer request.
