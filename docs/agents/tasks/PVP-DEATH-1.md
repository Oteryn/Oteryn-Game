# PVP-DEATH-1

```yaml
task_id: PVP-DEATH-1
title: "PVP-DEATH-1: outcome leaf; shared death integration after #2193 (D1105)"
mode: IMPLEMENTATION
status: source_authored_unqualified
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-death-1
issue: 1622
pr: 2205
base_sha: 5c65464f770272a315cedda4b6b6638a58363afc
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Death module D; App. 7 PARTY/PVP PVP-DEATH-1; wave 2"
class: "hard (persistence)"
migration: no
registry: no
owner: family_f_pvp
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pvp/death.rs
  - apps/game-server/src/combat/pvp/death_tests.rs
  - docs/agents/tasks/PVP-DEATH-1.md
slot_only_paths:
  - apps/game-server/src/durability/character_death.rs (after merge #2193)
  - apps/game-server/src/combat/pvp/mod.rs (composition)
public_contracts: []
depends_on:
  - "DEATH-1: MERGED"
  - "PVP-1a: MERGED"
  - "DEATH-3: BLOCKING: not merged (death module C)"
  - "PVP-1b: BLOCKING: not merged"
  - "DEATH-ML-LOSS-1 #2193: BLOCKING for shared death writer integration (D1105)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Current allocation: CP D1105 ([6219643346](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346),
2026-10-16) authorizes #2205's own leaf now. Family F allocates only the two leaf files and
this card to its sole writer. DEATH-ML-LOSS-1 #2193 remains the sole physical author of
`durability/character_death.rs` until that PR merges. Shared receipt/binding/load/decode/reconcile
and transaction hunks stay untouched here; no commit, push, ready or merge by this worker.

## Goal

PvP death outcomes (frags, skull consequences, unjust loss) persisted in the death transaction.

## D1105 leaf increment (2026-10-16)

Exact authoring predecessor: `15de676f07d29ed8883850927e8d6b08abdfdad5`, refreshed open/draft
PR #2205, `skeleton/pvp-death-1`. Accepted authority: PARTY-PVP0 §10 and DEATH-COMPLETE0
§4.3/§4.6. This increment implements typed death-outcome planning in `combat/pvp/death.rs`
and 15 tests in `death_tests.rs`, without a second death writer or a new schema.

- Exact 5% classification uses widened integer comparisons; player/summon final blows qualify
  independently of the ratio. Empty or inconsistent totals refuse; PVP-RT still owns genuine
  lethal-time totals, source provenance and the five-minute ledger observation.
- Ordinary outcomes reuse native death XP, current Bless reduction and deterministic item
  selection. Red/black skulls force all equipped-item loss without AoL or a draw; accepted D65
  and vocation exemptions precede item/AoL selection. Twist preserves regular blessings only
  on applicable PvP deaths; no regular blessings means Twist is retained.
- Adventurer's Open/level≤20/not-forfeited outcome precedes XP/item/amulet/blessing loss.
  Conflicting eligible Adventurer and red/black facts refuse rather than inventing precedence;
  the caller must reconcile the real skull/forfeiture preimage. Receipt data determines the
  black-skull 40 HP/0 mana respawn; no current skull read replaces that historical fact.
- The leaf returns semantic receipt data with unfair-fight reduction 0, not a persisted receipt,
  current authority, a mutation permit or a respawn/controller grant. Module composition and
  every real writer/placement caller remain pending; existing draft CI does not exercise an
  unregistered leaf.

**Storage reconciliation:** D1105 explicitly states that columns 0030 suffice and no migration
is authorized/needed. Historical proposals below for additional receipt columns or migration
leases are superseded planning assumptions, not current gates. This leaf does not pretend
that 0030 contains named PvP columns: the sole #2193 writer owns the actual compatible
receipt/binding representation and v1/NULL replay. #2205's shared hunks follow its merge.
No migration, guard or SQL storage change is made by this increment.

Validation performed: owned-file rustfmt and `git diff --check` pass. No Cargo compilation,
native execution or PostgreSQL run is claimed; the 15 tests are authored, not executed.
Independent leaf review is requested before candidate freeze. Full atomic death/replay
acceptance, points/revenge/kill-block writes, genuine current observation ports and native
PG qualification remain OPEN; this source increment does not activate PvP.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| DEATH-1 | MERGED |
| PVP-1a | MERGED |
| DEATH-3 | BLOCKING: not merged (death module C) |
| PVP-1b | BLOCKING: not merged |

**BLOCKING:** DEATH-3; PVP-1b. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Original implementation paths (historical; not this allocation)

- apps/game-server/src/durability/character_death.rs (PvP part)
- apps/game-server/tests/pvp_death_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- `durability/character_death.rs` only after merge #2193 (D1105); E is its sole writer until then.
- `combat/pvp/mod.rs` composition; no production registration in this leaf increment.
- No task migration: D1105 states existing 0030 capacity is sufficient.
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_death.rs` `commit_character_death`

