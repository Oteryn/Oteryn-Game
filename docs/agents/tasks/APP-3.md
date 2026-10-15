# APP-3 — appearance change command and actor appearance state (skeleton task record)

```yaml
task_id: APP-3
title: "APP-3: appearance change command and actor appearance state"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-3 (rows 126, 127); §7 supporting briefs row APP-3"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-3
needs_migration: false
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Change command and actor appearance state on the wire; other players see a committed look; a rejected change publishes nothing.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-3 (hard, protocol, Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| APP-1 | BLOCKING (Wave 4) |
| combat child E protocol lane (DEATH-2/ATTACK) | ATTACK-1b MERGED; DEATH-2: verify |
| registry lease | CP lease |
| acceptance of CHARACTER_APPEARANCE (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/appearance_change.rs`
- `apps/game-server/src/gameplay_transport/appearance_change_tests.rs`
- `crates/protocol-oteryn/src/appearance.rs`
- `crates/protocol-oteryn/src/appearance_tests.rs`
- `docs/agents/tasks/APP-3.md` (this record: status and notes only)

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

- apps/game-server/src/gameplay_transport/appearance_atomic.rs (existing)
- world_spatial_entities actor state (crates/protocol-oteryn/src/world_spatial_entities.rs)
- connection.rs dispatch; capabilities.rs `GATED`

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Other players see a committed look.
- [ ] Rejected change publishes nothing.
- [ ] Older clients gated by capability.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §4.5, §5, §8
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## APP-3 bounded integration packet — 2026-10-15

### Allocation and accepted floor

This is **CARD_ONLY / REPORT_ONLY_COMMIT_WIRE_SEAM**, not executable implementation.
The exclusive root allocation `app-3-allocation.json` permits only this task record,
on `skeleton/app-3`, exact HEAD `5ee80913b3a349fc8dcea47075b35363006e7544`,
existing draft #2175. Its narrower lease supersedes the historical builder permission
above to edit Rust or register modules. The plan row, CP slot rules, inseparable bundle
and all unchecked acceptance criteria remain intact. No code, registry, migration,
content/pin, shared caller, commit or worker publication is authorized in this pass.

CHARACTER_APPEARANCE is ACCEPTED on protected main
`bda15eba7a84f65fcb963a86e8b2bf18d69c800a` through ACCEPT-BATCH-2026-10-15;
the earlier CANDIDATE dependency row is historical. Acceptance of that decision
neither installs APP-1/APP-2 preparations nor accepts this proposed wire composition.
The exact source/hash inventory is
`/workspace/oteryn-prep/app-3-evidence/input-source-hashes.json`. It binds both the
allocated source floor and protected main, accepted decision copies, and the exported
APP-1/2/4 packets. Main routing/session/client files differ from the old floor, including
current LOGOUT routing and terminal-control handling; preserve those current bodies,
not stale line numbers or the old highest capability. No merge/rebase was performed.

### Actual residue and owner boundaries

