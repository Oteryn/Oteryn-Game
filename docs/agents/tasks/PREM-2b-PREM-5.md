# PREM-2b+PREM-5

```yaml
task_id: PREM-2b+PREM-5
title: "PREM-2b+PREM-5: promotion state and promotion/Premium blessing NPC services (skeleton)"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/prem-2b-prem-5
issue: 1622
pr: 2194
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 PREREQ-1; App. 7 Supporting PREM-2b+PREM-5; wave 2"
class: "hard"
migration: maybe
registry: maybe
owner: "Codex family lead: Rozwój postaci i trening; policy worker /root/training_review"
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-19
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/premium.rs
  - apps/game-server/src/premium/promotion.rs (new)
  - apps/game-server/src/premium/blessing_purchase.rs (sole leaf author /root/vitals)
  - apps/game-server/src/durability/npc_purchase.rs (D1176 lower transaction participant; recovery rebuild CP 6230877600)
  - apps/game-server/src/durability/item_fee_burn.rs (D1106 source-specific causes only)
  - apps/game-server/src/durability/item_fee_burn_audit.rs (matching D1106 audit causes only)
  - apps/game-server/src/durability/premium_activation_latch.rs (promotion-specific composition only)
  - apps/game-server/src/gameplay_transport/spell_premium_coordinator.rs (D1106 native facts/gate composition only)
  - apps/game-server/src/interaction/npc_promotion.rs (new)
  - apps/game-server/src/premium/mod.rs (D1160 own-module registration lines only)
  - apps/game-server/src/interaction/mod.rs (D1160 own-module registration line only)
  - apps/game-server/tests/promotion_*.rs (new)
  - apps/game-server/src/durability/mod.rs (D1160 single own-module `npc_purchase` registration only)
  - apps/game-server/tests/support/item_fee_burn_postgres_cases.rs (D1164 CharmUnassign let-else hunk only)
  - apps/game-server/tests/support/gold_fee_bank_postgres_cases.rs (D1164 CharmUnassign let-else hunk only)
  - apps/game-server/src/premium/blessing_catalogue.rs (new; BLESSINGS-CONTENT-1 source reader)
  - apps/game-server/src/gameplay_transport/promotion_npc_postgres_tests.rs (PREM-5 e2e)
  - apps/game-server/src/gameplay_transport/npc_command.rs (Premium talk and reconcile hunks; slot unlocked after #2197)
  - apps/game-server/src/gameplay_transport/mod.rs (Premium module registration, resume, death held-set and NpcTalkOwners install hunks; slot unlocked after #2197)
  - apps/game-server/src/node/serve.rs (production Premium NPC service install; CP authorization 2026-10-18)
  - rulesets/character/blessings/ (from #2237 BLESSINGS-CONTENT-1; owner pricing ruling 2026-10-18 edits)
  - docs/reference/content-prep/blessings-content-1/ (from #2237 BLESSINGS-CONTENT-1; owner pricing ruling 2026-10-18 edits)
  - apps/game-server/src/durability/character_promotion.rs (new; #2256 durable promotion read)
  - apps/game-server/src/durability/character_death.rs (#2256 D76 promotion and Premium inputs only)
  - apps/game-server/src/durability/character_wheel.rs (#2256 durable promotion read only)
  - apps/game-server/src/durability/charm_state.rs (#2256 BestiaryCharmFacts promotion read only)
  - apps/game-server/src/durability/premium_activation_latch.rs (#2256 shared-lock latch read for the death writer)
  - apps/game-server/src/durability/mod.rs (#2256 own-module `character_promotion` registration line only)
  - apps/game-server/src/gameplay_transport/mod.rs (#2256 death Premium gate, login promotion and PremiumSessions hunks)
  - apps/game-server/tests/character_authority_postgres.rs (#2256 death request literal only)
  - apps/game-server/tests/support/character_death_writer_postgres_cases.rs (#2256 cases and request literals)
  - apps/game-server/tests/support/character_wheel_postgres_cases.rs (#2256 durable promotion seed and case)
  - apps/game-server/tests/support/character_wheel_cast_postgres_cases.rs (#2256 durable promotion seed)
  - apps/game-server/tests/support/charm_state_postgres_cases.rs (#2256 durable promotion case)
  - apps/game-server/tests/support/character_last_position_postgres_cases.rs (#2256 death request literal only)
  - apps/game-server/tests/support/character_progression_admission_postgres_cases.rs (#2256 death request literal only)
  - apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs (#2256 death request literal only)
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs (#2256 death request literal only)
slot_only_paths:
  - apps/game-server/src/lib.rs (premium module registration after #2197)
  - migration number
  - mod.rs
  - content NPC service pins
  - apps/game-server/src/durability/bank.rs (cause arm after #2197)
  - apps/game-server/src/interaction/npc_talk.rs (dispatch after #2197)
public_contracts: []
depends_on:
  - "PREM-WIRE-1 (PREMIUM-1): MERGED #2110"
  - "NPC-TALK-1: MERGED #1976"
  - "PREMIUM_ACTIVATION ACCEPTED: MERGED #2089"
  - "promotion source: ACCEPTED #2209, ACCEPT_BATCH §3; WHEEL-0 §6.2 pre-delivery exception effective through #2161"
  - "spell P3b-2 vitals and progression #1143: MERGED/completed per accepted WHEEL-0 §6.2"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

D1160 exception: only single own-new-module registration lines in `lib.rs`/`mod.rs`/PG root may be authored now; all other shared integration below remains slot-only.

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

(PREM-2b) promotion state, effective promotion at login, soul maxima, death input, Wheel promotion read; (PREM-5) promotion and Premium blessing NPC services. One allocation.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PREM-WIRE-1 (PREMIUM-1) | MERGED #2110 |
| NPC-TALK-1 | MERGED #1976 |
| PREMIUM_ACTIVATION ACCEPTED | MERGED #2089 |
| a promotion source | Hold released by accepted #2209, ACCEPT_BATCH §3; see local preparation |
| spell P3b-2 vitals, progression readiness | UNKNOWN: CP verifies |

The promotion-source hold is released by effective ACCEPT_BATCH §3 (#2209). This does not
authorize slot-only paths or a durable writer: current transaction/gate/vitals seams and the
inseparable monetary/state bundle still require CP allocation and integration validation.

## Owned paths (builder edits freely)

- `apps/game-server/src/domain/premium.rs`
- apps/game-server/src/premium/promotion.rs (new)
- apps/game-server/src/interaction/npc_promotion.rs (new)
- apps/game-server/tests/promotion_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- mod.rs
- content NPC service pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `domain/premium.rs` (PREM-2a pure rules)
- `gameplay_transport/npc_talk.rs` service hook
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

PREM-2b+PREM-5 is one '+' bundle; migration+writer; gold burn+ledger for the promotion price.

## Acceptance criteria

- [ ] Promotion fail closed without Premium; price burned through the fee ledger.
- [x] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue'); the allocated head is recorded below.

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PREMIUM_ACTIVATION0 §2-§4.3; WHEEL0 §6.2. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 PREREQ-1; App. 7 Supporting PREM-2b+PREM-5.

## Builder instructions

- Work on branch `skeleton/prem-2b-prem-5`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: D1106 native promotion TX, fee audit, gate composition and NPC consumer authored
status: NOT_FULLY_QUALIFIED
branch: skeleton/prem-2b-prem-5
blocker: ordered sequencer handoff; installed schema/registration slot; qualified blessing catalogue
next_action: native typecheck/RED-GREEN and independent review; integrate D1106 native dependencies at their slot
```

