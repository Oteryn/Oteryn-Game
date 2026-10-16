# PREM-2b+PREM-5

```yaml
task_id: PREM-2b+PREM-5
title: "PREM-2b+PREM-5: promotion state and promotion/Premium blessing NPC services (skeleton)"
mode: IMPLEMENT
status: waiting
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
updated_at: 2026-10-16
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/premium.rs
  - apps/game-server/src/premium/promotion.rs (new)
  - apps/game-server/src/interaction/npc_promotion.rs (new)
  - apps/game-server/tests/promotion_*.rs (new)
slot_only_paths:
  - migration number
  - mod.rs
  - content NPC service pins
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
last_progress: accepted promotion pre-delivery exception retained in native status policy
status: waiting
branch: skeleton/prem-2b-prem-5
blocker: shared durable promotion/coin transaction and runtime seams require CP slot
next_action: native RED/GREEN qualification and independent review; CP allocates durable bundle
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
