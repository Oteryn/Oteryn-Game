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
pr: 2201
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 PARTY-1 (row 112); App. 7 PARTY/PVP; wave 2"
class: "hard (shared state)"
migration: yes
registry: yes
owner: family worker /root/party_owner (sole author)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/world_party.rs
  - apps/game-server/src/durability/world_party_tests.rs # ROOT-HELP-D-PARTY-20261016: first-policy fixture only; sole author /root/party_owner
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
- `apps/game-server/src/durability/world_party_tests.rs` — ROOT-HELP-D-PARTY-20261016; only the first-policy fixture correction, sole author `/root/party_owner`.
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


## Family implementation checkpoint — 2026-10-16

AUTHORING only on existing draft #2201, `skeleton/party-1`, exact starting head
`5b46fefaa424fabc754239c8d7dc987ba1766ed4`. Author `/root/party_owner` is the
sole writer of this checkout and `world_party.rs`; the PARTY-XP worker is read-only.
Lead refreshed #2201 and CP comment 6215954536 and supplied bound META 3.1.0 at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. No commit, push, slot allocation,
production mutation or completion claim is made here.

Current protected main `24fc332f256638e3b784bff1b3c85b18e2665a76` marks PARTY_PVP0
ACCEPTED. Its §4.2/§4.4 and accepted SOCIAL-MAP §1.1/§2.1 govern these changes.
The historical CANDIDATE conflict above is resolved. CHAT-2B #2128 closed without
merging; a sealed relay payload/listener successor and runtime composition remain
gates, rather than a reason to withhold independent cache source authoring.

Initial file plan before edits: `durability/world_party.rs`,
`foundation/runtime_actor_party.rs`, new `party/mod.rs`, `party/view.rs`,
`party/view_tests.rs`, possible `party/ingress.rs`, `tests/party_shared_xp_postgres.rs`
and this record. No ingress file was created because the accepted ownership and
wire schema gate below prevents exposing a fabricated protocol. Shared registrations,
connection/capabilities, migrations, registries and pins are untouched.

### Implemented source

- The real shared-XP writer increments the party revision only when the durable flag
  changes. An authorized no-op still writes its occurrence receipt and audit; replay
  keeps the original receipt and does not change the current flag or revision.
- `party/view.rs` owns node-local membership/presence DATA with explicit full-refresh,
  admission and invalidation tickets. Startup/reconnect/revision-gap reads disable
  readiness until replacement; superseded or consumed tickets cannot restore it.
  Revision polling is due at 5 seconds, unconfirmed views are stale at 10 seconds,
  and clock regressions fail closed. Periodic membership and presence replacement
  catches lost hints. Admission withdraws the old local observation before the source
  transaction; failure leaves that Character unavailable. Admission cannot confirm
  another party's age or replace the mandatory full-node refresh. Last local departure
  removes a view. Inconsistent, malformed or regressing reads are rejected atomically.
- The only public projection is membership DATA, without remote channel, position,
  health or mana. Cached presence/hints never build outbound exact-channel messages,
  authorize a mutation, decide harm or grant a shared benefit. Authoritative consumers
  continue to use their existing same-transaction World read.
- The native Root now exposes a committed scope-refresh pass. It validates actual
  recovery/Node/scope authority and the actual borrowed Channel runtime, derives its
  complete positioned-player census, resolves each canonical current actor/session
  binding, and reads all memberships, party revisions and presence in **one SQL
  statement**. This is a single indexed membership batch per scope, after the separate
  canonical authority checks. The World advisory lock orders it with native writers.
  SQL errors, missing roots, wrong Worlds or incomplete/invalid snapshots fail closed.
  Its snapshots become available only after the native transaction commits.
- `party/mod.rs` exposes the committed scope read and admission pass. Admission uses
  the existing native same-transaction presence writer, not a new permission source.
  The node owner must collect all its actual Channel scopes before installing the full
  refresh; runtime scheduling/ownership of that complete node census is shared wiring.
- The same native command engine now implements server-only Block, Unblock,
  party-invite setting and Channel-visibility setting operations. Block requires the
  genuine visible target qualifier, serializes its 100-entry bound on the acting root,
  deletes the blocked leader's invitation and closes its now-empty party in the same
  transaction. Unblock touches only the actor's own list. Visibility changes write
  current presence and invalidate the party revision atomically; idempotent writes
  retain both revisions. Invite creation and Accept now produce HIDDEN/NULL initial
  presence when the durable setting is HIDDEN. Missing settings consume the explicitly
  accepted ANYONE/PARTY defaults, while actual read errors still fail closed.
