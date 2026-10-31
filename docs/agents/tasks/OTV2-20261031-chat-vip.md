# OTV2-20261031-chat-vip

```yaml
task_id: OTV2-20261031-chat-vip
title: World VIP presence and chat channels
type: mechanic
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: chatgpt/chat-vip
pr: 2780
base_sha: ae4188a9c8d461acb152044814ea23075de02ed4
head_sha: null
final_head_sha: null
owner: chat-vip task lead
created_at: 2026-10-31T18:24:40Z
updated_at: 2026-10-31
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/
  - crates/session/
  - crates/client-ui/src/features/0258-vip-presence/
  - crates/client-ui/src/features/0259-chat-public-channels/
  - crates/gameplay/social-rules/
  - crates/gameplay/social/src/chat/
  - crates/gameplay/durability/src/durability/
  - crates/gameplay/dispatch/src/gameplay_transport/
  - apps/game-server/migrations/0716_vip_presence_unavailable_receipt.sql
  - apps/game-server/migrations/0717_chat_public_roles_help.sql
  - apps/game-server/migrations/0718_private_channel_world.sql
  - docs/contracts/protocol-oteryn/v1/
  - docs/contracts/resource-limits/entries/0421-CHAT0-RL-07.json
  - tools/architecture-check/client-unsupported-baseline.d/
public_contracts:
  - docs/contracts/protocol-oteryn/v1/vip-presence-v1.md
  - docs/contracts/protocol-oteryn/v1/chat-private-channel-world-v1.md
  - docs/contracts/protocol-oteryn/v1/chat-public-channels-v1.md
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Extend existing VIP, CHAT, World relay, admission and receipt owners for S04a/b/c.
The source and transport path support consent-aware VIP presence/notifications,
private channels across Channels/nodes in one World, the missing public rooms,
and Tutor Help moderation. PM between nodes has an actual wire/relay test.
The live draft PR records the final candidate SHA and exact-head qualification;
this task record does not claim merge, native UI delivery or CI success.

## Architecture and source of truth

- PROVEN: primary Crystal `201189f871d23597b82caf88fd611454a2ddefb8`;
  secondary Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- PROVEN: accepted VIP0 consent/privacy, 10-second ONLINE confirmation and
  60-second reconciliation remain the native authority. Hints never grant visibility.
- PROVEN: coordinator answers in PR #2780 authorize the Control role projection
  (6547489522), bounded per-session presence reads (6547776265), and reuse of
  existing command/route owners (6547817666).
- PROVEN: S19 was removed by coordinator comment 6547438264 because #2777 already
  delivered capability 233/command 447. This branch does not merge that main change.
- PROVEN: coordinator answer 6548400568 defers Guild/Staff labels and public-room
  TALKTYPE presentation; channel access, unchanged text and typed SDK remain in scope.
- DERIVED: the new payload bounds follow existing name/list bounds and varint widths;
  shared chat queue capacity stays 64, private invitation bounds stay 16/16.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current recovery, node incarnation and Channel assignment
  - exact GameSessionId, connection generation and character lease
  - current World scope, selected capability and actor binding
  - per-observing-character consent and target visibility
  - Control-issued role facts and native Guild membership
  - private membership, join time, owner lifecycle and channel epoch
consumer_boundaries:
  - wire admission and existing command dispatch
  - prepare, commit and retained-result replay
  - relay prequeue hints and authoritative queue custody
  - reconnect, terminal retirement and client confirmation expiry
mutation_operators:
  applicable:
    - absent capability, consent or role
    - stale lease or connection and mismatched World
    - changed request under the same command key
    - old asynchronous completion after reconnect or forget
    - delayed delivery before join or from an old channel epoch
    - listener loss, restart and replay after refusal conditions change
    - exact timeout boundary and revoked authority
  considered_not_applicable:
    - client-issued roles or timestamps; neither is accepted by these commands
one_invariant_per_negative_case: true
independent_current_fact_sources:
  - committed fresh admissions and independently registered nodes
  - current PostgreSQL session, consent, visibility, role and membership rows
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: completed
  protocol_versions: completed
  direct_and_reconciled_paths: completed
  fenced_durable_writes: completed
  restart_retry_replay_concurrency_pg_reload: completed
  evidence:
    - postgres_tests/private_messages.rs
    - postgres_tests/private_channel_world.rs
    - postgres_tests/vip_presence.rs
    - postgres_tests/public_channels.rs
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - old-connection cache and queue consumption after replacement
    - refusal results changing after listener or moderation recovery
    - logout notification depending on target CHAT negotiation
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - per-session SQL batching accepted by coordinator as a performance follow-up
    - Guild/Staff labels and public-room TALKTYPE presentation deferred by answer 6548400568
```

## Acceptance criteria

- [x] Existing protocol/command/route owners remain the sole production paths.
- [x] New public rooms and VIP presence are negotiated, bundle-World-only extensions.
- [x] Source-mapped rules, refusals, replay and boundary tests are implemented.

The immutable candidate and exact-head qualification are maintained in the live
PR; this record remains validating for coordinator integration.

## Reference parity

All Crystal paths below refer to the immutable primary revision above. Detailed
values and test mappings are retained in the three public integration contracts.