## Inseparable bundles

Shared receipt/binding + writer + replay compatibility, in the D1105 integration order;
no new migration for this task.

## Original family acceptance criteria — OPEN

- [ ] PvP death and its receipt commit as one transaction; reconcile after restart.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Original implementation validation — NOT EXECUTED in this packet

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0; DEATH-COMPLETE-0 #2043. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Death module D; App. 7 PARTY/PVP PVP-DEATH-1.

## Original builder and CP rules

- Work on branch `skeleton/pvp-death-1`. This worker performs no commit or push; root alone may publish the reviewed card to the existing draft.
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: source-bound victim death transaction packet prepared; family acceptance OPEN
status: prepared
branch: skeleton/pvp-death-1
blocker: DEATH-3; PVP-1b
next_action: CP allocates exact death/PvP guard and runtime integrations after review
```


## Historical owning victim death transaction packet (2026-10-15; pre-D1105)

### Scope, authority and prerequisite decisions

Source: `5c65464f770272a315cedda4b6b6638a58363afc`; read-only main:
`bda15eba7a84f65fcb963a86e8b2bf18d69c800a`. Evidence and SHA256 bindings:
`/workspace/oteryn-prep/pvp-death-1-evidence/`. This packet writes only this card; it introduces
no detached calculator, points/refund writer, table, root authority, callback or live boolean grant.
No Rust, runtime, PostgreSQL, death/replay, guard-mutation or client tests execute here.

Owning accepted rules on main:

- PARTY-PVP0 §6–10, through ACCEPT-SOCIAL-MAP0; main reconciles its stale candidate header.
- DEATH-COMPLETE0 §4.1–4.8, accepted by ACCEPT-BATCH-2026-10-15. Its body is unchanged from this
  draft: batch C changes the death receipt once, adds PvP columns/defaults and binds them; batch D
  fills them and changes no schema. Item causes B/C and blessing E delivery are module gates;
  this report does not revive the lifted D1022 item-loss waiver or expose D alone.
- DEATH0 §3, the Reference first-player-death decision, A13 §4.6 and current full guard union.
  A death advances the **victim's** CharacterRevision once with one death receipt. Build losses
  belong in that receipt, not a second BuildCause/death receipt; no contributor revision moves.

PVP0 gap closure remains CANDIDATE on main despite recorded D971 answers. Its15-minute white
extension after unjustified kill, expert reset, world-policy reconciliation and expanded activation
rules are pending, not silently inherited from the accepted15-minute **kill block**. Separate
accepted PARTY-PVP0 requirements from that proposed scope before implementation/release.
Optional/Open are supported rules; first World is Optional with no wars from this task. Hardcore
branches qualify independently, no World is configured Hardcore. Current creature-only/PvP-off
paths remain closed until the owning go-live gate is satisfied.

Prepared history is evidence, not merged APIs or current authority:
`deliverables/pvp-rt-1/RESULT.json` and its card describe actual harm/ledger/control gaps;
`deliverables/pvp-1b/RESULT.json` contains only detached `combat/pvp/skull.rs` and tests. The latter's
reported12 tests do not qualify any death writer; its age<window endpoint assumption and
PARITY_PENDING require architect review. No skull module is present in inspected native source.
Reuse these findings/evaluator at the future owning slot instead of duplicate logic. DEATH-3 and
full PVP-1b remain concrete gates; an archived preparation report is not their completion.

### Exact existing transaction and caller chain

Paths below are relative to `apps/game-server/src/` unless qualified.

| Existing boundary | What source actually does | PvP integration residue |
|---|---|---|
| `gameplay_transport/actor_spell.rs::PlayerDeath`, native lethal/vitals consumers; `ability/player_lethal.rs::PlayerLethalReceipt` | Lethal commit mints one UUIDv7 PlayerDeathOccurrence and cell, stores actor/session death with vitals; receipt includes victim actor/session/World/Channel/scope/vitals revision/cell and independently current carrier check. Receipt is provenance, not an authority token. | Runtime PlayerDeath has only occurrence+cell: no final-blow Character/summon owner, death-time ledger/all-source totals, per-contributor marks, ruleset or source snapshot. Cannot infer those later from UUID time, current viewport, arbitrary client killer ID, or a disappeared source. Capture admitted attribution at real lethal turn without minting another occurrence. |
| `gameplay_transport/mod.rs::{settle_session_death,settle_death,commit_death,player_death_request}` | Bound progression session only; build request from configured progression policy, death cell/map and respawn; held blessings currently Vec::new(). Gets genuine `current_quest_fence`, calls `settle_player_death`; mismatch unbinds that progression interpretation, other unknowns keep dead/retry. | Load actual held blessing/build/equipment/PvP observations, not caller assertions. No current source/final blow/ledger channel exists in request. Preserve unbound/mismatch refusal; never default missing PvP inputs to PvE and respawn. |
| `durability/character_death.rs::settle_player_death` → `RevisionSlot::commit_death` | Acquires victim sequencer slot, reconciles occurrence for original revision, commits at slot cursor/new or original/replay. Initializes missing progression first or consumes a prior already-placed pending respawn, then retries same death. | Retain this entry and one revision; bind extended intent only after required earlier training flush. Do not call raw death plus a separate points writer or acquire contributor revision slots. |
| `commit_character_death`: entry/replay | Validates canonical revisions/blessings/cell sizes, hashes binding/policy, obtains recovery record; opens semantic transaction, checks recovery/admission relations; takes occurrence advisory lock; loads receipt and compares complete binding **before** current gameplay fence. Exact receipt returns AlreadyCommitted. | Extend request/binding/receipt/decoder together with C/D owners. Replays retain original classification and outcomes; they do not reclassify against today's skull/ledger or reacquire live actor/session authority. Wrong semantic reuse must conflict before any mutation. |
| `commit_character_death`: NEW branch | Checks death World+Channel equals fence; `assert_gameplay_fence` locks victim root and current session/node/scope. Refuses pending respawn; locks progression and verifies root revision+context; reads canonical held blessings and matches request. | Join true current/locked PvP, build, equipment and ruleset data into this owning transaction. Immutable intent records expected death snapshot; it is not the independent current fence. Never require contributor live sessions to impose valid historical consequences on logged-out killers. |
| Calculation/write portion | `calculate_progression` calls existing `domain/death.rs::death_experience_loss`; ratio1/1 with Floor required, capped at current XP. Every held blessing currently treated regular; promotion/Premium=false. Root/progression advance once, Harmony/forced Serene zero. Receipt inserted before consuming all blessings, then pending respawn; semantic COMMIT returns committed outcome. | Twist is not an8th regular blessing. Qualified preservation/exemption needs exact before/after selection, not blanket DELETE then refund. PvP exemption cannot be implemented by changing death ratio to0: calculator rejects that policy. No complete skill/ML death-loss writer is invoked; nullable build receipt fields already exist. |
| `command_binding`, `load_receipt`, `decode_receipt`, CommittedCharacterDeath | Binding version1 includes victim/original revision/occurrence, policy digest/all revisions, held blessings, fixed false promotion and absent item inputs, death cell/map/respawn. Result/decoder lacks PvP outcomes and lost-item selection. | C/D add accepted PvP fields, classification inputs, policy and source observation bindings, equipment/item selection and applicable build/entitlement inputs. Preserve compatibility for historical receipts; do not reinterpret old version1 as current new-state authority or silently reject all old replay with a new hash. Physical/storage compatibility needs exact owning version decision, no invented version/wire ID here. |
| `durability/pvp_state.rs`, migration0081 | Strong Character+World state outside progression chain; writeahead DB+70s, admission remaining times, expiry cleanup.0081 protects World/deadline/revision/skull monotonicity, per-victim16+64 ledger; points/marks cite victim death receipt and same transaction. | No kill/death caller, ledger flush/read restore, classification or revenge-closure writer. Public stored-state read is not a complete admitted death snapshot. Existing standalone semantic-pass methods cannot be called to make a child transaction atomic with death. Need owning in-transaction joins. |
| `domain/death.rs`, `domain/progression.rs`, `durability/character_build.rs` | Mature XP/item PvE arithmetic, deterministic canonical slot sampler; build cumulative/relevel helpers. BuildCause has no death, by accepted A13. | Preserve calculators and exact RNG purpose/slot indices. Add accepted PvP outcome routing in victim owner; do not force PvP skull/Adventurer/Twist through PveDeathInput with fake flags or emit a build receipt for death. |
| `place_and_consume_pending_respawn_placed`, `gameplay_transport/mod.rs::{respawn_in_order,place_and_consume_settled_respawn}` | After durable settle, current fenced transaction places actor before consuming pending row and writes last position in same transaction. Refused placement consumes nothing. Current pending record contains occurrence+position. | Add C's completed item-cause gate before any placement/consumption at in-session and admission/recovery paths. PvP receipt determines vitals; no new pending table is required by accepted DEATH-COMPLETE. |
| `ChannelSpellStates::respawn`, `spell/cast.rs::PlayerSpellState::respawned` | Current path refills HP/MP maxima, clears lifecycle conditions and monk state, ends matching runtime death and source memos under one vitals revision. | Black skull at death requires40HP/0mana from committed receipt at placement, including later readmission. Do not refill then compensate or inspect a skull that expired after death. Bind new vitals to actual current actor/session, not receipt as grant. |

The relevant death/PvP/progression/guard sources match refreshed main; main's shared transport
changes add explicit logout and Familiar end-of-actor wiring, not the missing PvP transaction.
`0073_character_progression_guard_wheel_union.sql` is the latest inspected full progression guard
rewrite; it includes all existing receipt kinds and death build fields. Never restore0016's old
two-kind chain or change the receipt count to admit a second PvP/build/refund revision.

### Concrete storage blockers to resolve in owning C/PVP-1b slot

1. `0016_character_death_receipts.sql` still enforces NULL amulet and empty lost-item set.
   No accepted PvP outcome columns exist in native migrations. DEATH-COMPLETE §4.2 allocates this
   receipt change to C, not a second schema in D; fields/defaults and binding must land together.
2. `0081_pvp_state.sql::game_character_pvp_receipt_guard` requires the victim death receipt's
   `created_xact_id = pg_current_xact_id()`, same World, and an unexpired ledger contributor row.
   Insert points/marks **after receipt INSERT and before victim ledger DELETE in the same tx**.
   A later independent transaction citing the receipt is rejected by design, not a seam to bypass.
3. That guard checks contributor `last_at >= clock_timestamp() -5min` at settlement, while accepted
   classification is the victim's window **at death**. Delayed settlement or expiry cleanup can
   remove a valid frozen-at-death contributor before this check. Current PlayerDeath/request has
   no authoritative death-time snapshot for a guard to verify. CP must allocate death snapshot/
   cleanup/guard integration preserving accepted attribution and final-blow retention; do not
   recency-filter away contributors, forge last_at, extract death time from UUID or trust a caller
   bool. This is a source-bound integration gap, not a runtime reproduction in this packet.
4. The same guard rejects every revenge-mark UPDATE, explicitly reserving avenging for PVP-DEATH.
   Accepted §9 needs oldest open mark closed by the avenging death. Extend the owning guarded
   UPDATE predicate with current same-transaction avenging-death provenance and narrow immutable
   keys; never disable the guard, delete/reinsert a mark, or use a post-death refund job.
5. Existing0081 row checks/caps are not a complete death consequence/evaluation binding. CP must
   route any necessary guard/provenance extension with C/PVP-1b and migration lease, despite the
   historical D packet's expectation of no schema change. Reconcile that dependency explicitly;
   no schema number or alternate table is invented by this report.

### Accepted classification and outcome input contract

These are owning requirements for the future adapter, not new authority fields or a public schema.

- Resolve genuine victim Character/World and original lethal occurrence; source Character or
  actual summon master comes from admitted source lineage at hit/application, not client payload.
  Final blow is always represented even when older compact contributors were evicted. Frozen
  source provenance is separate from independently current victim transaction/actor authority.
- PVP-RT supplies complete victim all-source and PvP totals over the accepted five-minute window,
  per10s buckets (31 maximum),16 full+64 compact contributors, damage/assist type and last_at;
  all-source totals include monster, field and condition damage since last PvP hit. Summon damage
  belongs to actual owner. Friendly-fire damage never counts. Ledger caps cannot grant victim
  immunity: demotion/eviction preserves independent totals, death processes at most80 contributors
  and final blow per the accepted retention contract. Exact tie/window endpoint decisions must
  be admitted, not borrowed silently from detached PVP1b's age<window assumption.
- `pvp_death` is at least5% PvP of all damage in that window **or** player/summon final blow.
  Use checked exact comparisons, no guessed floating ratio or zero-total default. It is not
  equivalent to an unjustified kill: a monster final blow with5% PvP is still PvP death; a justified
  player kill is still PvP death. An assist-only record does not invent damage in the ratio.
- Classify each contributor at death using the victim's marks **toward that contributor**, war
  relation or Hardcore. White/red/black are public, yellow/orange viewer-relative. No global
  `marked` cache or just-current skull after long await. Optional cannot gain a fake war; current
  party/guild/friendly-fire membership comes from strong owning reads, not privacy/source role.
- Open unjustified damage contributor gets1000milli; when damage contributors n>5, floor(5000/n);
  assist-only500milli. Compact kinds participate correctly; no duplicate contributor or final-blow
  point. Insert keyed by (contributor,PlayerDeathOccurrence); points never updated/refunded.
  Justified/Hardcore classification gives no unjustified points, but direct/assisting contributors'
  accepted kill block is DB+15min on every World type, never lowered by existing longer deadlines.
- For each new point, reuse accepted PVP1b skull evaluator after integration review. Read complete
  locked DB history, new point's actual DB-stamped time and existing effective skull. Its API
  expects history **excluding** new increment; do not also include freshly inserted row and count
  it twice. Windows24h/7d/30d and red/black thresholds/durations follow §9; no downgrade of active
  skull, new points reset end, expiry remains lazy. Preserve DB timestamp/guard ownership.
- Unjustified death creates seven-day revenge mark; avenging closes oldest unexpired/unavenged
  reverse mark under its actual death transaction. No new XP, money, item or point refund is
  authorized here. Preserve blessings by correct same-transaction after-set; do not consume then
  compensate through a second Character receipt. Death Redemption and unsourced unfair-fight
  formulas are outside this task; unfair_fight_milli is explicitly0/PARITY_PENDING.

| Victim outcome | Required receipt-bound result |
|---|---|
| Effective red/black skull **at death**, PvP or PvE | All equipment/backpack selected lost, regular blessings consumed; Amulet/Twist do not protect, Twist not consumed. D65 no-loss throughlevel8 still applies. Use DEATH-COMPLETE's exemption/selection precedence, not a draw. |
| Other PvP death with held regular blessings+Twist | Regular blessings retained, Twist consumed; ordinary D62/AoL item selection otherwise. With no regular blessings, Twist stays. Separate regular count from twist key before existing XP calculation. |
| Open PvP, level≤20, not forfeited | Adventurer result: no XP/item/blessing loss. DEATH-COMPLETE evaluates this before AoL/draw, so no amulet consumption. Resolve any overlapping skull/forfeiture fact through accepted precedence/current rows; never synthesize impossible eligibility. |
| Other ordinary outcome | Existing D58/D59/D68 XP and admitted D62/item rules; current promotion/Premium provider needed before applying its benefit. Source hardcoded false does not prove entitlement status. |
| Black skull at death on respawn |40HP/0mana from committed receipt; other respawn vitals follow accepted native policy. Later skull expiry does not change this death's outcome. |

First aggression forfeits Adventurer before harm under acting Character's rule2 fence (PARTY §10.4,
PVP-RT owner). DEATH-COMPLETE §4.6 additionally names same-transaction forfeiture. Future C/D/RT
adapter must join those requirements against current row, not defer first aggression until death or
forfeit every non-aggressor victim as a convenience. Any unresolved event/provenance interpretation
returns to owning architect; no ad-hoc forfeiture state transition or fake initial false here.

### Proposed owning transaction and integration sequence

1. **Lethal physical turn (RT owner).** Commit original native vitals/death occurrence once, capture
   complete admitted death-time attribution/ruleset/marks/ledger observation with source and owner
   provenance. Retain bounded death-pending state across await/retry; no subsequent input or harm
   on dead actor. Do not hold runtime lock waiting for victim sequencer or durable transaction.
2. **Earlier training flush (build owner).** If pending mana/tries/stamina exists, use genuine
   `commit_character_build` checkpoint and reconciliation in its **earlier** transaction. Reuse
   native cumulative/formula helpers and known loss policy; no guessed ML/skill rate. Only after
   observed flush COMMIT choose victim cursor/fence and build death binding; unknown flush must
   reconcile, not become zero pending. A13 carries actual death loss in nullable all-or-none fields
   of this death receipt, not a second build receipt. No loss leaves those fields as allowed by
   existing guard. Current death caller does not implement this full join.
3. **Victim sequencing and receipt-first replay.** Keep `settle_player_death` → victim RevisionSlot
   → `commit_character_death`. Preserve occurrence advisory lock, recovery/node/admission/current
   session+lease/connection/scope fences, distinct World/Channel and root/context/revision checks.
   Original receipt/binding decides replay; historical terminal outcome needs recovery scope, not
   resurrected session control. On NEW-write, immutable snapshot defines expected inputs only.
4. **NEW-write locked observation.** Victim root is already locked by gameplay fence, then victim+
   contributors' PvP states in CharacterId order (missing rows inserted), then point/mark rows as
   accepted PARTY §9. Keep native progression/build/equipment/item-lock order with C/D owners;
   preflight full bounded row/resource/selection footprint before mutation. No contributor
   progression root locks/revision slots; offline contributors still receive consequences. Current
   ruleset/content and original source snapshots require an admitted compatibility/provenance
   interpretation, not live authority derived from the death record.
5. **One Character commit.** Stage exact outcome using existing XP/item formula owners, update
   victim root/progression/build once, insert receipt carrying before/after loss and PvP/item/
   blessing evidence, then mutate only blessing keys absent from after-set. In that **same SQL
   transaction** write justified/unjustified classification consequences, points/skulls/kill
   blocks and new/avenged marks while required ledger rows still exist; delete victim ledger in the accepted order and
   reconcile its totals with the owning death/window snapshot, insert/retain pending respawn and
   any events required by the owning accepted contracts.
   Schema/guard ordering must be verified against actual constraints; no callbacks that commit
   their own child transactions or broad role bypass. Check all guards at COMMIT, rollback all
   staged Character/PvP/blessing/ledger effects on rejection. Exactly one victim death receipt
   explains the revision; classification alone never emits a second XP/build/refund receipt.
6. **Ambiguous COMMIT.** `RevisionSlot` invalidates cursor on unknown, death caller stays dead/retry.
   Keep exact request/occurrence/binding/decision outputs, reconcile retained receipt, and use
   original revision for replay. Never recalculate after resetting ledger, reinsert points after
   receipt-only reconcile, retry under changed source, consume a second blessing, or call item
   causes because COMMIT was merely attempted. Historical receipt must expose every accepted
   outcome needed for replay; decoder/default/version compatibility is part of C/D acceptance.
7. **Post-commit outputs and independent descendants.** After observed death receipt, publish
   committed per-contributor skull/blocks/marks to current owning Channels before subsequent
   affected input under each actual current owner. Receipt is consequence provenance, not new
   controller authority. Source event/retry identifiers prevent duplicate notices/effects. C's
   PlayerDeathDrop/AoL burn causes are separate idempotent DUR-03 transactions in receipt order,
   never claimed atomic with Character COMMIT. Their unknowns reconcile by existing cause; a
   terminal custody violation holds death-pending/escalates, no automatic refund/recreate/drop
   repair. Descendant failure cannot erase parent or its points, and no PvP kill XP is minted.
8. **Placement, restart and output chain.** Resume unfinished item causes from authoritative
   receipt after restart/generation change, then actual current-fence placement and consume pending
   row/write last position, then receipt-selected vitals in one owner revision. Both admission and
   live respawn must carry original skull_at_death and item-work completion. Preserve replay of
   already consumed/placed outcomes and native no-control rejections. No pending-respawn schema
   fork, fresh default full HP, or compensation hit after a wrongly full refill.

### Future qualification matrix — NOT EXECUTED

Every row needs actual native positive control and one changed applicable invariant per negative;
use current owning DB/session/actor reads, not record-derived matching constructors. Bind runtime
and PG17.6 tests to final integrated source/migrations; source trace and old PVP1b tests do not count.

| Invariant / mutation | Owning consumer and required observation |
|---|---|
| Victim Character/session/account, connection/lease/scope generation, World vs Channel mismatch, node revoke, retired actor | NEW writer/placement refuse without unauthorized mutation. Exact historical receipt remains recoverable without granting controller authority. |
| Same occurrence replay vs substituted victim/cell/map/source/ruleset/item/marks/classification, future binding version, historical version1 | Advisory-lock/replay branch returns original once or exact conflict; no partial points/loss, no recomputation under current policy, no legacy receipt upgraded to PvP authority. |
| Concurrent XP/build/death, stale expected revision, exhausted revision, unknown training flush | Real victim sequencer preserves one chain; no bypass/retry-at-new-revision for semantic death conflict. Earlier committed flush retained; unknown reconciles before binding death. |
| Incomplete all-source ledger, missing final blow, source summon reparent/despawn, duplicate/compact contributors, friendly-fire | Lethal capture and death NEW observation refuse unqualified attribution; no fabricated killer or default PvE. Actual final blow survives cap eviction without duplicate point. |
|4.999%/5%, player final blow with small ratio, monster final blow with5%, zero total, checked-product overflow | Exact classification and receipt fields distinguish PvP death from unjustified kill; no assist damage invented, no floating threshold drift. |
| Five-minute death-time window vs settlement delayed past5min, cleanup races, lost new-contributor writeahead, restored bucket endpoints | Actual guard/cleanup/death snapshot join preserves accepted contributors and ratio. Missing source facts never repaired by forged timestamps or silent dropping; integration blocked until qualified. |
| Source/victim mark private to other viewer, expired orange/red/black at death, party/guild/war membership changed | Classification binds mark toward each contributor at death and actual admitted relation; no public/private cache grant or unmerged PVP-RT report treated as current authority. |
| Damage n5/n6/n7, assist-only500, full16/compact64/81st distinct, point duplicate or outside1..1000 | Accepted bounded points/ledger behavior, no immunity on ledger full, no second entry for same contributor/death, no contributor CharacterRevision advance. |
| New point included twice, incomplete history, exact24h/7d/30d endpoint, future stamp, red/black threshold/effective expiry/newpoint reset/overflow | In-transaction PVP1b adapter uses actual DB stamp/history and reviewed endpoint; guard rejects downgrade/shorten. Detached evaluator cannot attest provenance or authorize write. |
| Points/mark insert in later tx, ledger delete before insert, wrong victim/world/contributor, forged created_xact |0081 owning guards reject entire attempted consequence; no alternate post-death writer or disabled guard. |
| Avenging wrong/newer/expired/already-avenged mark, duplicate avenging death, changed immutable keys | New narrow guard only permits oldest valid reverse mark in actual avenging death transaction; no point, currency or item refund invented. |
| Twist counted regular8, zero regular+Twist, regular+Twist, red/black withTwist, Adventurer level20/21/forfeited, low-level/no-vocation exemption | Same transaction chooses exact before/after blessing/item/XP outputs, consumes only authorized keys; no delete-then-refund or extra build receipt. Unsourced unfair-fight stays0. |
| Death root update/receipt/blessing/points/marks/skull/ledger/pending/event failure, deferred guard failure, canceled await/unknown COMMIT | Rollback no partial Character/PvP outcome; unknown retains pending and reconciles one exact receipt. Child item failure after committed parent does not roll parent back. |
| C receipt fields missing/default mismatch, current latest guard arm omitted, nil/partial death-build fields, wrong source receipt chain | Migration/decoder/whole-chain tests reject inconsistent updates while all existing receipt families remain accepted; no restoring0016 two-kind chain or additional victim revision. |
| Restart after receipt/before item cause; cause unknown/terminal; after placement/before pending delete; last-position failure | Native C/D workflow resumes receipt-authorized causes once, holds respawn until complete; refused current fence calls no placement. Never loses pending death or fabricates value repair. |
| Black-at-death skull expires before placement, wrong/new actor session, already consumed respawn,40HP/0MP vs normalfull | Both live/admission placement derive vitals from receipt under current owner; no full-refill compensation and no stale authority restore. |
| Contributor logout/transfer/offline during victim settlement; concurrent state raise; duplicate outcome delivery | Historical consequences still commit in victim tx with ordered PvP locks; actual current Channel installs only legitimate own outputs; no contributor root lock or receipt-derived controller grant. |

### Local validation and remaining gates

Only the owned card changes. Appropriate local checks are governance validation, governance
unittests124, diff whitespace and exact allocation/base/branch/staging/untracked/sourcehash/push
hook+URL checks. Executed: governance passed (22 required documents,9 project lanes); governance
unittests124/124 passed; allocation-only scope and diff checks passed. Evidence binds28 repository
inputs and6 archived history inputs to final card bytes. No Rust/runtime/PG/replay/guard/client
test count is claimed.

Original family acceptance remains OPEN: one native PvP death/receipt transaction and restart
reconciliation still require implementation/test proof; integration-head residue must be rechecked.
Pending: C/B/E/full death-module delivery, PVP-RT current death-time observer/ledger, PVP1b writer/
rulesets/evaluator endpoint acceptance, exact C/PVP1b guard/provenance lease (including mark close
and delayed settlement), A13 loss/training joins, migration/receipt compatibility, current-authority
and workflow tests, independent persistence/combat/security/privacy/protocol and architect review,
and CP shared queue slot. No worker commit, push, ready flag, merge or external publication.
