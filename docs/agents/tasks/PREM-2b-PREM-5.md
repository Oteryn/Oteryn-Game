# PREM-2b+PREM-5 — promotion state and promotion/Premium blessing NPC services (consolidated task card)

```yaml
task_id: PREM-2b+PREM-5
title: "PREM-2b+PREM-5: promotion state and promotion/Premium blessing NPC services"
mode: IMPLEMENT
status: in_progress
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
branch: w4/prem-2b
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

## Module layout and prerequisite audit (2026-10-22)

Allocation baseline: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`.
Local authoring only; no publication or external review trigger in this worker.

- L1: `content/blessing_catalogue.rs` interprets admitted blessing evidence and prices;
  `npc_catalogue/premium_services.rs` interprets typed NPC Service declarations.
- L3: `progression/promotion.rs` owns eligibility and the login projection.
- L4: `durability/npc_purchase.rs` owns the transaction participant; separate
  `promotion_purchase.rs`, `blessing_purchase.rs`, `character_promotion.rs` and
  `character_death/promotion.rs` own their durable mechanics. Migration 0138
  registers causes, purchase receipts and promotion provenance.
- L4 NPC: `npc_promotion.rs` owns dialogue policy; L5 child modules own command
  composition, Premium purchase gating and login admission. Existing shared files
  receive registrations and calls only. New mechanic suites are mounted once.

Prerequisite audit: the fee BURN, Type2 transaction, revision slot, Premium latch,
NPC dialogue, Bank fallback and held-blessing death owners exist in production.
The blessing content packet is staged under `docs/reference/content-prep/`; its
runtime catalogue and typed NPC promotion/blessing service consumer are absent.
Promotion readers are absent and death currently supplies an empty held set.
The NPC crate has no durability dependency, so durable operations stay in durability
and dispatch composes them with NPC policy, preserving ADR-0022 dependency direction.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

