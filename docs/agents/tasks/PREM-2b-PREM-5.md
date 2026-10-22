# PREM-2b+PREM-5 — promotion state and promotion/Premium blessing NPC services (consolidated task card)

```yaml
task_id: PREM-2b+PREM-5
title: "PREM-2b+PREM-5: promotion state and promotion/Premium blessing NPC services"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
class: hard
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 PREREQ-1; App. 7 Supporting PREM-2b+PREM-5"
briefs:
  - PREMIUM_ACTIVATION0 §2–§4.4 (accepted #2089)
  - WHEEL-0 §6.2 (accepted; pre-delivery promotion exception)
  - ACCEPT_BATCH-2026-10-15 §3 (accepted #2209; promotion source)
  - DEATH_COMPLETE0 §4.7 (blessings at death; Bank fallback)
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2194, branch skeleton/prem-2b-prem-5

Status note: the draft reached QUALIFIED_LOCAL with a production NPC install, both
purchase writers, durable promotion readers (#2256) and migration `0137_character_
promotion_purchases.sql` (that number is not reserved; it must be re-allocated). It used
the pre-ADR-0022 `apps/game-server/src/...` layout. It is closing; the mechanic is
re-implemented fresh on the current layout. It also absorbed draft #2237
(BLESSINGS-CONTENT-1, 8 content/evidence files), which closes with it.

## Goal

(PREM-2b) promotion state, effective promotion at login, soul maxima, death input, Wheel
promotion read; (PREM-5) promotion and Premium blessing NPC services. One '+' allocation.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| PREM-WIRE-1 (PREMIUM-1) | MERGED (#2110) |
| NPC-TALK-1 | MERGED (#1976) |
| PREMIUM_ACTIVATION acceptance | MERGED (#2089) |
| Promotion source (ACCEPT_BATCH §3) | ACCEPTED (#2209) |
| WHEEL-0 §6.2 pre-delivery exception / PROG-ADV-1 | MERGED (#2161) |
| NPC-ECON-1 (Bank cause arm, npc_talk dispatch slot) | MERGED (#2197) |
| DEATH-ML-LOSS-1 (death build loss) | MERGED (#2193) — still sends an empty held blessing set |
| BLESSINGS-CONTENT-1 (#2237) | BLOCKING — re-implementation pending (draft #2237 closing; content absorbed here) |
| #2257 CI admin URL for lib-registered PG e2e (fix routed to #2200) | not on main — check at allocation |
| Client wire field for shown vocation/title | OPEN owner question (no caller for `shown_vocation`) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to extend (wiring-sized):

- `crates/gameplay/domain/src/domain/premium.rs` — one Premium gate,
  `promotion_benefit_current`, Premium soul maximum and clamp (keep unchanged in substance).
- `crates/gameplay/durability/src/durability/item_fee_burn.rs` / `item_fee_burn_audit.rs`
  — `FeeBurnCause` / `FeeBurnCauseV1` are `CharmUnassign`-only today.
- `crates/gameplay/durability/src/durability/premium_activation_latch.rs` — latch; shared
  advisory lock read for the death writer.
- `crates/gameplay/durability/src/durability/character_revision_sequencer.rs` — revision slot.
- `crates/gameplay/durability/src/durability/character_death.rs` (2,764 lines) — promotion/
  Premium currently bound false; D76 inputs.
- `crates/gameplay/durability/src/durability/character_wheel.rs` (`WheelChangeFacts`
  takes a caller `promoted`) and `charm_state.rs` (`BestiaryCharmFacts::promoted` = false).
- `crates/gameplay/dispatch/src/gameplay_transport/spell_premium_coordinator.rs`,
  `npc_command.rs`, `npc_talk.rs`, `mod.rs` (8,167 lines: wiring only).
- `crates/gameplay/npc/src/npc_talk.rs`, `npc_command.rs`, `bank_npc.rs` — NPC talk owners.
- `crates/gameplay/social/src/premium/` — Premium runtime (client/snapshot/refresh).
- `apps/game-server/src/node/serve.rs` — production NPC service install.
- `rulesets/character/blessings/index.json` — blessing ruleset (only index on main).
- `docs/contracts/game-events/v1/native_one_item_transaction.proto` — `OneItemFeeBurnCauseV1`.

Proposed new files (one mechanic per file, layer per ADR-0022; confirm crate dependency
direction at allocation):

- `crates/gameplay/progression/src/promotion.rs` (new) + `promotion_tests.rs` (new) — L3
  eligibility (level ≥20, not promoted), 20,000-gold quote, login display projection.
- `crates/gameplay/content/src/content/blessing_catalogue.rs` (new) + tests (new) — L1
  source reader admitting `PROVEN`, `DERIVED`, `PROVEN_BY_OWNER` (with ruling date) only.
- `crates/gameplay/durability/src/durability/npc_purchase.rs` (new) — L4 durability-only
  transaction participant (D1176: lower operation traits, no upper-layer types).
- `crates/gameplay/durability/src/durability/character_promotion.rs` (new) —
  `read_durable_promotion`.
- `crates/gameplay/durability/src/durability/promotion_purchase.rs` (new),
  `blessing_purchase.rs` (new) — L4 SQL writers (receipts, fee, provenance).
- `crates/gameplay/npc/src/npc_promotion.rs` (new) — L4 NPC consumer for both services.
- `crates/gameplay/dispatch/src/gameplay_transport/promotion_npc_postgres_tests.rs` (new) — e2e.
- `apps/game-server/tests/support/character_promotion_postgres_cases.rs` (new).
- `rulesets/character/blessings/` + `docs/reference/content-prep/blessings-content-1/`
  (from #2237, with the owner pricing ruling).
- `apps/game-server/migrations/<next>_character_promotion_purchases.sql` (new) — migration
  number: next free number at allocation time (only one migration PR in the queue).

## Accepted design (as built and reviewed in the draft)

Promotion (ACCEPT_BATCH §3, PREMIUM_ACTIVATION §4.2, WHEEL-0 §6.2):

- PREM-5 NPC promotion: 20,000 gp, level ≥ 20, paid in coins only (GOLD-FEE-1b; never the
  GOLD-FEE-2 bank fallback). Bought once; durable promotion survives Premium lapse.
- Benefits checked at use; displayed vocation/title change at login only. A login display
  snapshot never feeds per-use gameplay benefits.
- Pre-delivery exception: promotion policy borrows native `PremiumStatus` inside the existing
  gate closure and selects the exception only with `&PreDelivery` (witness borrowed from the
  gate; no new witness, cached approval or entitlement class). After delivery: unknown/
  unavailable/NotCurrent refuse; Current permits. The general gate and the three blessing
  gates are unchanged.
- `character_build.rs::BuildCause::Promotion` is not the promotion state/fee source.

Transaction (D1106, D1176):

- One Type2 transaction stages coins-only fee, Character receipt and permanent flag +
  provenance. Historical replay resolves first and grants no authority; fresh purchase checks
  current state. No SAVEPOINT: the native fee guard proves root xmin equals the top-level XID
  (draft generation-2 finding). Typed callback Err rolls back the same transaction.
- The promotion-only gate opens the Type2 transaction before taking the Premium latch.
- Consumer holds the Character revision slot through replay, actor/NPC/conversation/range
  checks, one Premium gate operation and COMMIT; then adopts the receipt. Stable CommandRef/
  target-derived identities survive retry. A session fence that moved while the command
  waited (new generation, lease or scope) refuses.
- Participant binds to the physical PG XID, obtains recovery from the reconciled root, and
  requires request account == fenced root account.
- Fee cause kinds 2 (`promotion_purchase`) and 3 (`blessing_purchase`) and matching
  `OneItemFeeBurnCauseV1` oneof tags 2/3 are owner-approved (2026-10-18). Existing Charm
  request bytes preserved. Existing test `let FeeBurnCause::CharmUnassign {..}` bindings in
  `item_fee_burn_postgres_cases.rs` / `gold_fee_bank_postgres_cases.rs` become `let … else` (D1164).

Blessings (PREMIUM_ACTIVATION §4.4, DEATH_COMPLETE0 §4.7):

- Current-only Premium (no PreDelivery exception). Server-resolved NPC/service/key/price;
  the command accepts no client fee or key. Price chosen at the locked Character level.
- Payment coins-then-BANK fallback (V2/nonjunior rule). No backpack => bank pays fully;
  a backpack equipped before the fee transaction is refused.
- Owner pricing ruling 2026-10-18 (`PROVEN_BY_OWNER`):
  - Regular (incl. Wisdom of Solitude): L ≤ 30: 2000; 30 < L < 120: 2000 + 200(L−30);
    L ≥ 120: 20000 + 75(L−120).
  - Blood / Heart of the Mountain: ≤30: 2600; 30<L<120: 2600 + 260(L−30); ≥120: 26000 + 100(L−120).
  - Twist of Fate (data only): ≤30: 2000; 30<L<270: 2000 + 200(L−30); ≥270: 50000.
  - Promotion has no price effect (`promotion_price_effect` must be `none`).
  - Store Tibia Coins (data only): all seven 130; Twist 8; Blood/Heart 25; Spark 20; other four 15.
- Storage key `oteryn:blessing.<service>`; NPC offers need the NPC's own Service
  declaration (`oteryn:service.promotion.<npc stem>` for promotion) — content step.
- Death: `commit_death` sends the held blessing set (committed death's or current under the
  revision slot); settlement counts them as regular blessings in experience loss and keeps
  the #2193 build-loss formula.

Readers (#2256): death binds `promoted_with_current_premium` from the durable row after its
fence (pre-delivery: shared latch lock first; a newly committed latch refuses, retry later);
Premium evaluated only once the revision slot is held. Login/ADMIT `admit_promotion` under
the session fence, kept in `PremiumSessions`. Wheel and Bestiary Charm read the durable row.

## Review dispositions (from the draft)

- Codex P1 missing fee-cause schema → migration carries causes, receipts, state, guards.
- Codex P1 (npc_command resume): first command after resume re-keys promotion/blessing
  receipts by CommandRef (`reconcile_premium_purchase`) before the Bank reconcile.
- Codex P2 death Premium evaluated before slot → evaluate inside the slot.
- Codex P2 NPC purchase fence → keep the command's starting fence; moved fence refuses.
- Reported, not acted on: official manual flat regular prices above 120; premiumfeatures
  lists promotion as Premium-only (WHEEL-0 §6.2 admits it pre-delivery).
- Standalone `item_fee_burn_postgres` target (`test = false`) did not compile after #2157;
  cases ran via `character_authority_postgres`.

## Remaining work

- Re-implement all of the above on the current layout; NPC Service declarations and pins
  (content slot) so production offers appear; wire field for shown vocation/title.

## Acceptance criteria

- [ ] Promotion fail closed without Premium after delivery; price burned through the fee ledger in coins only.
- [ ] Promotion state, receipt, fee and audit in one transaction; replay never burns twice.
- [ ] Blessing purchase prices at the locked level per the owner ruling; bank fallback works; already-held refused.
- [ ] Death, login, Wheel and Charm read the durable promotion under their own fence.
- [ ] Canonical PG cases (including e2e via dispatch) show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`character_death.rs`,
      `gameplay_transport/mod.rs`, `npc_command.rs`, `spell_premium_coordinator.rs`,
      `item_fee_burn*.rs`, `character_revision_sequencer.rs` are all over 1000).

## Inseparable bundles

- PREM-2b+PREM-5 is one bundle; migration + writer; gold burn + ledger for the promotion price.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-content promotion blessing
cargo test --locked -p oteryn-game-server --test character_authority_postgres -- promotion blessing death wheel --test-threads=1  # PG 17.6
python3 tools/agents/validate_governance.py
```

## Reference evidence

- PREMIUM_ACTIVATION0, WHEEL-0 §6.2, ACCEPT_BATCH §3, DEATH_COMPLETE0 §4.7, BANK-FEE-0 A3
- Coordination #2131 / #1622 (D1106, D1122, D1160, D1164, D1176); issues #2256, #2257; PR #2200
- Draft #2237 (BLESSINGS-CONTENT-1) research audit and sources