- `PartySnapshot::is_well_formed` rejects duplicate members/sequences, missing self or
  leader, zero revisions, and illegal presence/channel combinations at both cache
  boundaries. It is a shape check and makes no current-authority claim.

### Validation and executable handoff

18 cache regression bodies cover startup, solo versus unknown, 5s/10s boundaries,
lost hints, reconnect, superseded/replayed tickets, failed admission, full-refresh
readiness, local departure, old membership/presence rejection, channel movement without
party revision change, malformed/inconsistent reads and clock regression. One new
native PostgreSQL body covers false/false, true/true and false/false toggle no-ops,
receipts/audit counts and exact replay. The **13 original PostgreSQL bodies remain
literal and unchanged**; the ordinary wrapper now contains original13 + sharedXP18 (31 owned PG bodies).
Additional native tests invoke the actual Root with real Fresh-derived runtime actors:
complete solo/party census, committed toggle/privacy data, missing independently current
Character eligibility, physical transaction substitution, actual Node revocation,
privacy no-op/replay/rollback, visible block plus invitation cancellation/own Unblock,
100/101 blocks, HIDDEN before creation/Accept, and missing-setting defaults at Invite,
Accept and Channel entry. Historical cap targets use the real native bootstrap and retain
complete creation receipts/history; raw root insertion cannot qualify native recovery.

Direct pinned Rust 1.94 `rustfmt --edition 2024 --check` on all changed Rust files
and `git diff --check` pass. No worker Cargo job was launched: the lead owns the one
native cloud validation slot. Rust compilation, Clippy and PostgreSQL runtime outcomes
must come from its exact-source overlay, not from the presence of these test bodies.

Minimum validation-only registration: add `mod party;` to the native server library
root. For the existing native PG wrapper, remove only its first `cfg(any())` in the
validation overlay. Do not change its original cases, fences, schema or grants. Run
strict package Clippy, the 18 party cache unit tests and configured PostgreSQL17.6
`party_shared_xp_postgres` filters `shared_xp_cases::` (18 bodies) and
`world_party_runtime_postgres_cases::` (13 original bodies). The new complete-census SQL
and current-source refusals now have executable native tests; runtime outcomes remain
pending lead execution. Governance validation and 124 agent tests pass.

One inherited assertion conflicts with the accepted contract: original
`genuine_two_fresh_players_require_target_consent_and_block_policy_before_invite_accept`
expects NOT_VISIBLE for its first Invite when settings are absent, while PARTY_PVP0 §3
requires ANYONE/PARTY defaults. Production now obeys the accepted default; that literal
original test is retained unchanged and is expected to expose its obsolete assertion.
No fixture workaround, skip, assertion edit or false green result is authorized here.
The lead must record actual results and resolve this test ownership/constraint conflict.

### Remaining scope and concrete gates

1. **Wire/ingress ownership conflict:** CP2201 asks for `party*.rs` ingress, but accepted
   SOCIAL-MAP §1.1 explicitly says “No wire. Party commands are built as server
   operations with their tests. The client messages are PVP-WIRE-1's and
   PARTY-CHAT-1's.” Its §2.1 excludes wire; PARTY_PVP0 §11 names PVP-WIRE-1 and requires
   per-session invitation handles, never a raw PartyId/CharacterId. No later accepted
   schema/handle producer/number allocation was supplied. CP/architect must reconcile
   this request with the owning wire child before protocol authoring; existing internal
   raw-ID `PartyAction` values must not become untrusted wire authority.
2. **CP shared integration:** register `party`, compose the complete node scope census,
   apply the committed source result to the cache before command results/presence,
   schedule 5-second checks, and call admission invalidation/read at every admission,
   readmission, reconnect and Channel entry. Wire command admission must fail with
   PARTY_UNAVAILABLE while node/view/Character is unavailable. Nothing currently claims
   this production composition is present.
3. **CHAT-2 successor:** add only `{party_id,revision}` to its real sealed payload enum,
   send after commit, and call reconnect/gap invalidation from its real listener. No
   alternate relay, payload schema or invented sealing/transport was added.
