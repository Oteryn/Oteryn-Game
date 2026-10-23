# PARTY-XP-1 — party shared experience and party spells (consolidated task card)

```yaml
task_id: PARTY-XP-1
title: "PARTY-XP-1: party shared experience and party spells"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PARTY-XP-1; §7 'PARTY-XP-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
base_branch: main
branch: null
pr: null
class: hard
needs_migration: false
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2192](https://github.com/Oteryn/Oteryn-Game/pull/2192), [#2280](https://github.com/Oteryn/Oteryn-Game/pull/2280). Existing shared-experience pure rules and party-spell owner view remain on main; the new multi-principal settlement draft is not retained. Evidence: `crates/gameplay/social-rules/src/shared_experience.rs`.

Remaining: Real shared-XP kill settlement, toggle ingress/activity/PZ qualification, Prey/stamina ordering, content/feedback and replay/concurrency PG cases remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2172, branch skeleton/party-xp-1

Status note: the draft added a detached shared-XP planner (`gameplay_transport/party_xp.rs`,
501 lines + 801 test lines) and a `plan_shared_xp_in_transaction` read on the pre-ADR-0022
`apps/game-server/src/...` layout; 15 native cases passed (13 rule bodies + 2 PG 17.6
strong reads) through a disclosed out-of-tree bridge, so no in-tree execution qualifies it.
Its World-owner hunks were inverted to PARTY-1 (CP D1101). No kill-settlement caller, PZ
accessor, toggle command or feedback was wired. It is closing; the mechanic is re-implemented
fresh on the current layout. Feeds BOSS-REWARD-1 (MONSTER-META-1).

## Goal

Parties share experience under PARTY_PVP0 §5.1 (level range, activity, leader/PZ) inside the
real kill transaction; party spells affect only party members with feedback through CHAT-2.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| PARTY_PVP0 acceptance | ACCEPTED (plan §7); D118 §2.4 amendment fixes party → Prey → stamina order |
| PARTY-1 (World owner, `shared_xp_enabled` reads) | MERGED (#2201; card still active — toggle command: verify at allocation) |
| D118 XP lane (LOOP-PROGRESS-1) | MERGED (#2192, skeleton) |
| LOOP-DAMAGE-1 (D3-4) | MERGED (#2195, bounded checkpoint) |
| CHAT-2B relay (party spell feedback) | MERGED (#2227) |
| DEATH-3 | waived by D1022 (#2028); verify at allocation |
| DEATH-1, ATTACK-1b, COND-1, SPEED-1, VIS-2, ATTACK-WIRE-1 | MERGED (archive records, #1798, #2122) |
| Party bonus (vocation normalizer) and summon share accounting | BLOCKING (no accepted owner on main) |
| Leader PZ production accessor | BLOCKING (`combat_lock_fact.rs` read is `pub(super)`; verify reuse at allocation) |
| HUNT-1 per-Character Prey (after the split) | BLOCKING — re-implementation pending (draft #2149 closing) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/world_party.rs` (1037 lines) —
  `WorldPartyXp`, `read_world_party_xp_in_transaction`,
  `read_world_party_xp_for_scope_actor_in_transaction` (toggle + strong membership read).
  PARTY-1 owns it; edits here are wiring-only.
- `crates/gameplay/social-rules/src/shared_experience.rs` — 120 s activity window,
  `committed_shared_experience_activity`, `heal_credits_party_member`.
- `crates/gameplay/dispatch/src/gameplay_transport/attack.rs` — existing activity recording
  (`record_committed_shared_experience_batch`); reuse, do not duplicate.
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs` (1213 lines) and
  `crates/gameplay/combat/src/combat/death_reward.rs` (1002 lines) — the kill transaction;
  today single-principal. Over 1000 lines: call into a new settlement file, ≤ ~150 lines growth.
- `crates/gameplay/dispatch/src/gameplay_transport/party_spell_owner.rs` — party spell
  selection (target selection stays intact).
- `crates/gameplay/dispatch/src/gameplay_transport/combat_lock_fact.rs` — leader PZ fact source.
- `crates/gameplay/entities/src/carrier/runtime_actor_party.rs`,
  `crates/gameplay/social-rules/src/party.rs` — party snapshot (unchanged).
- `crates/gameplay/dispatch/src/lifted_tests/durability_world_party_tests.rs`,
  `apps/game-server/tests/support/attack_kill_reward_postgres_cases.rs`,
  `apps/game-server/tests/combat_death_reward_postgres.rs` — existing PG patterns.
- `apps/game-server/migrations/0046_world_parties.sql` — `game_parties.shared_xp_enabled`
  already exists; no schema is invented.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/social-rules/src/shared_experience_split.rs` (new) +
  `shared_experience_split_tests.rs` (new) — L3 pure eligibility and equal split.