## Historical local preparation (2026-10-15; exception superseded below)

Draft #2194, `skeleton/prem-2b-prem-5`, initial/unchanged head
`1a02f222cfd0b2102e11e828e66b6282ce6ac469`. The lead confirmed accepted #2209 at main
`e2da9805d296a27b06b32335bf67394509d4910b`; its effective documents were read in the
primary checkout read-only. Publication is root-only; this worker makes no commit/push.

**Exact source/precedence.** ACCEPT_BATCH §3: “The promotion source is the PREM-5 NPC
promotion: 20 000 gp, level 20 or higher, Premium … paid in coins via GOLD-FEE-1b.”
PREMIUM_ACTIVATION §4.2 says promotion is bought once and durable promotion survives lapse;
benefits are checked at use, while displayed vocation/title change at login. §4.4 requires
Premium to buy the three named Premium blessings and keeps held blessings until death.
The older WHEEL-0 §6.2 says “before PREM-1, the Premium condition is not applied”, but that
document still has status **CANDIDATE**. The lead resolved application of the accepted §3:
use the existing current `PremiumGate`, with no new `NotActivated` bypass. No pending
amendment or deployment activation is presumed effective, and no new product decision is made.

**Residue confirmed before code.** The native `domain/premium.rs` already provides the one
Premium gate, `promotion_benefit_current`, Premium-selected soul maximum and immediate soul
clamp. It remains unchanged, including its later owner soul amendment; the older §4.3 prose
does not replace it. The new promotion policy calls those existing predicates and adds only
level >=20 / not-already-promoted eligibility and login display projection. The NPC policy
quotes exactly 20,000 coin gold and gates the three accepted Premium blessing purchases.
It assigns no blessing price, content key or durable bit. Held blessings are not modified.

All outputs are policy data, not an authorization/payment permit. Level and promotion inputs
do not establish Character/World/session ownership or locked durable state. The native gate
must be obtained through the existing current gate seam at the actual command/use boundary;
no entitlement class is recreated and no approval is cached by these helpers. A login display
snapshot never becomes the input of per-use gameplay benefits.

### Cross-domain findings for the CP slot

- `durability/item_fee_burn.rs::FeeBurnCause` and
  `item_fee_burn_audit.rs::FeeBurnCauseV1` admit only `CharmUnassign`. A promotion source cannot
  be passed to the current closed writer/audit shape. Extend it with its accepted cause and
  guards in the inseparable bundle; preserve coins-only GOLD-FEE-1b behavior rather than
  silently using the existing GOLD-FEE-2 bank fallback. This preparation invents no writer,
  table, receipt, fee plan, transaction or atomicity claim.
- Durable `promoted` plus provenance and the coin burn need one current fenced, idempotent
  Character transaction. `character_build.rs::BuildCause::Promotion` exists but is not that
  promotion-state/fee source. Current vocation transitions and qualified NPC service identity
  must be resolved by their owners, not guessed from displayed names or client data.
- `gameplay_transport/npc_talk.rs`, `SpellPremiumCoordinator::with_premium_gate`, login
  vocation/title projection and NPC content service mapping need allocated wiring. The
  blessing purchase writer, prices and held-state handling are not supplied by a Premium check.