4. **Full accepted family residue:** logout and legally-absent/combat-clear cleanup;
   native combat-lock owner hookup; production SECURITY DEFINER/grants/deferred presence/channel guards; ordered
   session view generations and fresh-read-only remote presence output; required
   concurrent accepts/recovery/privacy/performance qualification and independent
   exact-head hard/persistence/security/privacy review. These are not certified by this
   cache/no-op increment. Benefits remain PARTY-XP-1 and party chat PARTY-CHAT-1.

Final packaging, candidate freeze, native results and any publication belong to the
lead. Integration remains NOT_REQUIRED for this AUTHORING-only increment / CP_ONLY
for shared composition; full PARTY-1 completion and readiness remain unclaimed.


### Independent finding dispositions for this source generation

- P2 mixed-time local/party membership accepted and repaired: full replacement validates
  local observations against all party rosters; admission validates its merged candidate
  against retained views and disables readiness on a conflict.
- P2 disconnect/replacement race accepted and repaired: removing a local Character
  advances the generation, rejecting old full-refresh and admission completions.
- P1 HIDDEN join privacy accepted and repaired: initial Invite/Accept member records
  consume current durable visibility; the native regression verifies source census.
- P2 cap fixture recovery invalidity accepted and repaired: all 100 retained block
  targets are created through the real native bootstrap with their history intact.
- P2 absent-setting semantics accepted and repaired: Invite and admission presence use
  the accepted defaults. The contradictory inherited test remains literally unchanged.

The family reviewer independently rechecked all five repaired finding families against
final source hashes (`world_party.rs` db91b5bf… and native wrapper6f40a065…). No further
confirmed product finding remained. Native execution and exact frozen-candidate review
remain pending. The inherited default-test contradiction remains an actual publication
hold; its external minimal proposal is `/workspace/oteryn-family/party-defaults-test-proposal.patch`.
The original test file is outside PARTY-1 owned_paths and remains literal.


### Family-owned XP read union — lead-directed, 2026-10-16

The sole family author added PARTY-XP-1's existing `WorldPartyXp`, command-authority
and autonomous scope-actor read ports plus their strong toggle/predecessor query to
this task's owned World source. PARTY commands/defaults/cache behavior and the
PARTY-owned current foundation snapshot remain intact. This private source union
adds no contract, reward mutation, authority reconstruction or registration.
World source SHA256 now
`99d3c9eaf8843cc7897e9c2ab32ab62c59ba893ff249db58d95a2b38413c65a0`.

The XP card does not allocate the PARTY foundation file; lead explicitly prohibited
copying that provider into the XP branch. XP production remains canonical, with its
new native regression increment separate. CP composition must carry the exact
World+foundation provider after upstream/main and adapt XP's owned pure fixture
helper to the real added snapshot field, retaining all test bodies. Separate old-XP
native qualification does not certify the final joint-source candidate. No main or
other branch was merged and no cross-card path was silently written. Publication
remains held on the inherited default-test constraint and final native/review gates.


### ROOT-HELP-D-PARTY-20261016 — narrowly allocated fixture repair

This current entry supersedes the earlier unresolved literal-test/path hold only.
The user's explicit ROOT-HELP-D-PARTY-20261016 instruction allocates
`apps/game-server/src/durability/world_party_tests.rs` on `skeleton/party-1` to sole
source author `/root/party_owner` exclusively for this contract-fixture repair.
The task returns to AUTHORING from local `1538cd2bcac320a64507d1a0fee579171189a72c`;
remote draft #2201's expected predecessor remains `5b46fefaa424fabc754239c8d7dc987ba1766ed4`.

The entire Rust delta changes the first policy tuple in
`genuine_two_fresh_players_require_target_consent_and_block_policy_before_invite_accept`
from `(1, None)` to `(1, Some(2))`, making its initial refusal case explicitly NOBODY.
All original NOT_VISIBLE/zero-party assertions and authority/visibility guards remain
unchanged. Accepted missing-row ANYONE/PARTY semantics remain covered by the existing
owned `party_missing_settings_defaults_allow_invite_accept_and_current_channel_entry`
in `tests/party_shared_xp_postgres.rs`: it verifies zero settings rows before actual
Invite APPLIED, Accept APPLIED and CHANNEL entry. No duplicate test was added.

The lead's retained pre-repair native result, 48 passed / 1 failed out of49, remains
historical evidence of the obsolete absence/refusal fixture; it is not relabeled as
a post-repair success. Worker validation covers format, exact delta, whitespace and
governance only; no Cargo/PostgreSQL job ran for this repair. The lead must validate
and independently review the new exact candidate before publication. The earlier
assertion-rewrite proposal is superseded and must not be applied.

