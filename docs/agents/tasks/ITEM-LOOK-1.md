# ITEM-LOOK-1

```yaml
task_id: ITEM-LOOK-1
title: "ITEM-LOOK-1: look/examine message (skeleton)"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/item-look-1
issue: 1622
pr: 2182
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 ITEM-LOOK-1 (row 75); wave 1"
class: "impl"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/item_look.rs (new)
  - apps/game-server/src/gameplay_transport/item_look_tests.rs (new)
  - apps/game-server/src/interaction/look.rs (new)
  - crates/protocol-oteryn/src/item_look.rs (new codec)
  - crates/protocol-oteryn/src/item_look_tests.rs (new)
  - docs/agents/tasks/ITEM-LOOK-1.md (this task record)
slot_only_paths:
  - connection.rs dispatch
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json (look rate row)
public_contracts: []
depends_on:
  - "none (plan row): MERGED/none"
  - "item description content (item_description_promotion.rs, item_description_wiki_promotion.rs): MERGED on main"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

A player can look at an item (inventory, container, ground) or a creature and receives the server-authored examine text (name, article, count, weight, description, readable text where admitted). Server-authoritative; no client-side text assembly.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| none (plan row) | MERGED/none |
| item description content (item_description_promotion.rs, item_description_wiki_promotion.rs) | MERGED on main |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- apps/game-server/src/gameplay_transport/item_look.rs (new)
- apps/game-server/src/gameplay_transport/item_look_tests.rs (new)
- apps/game-server/src/interaction/look.rs (new)
- crates/protocol-oteryn/src/item_look.rs (new codec)
- crates/protocol-oteryn/src/item_look_tests.rs (new)
- docs/agents/tasks/ITEM-LOOK-1.md (this task record)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs dispatch
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json (look rate row)
- CP D1292 accepts the slot-only hunks already on this branch (`capabilities.rs`,
  `crates/protocol-oteryn/src/lib.rs`, `PROTOCOL_OTERYN_V1_REGISTRY.json`, and the module
  mount in `gameplay_transport/mod.rs`) as this task's merge-queue turn; `interaction/mod.rs`
  is unchanged from main; `connection.rs` dispatch and `RESOURCE_LIMITS_REGISTRY.json` remain untouched.
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `trait FreshAdmissionAuthority` (l.331): new authority method (pattern: `npc_trade` l.696, `fight_modes` l.804)
- `connection.rs` `serve_admitted_actor` (l.1396): command-type dispatch arm and delta/snapshot frame (pattern: `quest_log_delta` l.3574, `chat_line_delta` l.3595)
- `mod.rs` `impl FreshAdmissionAuthority for ComposedFreshAdmission` (l.2866) and the `mod` list (l.5-53)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`
- `crates/protocol-oteryn/src/lib.rs`: `pub mod` + re-export of the new codec module; constants live in the module (pattern: `world_object.rs` `CAPABILITY_RUNE_USE_V1 = 22`; highest offered capability on main is 23, PRESENT-WIRE-1 #2124; numbers leased on #162 by CP)
- target resolution: reuse `gameplay_transport/item_ref_admission.rs` and `item_view.rs` (domain 9) for the looked-at item reference

## Inseparable bundles

wire+registry+capability+handler+tests. Registry order (plan §5): ITEM-LOOK-1, then SEC-1, then WEATHER-1.

## Acceptance criteria

- [ ] LOOK command on a visible item/creature returns text built only from admitted content facts.
- [ ] Unknown/unadmitted target -> typed refusal, no leak of hidden facts.
- [ ] Rate limit row registered and enforced (max vs max+1).
- [x] No brief exists: builder records a short packet in this file before code; architect review at queue turn.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

Reference look text format (Tibia 'You see ...'), item description promotion modules on main. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 ITEM-LOOK-1 (row 75).

## Builder instructions

- Work on branch `skeleton/item-look-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: CI repair (2026-10-17) mounts look.rs from item_look.rs, so interaction/mod.rs is unchanged and the interaction_workflow test target compiles
status: authoring
branch: skeleton/item-look-1
blocker: command 30 handler, look rate row and runtime Creature/name binding are not implemented; CI result on the repair head pending
next_action: CP reads exact-head CI and Codex review of the repair head
```

## Local preparation packet (proposed, 2026-10-15)

This is a bounded authoring proposal for existing draft #2182, not an accepted
architecture brief or a publication candidate. Local starting head:
`5541de017c603244d6ed190955c5655c2f4866a9`; branch `skeleton/item-look-1`.
The allocated checkout has no `origin/main` tracking ref; residue is confirmed
against the supplied starting source, not against an independently refreshed main.
The owned look modules are absent. Existing `SessionItemView::resolve` and
`item_ref_admission::pinned_index` provide the handle and pinned-definition seams;
`NativeGameplayState::item_policy` supplies qualified presentation facts.

Proposed safe subset, recorded before implementation:

- Reuse the existing strict `ItemTargetV1` handle codec for item LOOK requests.
  Do not assign a capability, command, domain or rate-limit number.
- Prepare an unregistered candidate response codec: exactly one field, either
  UTF-8 server-authored text (field 1) or an empty unavailable marker (field 2).
  Unknown, stale, hidden and unadmitted items share that refusal without text.
  These field meanings require architect acceptance before registration.
  Preserve the existing command-result envelope byte ceiling; no limit is raised.
- Prepare a read-only formatter using only known admitted name/description and a
  nonzero authoritative count. Proposed wording is `You see NAME.`, `Count: N.`
  and the known description on separate lines. Unknown/conflicting name refuses;
  unknown/conflicting description is omitted. Do not infer articles, plurals,
  weight units or readable instance text.
- Prepare a transport helper for an owner-observed item, checking the session's
  live handle, exact observed key, current World/activation/Channel generation pin, compact
  definition reference and exact production family/key/revision before reading
  presentation. The eventual caller must re-observe under the owner lock and
  refresh visibility; this helper is not a current-world observer or a handler.
- Test malformed/duplicate/oversized wire input and isolated negative identity,
  pin, admission and unknown-content cases without changing registration files.

### cross_domain_findings

1. No accepted LOOK brief/schema/IDs/rate row is supplied. Proposed field meanings,
   wording and the generic no-leak refusal need architect review. CP owns leasing,
   rate max/max+1 qualification, capability negotiation and command integration.
2. `ReferenceItemPresentation` has name/description, no article. Physical weight
   exists but examine units/grammar are not established here. Readability flags do
   not establish readable instance text. These acceptance parts remain incomplete.
3. Creature targeting and current item observation for inventory/container/ground
   require owner integration beyond the allocated paths. A live handle alone does
   not establish fresh visibility after movement. Do not activate the helper until
   the caller binds the view and observation to the current actor/session/Channel,
   supplies a current owner observation and closes that race.
4. All modules remain unregistered. Ordinary package tests cannot establish their
   coverage; isolated source harness results must be reported separately. Runtime,
   IDs, registries, rate limits, activation and independent review await CP's slot.

Local policy: no commit, push, external write, review request, regeneration,
pinning, migration or registry/wiring edits. Preserve local diff for lead review.

### Local validation boundary

The isolated harness is `/tmp/oteryn-item-look-1-harness`. It mirrors this checkout
with symlinks except two copied registration files:
`crates/protocol-oteryn/src/lib.rs` appends `pub mod item_look;` and
`apps/game-server/src/gameplay_transport/mod.rs` appends
`#[allow(dead_code)] mod item_look;`. Only the isolated copies are registered;
checkout originals remain untouched. The source files themselves are shared with
this owned checkout. `/tmp/oteryn-item-look-1-harness/item-look-harness.json` records
SHA-256 for all five proposed Rust files and both original registration files.
The harness never formats, regenerates, publishes or changes those originals.

The helper's negative cases vary only the observed key, compact definition ref,
family/key/revision, World, activation sequence, artifact pin, missing generation,
missing owner observation, absent qualified profile or dropped session handle.
Formatter tests separately cover unknown/conflicting name/description and the
existing UTF-8 byte ceilings. These are read-only boundaries: durable mutation,
replay/concurrent durable writes and PostgreSQL qualification are not applicable
to this inactive subset. Fresh visibility and session authority at command dispatch
remain unvalidated integration obligations, not claims of these helper tests.

Round 1 local results (before P2 repair; not candidate qualification):

- `cargo fmt --all --check`: pass on the allocated checkout. Direct `rustfmt
  --edition 2024 --check` of all five otherwise unregistered Rust files: pass.
- Governance validator: pass (22 required policy documents, 9 lanes).
  Governance unittest suite: 124 passed, 0 failed.
- Isolated registered-source codec tests: 4 passed, 0 failed, 242 filtered out.
  Full protocol-package tests in that overlay: 246 passed, 0 failed.
- Isolated registered-source server tests: 3 passed, 0 failed, 3179 filtered out.
  The formatter and all owner-observed binding negatives run in these three tests;
  this does not establish a current owner observer, client path or real E2E.
- First server harness compile found a test fixture using nonexistent
  `SessionItemView::default`; corrected to the existing `resume` constructor.
  Initial failure preserved at
  `/tmp/oteryn-item-look-server-harness-initial-failure.log`.
- Clippy for both affected packages/all targets in the isolated overlay: pass
  (`--locked --quiet -- -D warnings`). The existing vendored Tokio missing-docs
  dependency warning remains outside this allocation; neither source nor lints
  were altered for it.
- Per lead's inactive-preparation scope, no broad full-server test suite or E2E
  was run. Actual runtime integration and independent exact-head qualification
  remain future slot obligations.
- Whole local diff self-review and owned-path/unchanged-registration checks: pass.
  Head remains the supplied initial SHA; all work is uncommitted and unpublished.

Harness manifest SHA-256:
`2f599d2851c2318cca5da2cecffe9446327f9fad9813ab552c60d5172560d311`.
The manifest contains exact source/registration hashes and focused commands. Local
logs are `/tmp/oteryn-item-look-*.log`; the review patch (including untracked owned
Rust files without staging) is `/tmp/oteryn-item-look-1-local.diff`.

### Local review repair round 2 (2026-10-15)

Accepted P2 from `/workspace/oteryn-prep/reviews/item-look-1.md`: a retired
runtime preserves its content pin, so content/handle checks alone can still return
text after `retire_owner_cycle`. The new regression starts with a working
owner/runtime and the existing admitted target fixture, then changes only that
runtime's retirement. The same generation/view/handle/observation and unchanged
content pin must yield `Unavailable`.

This repair adds a local known-retired refusal through `owner_fence`; it does not
establish current DB/session authority or fresh visibility. The caller obligations
and all architecture/slot dependencies above remain open. Round 1 is preserved by
the lead at `/workspace/oteryn-prep/history/item-look-1/round-1`.

P2 disposition: **fixed locally**. The helper now refuses before target handling
when `runtime.owner_fence().is_err()`. This is a known-retired/local-scope denial,
not a grant or proof of current DB/session authority or fresh visibility.

Round 2 validation:

- Actual focused RED: 2 passed, 1 failed. The retirement-only regression observed
  `Text` where `Unavailable` was required; this was an assertion failure, not a
  build failure. The same regression test was left unchanged for GREEN.
- Actual focused GREEN: 3 passed, 0 failed, 3179 filtered out. Only the production
  helper changed between the RED and GREEN source manifests.
- Relevant game-server library clippy (`--locked --lib --quiet -- -D warnings`):
  pass. Checkout `cargo fmt --all --check` and direct rustfmt check of both changed,
  otherwise-unregistered Rust files: pass.
- Governance validator: pass; governance unittest suite: 124 passed, 0 failed.
- Protocol code and test SHA-256 values are unchanged from round 1, as is the
  protocol registration-root copy. The previous 246/0 protocol-package result is
  retained against those exact hashes; it was not rerun for this server-only fix.
- Original registration roots still equal HEAD; overlay registration roots retain
  their prior hashes. The overlay still resolves the proposed sources directly to
  this allocated checkout. No production registration or activation was added.

Durable evidence is preserved at
`/workspace/oteryn-prep/item-look-1-evidence/`: both rounds' manifests/source
hashes, exact RED/GREEN and final-check logs, explicit registration-root copies,
small source/patch artifacts and restoration instructions. It contains no target,
ELF, dependency tree or recursive overlay/symlink copy. The temporary paths above
are conveniences; they are not the sole copy of the repair evidence.

### D1143 default authoring packet — NOT_FULLY_QUALIFIED

CP D1143 (comment6224931148 on #2131) supplies this authoring contract:
request = Item (position or container reference) | Creature (id); result = UTF-8
text at most 512 bytes; refusals = NOT_VISIBLE | NOT_REACHABLE | RATE_LIMITED |
NOT_AVAILABLE; rate = 10 requests per 1 second per GameSession. These defaults
remain subject to CP's protocol review; this entry does not assign wire IDs/tags
or authorize a negotiated runtime offer.

This increment implements only the unambiguous 512-UTF-8-byte text bound in the
existing codec and its single `pub mod item_look;` native registration under
D1160. Encode refuses before output allocation; decode refuses before `to_owned`.
The existing envelope ceiling, nonempty success and strict malformed/oneof rules
remain in force. Native tests are authored for 512/513 ASCII bytes, 512/>512
multibyte bytes in both directions, and the stricter limit within the old envelope.

The current request still uses the existing ItemTargetV1/ItemHandle shape; the
Creature request, the new reference/refusal shapes, rate enforcement and actual
dispatch are NOT_IMPLEMENTED/NOT_RUN by this increment. Existing capability and
command IDs remain None, with no offer, new tags, server changes or activation.
Actual changed-file Rustfmt, strict native protocol all-target Clippy and the
complete native protocol package test command passed: **248 tests, 0 failed,
0 ignored, 0 filtered**, plus zero doc tests. Commands used the actual full Git
checkout/package, `--locked --offline`, the compatible cache and one Cargo job;
no excerpt harness, mounts, replacement types or server test rerun is involved.
Root independently reviewed the bounded worker-authored source before execution.
Exact source/commands/log hashes are in `/workspace/oteryn-family/look-d1143/`.
Earlier server qualification remains bound to its original source and context;
new server runtime/Creature/rate/dispatch and exact-head CI remain NOT_RUN.
This checkpoint is NOT_FULLY_QUALIFIED and does not establish complete ITEM-LOOK-1.

### D1143 Creature/refusal codec proposal — NOT_FULLY_QUALIFIED

This authoring increment extends only the owned protocol codec and its tests.
It preserves the Item branch's existing ItemTargetV1 bytes, handle bounds and
strict decoder. It adds `Creature(EntityRef)` using the actual native
`world_spatial_entities::EntityRef` carrier: opaque 16-byte identity plus u64
actor generation. The native spatial codec permits zero generation, so this
proposal preserves the full 0..u64::MAX domain; no LOOK-specific Creature ID,
Creature kind, name or visibility truth is created. Runtime admission must still
bind the reference to the admitted Channel's current visible creature census.

Proposed request payload layout (local fields only, pending CP protocol review):

- Item: field 1 varint = existing nonzero ItemHandle, byte-for-byte unchanged.
- Creature: field 2 length-delimited EntityRef; inside, field 1 is exactly 16
  identity bytes and field 2 is u64 generation (omitted means zero), matching
  the existing native spatial reference layout. Maximum encoded request: 31
  bytes; the Item branch retains its original 11-byte ceiling.
- Exactly one target is permitted. Unknown, duplicate, mixed, truncated or
  trailing fields, invalid identity lengths and overflowing generations refuse.

Proposed result payload retains field 1 nonempty UTF-8 text, at most 512 bytes,
and the existing result-envelope ceiling. The former empty unavailable marker
at field 2 remains the same bytes, now typed `NotAvailable`. Empty marker fields
3, 4 and 5 represent `NotVisible`, `NotReachable` and `RateLimited`. Every refusal
is fact-free and mutually exclusive with text and every other refusal. These are
proposed local payload fields, not assigned command/capability IDs; both IDs
remain None and no capability offer or runtime dispatch is added.

Native unit cases are authored for Item byte compatibility; native EntityRef
zero/one/maximum generation; exact identity width, truncated/overflowing fields,
strict target/result oneofs, all four refusal bytes and fact-free failures. The
previous ASCII/multibyte 512-byte tests remain. Actual native full protocol
package tests passed **251/251,0failed,0ignored,0filtered**, including all three new
parameterized cases; doctests0/0. Strict native package all-target Clippy with
-D warnings passed. Changed-file fmt, diff and governance passed. Exact-head CI
and server/runtime tests for this increment are **NOT_RUN**. These results bind
the current codec bytes; earlier248/0 evidence remains historical. Exact logs,
commands, source hashes and independent review are retained in
`/workspace/oteryn-family/look-d1143/codec-next/`.

CP's final protocol review, runtime Creature/name binding, visibility/reach
checks, 10 requests/1 second/GameSession rate enforcement and actual dispatch
remain pending / NOT_IMPLEMENTED / NOT_RUN. This source proposal does not
establish complete ITEM-LOOK-1 or change the earlier server qualification.

### D1240 capability28 selection — NOT_FULLY_QUALIFIED

CP D1240 (comment6226466175, #2182 section) accepts the preceding payload layout
and assigns capability28. This supersedes that proposal's pending layout review
and unassigned-capability statements. Command assignment, shared registry/offer
installation and runtime dispatch are not implemented by this owned increment.
The earlier251/251 protocol result remains bound to the previous87d9 source,
not to the changed codec described here.

Following the existing ItemMoveSelection style, `ItemLookSelection` defaults to
legacy (no cap28); selection-aware `_for` codecs require the actual negotiated
selection from the runtime owner. A Creature request without cap28 is malformed,
on encode and decode. The server encoder maps all extended refusals to the
unchanged field2/empty `NotAvailable` for legacy clients. Legacy result decode
rejects fields3–5; negotiated decode retains their typed meanings. Item request
bytes, nonzero handles, the11-byte Item bound,31-byte Creature bound,512-byte
UTF-8 text bound and strict oneof/malformed handling remain intact. The base
intent encoder now returns a checked Result so it can reject an unnegotiated
Creature request; all actual callers were owned codec tests and were adapted.

Generation0 remains wire-representable and means unknown. The owned pure
`known_creature_generation` helper returns `NotVisible` for0; it cannot substitute
or compare a current generation. A nonzero return is only a generation value,
not current visibility, reach, actor/profile or name authority. Runtime binding
visibleActor→profile/name remains with its producer and is NOT_IMPLEMENTED here.
No matching current actor or implicit fallback is invented.

Three new native test functions cover legacy/negotiated Item and Creature
selection, all four refusal mappings and gated decode, and wire-representable
zero generation refused before runtime binding. Previous malformed, boundary,
UTF-8 and strict oneof cases remain in the source-derived test inventory.
Current native tests, strict Clippy and exact-head CI are **NOT_RUN**. Only direct
changed-file rustfmt and scoped diff checks are claimed, with source hashes and
actual test names in `/workspace/oteryn-family/look-d1143/d1240/AUTHORING.json`.
The10requests/1second/GameSession rate owner, shared offer/registry integration
and actual dispatch remain pending; this is not complete ITEM-LOOK-1 acceptance.

### D1240a registry/closure allocation — NOT_FULLY_QUALIFIED

CP D1240a (comment6226563247) allocates this PR's sole-writer hunks in the
protocol registry and native capability list/closure. Capability28 is registered
as ITEM_LOOK_V1 with no offered runtime, command or state domain. D1240 assigns
no additional capability requirements; the registry and actual closure therefore
both use an empty requirements set, without inferring cap4 or cap6 dependencies
from shared ItemHandle/EntityRef types. The same registry allocation ledger now
records next_unallocated capability29; prior reserved/leased IDs are not reused.

One new owned test invokes the actual acceptance validator used by native
ServerAccepted/ServerResumeAccepted codecs:28 is accepted without invented
requirements,29 remains unallocated and is rejected, and the registry's sole
entry/empty closure/not-offered state and next29 ledger are pinned. This adds no
runtime offer, server dispatch, command assignment or visible-actor/name binding.
The registered source now contains13 owned LOOK test functions. This allocation
increment's native tests/Clippy/CI are **NOT_RUN**; passing checks of earlier
D1240 source remain bound to their earlier input. Changed-file fmt/diff checks,
source-derived inventory and exact hashes are retained in
`/workspace/oteryn-family/look-d1143/d1240a/AUTHORING.json`.

Final D1240a actual native package qualification:255passed,0failed/ignored/filtered,
0doctests; strict package all-target Clippy PASS. All13 owned LOOK cases were
registered in that native run. Current fmt/diff/governance and independent source
review PASS; the final card update changes documentation only. Earlier254/251
receipts remain attached to their exact older inputs. Current protocol payload
review and cap28/next29 registry allocation are closed by D1240/D1240a. Actual
command assignment, server negotiation/dispatch, rate owner and current native
visibleActor/profile/name binding remain open; runtime is NOT_FULLY_QUALIFIED.

### D1268 reserved command30 — local source checkpoint

CP D1268 (#2131/6227075059; HOUSE_QUERY correction D1268a/6227265767)
reserves command30 for ITEM_LOOK_V1 and keeps capability28. The owned protocol
constant and its existing assertions now record that reservation. This metadata
change installs no connection negotiation, capability offer, rate limiter or
dispatch. The connection.rs/capabilities.rs/rate registry grant is slot-only at
the final MQ turn after main integration; those paths are untouched. Actual
current-actor visibility and admitted Creature name/profile production remain
required. Earlier protocol255/strictClippy PASS is scoped to 8176aab1; current
protocol execution is NOT_RUN until an available execution budget, not inferred
from the constant or source review. No wire payload or selection-default change.

### D1291/D1292 CI qualification route and main integration

CP D1291 selects the existing changed-crate GitHub CI test/strict-Clippy lane
instead of CLOUD. After publication of4c452f6f, the actual PR was dirty and had
only merge-authority audit checks. CP D1292 explicitly assigns a merge of
origin/main into this task branch, preserving both registry/module sides. Main
efecd2209e71e209878ac543cd70966c6300e8dd is integrated without rebase; LOGOUT
capability21 and LOOK capability28 coexist. The command30 reservation remains
owned metadata; no command/rate/connection activation is added. HOUSE_QUERY28
is reserved by D1268a and is not added by this family. Main's current registered
commands do not consume30. One integration push; CI starts automatically.
Current protocol test/strict-Clippy results remain NOT_RUN until actual CI
records them; the previous255PASS remains scoped to8176aab1.

### CP final-review repair (2026-10-17)

CP final review on d6a72169 found three gaps, fixed in one push:

1. The server look sources were never compiled. `gameplay_transport/mod.rs` now mounts
   `mod item_look;` under a reasoned `dead_code` allow (the helper has no caller until
   command 30 dispatch). `look.rs` is mounted once, by `item_look.rs` (see the CI repair
   below). Its tests stay in
   `item_look_tests.rs`, included by `item_look.rs` as `mod tests` (the `capabilities.rs`
   pattern), so `mod.rs` does not mount that file a second time.
2. Command 30 is now in `PROTOCOL_OTERYN_V1_REGISTRY.json` as `ITEM_LOOK_INTENT`, gated by
   capability 28 like the other capability-owned, not-offered commands (NPC 7/8 under 3):
   capability 28 lists `command_types: [30]` and `capabilities.rs` `GATED` lists
   `(28, [30], [])`. Bounds are the codec's: 31 request bytes and 515 result bytes (512 text
   bytes plus field framing). No `.proto` is published yet, so both schema fields name the
   codec types and describe the D1240 layout. `allocation_ranges` is unchanged. The owned
   protocol test now pins the command entry and its bounds against the codec.
3. This record is listed in its own owned paths, and the D1292 slot-only acceptance is noted
   above.

Still not implemented: a command 30 handler in `connection.rs`, the look rate row, a
capability 28 offer and runtime Creature/name binding. Command 30 cannot be dispatched,
because no session can select 28. No local cargo build or test ran (disk-limited worker);
changed-file rustfmt passed, and exact-head CI is the qualification.

### CI repair (2026-10-17)

CI on 0de7da07 failed Clippy: `tests/interaction_workflow.rs` (and the
`chest_use_postgres` and `character_authority_postgres` targets) recompile
`interaction/mod.rs` with `#[path]` in a test crate that has no `crate::content`, so the
`pub(crate) mod look;` mount broke them (E0432 in `look.rs`). The mount is removed, so
`interaction/mod.rs` is again identical to main. `item_look.rs` now mounts the file once as
`#[path = "../interaction/look.rs"] mod text;`, the pattern `item_view.rs` uses for
`interaction/corpse_open.rs`; no test target includes `look.rs` or `item_look.rs`, so
`clippy::duplicate_mod` does not apply and `text` inherits the `item_look` `dead_code` allow.
No local cargo build ran (disk-limited worker); rustfmt passed, and exact-head CI is the
qualification.