- `durability/character_death.rs` still binds promotion/Premium as false; D76 requires current
  promotion/Premium inside death, not the login display. `character_wheel.rs::WheelChangeFacts`
  still takes a caller promotion value; `charm_state.rs::BestiaryCharmFacts::promoted` returns
  false. They need the actual durable owner read. These paths are outside this allocation.
- `spell/cast.rs::PlayerSpellState::apply_owner_premium_transition` already uses native soul
  rules. Current vitals/readiness and promotion-regeneration hooks need owner verification
  at the slot; this preparation does not replace those seams or alter native soul behavior.

### Local validation

The isolated no-dependency harness compiles the actual new modules via the actual portable
`tests/promotion_policy.rs`, plus the unchanged actual native Premium domain. Its facade uses
the expected crate name only for that test import; it is not the full server. The only copied
type fragment is verbatim `EmptyLatchHeld` plus its existing `cfg(test)` helper, bound to the
full original source hash. Native logical `Current` fixtures are not producer/DB authority
qualification. One harness-only `dead_code` allowance covers the unchanged native domain's
uncomposed coordinator constructor, with no allowance on new policy/test code or production.

- Initial actual-source GREEN: 9 policy tests and 7 existing native tests passed. Its original
  unused-constructor warning log is retained separately; no artificial behavioral RED is claimed.
- Final actual-source GREEN: 9 new policy tests +7 unchanged native Premium tests passed,
  0 failed/ignored. Harness all-target clippy with `-D warnings`, explicit owned-file
  `rustfmt --check` and workspace `cargo fmt --all --check`: passed.
- Thirteen original-source/contract/harness inputs were hash-verified before and after final
  checks. Commands and exact exit statuses are retained in `checks-results.json`.
- Governance validator passed (22 documents, 9 lanes); all 124 governance unit tests passed.
- Whole-diff adversarial self-review, owned-path/unchanged-head/native-source checks and
  `git diff --check`: passed. Only the three new allocated Rust files and this task record
  changed; native `domain/premium.rs` is byte-identical to the allocated head. No worker
  commit, push, paid review or Rust job remains queued.
- No full-server compilation, fenced monetary/state transaction, PostgreSQL, deployment
  activation or playable NPC E2E is claimed. The atomic fee-ledger acceptance remains open.

Raw logs, harness scripts, source/contract hashes and check commands are retained outside the
repository at `/workspace/oteryn-prep/prem-2b-prem-5-evidence/` for handoff/rotation.

## Accepted pre-delivery policy correction (2026-10-16)

- Exact starting draft: `b0644c352c214368de328f87a36a0bddeb0ac210`, #2194,
  `skeleton/prem-2b-prem-5`. One family author changes only
  `apps/game-server/src/domain/premium.rs`, `src/premium/promotion.rs`,
  `src/interaction/npc_promotion.rs`, `tests/promotion_policy.rs` under that app,
  and this task record. No shared registration, durable writer, fee shape or
  monetary/state transaction is added.
- Protected main `24fc332f256638e3b784bff1b3c85b18e2665a76` accepts WHEEL-0.
  Its §6.2 and PREMIUM_ACTIVATION §4.2 exempt promotion purchase/benefits from
  the Premium condition until delivery. ACCEPT_BATCH activates accepted
  amendments; the historical CANDIDATE rationale and no-exception policy above
  are superseded. The already accepted NPC source does not need reapproval.
- Promotion policy now borrows native `PremiumStatus` inside the existing gate
  closure. The promotion-specific native requirement selects its exception only
  with `&PreDelivery`, whose empty-latch witness remains borrowed from that gate.
  It creates no witness, stored approval, grant, fee permit or alternate entitlement
  classification. Each command still needs its own native status and current facts.
  General `PremiumGate`, all three blessing gates and the owner soul amendment are
  unchanged. After delivery, unknown/unavailable/NotCurrent still refuse; Current
  permits eligible promotion. Durable promotion is not cleared by lapse.
- Native domain regression tests use the owner's existing test-only latch witness
  to distinguish pre-delivery promotion eligibility from the still-closed general
  gate, then verify post-delivery refusal/renewal. Existing actual-source policy
  tests retain level 20, exactly 20,000 coins, duplicate-purchase refusal, login
  display and Current-only blessings using borrowed native status. The tests
  are logical policy evidence, not database/producer authority qualification.
- The behavioral RED snapshot preserves the old status-to-gate behavior with the
  new native regression assertions. A real native library run produced exactly
  the two expected failures (NotCurrent instead of the pre-delivery allowance).
  Restoring the fix passed all nine native Premium tests and all nine explicit
  actual-source promotion/NPC policy tests on Rust 1.94. No extracted-owner bridge
  or database execution is claimed. Broader exact-candidate checks remain separate.
  Owned-file rustfmt and diff checks pass; governance validator and all 124
  governance tests pass. The pre-delivery witness fixture remains inside its
  native owner test module; no public test factory or duplicate policy-source
  compilation context is introduced for the detached integration tests.
- Full PREM-2b+PREM-5 remains PARTIAL: durable promotion/provenance plus coins-only
  burn/audit in one fenced idempotent transaction, blessing purchase writer,
  qualified NPC/login/death/Wheel consumers and PostgreSQL qualification still
  require their CP allocations. Re-entry is an actual allocation/integration event;
  no background continuation, ready, merge or deployment authority is asserted.