Implemented prerequisite modules: `content/blessing_catalogue.rs`,
`content/npc_catalogue/premium_services.rs`, `npc/npc_promotion.rs` and the durable
purchase participant. They extend the existing content, dialogue, fee and fence owners.
Additional prerequisite repairs reuse existing owners: `npc_purchase/recovery.rs`
registers paid purchases in Character recovery; the common receipt view extends the
existing SQL consistency guard without copying it. The canonical PostgreSQL target
now reuses native durability modules after ADR-0022. The dispatch e2e runs in CI
through CI-PG-LIB-1 (#2300), which runs the whole dispatch library suite against
PostgreSQL in the PR and Merge Queue `rest` legs; no workflow or pin changes.
No prior qualification command is removed.

The `shown_vocation` wire field is omitted: the owner question remains open and
there is no accepted wire consumer. Login retains only the internal display snapshot.

## Dependencies (allocation baseline 9f814e72; local changes 2026-10-22)

| Dependency | State |
|---|---|
| PREM-WIRE-1 (PREMIUM-1) | MERGED (#2110) |
| NPC-TALK-1 | MERGED (#1976) |
| PREMIUM_ACTIVATION acceptance | MERGED (#2089) |
| Promotion source (ACCEPT_BATCH §3) | ACCEPTED (#2209) |
| WHEEL-0 §6.2 pre-delivery exception / PROG-ADV-1 | MERGED (#2161) |
| NPC-ECON-1 (Bank cause arm, npc_talk dispatch slot) | MERGED (#2197) |
| DEATH-ML-LOSS-1 (death build loss) | MERGED (#2193); this lane supplies held blessings and current promotion |
| BLESSINGS-CONTENT-1 (#2237) | IMPLEMENTED locally here: admitted catalogue, source evidence, Service declarations and pins |
| #2257 CI admin URL for lib-registered PG e2e (fix routed to #2200) | Local dispatch e2e wired to existing PR/MQ PostgreSQL jobs in this lane |
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

Mechanic files implemented (one mechanic per file, layer per ADR-0022; complete
changed-path and reuse inventory is in the local validation report):

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
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/promotion_npc.rs` (new) — e2e.
- `apps/game-server/tests/support/authority_cases/character_promotion_postgres_cases.rs` (new).
- `rulesets/character/blessings/` + `docs/reference/content-prep/blessings-content-1/`
  (from #2237, with the owner pricing ruling).
- `apps/game-server/migrations/0133_character_promotion_purchases.sql` (new) — provisional
  number assigned by the integrator; renumber only through CP integration.

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

- Accepted implementation is complete locally; status remains `in_progress` in AUTHORING,
  with no frozen SHA or PR and no builder ready/merge action.
- CP owns independent review, exact candidate freeze, CI/Merge Queue and publication.
  Migration 0138 is provisional. `shown_vocation` remains the explicitly excluded owner question.
- The full Merge Queue workflow canary needs `pwsh`, unavailable in this worker. Its
  existing native PowerShell propagation check was not skipped or changed.

## Acceptance criteria

- [x] Promotion fail closed without Premium after delivery; price burned through the fee ledger in coins only.
- [x] Promotion state, receipt, fee and audit in one transaction; replay never burns twice.
- [x] Blessing purchase prices at the locked level per the owner ruling; bank fallback works; already-held refused.
- [x] Death, login, Wheel and Charm read the durable promotion under their own fence.
- [x] Canonical PG cases (including e2e via dispatch) show real database-start markers.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests in their own child files (not appended to existing files).
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`character_death.rs`,
      `gameplay_transport/mod.rs`, `npc_command.rs`, `spell_premium_coordinator.rs`,
      `item_fee_burn*.rs`, `character_revision_sequencer.rs` are all over 1000).

## Inseparable bundles

- PREM-2b+PREM-5 is one bundle; migration + writer; gold burn + ledger for the promotion price.

## Validation

Local Rust checks and relevant unit/PostgreSQL suites PASS. All cargo commands used
`+1.95.0`, `--locked` and the queued wrapper (`login=false`); PG fixtures assert
`server_version_num=170006`. The new dispatch and canonical purchase suites emit
`POSTGRES_STARTED PREM-2b+PREM-5 ... version=17.6` after actual database creation.
The whole diff was re-read against `~/oteryn-cache/CODEX-CHECKLIST.md`; all new
variant/field/trait constructor and implementation sites were searched across
`apps/`, `crates/` and `tools/`. New production APIs have native callers and real-path coverage.

Complete exact filters, changed files, reuse and size evidence:
[`PREM-2b-PREM-5-local-validation.md`](../evidence/PREM-2b-PREM-5-local-validation.md).

- Formatting and Clippy `--all-targets -- -D warnings` PASS for all five touched gameplay crates
  plus game-server; no Cargo manifest/lock changes, so the worker used bounded `-p` checks.
- Promotion/NPC 3, blessing catalogue 2, Premium/soul 7, fee/audit 19,
  sequencer 11 and Wheel projections 8 unit tests PASS.
- PostgreSQL: new promotion/guard/recovery filter 3, dispatch e2e 1,
  Wheel 5, Wheel cast 4, death 46, fee BURN 8 and Bank fallback 8 PASS.
- Content: NPC catalogue 4, NPC service model 1 and canonical World package 5 PASS;
  Python tree generator checks and materialized tree 97/97 PASS.
- Governance, repository policy and PR qualification-placement regressions (7) PASS.
  Full MQ canary stopped at its existing missing-`pwsh` requirement; CI must finish it.

## File sizes and reuse

No existing file grows by more than 150 lines. Shared `mod.rs`/`lib.rs`/registry
changes are registration/call wiring. Old death and fixture bodies were removed
when extracted; no parallel implementation or duplicate new test registration remains.

| New file growing by >200 lines | Lines | Owner/layer |
|---|---:|---|
| `dispatch/.../npc_command/premium_purchase.rs` | 299 | L5 live NPC/fence/receipt composition |
| `dispatch/.../postgres_tests/promotion_npc.rs` | 437 | Own real dispatch/PG suite |
| `durability/.../character_death/commit.rs` | 316 | Existing L4 fenced SQL writer extracted |
| `durability/.../character_death/settlement.rs` | 312 | Existing L4 sequenced settlement extracted |
| `rulesets/character/blessings/reference-evidence.json` | 752 | Archived source evidence data |

Reused owners include native NPC talk/rate/range, Bank coin facts and fallback,
Type2 fee/audit, Character fences/recovery/sequencer, the Premium gate/latch,
canonical content materialization, domain soul/death/Wheel/Charm rules, and the
existing SQL consistency guard. Test reuse: moved GOLD-FEE builders in
`item_fee_burn/test_support.rs`; the monk-save `Dependencies::with_content` constructor
takes the real pin (FLOW port; the lane's own default-constructor file is gone); `tests/purchase_fixture.rs` exposes private native
clock/evidence; `charm_state_promotion_fixture.rs` exposes the existing private
Charm catalogue. The report lists every reuse site and all 110 changed files.

## Reference evidence

- PREMIUM_ACTIVATION0, WHEEL-0 §6.2, ACCEPT_BATCH §3, DEATH_COMPLETE0 §4.7, BANK-FEE-0 A3
- Coordination #2131 / #1622 (D1106, D1122, D1160, D1164, D1176); issues #2256, #2257; PR #2200
- Draft #2237 (BLESSINGS-CONTENT-1) research audit and sources