This allocation adds no general permission to rewrite required assertions, touch
other files, wire/register modules, allocate shared slots, mark ready, merge or
operate production. Full PARTY runtime/CHAT/cleanup and other integration gates
remain as recorded above. The lead retains commit/push authority; the worker makes
no commit or push.


### Current b801 native qualification — 2026-10-16

Actual #2201 `b8017162ce09bfd1a56f4c6f4e29f22efca13f2b` changes the cache's
`remove_local` nested condition to the same edition2024 let-chain; no behavior or
World/settings source changes. View SHA256 is
`f01aac5014e732254074b811bd9656a25dbbd7ee5478697f23f294439c04ab30`.
The lead's new whole-crate native composition executes **18 cache/view bodies and
13 actual World bodies, all PASS**, each once with0 failed/0 ignored. PostgreSQL
qualification uses configured17.6 (`server_version_num=170006`). Independent
`D1102-execution-review.md` verifies exact invocations and archived compiler inputs:

- ELF SHA256: `889ab44ebd5c27b203d921940ced69ed7cb25aa01c15b92a2d5218c84bdc1dd1`.
- Source archive SHA256: `b67d061700bf01e7620a54eaf7355d23a1b249cfc92e86df865ac4993b4d17d7`.

The compiled World/foundation equal actual2201d2f provider bytes, unchanged in b801:
World SHA256 `99d3c9eaf8843cc7897e9c2ab32ab62c59ba893ff249db58d95a2b38413c65a0`,
foundation SHA256 `eb8ca50bfc4f181987e859c1d1857285ef15386418469f5990b67e774929f039`.
The private World test input additionally carries the outside D1102 consumer patch
`826c05c129faa2fb6286d91a2ab7d2614cfc8643e329812785e3563ec0fb86e5` and CP
registration; it is not a canonical2201 test edit or production integration.

Earlier49-body World/settings/cache proof remains bound to its own exact generation
and unchanged World/settings bytes. Its earlier48PASS/1RED default-fixture outcome
is preserved as history, followed by the explicit ROOT-HELP-D-PARTY-20261016 fixture
correction recorded above. **This new execution is18+13, not a new49-body claim.**
Strict native unit Clippy remains a separate lead qualification. Shared runtime/wire/
CHAT/cleanup and accepted whole-feature gates remain open; no readiness, complete
PARTY-1 or production claim follows from these component results. This sequential
docs-only update changes no Rust, tests, ownership or integration authorization.


### CP takeover D1372 — PostgreSQL case relocation, 2026-10-17

CP took this draft over in D1372 and merged `origin/main` into the branch (merge commit).

**The retained all-target RED.** `tests/party_shared_xp_postgres.rs` was a second crate
root that re-mounted every `src/` module through `#[path]`, disabled with
`#![cfg(any())]`. Enabling it could not land: it had drifted from `src/lib.rs` (no
`world_change` after #2178), it compiled every test-bearing `src/` module a second time
(including the 13 World bodies the library already runs), which
`tools/repository/check_duplicate_test_sources.py` rejects because its allowlist only
shrinks, and the only other option, a `test = false` manifest entry, is a shared
`Cargo.toml` hunk that would keep it out of CI anyway.

**Fix within owned paths.** The 18 `shared_xp_cases` bodies are moved verbatim to
`apps/game-server/src/party/shared_xp_postgres_tests.rs`, mounted under `cfg(test)` from
`party/mod.rs`. The only change is the PRE-ROUTING / NONCANONICAL early return
(repository harness convention) in place of a hard failure when
`OTERYN_TEST_POSTGRES_ADMIN_URL` is absent. The crate-copy wrapper is deleted. No
assertion, fixture, fence, schema or grant changed. Mounting from `durability/` was
tried and rejected: the allowlisted crate-copy targets `native_admission_source_postgres`
and `runtime_scope_assignment_postgres` include `durability/` without the PostgreSQL
harness and fail to compile.

**Remaining shared hunk (NOT APPLIED).** `mod party;` in `apps/game-server/src/lib.rs`.
Until CP applies it at this PR's slot turn, the `party` module, its 18 view tests and the
18 relocated PostgreSQL bodies are not compiled. With that line applied locally the
validation recorded in the PR status comment passed. The wire, CHAT-2 relay, runtime
composition and family residue gates in *Remaining scope and concrete gates* are
unchanged.
