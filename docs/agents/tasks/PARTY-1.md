# PARTY-1

```yaml
task_id: PARTY-1
title: "PARTY-1: party membership and state (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/party-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 PARTY-1 (row 112); App. 7 PARTY/PVP; wave 2"
class: "hard (shared state)"
migration: yes
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/world_party.rs
  - apps/game-server/src/foundation/runtime_actor_party.rs
  - apps/game-server/src/party/ (new)
  - crates/protocol-oteryn/src/party*.rs (new)
  - apps/game-server/tests/party_*.rs (new)
slot_only_paths:
  - migration number
  - connection.rs
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "CHAT-2: BLOCKING: CHAT-2A merged #2076; CHAT-2B #2128 / 2C in flight"
  - "PARTY_PVP0: CONFLICT: plan §7 lists ACCEPTED, decision file Status line CANDIDATE; CP verifies"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Party invite/join/leave, succession, shared-XP toggle, `PartyView` cache (membership only).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CHAT-2 | BLOCKING: CHAT-2A merged #2076; CHAT-2B #2128 / 2C in flight |
| PARTY_PVP0 | CONFLICT: plan §7 lists ACCEPTED, decision file Status line CANDIDATE; CP verifies |

**BLOCKING:** CHAT-2; PARTY_PVP0. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/durability/world_party.rs`
- `apps/game-server/src/foundation/runtime_actor_party.rs`
- apps/game-server/src/party/ (new)
- crates/protocol-oteryn/src/party*.rs (new)
- apps/game-server/tests/party_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- connection.rs
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `trait FreshAdmissionAuthority` (l.331): new authority method (pattern: `npc_trade` l.696, `fight_modes` l.804)
- `connection.rs` `serve_admitted_actor` (l.1396): command-type dispatch arm and delta/snapshot frame (pattern: `quest_log_delta` l.3574, `chat_line_delta` l.3595)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`
- `gameplay_transport/party_spell_owner.rs` reads membership

## Inseparable bundles

wire+registry+capability+handler+tests; migration+writer+durability.

## Acceptance criteria

- [ ] Membership consistent across nodes; leader succession deterministic.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0 §30-33. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 5 PARTY-1 (row 112); App. 7 PARTY/PVP.

## Builder instructions

- Work on branch `skeleton/party-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/party-1
blocker: CHAT-2; PARTY_PVP0
next_action: CP rechecks BLOCKING rows
```

## Local-only residue inspection (2026-10-15)

Inspected the exclusive `skeleton/party-1` checkout at
`9b35c6659d0b89b77bab0fc851d20fe82553ba7c` for existing draft #2201. This is
AUTHORING-only preparation under the lead-supplied META 3.1.0 policy
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; it does not complete PARTY-1, freeze a
candidate, change the historical status above, or allocate integration. The available
GPT-6.1 high worker substitutes for the unavailable hard-worker model.

### Accepted sources

Read PARTY-PVP-0 §3 and §4.1, its §13 limits, and SOCIAL-MAP §1.1, §1.12 and §2.1
before deciding whether semantic code was needed. Fresh read-only GitHub metadata
confirmed #1771 merged at `554960245f662321abe7855edc15c6397eb587ac` and #1773
merged at `48edf99b6b702dcf9d421f6c1930580df4e2cfd6`; both are ancestors of the
allocated head. #1771 accepts PARTY-PVP-0 despite its unchanged CANDIDATE heading.
SOCIAL-MAP §2.1 names `depends_on: [CHAT-2]` and candidate bases `[CHAT-0, ATTACK-0]`;
this inspection does not reclassify those bases or depend on new #2207 acceptance.
CHAT-2 remains a full-runtime/slot integration condition. The accepted packet has no
wire in PARTY-1 and no channel-loop, admission or session wiring (§1.1, §1.12).

### Existing implementation and no-duplicate disposition

The bounded allocation allowed only new `party/policy.rs`, `party/policy_tests.rs`
and this record, with an explicit evidence-only exception when useful requested rules
already exist. Inspection found these predicates already implemented in the existing
World writer, rather than an empty party skeleton:

| Requested semantics | Existing source at the allocated head |
|---|---|
| Visible invite target, target consent, leader requirement and 10/20/50 limits | `apps/game-server/src/durability/world_party.rs:251` invite branch; target-root lock before consent/cap reads |
| Valid unexpired invitation, one-party membership and 50-member bound | `world_party.rs:311` accept branch; it deletes only the accepted invitation |
| Decline, leader revoke, transfer to a member, combat-qualified leave | `world_party.rs:330` through the leave branch at line 354 |
| Leader-only party lifetime and last-member deletion | `world_party.rs:383` and `remove_member_with_succession` at line 391 |
| Online-first, lowest-invitation-sequence successor | `world_party.rs:402`: independently queried session state 1/2, then `m.seq` |
| Expired invitation cleanup, one party and at most 100 rows per pass | `world_party.rs:427` |
| Read-only actor snapshot cache, explicitly no authority grants | `apps/game-server/src/foundation/runtime_actor_party.rs:1` |

The existing writer is registered in `durability/mod.rs:81`; the actor cache is
re-exported through `foundation/mod.rs:1535`. These are private source ports; source
presence does not prove production admission or runtime completeness. They are not
modified here. A detached
copy of their invitation, membership or succession rules would create another unused
policy implementation without making the existing writer consume it. Therefore no
`party/policy.rs` or `party/policy_tests.rs` was created. This satisfies the allocation's
no-duplicate exception; it does not certify the whole existing writer or claim PARTY-1
acceptance. Observed source is implementation evidence, not an authority source.

`world_party_tests.rs` already declares **13** PostgreSQL tests, mounted by
`apps/game-server/src/lib.rs:145`. They include current-fence refusal, expired
invitation refusal, 50/51 members, 20/21 invitee invitations, 50/51 party invitations,
World binding, consent, expiry cleanup and absence-driven hidden presence. These tests
were inspected, not executed or counted as passes in this preparation.

Source SHA-256 evidence:

| Input | SHA-256 |
|---|---|
| `apps/game-server/src/durability/world_party.rs` | `7ff6285716deb6bb7af5404695eb10ecc39cfabcdf4890457036e067328914d3` |
| `apps/game-server/src/durability/world_party_tests.rs` | `70812f10a1731852d0d040e5a3fe38bc7e5cac3afbd04610d102eb41925b3740` |
| `apps/game-server/src/foundation/runtime_actor_party.rs` | `0185148f0df3b11c578a220ca92164141c6a674495e8958c17a2673390ae4a1a` |

### Validation and remaining boundaries

Only this task record changed. Governance validation and **124/124** agent tests passed;
`cargo fmt --all --check` passed under the shared Rust lock. Whole-diff self-review,
whitespace, three recorded source hashes, the 13 inspected test declarations and the
task-record-only owned-path check passed. No Rust source changed, so no new Rust harness
or large server recompilation was needed. No fresh Rust/PostgreSQL runtime validation
is claimed.

The existing writer keeps the current session/lease/node fence and database transaction
checks, target-root/party locking, occurrence binding and atomic writes. A cached
`PartySnapshot`, online-presence flag or retained record must not be substituted for
independently current facts. The existing offline-presence pass explicitly does not
authorize membership removal (`world_party.rs:604`); full cleanup still requires the
accepted legal-absence and combat-lock obligations. Atomics, fence/lock validation,
CHAT-2 integration, PartyView freshness, recovery and independent hard, persistence,
security and privacy review remain with the full implementation and CP/architect
integration. No shared owner, DB/schema assumption, wire, PvP, party spell or XP
implementation was added. No commit, push, external comment, PR state change, merge,
registry, migration, regeneration, pin or wiring edit was performed.


## Shared-XP native owner implementation — 2026-10-16

This section supersedes the earlier evidence-only allocation for this bounded
increment. The coordinator assigned an exclusive four-path source lease at existing
draft #2201, `skeleton/party-1`, exact predecessor
`18cddbcc85346817ebca76e32ccc161c4d1051ee`. The owner explicitly requested implementation
of the documentation-only positions. Accepted #1771/#1773 and PARTY-PVP-0 §§4.1/4.2
supply this operation; stale CANDIDATE headings do not block source authoring.
CHAT-2, shared registration and production composition remain their actual CP gates.

### Concrete code

The actual `world_party` transaction engine now consumes
`SetSharedExperience { enabled }`. It uses its existing independently current
`SpellItemAuthority`, physical transaction check and serialized World owner, reads
the acting Character's current party/leader, and permits only that current leader.
It updates the already-existing `game_parties.shared_xp_enabled` and party revision
in the same transaction as the original immutable occurrence receipt and audit.
It adds no parallel command engine, schema, fake authority or pure policy module.
Existing action intent serialization, result serialization and audit bytes are unchanged.
Exact occurrence replay reuses the original receipt without reverting a later flag;
a changed enabled value under the same occurrence is a binding conflict.

The existing strong `PartySnapshot` now carries the flag from the durable party row.
An explicitly observed solo Character has it disabled, and cache projection refuses
an enabled flag on a solo snapshot. This is observed preference DATA, not a claim of
current cache readiness, shared-XP eligibility or the XP split. Both snapshot literals
and the only exhaustive production action match are inside the owned paths; no
unowned consumer needs an edit. A toggle advances the party revision seen by existing
strong-read consumers.

### Native PostgreSQL source and qualification state

`tests/party_shared_xp_postgres.rs` is an inactive ordinary native crate-root wrapper,
using actual physical module declarations and the existing native Root. It mounts the
original `world_party_tests.rs` unchanged, including its **13 original PostgreSQL bodies**.
Its new cases reuse the existing signed-fixture producer/bootstrap/Fresh orchestration:
real native bootstrap and `FreshAdmissionStore::commit` create the current sessions;
the actual Root/recovery/Node and transaction constructor create authority. No
`SpellItemAuthority`, synthetic grant, copied currentness guard or widened private
helper substitutes for these operations. Administrative membership rows are explicitly
constraint fixtures; they do not prove invitation production (the original cases cover it).

**Nine new PostgreSQL bodies are authored, not executed:** enable/disable with same-xid
receipt/audit and strong read; solo/member refusal; exact replay plus changed-intent
conflict; rollback/retry; real leadership transfer before consumption; physical-TX
substitution; one-invariant expected session/lease/scope mismatch; actual Node revocation;
and actual scope revocation while the Node remains current. Node/scope negatives use
the native revocation owners and keep expected session facts unchanged.

Current status is **SOURCE_AUTHORED / NATIVE_UNEXECUTED**. Cheap targeted syntax/format
and exact owned-path checks are recorded separately. No Rust compilation, PostgreSQL
execution, large hydration, strict Clippy or complete runtime success is claimed.
The coordinator's single-heavy gate remains in force behind the three current jobs.
The original13 plus new9 require actual native PostgreSQL qualification and independent
source/architecture review before any publishable component claim.

### Remaining family boundary

This increment implements the accepted shared-XP command and durable flag view only.
Full legal-absence/combat-qualified member cleanup, the complete PartyView freshness/
relay lifecycle, consent-setting operations, actual caller/CHAT integration, production
SECURITY DEFINER/grant qualification and the separate PARTY-XP benefits remain owning
work. No wire, migration, registry, pins, shared module/Cargo, runtime loop, admission or
session file is changed. Worker publication/commit/push/ready/merge/queue authority is none.

### PC native qualification repair (2026-10-16)

The historical unexecuted state above is superseded only by these measured results:
the exact prior source `44f60bc5` passed scoped production Clippy with strict warnings
and produced its own native production library. The enabled ordinary wrapper then
failed configured compilation with one `E0308`: its new test fixture assigned the
protocol CharacterId directly to the domain CharacterId fence field. The selected
driver and all 22 PostgreSQL bodies were **not run**; the actual failure is retained.

The private test helper now passes the exact current lease ID bytes through the
existing fallible domain constructor and propagates errors at its two callers.
All assertions, real Fresh/current-session producers and authority guards remain
unchanged. The 13 original test bodies remain literal; one new negative-case body
only gains error propagation on that helper call. This is a new source candidate,
awaiting independent repair review and actual native continuation. No PostgreSQL
success or full PARTY-1 completion is claimed.

The repaired ordinary wrapper subsequently passed configured native compilation;
its strict warning check remained red. Selected-body compilation then found a
second test-only type error: the one-invariant negative case applied `+= 1` to
the opaque native scope generation type. The run stopped before a redundant
configured replay, native linking or PostgreSQL tests. That mutation now uses
the existing fallible `ScopeOwnershipGeneration::new(current.get() + 1)` API.
It still changes only the expected scope generation and preserves all assertions.
The original 13 bodies, production owner and actor remain unchanged. This new
candidate still requires selected native compilation and all 22 PostgreSQL cases.


### Actual native PostgreSQL qualification — 2026-10-16

The repaired source candidate `a085f1a8` now passed the native component suite:
**22 tests passed, zero failed, zero ignored and zero filtered out**. All 13
original PostgreSQL bodies and nine new shared-XP bodies executed, each with
an actual PostgreSQL `server_version_num=170006` check. Native selected library
code generation, driver linking and the configured Clippy check passed. The
ordinary native wrapper had already passed configured compilation; its result
retains its actual predecessor provenance.

The exact unchanged production owner and actor reuse their successful predecessor
strict production check and native production library. Compiler, source, dependency, generated-input and artifact hashes were verified
before reuse; source, dependency, generated-input and artifact hashes were checked
again after consumers.
The selected validation dispatcher was moved before the test module to satisfy
Clippy, and a private canonical-name copy of the same production library supplied
native transitive crate discovery. These are validation-only changes; no production
source, authority guard, test assertion or original body changed for them.

Strict warning-free qualification remains separate: the retained strict wrapper
check was red, and it was not relabeled as a fresh strict pass. The native result
qualifies this shared-XP command/flag component only. The remaining family and
CP integration boundaries above still apply; no publication or full PARTY-1
completion is claimed by this worker. The actual proof and every earlier failed
attempt remain retained under `reviews/party-pc` with exact source/body hashes.