| Actual owner / source | Existing behavior and precise missing composition |
|---|---|
| `domain/appearance.rs::{AppearanceSelection, validate_selection, displayed_look, AppearanceContent, AccountUnlocks}` | Native pure selection/compatibility and fallback rules already exist. Reuse them; do not copy a validator into transport/codec/client. The traits consume supplied facts, not current admission authority. |
| APP-1 prepared `durability/character_appearance.rs::{read_selected_appearance, read_account_appearance_unlocks}` | Private historical readers only, not merged native writer APIs. Selected look/provenance/revision and earned facts are retained; an absent selection is not a persisted guessed fallback. Prepared source SHA256 `b8f5dd93925316ce05c9896bb34c0745cc273ee9a224a2b7891bead2f748ce8c`. |
| APP-1 task packet / native `character_stance.rs` and `character_authority.rs` | APP-1 still needs its actual fenced selection writer, immutable appearance receipts, projection/receipt-chain integrity, typed replay and reconciliation. Its proposed receipts are not installed schema. Native stance is an integration pattern, never an appearance receipt substitute. |
| APP-2 prepared `content/cosmetics_loader.rs::{compile_candidate, CosmeticsCandidate}` | Candidate offers `look_type(key, sex)`, `palette`, `fallback`, content revision/digest and `AppearanceContent`. It grants no current World/artifact authority or unlock compatibility. Source SHA256 `33bef8af2f4a0156e51c17743888b8dc51c307884e2f24b646131726748d592d`. Current Outfit index is READY_UNPOPULATED and Mount records lack authored facts; synthetic tests are not live cosmetics. |
| `character_bootstrap_intent.rs::CharacterBootstrapIntentV1`, native Character root | No actual Character sex creation/persisted input in these owners. `domain::appearance::Sex` alone does not resolve a character's sex. Require the creation owner's admitted sex; only the accepted native test character may use its content-declared default. Never infer sex from outfit, account, client bytes or look128. |
| accepted PREMIUM_ACTIVATION §4.7, `domain/premium.rs` | D61 explicitly remains: every unlocked outfit/addon/mount is usable by everyone in V1. Native current Premium surfaces exist, but adding `PremiumGate::require` to cosmetics would invent a restriction. Premium metadata stays recorded; Store authority remains separate/unavailable under gap §32. |
| `gameplay_transport/appearance_atomic.rs::native_source_appearance_batch`, `ability/condition_spell.rs::displayed_temporary_appearance_at` | Existing temporary Outfit/Item conditions consume their actual current owner, source and native timer/conflict rules. They do not durably change the selected look. APP-3 must compose, not create another condition store or reset/cure one on selection change. |
| `condition_snapshots.rs::{prepare_current_player_in_transaction, PreparedConditionSnapshot::validate_current}` | Actual source-qualified player projection checks native transaction/facts/state/position/content; publication requires another independently read current projection. It is explicitly awaiting a production caller. No generic reuse of spell authority as a permanent-change permission. |
| `protocol-oteryn/actor_condition_snapshot_candidate.rs` | Existing candidate carries EntityRef, content/map/frame, native coordinates, owner time/player revision and temporary Outfit/Item. Numeric admission/routing and a client SessionEvent are absent. Candidate bytes/name do not authorize publication or select a capability. |
| `gameplay_transport/world_spatial.rs::{ChannelEntity, wire_entity, SessionVisibility}` | `PLACEHOLDER_APPEARANCE_REF=0`; ChannelEntity revision is presently position revision. Stationary appearance-only changes need actual owner revision/update detection. Existing per-observer visibility and delta limits remain; appearances cannot reveal actors omitted by that owner. |
| `protocol-oteryn/world_spatial_entities.rs::EntityDetail::Actor` | `appearance_ref:u32` exists without admitted permanent-appearance dictionary/binding. It is neither a DAT look type nor an authority/content key. Reinterpreting ref0 or sending unnegotiated fields to old clients is forbidden. |
| `connection.rs::{FreshAdmissionAuthority, AdmittedSession, SessionContinuity}`, `capabilities.rs` | Current admitted actor/controller/World/Channel plus envelope CommandId, connection generation, server sequence and selected capabilities are real boundaries. `item_fence` is explicitly expected evidence and rechecked for item writes; it is not a new appearance writer fence. No appearance dispatch/caller is composed. |
| `crates/session/src/lib.rs`, `apps/client/src/{play,scene,world}.rs` | Session has validated gated command/delta routing; no admitted permanent/temporary appearance surface. Play reduces actor state to targeting/position; World uses fixed PLAYER_LOOK_TYPE128. APP-4 must retain appearance by full EntityRef separately from combat targeting and replace fixed presentation through its actual assets/renderer owner. |