## D1106 native source authoring (2026-10-16)

CP [6219643346](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6219643346)
and the direct family handoff expand authoring at head
`226b1f7ab1ef239922ba6a96504969160ef292d5`: durable promotion/provenance,
coins-only fee/audit, sequencer admission/cause, blessing purchase and NPC consumer.
This supersedes the historical instruction to wait for a promotion writer allocation.
The existing-file author changes promotion/NPC files, tests and this card, plus
the exact Promotion cause/audit hunks in `durability/item_fee_burn{,_audit}.rs`
and source-specific native gate composition in
`durability/premium_activation_latch.rs` and
`gameplay_transport/spell_premium_coordinator.rs`.
The separately allocated blessing leaf has sole author `/root/vitals`; no same-file
writers overlap. No `bank.rs`, shared registration, registry or migration file is changed.

- The promotion SQL source stages the native coins-only 20,000-gold fee,
  Character receipt and permanent flag/provenance in one Type2 transaction.
  It resolves historical replay first; a fresh purchase independently checks
  native recovery/session/lease/scope/node/root/current-content and locked level
  and promotion state. The actual outer Type2 owner rolls the whole transaction
  back on a typed source refusal; no subtransaction changes native xmin provenance.
  A fee without its source receipt is refused, never converted into another grant.
- The promotion-only gate opens the actual Type2 transaction before taking
  the Premium latch, preserving the same transaction and borrowed witness.
  The existing ordinary Premium gate path is unchanged. The source consumer
  holds the Character slot through replay, actual actor/NPC/conversation/range
  checks, the single Premium gate operation and COMMIT; only then does it adopt
  the source receipt. Stable CommandRef/target-derived identities survive retry.
  Historical replay precedes live NPC/catalogue lookup and grants no new authority.
- Fee cause-kinds 2 and 3 and protobuf oneof tags 2 (`promotion_purchase`) and
  3 (`blessing_purchase`) of `OneItemFeeBurnCauseV1` are owner-approved
  (2026-10-18) and land in `native_one_item_transaction.proto` with this PR.
  Existing Charm request-binding bytes and fee tuple selection are preserved.
  Promotion cannot open the bank under either actual Type2 tuple.
- A typed source catalogue port supplies server-resolved NPC/service and
  blessing key/level-price data; the command accepts no client fee or blessing
  key. No qualified implementation or live blessing offer is claimed while
  the target-date price evidence conflicts and the canonical surface is empty.
  Blessing payment must retain DEATH_COMPLETE0 §4.7's shared bank fallback;
  promotion's coins-only rule must not be copied to blessings.
- The sequencer's current physical author remains LOOP-PROGRESS-1. Its native
  recovery/admission, delivered transaction and post-gate cursor-adoption hunk
  is prepared outside source for an explicit ordered handoff. Proposed state,
  receipt, guard and privilege DDL is unnumbered and uninstalled. CP must assign
  its migration/guard-union slot; no private schema substitutes for that event.
- The nine existing policy cases are preserved in the source's native unit
  context; the standalone target no longer path-compiles the production writer
  into a different native type graph. Two source-binding tests and three native
  fee/audit cases cover mismatched identities, replay-local fences, wrong price,
  bank exclusion, bad occurrence and typed audit round-trip. Native execution
  and PostgreSQL rollback/restart/concurrency qualification are pending; no
  native test pass is claimed by authoring. Owned rustfmt and diff checks pass.

Validation dependency/registration manifests and proposed DDL are retained at
`/workspace/oteryn-family/evidence/pr2194/d1106/`. They authorize no tracked shared
registration or premature slot edit. This child still needs native qualification,
ordered dependency integration, installed schema/guards and the qualified blessing
source. Login/ADMIT, death after #2193 and Wheel reads are later children, not
blockers for implementing this D1106 source. Full PREM-2b+PREM-5 remains PARTIAL.


### Native PG generation2 finding — repaired authoring generation3

Actual native PG COMMIT rejected the promotion source: a source SAVEPOINT gave
root writes a subtransaction xmin, while the unchanged native fee guard proves
root xmin equals the top-level transaction ID. The repair preserves that native
guard, removes the source savepoint, and makes only the promotion-specific latch
callback typed-fallible: Ok commits its actual Type2 TX; Err rolls that same TX
back. Ordinary Premium gates remain unchanged. The pre-delivery witness stays
borrowed inside the gate. The native delivered owner already rolls back Err.

The external negative test now also prepares all source writes and then returns
a typed error, requiring no coins/fee/audit/promotion/receipt/revision persistence.
Generation2 real ELF, source postimages and execution logs are retained. No new
native/PG pass is claimed before root schedules the repaired generation. CP/arch
comment6221184326 also adds lib.rs to deferred slot-only registration; blessing
source authoring continues as a separate allocated leaf, not a policy-only close.


### Coherent blessing integration — generation4 preparation