- `crates/gameplay/dispatch/src/gameplay_transport/party_xp.rs` (new) —
  `plan_shared_xp_in_transaction` (same-transaction read + planner).
- `crates/gameplay/dispatch/src/gameplay_transport/party_kill_settlement.rs` (new) + tests
  (new) — atomic multi-principal settlement called from the kill transaction.
- `apps/game-server/tests/party_xp_postgres.rs` (new) — PG cases (or mount via an existing
  aggregate target; mounted once).

## Accepted design (PARTY_PVP0 §5.1/§13, D118 §2.4, as built in the draft)

Planner (pure data):

- One activity cell per Character/session on a World/Channel for committed qualified healing
  (of an actual party member) or aggressive-monster attack events; replaced sessions get a
  fresh cell; recovery starts empty; party changes keep recent activity (incl. a solo attack
  before joining).
- 50-principal bound; 120 s inclusive freshness; all participants within 30 tiles Chebyshev
  on any floor; exact two-thirds level test (no float); leader presence, toggle on, PZ
  conditions.
- Equal shares rounded up without `pool + count - 1` overflow (wide-integer oracle up to
  `i64::MAX`). Foreign-Channel members neither participate nor disable sharing. One failing
  participant disables sharing for the whole kill. Missing/mismatched inputs refuse planning.
- Pool input is after admitted party bonus and summon deduction; then the split; then each
  Character's Prey and stamina.

Integration obligations:

1. The real kill owner holds runtime/player locks, current session/lease/node/Channel/World
   fences and the transaction through planning and commit; a stored snapshot is never a grant.
2. Toggle/membership read via the existing `world_party.rs` port in the same transaction;
   predecessors rechecked at the consuming owner.
3. Activity uses the committed event's original semantic timestamp; retries never refresh it.
4. Deterministic CharacterId lock order and native Character sequencers/receipts settle each
   death exactly once; retry, concurrent kills and recovery never duplicate awards.
5. Leader-authorized shared-XP toggle goes through PARTY-1's revision/receipt/event rules.
6. Party buff/Serene behaviour already exists; content and CHAT-2 feedback are its integration,
   not a second spell implementation.

## Review dispositions (from the draft)

- Preliminary review: PartyId reset lost recent solo activity on joining → one cell per
  Character/session + World/Channel.
- D1101: World-owner hunks inverted; PARTY-1 is sole owner of `world_party.rs`.
- Out-of-tree bridge evidence and codegen SIGKILL are not qualification; re-run in-tree.

## Remaining work

Real shared-XP kill settlement, toggle ingress/activity/PZ qualification, Prey/stamina ordering, content/feedback and replay/concurrency PG cases remain.

## Acceptance criteria

- [ ] Shared XP split exactly once per kill, only to eligible active members.
- [ ] Ineligible members (range/activity/level) disable sharing per §5.1.
- [ ] Party spells affect only party members; feedback via CHAT-2.
- [ ] Concurrent kills serialize per Character XP lane.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- XP split + kill reward + audit in one transaction
- party spells + bundle content + regeneration + tests

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-social-rules shared_experience
cargo test --locked -p oteryn-gameplay-dispatch party_xp
cargo test --locked -p oteryn-game-server --test party_xp_postgres -- --test-threads=1  # PG 17.6
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- PARTY_PVP0 §5.1, §13, PARTYPVP0-RL-06; D118 §2.4
- `docs/reference/spells/r21-local-candidate/bundles/instant-*_party/`; PR #1798
- Completion plan §4, §5 (Wave 3), §7; coordination #2131 (D1101, D1102)
