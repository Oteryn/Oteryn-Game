# PARTY-XP-1 — party shared experience and party spells (skeleton task record)

```yaml
task_id: PARTY-XP-1
title: "PARTY-XP-1: party shared experience and party spells"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PARTY-XP-1; §7 'PARTY-XP-1' (PARTY_PVP0 ACCEPTED)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/party-xp-1
needs_migration: false
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Parties share experience under PARTY_PVP0 rules (level range, activity) and party spells work with feedback through CHAT-2.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Single row; feeds BOSS-REWARD-1 (MONSTER-META-1).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PARTY-1 | BLOCKING (Wave 2, not merged) |
| D118 XP lane (LOOP-PROGRESS-1) | BLOCKING (Wave 1, not merged) |
| D3-4 (LOOP-DAMAGE-1) | BLOCKING (Wave 2, not merged) |
| CHAT-2 (party spell feedback) | BLOCKING (CHAT-2B #2128 open) |
| PARTY_PVP0 | ACCEPTED (plan §7 status list) |
| DEATH-1, ATTACK-1b, COND-1, SPEED-1, VIS-2, ATTACK-WIRE-1 | MERGED (archive records, #1798, #2122) |
| DEATH-3 | BLOCKING (death module batch C, not merged) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/world_party.rs`
- `apps/game-server/src/durability/world_party_tests.rs`
- `apps/game-server/src/gameplay_transport/party_spell_owner.rs`
- `apps/game-server/src/gameplay_transport/party_xp.rs`
- `apps/game-server/src/gameplay_transport/party_xp_tests.rs`
- `apps/game-server/tests/party_xp_postgres.rs`
- `docs/agents/evidence/PARTY-XP-1-*.md`
- `docs/agents/tasks/PARTY-XP-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/kill_reward.rs (XP split hook)
- D118 XP lane writer (LOOP-PROGRESS-1)
- party_spell_owner.rs spell registration
- GAME_EVENT_FOUNDATION_REGISTRY.json

## Inseparable bundles (never split across PRs)

- XP split + kill reward + audit in one transaction
- party spells + bundle data + tests

## Acceptance criteria

- [ ] Shared XP split exactly once per kill, only to eligible active members.
- [ ] Ineligible members (range/activity) receive solo-rule XP or none per brief.
- [ ] Party spells affect only party members; feedback via CHAT-2.
- [ ] Concurrent kills serialize per Character XP lane.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server party_xp
cargo test -p oteryn-game-server world_party
cargo test -p oteryn-game-server kill_reward
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md
- docs/reference/spells/r21-local-candidate/bundles/instant-*_party/
- PR #1798
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation (2026-10-15; incomplete integration)

Exact authoring base: `3789b07c449ca016335f733b61ec299e7a3aab7d`,
`skeleton/party-xp-1`, draft #2172. The allocation narrows writes to its seven paths;
its prohibition on shared registrations overrides the historical registration allowance above.
Accepted owning input: PARTY_PVP0 §5.1/§13, resolved by the coordinator on
`main@3ef900c160c9713092a86bfcd1a8377c7587c898`; D118 §2.4 amendment fixes
party → per-character Prey → per-character stamina order.

Source inspection found real residue: native PARTY-1 membership/strong reads and
party-spell selection exist, but no shared-XP activity, eligibility or equal split.
Migration 0046 already supplies `game_parties.shared_xp_enabled`; no schema is invented.
The shared `PartySnapshot` deliberately remains unchanged. The new same-transaction
read returns that toggle and the native membership predecessor under existing World
and party/member locks. The command variant reuses independently checked
`SpellItemAuthority`; the autonomous scope-actor variant reuses native
`SpellItemScopeAuthority` and current colocated actor resolution, without a synthetic command.
The read writes no receipt, revision, resource or event.

The existing source party owner now retains a separate XP projection of every actual
colocated member, including a dead member omitted by spell-target selection. Each
Character/session association comes from its existing native journal resolution, each
level/position from actual player state/runtime. Spell target selection stays intact.
The XP consumer rejects a known retired runtime and changed physical/player predecessors,
and compares the strong membership predecessor. This does not independently prove
current database/session/visibility authority after locks or the SQL transaction end.

The detached XP module provides one bounded activity cell per Character/session on a World/Channel for actual qualified
healing or aggressive-monster attack events;
50-principal bound; 120-second inclusive freshness; all-participant 30-tile Chebyshev
range on any floor; exact two-thirds level test; leader-presence/toggle/PZ conditions;
and equal shares rounded up without `pool + count - 1` overflow. Foreign-Channel
members do not participate or disable sharing. A failing participant disables sharing
for the entire kill; missing/mismatched source inputs refuse planning. Empty/restarted
activity grants nothing. No partial XP write or redistribution occurs here.

The integer pool input is explicitly **after admitted party bonus and summon deduction**,
before the equal split. The source has a private base-vocation normalizer in an unowned
module, and the native kill writer has no integrated party/summon calculation. This
preparation does not duplicate that normalizer or invent intermediate rounding. It does
not implement bonus calculation, summon accounting, Prey, stamina or soul awards.

### Current-authority and CP integration gates

- The real kill owner must hold current runtime/player locks, independently current
  session/lease/node/Channel/World fences and the source transaction through planning
  and commit. A matching stored snapshot or supplied deadline is never a current grant.
- Read the toggle/membership through the new real SQL port in that same transaction;
  construct the source physical view using the existing current colocated actor journal
  resolution. Recheck actual predecessors at the consuming owner boundary.
- Produce activity only from committed, qualified events: a heal needs its actual source
  party member target; an attack needs aggressive-monster qualification and may be solo.
  Use the committed event's actual
  semantic timestamp; retries keep that timestamp and never refresh activity as a new event.
  No client flag or arbitrary enum value proves those facts. Keep each cell per
  Character/session on a World/Channel; replaced sessions use a fresh cell and recovery
  starts empty. Party changes preserve recent activity, including a solo aggressive attack
  before joining. These event hooks remain absent.
- Supply the current leader's actual PZ deadline under its lock for that same semantic
  instant. The private native `CombatLock` is not a production accessor here; its
  test-only re-export is not used. The logout/battle block is never substituted.
- Add the actual leader-authorized shared-XP toggle command through PARTY-1's owner
  revision/receipt/event rules. The new port only observes the existing stored setting.
- Compose the accepted vocation bonus and summon share first, then this split, then
  each character's Prey and stamina. Use deterministic CharacterId lock order and native
  Character sequencers/receipts to atomically settle each death exactly once; retry,
  concurrent kills and recovery must not duplicate awards. The shared kill writer,
  actor events, registrations, audit/feedback and CHAT-2 remain slot-only work.

No content, imports, pins, migrations, registry, shared module or production wiring
is edited. Party buff/Serene source behavior already exists; its accepted content and
CHAT-2 feedback remain their owning integration, not a second spell implementation.
The skeleton dependencies/checkboxes are not represented as completed by this subset.

### Validation and evidence

Actual native qualification: **15 passed, 0 failed, 0 ignored**: 13 owned rule
bodies and two PostgreSQL 17.6 strong-read cases. The original, untransformed
corrected unit sources pass strict native Clippy metadata; normal cfg(test)-off native
library Clippy and owned/workspace formatting also pass. The command-authority read is executed; the autonomous scope-actor read is
compile/lint qualified only. No full-runtime retirement/current-source, PZ/activity
producer, kill receipt, exactly-once/concurrency/recovery or playable E2E is claimed.

Preliminary review found an unaccepted PartyId reset losing recent solo activity on
joining. **Fixed**: one per-Character/session+World/Channel cell; the native regression
covers solo attack → joining → party change, solo-heal refusal, and immutable cell
binding. Earlier Clippy evidence covers intermediate bytes only. Its unstarted bridge
was cancelled without executing tests. The first full lib-test codegen SIGKILL and
cfg(test)-off fixture setup failure remain explicit resource/setup evidence, not REDs.

The approved final route builds the actual native cfg(test) library without the global
unit-test harness. A disclosed outside-tree bridge removes selected test attributes, exposes the unchanged
bodies and adds forwarding exports for a tiny driver. Its PG helper excerpt adds only
an unused-import allowance; every transformation is manifested. Native
private types, genuine recovery/session/node fixtures, SQL authority checks and PG
migrations remain actual source. Original-source strict unit-context Clippy runs with
normal test attributes intact; no fabricated SourcePartyWorld or authority type exists.

Evidence, manifests, exact native commands and hashes live at
`/workspace/oteryn-prep/party-xp-1-evidence`. `corrected-tests.log` contains the actual
15-case output; `corrected-bridge-manifest.json` and `corrected-bridge-only.diff` disclose
all bridge mutations. Five final Rust hashes match the untransformed native snapshot.
The unchanged accepted §5.1/resource rows and D118 §2.4 plus seven native inputs are
bound to exact accepted main hashes. `self-review.md` records the ownership/provenance
limits. Obsolete snapshots and completed private rlib/ELF are retained losslessly under
`owned-compression.json`; no shared-target dependency was manually edited or deleted.

Final governance: validator PASS (22 documents/9 lanes), 124/124 governance unit tests
PASS. Exact six-path ownership, unchanged shared registrations/push deny, no staged
changes or commits, and whitespace checks PASS. The exported patch is verified against
the exact authoring base; this packet stays a draft preparation pending CP/architect
review and the integration gates above.