The accepted §4.2 amendment assigns mount speed to the World ruleset (+10 Reference)
and activation Global (D125); selected `mount_key` alone is not current mounted-state
evidence. Native `DisplayedLook` has no mount field. Do not infer activation or speed
from a requested/stored mount, or add a mount effect through an appearance codec.
Its actual runtime owner must supply any admitted active mount observation separately.

### Proposed future own files/functions (not implemented or newly allocated)

Use the four historical APP-3 Rust paths once CP allocates the complete accepted
integration. Names below are proposed private interfaces, not claims of native symbols.

- `gameplay_transport/appearance_change.rs`: `appearance_catalogue_for_session`
  resolves the admitted catalogue/base for the admitted Character; `change_appearance`
  coordinates one existing command with APP-1's eventual writer;
  `observe_actor_appearance` projects current committed base/effective display inside
  the Channel observation owner. No separate writer, authorization token constructor,
  parallel validator, detached task or abstract authority framework is needed.
- `gameplay_transport/appearance_change_tests.rs`: native command/current-owner and
  late-completion/observation regressions against the real APP-1 writer and native
  Channel/condition owners, not fixture booleans or record-derived matching authority.
- `protocol-oteryn/appearance.rs`: `decode_appearance_change_intent`,
  `encode_appearance_change_result`, and paired catalogue/actor-state codecs. Request
  is raw selection plus current-view binding; result correlates the existing command.
  Codecs validate shape, not ownership/unlocks/sex or current artifact admission.
- `protocol-oteryn/appearance_tests.rs`: exact roundtrip/bounds/malformed/unknown
  variant and binding/order cases using the eventual leased contract. No numeric
  command, capability, domain, reference assignment or new resource bound is selected
  by this packet. Keep existing envelope ceilings; derive inner limits from the owning
  native key/palette/selection representations and reviewed registry before code.

### Durable command and current publication sequence

1. Existing transport gates the leased command under selected capabilities, current
   connection generation, CommandId order and input permit. Derive Character, account,
   exact actor and World/Channel from the actual admitted controller, never payload
   identities. Missing controller/actor/sex/current catalogue is unavailable, not a
   fabricated starter, empty unlock set or guessed authorization. A request's catalogue
   digest/revision is expected binding; independently resolve current admitted content.
2. At the real APP-1 transaction boundary resolve active definitions, sex, earned
   unlock provenance compatibility and expected prior selection/revision. Reuse native
   `validate_selection`; known key/revision equality alone is not provenance compatibility.
   Starter outfit/addon facts need no grant, earned facts need compatible provenance,
   Store has no usable route yet. D61 stays regardless of Premium classification.
3. APP-1 supplies its proposed `DurabilityRoot::commit_character_appearance` and
   reconciliation owner only after native receipt/chain integration exists. Follow
   `begin_semantic_transaction`, semantic deadline, `assert_recovery_fence`,
   `lock_admission_relations` and `character_progression::assert_gameplay_fence`,
   including independently current node/session/connection/lease/scope generations,
   locked root/prior selection and interpretation. Like the actual stance writer,
   lock occurrence before root/progression and compare full versioned command binding
   on replay; selection commit advances Character/progression revision exactly once.
   An earned grant remains inside its owning fenced quest/NPC event, not APP-3.
4. Only an actual durable committed receipt permits a base-state observation. A known
   precommit rejection changes no durable/base/effective state and emits no appearance
   delta. Stage/validate encoding and bounded publication work without exposing it
   before commit. Distinguish rejection from commit failure with unknown outcome;
   never report `Rejected` after a possibly successful commit or manufacture success.
5. Before runtime installation/publication independently re-read the current actor,
   session/connection/controller, scope owner/generation, content activation/pins and
   source/condition state through the real owner. A retained receipt/projection cannot
   recreate current authority. Late owner loss suppresses old runtime publication;
   the committed selection remains durable and the next valid owner loads it. Do not
   roll it back, reactivate the old actor, or rewrite it with a stale runtime snapshot.
   Native current-owner/retirement refusal must be included, not just matching pins.