| Behaviour | Reference value/source | Oteryn | Test |
|---|---|---|---|
| VIP login/logout | `src/creatures/players/vip/player_vip.cpp:48-66` | Existing consent/visibility plus coarse World presence | `vip_presence_postgres_cross_node_login_logout_consent_and_privacy` |
| Private channel creation/access | `src/creatures/interactions/chat.cpp:39-54,458-472,529-545,707-709` | Premium, one channel, IDs 100..9999, invited/joined membership | `private_channel_postgres_world_lifecycle_fences_atomic_replay_and_epoch` |
| Cross-Channel invitations/exclusion | `src/game/game.cpp:3862-3905` | Existing World name/session owners and encrypted relay | Same private-channel PostgreSQL case |
| PM between nodes | `src/game/game.cpp:6861-6887` | Existing PM command, two real assigned nodes and wire ingress | `private_messages_postgres_cross_node_wire_delivery_replay_and_world_boundary` |
| Public channel access | `data/chatchannels/scripts/advertising-rook.lua:1-3`; `tutor.lua:1-3`; `gamemaster.lua:1-3`; `guild_leaders.lua:1-3` | Independent group/account/access/Guild facts | `public_channels_donor_access_uses_distinct_group_account_and_guild_facts` |
| Advertising timers | `advertising.lua:1-23`, `advertising-rook.lua:1-23` | Independent 120-second intervals and donor staff exceptions | `public_advertising_intervals_are_independent_and_gamemasters_bypass_them` |
| Help moderation | `help.lua:3-60`; `src/creatures/interactions/chat.cpp:216-228,588-610` | Separate 180-second speech and one-hour condition deadlines | `public_help_mute_preserves_both_donor_deadlines_and_strict_rank`; public-channel PostgreSQL case |
| Public presentation (deferred) | `guild_leaders.lua:5-20`; `tutor.lua:5-21`; `gamemaster.lua:5-21`; both advertising scripts `:11-17,30-40`; `help.lua:63-81`; `world_chat.lua:8-21`; `english_chat.lua:7-21` | Guild/Staff labels and Y/O/R1 conversions deferred by answer 6548400568 | No presentation parity claimed; SDK capability/state guards tested |

## Excluded scope

S19 is already delivered separately. External Control role issuance UI/tooling
is excluded by the coordinator. Node-wide presence SQL batching is an accepted
performance follow-up. Answer 6548400568 defers Guild Leaders `[Guild]`, `[Staff]`,
`[Staff][Guild]` labels and TALKTYPE Y/O/R1 conversions for Guild Leaders, Tutor,
Gamemaster, Advertising, Advertising-Rookgaard, Help, World and English. Native UI
for capabilities 258/259 and the pre-existing capability-80 private-channel panel
remains explicitly unsupported; typed protocol/session consumers are implemented.
No chat history/offline mailbox is added. Chat remains ephemeral and at most once,
without recipient acknowledgements. Lines authorized before exclusion/close may
finish an in-flight socket write afterward.

## Implementation / findings

Use migrations 0716-0718 and capabilities 258-259. Existing VIP commands 48-50,
CHAT_INTENT 13, capability 80 and CHAT domain 12 remain the owners. Reserved
0719, commands 486-489 and mechanic ordinals/offer orders 626-629 are unused.
Migration 0716 extends the existing VIP writer's receipt path with a denial-only
selector; 0717 supplies the audited Control projection and Help state; 0718 supplies
World private-channel metadata and receipts. No merged migration is edited.

## Validation

Authoring checks and final exact-head results are recorded with full commands in
PR #2780. Four focused PostgreSQL cases use isolated PostgreSQL 18.6 and print
completion markers only after all assertions pass. Broad package tests without
the PostgreSQL variable do not count as durable-transition evidence.

Authoring evidence:
- Formatting, registry fragments, architecture, governance and governance tests PASS.
- All-target/all-feature clippy for all eight changed packages PASS.
- Protocol 595 tests and session 94 tests PASS after updating two strict fixture
  expectations for the newly registered deltas. The other six changed packages
  passed the broad authoring run; the final aggregate run is recorded in the PR.
- All four new mounted PG cases PASS with completion markers. VIP includes real
  terminal persistence without target CHAT and listener restart/refusal replay.
- Existing signed-staff PG and dynamic CHECK-function privilege PG both PASS.
- Eleven focused SDK cases PASS; no existing assertion was weakened or newly ignored.
- Allocation check PASS against freshly fetched main `60f2542a5f`; the entire
  reserved block remains free and no open-PR number collision remains.

These authoring results are not substituted for final-head evidence. The live
PR records every exact command, immutable candidate, final result and CI status.

## Self-review

The task lead reviews all shared integration changes and the complete changed-file
list/diff; three disjoint implementation lanes cross-check privacy, authority,
receipt stability, queue custody and wire compatibility. Final exact head and
verdict are recorded in the PR after the last authoring commit.

## Independent review

An external review request is excluded by the owner's draft-only handoff. Internal
cross-review supplies authoring feedback and does not grant integration authority.

## PR and closeout

Canonical DRAFT PR: https://github.com/Oteryn/Oteryn-Game/pull/2780.
No unanswered QUESTION remains. No review request, heavy-tests label, main merge,
rebase, force-push or protected integration is authorized. The coordinator owns
subsequent integration and post-merge qualification. A commit cannot contain its
own SHA; immutable final-head evidence belongs in the PR, not a follow-up commit.