CP/architect [6221184326](https://github.com/Oteryn/Oteryn-Game/pull/2194#issuecomment-6221184326)
requires the actual blessing writer alongside promotion, and defers bank cause,
migration, premium module/lib registration and npc_talk dispatch after #2197.
The new typed blessing participant and native retry/transaction wrappers now
retain the selected service/key, whole held sets, qualified source price band,
fee/audit/provenance and optional native BANK debit in one outer Type2 TX. The
source chooses its price at the locked Character level, disregarding any quoted
fee. Exact replay resolves before current Premium/fences and grants no authority;
new purchases are strictly Current-only, with no promotion PreDelivery exception.

The actual distinct blessing NPC consumer rechecks current physical actor/NPC
conversation facts, requires the pinned NPC declaration to offer that selected
blessing service, resolves its stored receipt first, and keeps its Character
slot through one ordinary Premium gate and real source COMMIT. No client fee/key
or reusable permit is accepted. Production catalogue entries remain unpopulated
until the source owner qualifies the conflicting price/key evidence. This is an
explicit integration limitation, not permission to invent source facts.

Proposed fee cause/tag3 binds a distinct native blessing_key; existing Charm
request bytes, SQL path, fee/bank guards and promotion coins-only behavior stay
compatible. Blessings use the accepted DEATH_COMPLETE0 §4.7 coins-then-BANK fallback
under the actual V2/nonjunior native rule. No bank.rs edit occurs. The source and
SQL proposals retain both directions of receipt/fee/held-grant proof, the complete
latest0073+promotion+blessing chain and recovery history. All DDL remains external,
unnumbered/uninstalled, with final cause/tag/migration/registration slots reserved
for CP. The independent blessing leaf review and independent xmin repair review
approve their exact source slices only.

Actual generation2 native unit compilation passed; five focused pure tests
passed, but the first PG case rejected COMMIT in the preserved native xmin guard.
The repaired generation3 source removes the savepoint and rolls the actual outer
TX back on typed Err; it has source approval but no PG PASS yet. Generation4
combines that repair with blessing writer/consumer, native fee/audit negatives,
locked-price/replay/concurrency/Current-only/real PreDelivery refusals and rollback
proposals, plus a V2 bank-backed/one-gold-shortfall case. No generation4 build or
PG test has run; root serializes qualification. A native root/pool reopen is not
a process/server restart or full task completion. Full scope remains PARTIAL.


### 2026-10-16 — actual generation5 evidence; generation6 lint repair proposal

The generation2/3/4 preparation statements above are historical. Actual native
generation5 compilation and all 25 focused tests passed, including all 7 native
PostgreSQL tests. This result belongs only to the preserved generation5 ELF
(`28e5aad231058e501686eec0e67c08f8dd491b17fcd566fc564166265ea87e5a`)
and exact native source/composition manifest
(`be979b65d6046b4d7c4ace41bba61ecd943a57f3ce61e89a9757b610ee161fcb`),
as recorded in `native-validation/generation-5-prepared/EXECUTED-GEN5-RESULT.json`.
The private schema/registration overlay and explicit catalogue policy fixtures
remain the execution scope; production source/content/CP slots remain separate.

Actual generation5 strict all-target Clippy found five errors: the pending login
projection caller, three retained error payloads, and the borrowed promotion
bypass signature's type complexity. Actual fmt found the private native include
needed wrapping. The generation6 proposal addresses those findings with five
reasoned function/variant/method-scoped allowances and rustfmt include layout.
It preserves API/visibility/error payloads, runtime bodies, native guards and the
25-test/7-PG inventory. The generation5 blessing permission repair remains intact.
No generation6 native/strict/full PASS is claimed here. Exact generation6 head,
native, strict and full-check results will be recorded in the PR after execution.
Full scope remains PARTIAL, including deferred CP-owned integration.


### 2026-10-16 — D1122/D1160 existing-draft checkpoint proposal

Checkpoint status: **NOT_FULLY_QUALIFIED**. This checkpoint retains all eight Rust
source/test files from frozen source-generation-8 byte for byte and adds only
three own-module registration lines: premium::promotion,
premium::blessing_purchase, and interaction::npc_promotion. Existing durability
fee/audit/latch and gameplay spell_premium_coordinator modules were already
registered. Native module registration is now allowed by CP D1160; it does not
release any foreign context, API re-export, transaction, schema or NPC caller slot.
CP authority: https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6224912485.

Actual private generation8 native composition passed all 25 focused tests,
including all 7 PostgreSQL 17.6 tests; retained ELF SHA256
`ae8a61a947ed5f200a786cf9a3bc5bf5a423f2c9d92a2c8a1f4e4b3d0096473b`,
private source/composition manifest SHA256
`11c3e097d318983f025231317a493789121690979374c7443a5087a79a003081`.
The same private native library passed strict `cargo +1.94.0 clippy --locked -p
oteryn-game-server --lib --quiet -- -D warnings`. These are PRIVATE composition
results with the captured uninstalled schema/foreign context/test overlay. They
are not qualification of the canonical source9 graph or this newly registered graph.

Canonical source9 strict qualification completed RED: E0005 in foreign
`tests/support/item_fee_burn_postgres_cases.rs:333` and
`tests/support/gold_fee_bank_postgres_cases.rs:421`: formerly irrefutable
CharmUnassign destructuring is now refutable with the new PromotionPurchase and
BlessingPurchase variants. Evidence: native-validation/canonical-owner-generation-8-gates-attempt1/FINAL-RESULT.json,
SHA256 `30368fa5197eee888ff988658e22acd8bde10aaa0334ce56bee9716a15824122`.
CP6225267177 assigns the two thin caller fixes to #2194 in its final slot after
#2197; actual raw paths corrected in CP6225385170. No foreign test edit included.
This result covers only its original source9 inputs and unchanged module graph. The checkpoint's newly registered canonical graph, full-package tests
and full feature are NOT_RUN / NOT_FULLY_QUALIFIED. No prior PASS transfers to them.

The newly visible native writers and NPC consumer import
`durability::character_revision_sequencer::NpcPurchaseWriteContext`, absent from the
canonical source at preparation. That foreign context remains a held integration
input, not a local substitute or a guessed fix. Other foreign visibility/re-export
and physical-transaction/XID work, BANK cause integration, installed schema,
qualified blessing catalogue and real NPC dispatch/callers retain their assigned
final shared slots. No such foreign changes are included here. The existing
`tests/interaction_workflow.rs` path-loads `interaction/mod.rs` as another crate:
its new native-module dependency closure needs explicit qualification and may expose
missing native imports. No cfg, alternate module, hidden tests or copied authority
context is introduced to suppress this boundary.

Next qualification: independently review this exact checkpoint and source/preimage
hashes; root may author it to the existing draft under D1122 while preserving the
explicit NOT_FULLY_QUALIFIED status. Resolve missing native context and foreign
transaction/schema/caller dependencies with their allocated owners at their slots.
Then qualify the actual canonical registered graph and alternate test root, run
required canonical strict/full-package checks, and native PostgreSQL cases against
the installed schema. Private overlay proof remains separately scoped. This is no
ready/merge request and no whole kill-transaction publication.

CP D1176 (6225337534) requires the final DurabilityRoot transaction port to use
only durability-tree types; premium/interaction consume that port. The private
generation8 sequencer extension names upper-layer types and requires a separate
source repair and fresh qualification. It is not installed or approved as the
final shared context by this checkpoint.

## Recovery rebuild — NOT_FULLY_QUALIFIED (2026-10-17)

CP [6230877600](https://github.com/Oteryn/Oteryn-Game/issues/2131#issuecomment-6230877600)
permits rebuilding the lost increment from public `d09323200f5f00f2283c31fe9ea0763e35459677`.
The unavailable private generation9/26-case sources are not reconstructed from
their old execution claims. This increment retains the public promotion and
blessing SQL writers, immutable replay receipts, fee audit and provenance, and
rebuilds the missing D1176 durability-only transaction participant.

`durability/npc_purchase.rs` accepts lower operation traits; Premium owns their
implementations. It binds the context to the physical PostgreSQL XID, obtains
recovery from the actual reconciled root, rechecks current recovery, and admits
fresh writes through the existing gameplay fence and locked progression/root
interpretation. The request account must equal the independently fenced root
account; the operation cannot substitute another Character. Source receipt
lookups remain before fresh gameplay/Premium guards, with no second fee on
exact replay. Promotion retains coins-only payment; the existing blessing
policy still permits Bank fallback. The lower runner commits the original
Type2 transaction once on success and rolls it back on a typed callback error;
it introduces no savepoint and exposes no pending answer before commit.

Held integration: durability module registration, actual semantic-pass and
RevisionSlot invocation, native Premium gate composition, schema installation,
fee consumers/Bank cause, qualified blessing catalogue and NPC dispatch remain
with their assigned owners/final slot after #2197. The new lower entry point is
restricted to durability callers and requires that native owning composition;
it does not claim a runnable production purchase or new current authority.

Validation: exact changed Rust paths passed official Rust 1.94 rustfmt syntax/
format and `--check`; `git diff --check` passed. These are source checks only.
Fresh compiler, strict Clippy, canonical/full-package and mandatory PostgreSQL
replay/concurrency/fencing/XID/partial-callback-rollback qualification are
**NOT_RUN**. All earlier private 25/25 and 26/26 (7/8 PG) results above are
historical evidence only and do not qualify this rebuilt source. No ready,
review trigger, merge, rebase or heavy/native execution is requested here.

## 2026-10-17 — registration and D1164 E0005 hunks: NOT_FULLY_QUALIFIED

`origin/main` (`cecbfda3`) is merged with a merge commit. Under D1160 the one
own-module line `pub(crate) mod npc_purchase;` is added to `durability/mod.rs`
(declaration-scoped `dead_code` allowance until the held caller exists). This
clears the canonical E0432/E0433/E0223 errors on `durability::npc_purchase` in
`premium/{promotion,blessing_purchase}.rs`. Under D1164 (#2131
comment6225267177) the two `let FeeBurnCause::CharmUnassign { .. }` bindings in
`tests/support/item_fee_burn_postgres_cases.rs` and
`tests/support/gold_fee_bank_postgres_cases.rs` become `let … else` refusals
that return a test error for any other cause. Nothing else in those files changes.

Qualification on the merged tree, Rust 1.95.0 strict all-target Clippy for
`oteryn-game-server` (`-D warnings`): **RED**, 12 lib errors, all in
`interaction/npc_promotion.rs`. It still imports
`character_revision_sequencer::NpcPurchaseWriteContext` (the rebuilt context now
lives in `durability::npc_purchase`) and calls `RevisionSlot::finish_promotion_gate`
/ `finish_blessing_gate`, `NpcPurchaseWriteContext::from_native_owner`,
`read_retained_promotion` and `prepare_delivered_promotion_purchase`. None of these
exist on main. They are the held semantic-pass/RevisionSlot integration in
`character_revision_sequencer.rs` (final slot after #2197), so they are not
authored here. Because the lib does not compile, the integration-test targets that
hold the E0005 sites were not compiler-checked; only rustfmt passed. Canonical
full-package, PG and runtime qualification remain **NOT_RUN**.

## 2026-10-18 — promotion NPC wired through dispatch: QUALIFIED_LOCAL (blessings fail closed)

`origin/main` (`64d5e4da`) is merged with a merge commit. Migration
`0137_character_promotion_purchases.sql` adds the promotion and blessing
fee-burn causes, the receipt tables, promotion state and the progression and
purchase-consistency guards (answers the Codex P1 on the missing fee-cause
schema). The purchase context lives in `durability::npc_purchase`. The
promotion gate runs through the native Premium coordinator. NPC talk prompts and
confirmations reach the purchase through `FreshAdmissionAuthority::npc_talk`.
Blessing offers fail closed behind the purchase seam: there is no qualified
blessing catalogue or price source, and no values are invented.

E2E: `gameplay_transport::monk_save::postgres_tests::promotion_npc_e2e::promotion_talk_through_dispatch_burns_fee_and_promotes`.
It covers the prompt, the confirm, one receipt, the 20000 gold fee burn,
promoted state, revision +1, replay without a second burn, the
already-promoted refusal and the blessing topic failing closed.

Qualification on the exact pushed tree (owner PC, Rust 1.95.0, shared PG 17.6
via `in-pg-netns.sh`):

- `cargo fmt --all --check` passed.
- Strict all-target Clippy for `oteryn-game-server` passed.
- The filtered lib tests, e2e included: 230 passed, 0 failed.
- `character_authority_postgres`: 170 passed, 0 failed. The filter covered
  death_receipt, item_fee_burn, gold_fee_bank, progression, premium,
  check_function_privileges, promotion, blessing, wheel, grant and migration.
- `character_death_receipts_postgres`: 6 passed, 0 failed.
- `promotion_policy`: 1 passed, 0 failed.
- `wp5_s3b_composition`: compiled with `--no-run`.
- The routing contract is healthy.

The standalone `item_fee_burn_postgres` target (`test = false`) does not compile
on main since #2157: `npc_travel` path-loads a fixture that needs
`crate::bestiary_postgres_harness`. That target is not owned here and was not
changed; its cases ran through `character_authority_postgres` instead.

## 2026-10-18 — source-backed blessing offers and Premium purchase reconcile

CP decision: the blessing source is draft #2237 (BLESSINGS-CONTENT-1). Its
eight files (`rulesets/character/blessings/` and
`docs/reference/content-prep/blessings-content-1/`) are taken unchanged; #2237
is superseded by this PR and closes after the merge.

- `premium/blessing_catalogue.rs` reads both evidence files at build time and
  admits only `PROVEN` or `DERIVED` facts. Blood of the Mountain (Kais) and Heart
  of the Mountain (Nomad) get the `enhanced` bands (`PROVEN`). The Wisdom of
  Solitude (Eremo) stays held: its `regular` family is `CONFLICT`.
  `promotion_price_effect` is `UNKNOWN`, so the writer refuses any price to a
  promoted buyer (`PriceUnavailable`). The storage key is
  `oteryn:blessing.<service with '-'>`. The offer still needs the NPC's own
  Service declaration, which is no content NPC's yet. The node does not install
  the catalogue: that is the CP content step (Service declarations and pins).
- The writer prices the band at the locked level: `gold + slope * (L - base)`,
  checked. The receipt binding covers both the slope and the promoted
  admission.
- Codex P1 (npc_command.rs:854): a resumed connection's first command first
  re-keys the Character's promotion and blessing receipts with its CommandRef
  (`reconcile_premium_purchase`). A match answers with the purchase line; an
  unknown read holds the command; anything else falls through to the Bank
  reconcile.
- The DEATH_COMPLETE0 §4.7 hook: `commit_death` sends the held blessing set,
  either the set of the already-committed death or the current set read under
  the revision slot. The loss table of the source matches
  `domain::death::death_experience_loss`. #2193 (DEATH-ML-LOSS-1) merged as
  3aa52fd3e but still sent an empty held set. This PR supplies the set to
  `settle_runtime_player_death`: the settlement counts it as regular blessings
  in the experience loss, keeps the #2193 build-loss formula and consumes the
  held blessings.

E2E: `gameplay_transport::monk_save::postgres_tests::promotion_npc_e2e::blessing_talk_through_dispatch_burns_fee_and_blesses`.
It covers the prompt, the 7800 gold burn at level 50, the held key, the receipt
band 31..119, replay, reconcile, the death held set, the already-held refusal,
the unoffered Heart and Wisdom, and the promoted fail-closed price. The
promotion e2e also covers the reconcile.

Qualification is recorded in PR #2194.

## 2026-10-18 — owner pricing ruling and production install

Owner ruling 2026-10-18 (`PROVEN_BY_OWNER`, recorded in
`rulesets/character/blessings/reference-evidence.json` and
`docs/reference/content-prep/blessings-content-1/sources.json`):

- Regular blessings: level <= 30: 2000; 30 < L < 120: 2000 + 200 (L - 30);
  L >= 120: 20000 + 75 (L - 120). Wisdom of Solitude is regular.
- Blood and Heart of the Mountain: <= 30: 2600; 30 < L < 120: 2600 + 260 (L - 30);
  L >= 120: 26000 + 100 (L - 120).
- Twist of Fate (data only): <= 30: 2000; 30 < L < 270: 2000 + 200 (L - 30);
  L >= 270: 50000.
- Promotion has no price effect: the promoted fail-closed, its receipt binding
  bit and its DB check are removed. The source refuses to load unless
  `promotion_price_effect` is admitted as `none`.
- Store Tibia Coins (data only): all seven 130; Twist of Fate 8; Blood and
  Heart 25 each; Spark 20; the other four regular 15 each.
- Cross-references: tibiamonk blessings calculator (owner-provided, fetched
  2026-10-18, client-side); TibiaWiki revision 1630:1113782 (identical
  formulas); official premiumfeatures page (snapshot sha256
  `addac5f6…8bb`; lists no blessings). Differences, reported not acted on:
  the official manual has flat regular prices above 120; premiumfeatures
  attributes the 30% death reduction to Premium; it lists promotion as
  Premium-only, while the accepted WHEEL-0 §6.2 admits it pre-delivery.

`admitted()` accepts `PROVEN_BY_OWNER` only with an owner ruling date. The
storage key stays `oteryn:blessing.<service>`.

Production install (CP authorization): `node/serve.rs` builds
`SourcePremiumNpcCatalogue::embedded` over the boot NPC catalogue and installs
it with the talk book (`NpcTalkOwners`, `install_npcs`). The catalogue offers
promotion for a declared `oteryn:service.promotion.<npc stem>` Service and a
blessing for a declared source service at a source NPC. The content NPC
records declare none of these services yet; their declarations and pins are
the slot-only content step, so production answers no Premium offer until then.

The e2e tests install the same production catalogue over a declared NPC
catalogue: `promotion_talk_through_dispatch_burns_fee_and_promotes` and
`blessing_talk_through_dispatch_burns_fee_and_blesses` (promote, then Blood
of the Mountain at level 50 for 7800 gold, band 31..119, replay, reconcile,
death held set, refusal, Heart and Wisdom not offered). Boundary tests at
30/31, 119/120 and 269/270 are in `premium::blessing_catalogue::tests`.

Bank-paid blessing: a Character with no equipped backpack gets change facts
that name no definition, so the fee writer reads no coin input and only the
bank part pays (BANK-FEE-0 A3, DEATH_COMPLETE0 A4.7). A backpack equipped
before the fee transaction is refused, because the empty definition never
matches it. The e2e `blessing_without_backpack_is_paid_from_the_bank` covers
the bank debit, the zero burn, the balance, the held blessing and idempotent
reconcile.

Deferred review findings (tracked, outside this allocation):
- #2256: done in this PR (CP decision 2026-10-19); see the section below.
- #2257: CI runs no lib-registered PostgreSQL e2e (including this PR's
  `promotion_npc_postgres_tests`). The fix is in `.github/workflows`. Locally,
  all 3 `promotion_npc` e2e tests pass against PostgreSQL 17.6.

## 2026-10-19 — #2256: gameplay readers of the durable promotion

CP decided that #2256 lands in this PR: without a reader, a player who buys the
promotion pays the fee and gets nothing. Every reader takes the
`game_character_promotions` row through its owner
(`durability::character_promotion::read_durable_promotion`), on the
transaction that holds its own current fence:

- Death (D76): `CharacterDeathRequest::premium` carries the attempt's Premium
  evaluation (`premium_promotion::death_premium`: `Current`, `PreDelivery` or
  `NotCurrent`). The death transaction reads the durable promotion after its
  fence and held-blessings checks and binds `promoted_with_current_premium`
  from it. Before delivery it first takes the latch's shared advisory lock
  (`latched_under_shared_lock`) as its first statement; a latch that has
  committed meanwhile refuses the attempt with nothing written, and the next
  cadence retries it under the delivered status. A replay returns the first
  receipt whatever the new evaluation.
- Login/ADMIT (A4.2, D73): `admit_quest_session` calls `admit_promotion`, which
  reads the promotion with `DurablePromotionRead` under the session's current
  fence in the Character's revision slot and applies
  `effective_promotion_at_login` with the account's Premium gate. The choice is
  kept per session in `PremiumSessions` until the session's Premium release;
  `shown_vocation` gives the promoted vocation (and title) key.
- Wheel: `WheelChangeFacts` no longer carries `promoted`;
  `commit_character_wheel` reads the durable promotion.
- Bestiary Charm: `BestiaryCharmFacts::promoted` reads the durable promotion.

Tests (PostgreSQL 17.6, locally):
- `character_authority_postgres` (CI-registered, `pr-postgres-harness`):
  the death writer's five promotion and Premium combinations and their replay,
  the pre-delivery death refused after the latch, the fenced login read, the
  Wheel change refused before the promotion, and the Charm echoes after it.
- Lib e2e
  `promotion_bought_at_the_npc_shows_at_login_and_prices_the_death`
  (`promotion_npc_postgres_tests`): login shows the base vocation, the
  purchase goes through command-7 dispatch, the next admission shows
  `master_sorcerer`, and a real lethal hit commits the promoted loss (7546,
  not 10780, at level 20). Like the other lib PostgreSQL e2e, CI does not run
  it until #2257 (and #2164) give the lib tests an admin URL.

Open (owner question): no client wire field carries the shown vocation or
title yet, so `shown_vocation` has no wire caller.