6. Reconciliation compares the full immutable request/receipt binding. A replay may
   describe historical committed disposition, never grant actor authority or perform
   a second write. Unknown transport/commit outcome remains unresolved until the
   native owner reconciles it. Reconnect does not automatically send a new occurrence.
   Fresh admission/recovery revalidates current content/sex/unlocks and loads durable
   base; fallback display preserves incompatible stored keys and provenance.

### Proposed committed-observation handoff to APP-4

The aligned APP-4 packet is `app-4-evidence/local-owned.patch` (read-only input,
not acceptance). It needs catalogue, correlated change result, committed/base versus
effective observation, and current temporary state/clear. The own client consumes
**the same effective actor observation as other observers**. Its private editor may
also receive its own durable base/current allowed catalogue. Never expose account
unlocks, granting receipts, private AccountId/session credentials or raw fence proof
to other players just to correlate a public look.

Propose a negotiated per-session reference mapping: every appearance_ref resolves
only with its current admitted content/definition/palette provenance and the exact
EntityRef identity+generation. References never become DAT look types and are never
rebound across session/content/generation reset. The codec owner must finalize this
layout, its bounded allocation/exhaustion/reset and registry dependency on visible
entities before code. No permanent mapping/table IDs or undocumented wrapping counter
are introduced here. An unresolved ref cannot choose nearby look/default sex/128.

A single StateDelta envelope carries one domain, so cross-domain observation is not
implicitly atomic. Publish the qualified mapping/state before an entity update that
references it; carry an explicit reviewed observation/version dependency in that
surface and make Session defer presentation until the complete matching state exists.
Initial join/snapshot must supply the complete matching mapping/state as well. On
missing/mismatched mapping or a gap, request the existing authoritative resync path
(or end the unusable session under its actual policy), never render a guessed look.
An appearance-only update must reach a stationary visible entity without forging a
position revision or relying on movement. Existing enter/update/leave and limits apply.

Result is command settlement, not a render update. Proposal: a committed result
identifies its committed state revision/binding; observation-before-result and the
reverse converge idempotently. A later valid observation may supersede the command's
look, so do not wait forever for exact old pixels or repaint old state after seeing a
newer revision. Do not invent a timeout/retry budget; final liveness/resync settlement
belongs to the existing Session/command owner and requires explicit qualification.
The UI preserves a separate draft, keeps the scene on observations and submits no
second change merely because an acknowledgement/observation is delayed or lost.

Compose current effective temporary Outfit/Item from the existing condition owner
against the same qualified base/context read. Temporary state or compatibility
fallback changes presentation, not selected storage or editor draft. An explicit
newer authoritative clear/expiry transition restores the **latest** committed base,
not the base captured when the condition began. Preserve native conflict/expiry and
item-presentation semantics; do not add local client expiry, use owner_time as client
authority, infer omission as clear, or use a permanent change to mutate conditions.
If separate candidate/base/visibility domains are retained, CP must lease their
coherent revisions/correlation and current consumer, including out-of-order clears;
the existing candidate is not already an admitted substitute for this contract.

### Exact CP/caller deltas and test gates

- APP-1 owner: actual selection receipt/writer/reconcile plus latest progression
  guard union (`0073_character_progression_guard_wheel_union.sql`) and
  `character_authority.rs::verify_character_integrity` appearance projection/receipt
  chain/replay; production DDL/permissions numbered only at slot. Preserve all native
  families. No borrowed stance event or uncommittable independent APP-3 writer.
- APP-2/content/admission and creation owners: complete actual source facts,
  fallback per sex, palette/look mapping, provenance compatibility and current
  World/artifact/profile admission; authoritative sex input/persistence. Candidate
  expected hashes, Premium flags or a client-selected sex cannot fill those seams.
- CP shared transport/runtime: current `connection.rs` trait/dispatch/result and
  snapshot/refresh/continuity callers; `gameplay_transport/mod.rs` concrete authority
  and actual admission/late-owner reload; `world_spatial.rs` current projection,
  appearance-only update detection and visibility/reference ordering; actual condition
  projection publication/clear caller. Current spell authority must not be extended
  by an APP-3 wrapper that fabricates a transaction or grants permanent-change power.
- CP registries/codecs: reviewed capability/dependency, command/result/domain/delta
  IDs and payload/counter bounds; protocol `lib.rs`, current server capabilities and
  current client Session GATED_ROUTES/domain store/event routing. Resume preserves
  negotiated selection and cannot widen it; unsupported/old clients get neither
  unnegotiated command effect nor appearance bytes. Preserve current logout/death,
  movement, item and all other capability routes. No numbers are reserved here.
- APP-4/#1942: exact client ownership for Session events, PlayLink ordered task,
  EntityRef appearance store separate from targeting, outfit UI and verified
  asset/palette/renderer composition. Its cache variant identity and multi-actor
  rendering concerns remain real renderer work, not solved by a wire acknowledgement.

The following matrix is **planned qualification**, not tests run by this card.
Each negative starts from a valid real fixture and changes one invariant at the
named boundary; independent current evidence cannot be derived from the record tested.

| Boundary / invariant | Required evidence before integrated qualification |
|---|---|
| Decoder / identity and bounds | Valid selection roundtrip; missing/duplicate/malformed/unknown variant, truncated/oversized key/provenance, palette boundary/max+1, addon mask range; unknown capability/type has no effect. Values follow actual leased/native bounds, not new constants here. |
| Current admission / binding | Change only Character/account binding, World versus Channel, exact actor generation, sex, request catalogue revision, current artifact/map/frame or current activation; reject before write/publication. Unknown sex/catalogue/source fails closed. |
| Native selection / authorization | Starter without grant succeeds; locked outfit/addon/mount, incompatible provenance and unavailable Store reject. Unlocked premium-flagged cosmetics succeed under both current/not-current Premium (D61); do not add a gate. |
| Durable write / current liveness | Missing/stale/mismatched current session, connection, lease, scope, node incarnation or root/prior state; expired/future/non-monotonic semantic deadline. Verify real DB state, revision, receipt and runtime display remain unchanged on known rejection. |
| Commit / replay and concurrency | Exact retry returns one historical disposition; occurrence rebound to different selection/provenance/Character refuses. Competing root revisions have typed loser; one commit/revision advance only. Actual commit ambiguity/reload survives restart. |
| Publication / current owner | Valid commit visible to own+other current observers; inject owner retirement, generation/connection replacement, stale pin/activation or condition change after commit before publication. No stale actor mutation or credential reuse; next current owner reloads durable state. |
| Receipt / provenance substitution | Substitute source compatibility, occurrence binding or stored revision independently while current fence remains valid; reject at actual writer/reconciler/integrity boundary. A historical match never restores authority. |
| Observation / order and visibility | Result-first/observation-first, duplicate, newer superseding observation, lost ack and unresolved outcome converge without duplicate write. Stationary appearance change emits update; leave/reappear and EntityRef generation reset purge old mapping; hidden actor never revealed. |
| Client domain / downgrade | Wrong mapping/content/palette/session/context, stale sequence/revision, unknown ref, missing mapping and unsupported capability fail/refresh via native policy. Reconnect without previously selected support is refused; no cross-session refs/draft/replay. |
| Temporary/fallback composition | Base changes under active Outfit and Item transforms, explicit clear and owner expiry restore latest base; stale/non-monotonic projection/clear refuses. Incompatible-world fallback preserves durable selection; client timer and missing packet cannot clear state. |

No value write, protocol/runtime test, renderer/Windows/E2E, live relog, authority
qualification or independent architect acceptance is claimed. This card's actual
checks are governance, documentation diff/scope and exact source binding only.
Evidence/restoration packet: `/workspace/oteryn-prep/app-3-evidence/`.
