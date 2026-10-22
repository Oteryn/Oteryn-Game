# PVP-DEATH-1 — PvP death outcomes in the victim's death transaction (consolidated task card)

```yaml
task_id: PVP-DEATH-1
title: "PVP-DEATH-1: PvP death classification, lethal attribution ledger and outcomes in the death transaction"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Death module D; App. 7 PARTY/PVP PVP-DEATH-1"
brief: "docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md (§6–§10); docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md (§4.1–§4.8)"
class: "hard, persistence, combat and security review"
base_branch: main
branch: null
pr: null
needs_migration: false   # D1105: migration 0030 capacity suffices; re-confirm against the storage blockers below
needs_registry: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2205, branch skeleton/pvp-death-1

## Goal

PvP death outcomes (classification, frags, skull consequences, blessing/item/XP effects,
black-skull respawn vitals) computed from a death-time lethal attribution snapshot and
persisted in the victim's single death transaction with its one death receipt, with
restart reconciliation. No second death writer, no second Character receipt, no contributor
revision. Live PvP stays off until the PVP-0 §8 go-live gate (PVP-RT-1b, PVP-WIRE-1,
PVP-CLIENT-1) is met.

## Children (as the draft split them)

1. **Lethal attribution ledger** (D1244/D1244a/D1253/D1255/D1258/D1258a): the native
   per-victim player-damage ledger and its consumers in the real HP commits.
2. **Outcome leaf** (D1105): pure typed death-outcome planning (classification + outcome
   table) and its 15 tests.
3. **Shared death integration** (slot-only): the PvP hunks in `character_death.rs`
   (binding, receipt, decoder, NEW-write branch, replay), together with PVP-1b's
   `apply_pvp_death_in_transaction`.
4. **Respawn vitals** from the receipt at live and admission placement.

## Accepted decisions

- **D1105 (CP, PR #2131 comment 6219643346):** authorizes the outcome leaf first.
  DEATH-ML-LOSS-1 owned `character_death.rs` until it merged (now MERGED, #2193); the
  shared receipt/binding/load/decode/reconcile hunks follow that merge. D1105 states the
  migration 0030 columns suffice and no migration is authorized. Earlier proposals for new
  receipt columns or a migration lease are superseded planning assumptions — but see the
  storage blockers; if any must be resolved by schema, CP re-decides.
- **D1122:** the leaf checkpoint is qualification-only; a private `mod death;` registration
  overlay is not product registration.
- **D1128 / D1244 / D1244a / D1253 / D1255 / D1258 / D1258a:** allocate the native lethal
  ledger, its real commit consumers (source attack, composite, bite, conditions), the
  first-instance condition carrier and the same-commit lethal-tick receipt, plus mechanical
  type/Clone forwarders and test-root mounts. Explicit `Unattributed` when the historical
  carrier is missing.
- **D1160:** only a `mod death;` mounting hunk in the PvP `mod.rs`.
- **D1022:** the earlier DEATH-3 item-loss waiver (DEATH-WIRE-1) is lifted for this module;
  do not revive it.
- **D1443 (from PVP-1b):** frag windows are exclusive-end on `committed_at`.
- Authority: PARTY-PVP0 §6–§10 (through ACCEPT-SOCIAL-MAP0); DEATH-COMPLETE0 §4.1–§4.8
  (accepted by ACCEPT-BATCH-2026-10-15); DEATH0 §3; A13 §4.6. The PVP0 gap-closure
  candidate (`docs/architecture/reviews/OTERYN_GAME_PVP0_PVP_GAP_CLOSURE_CONTRACT_CANDIDATE_2026-10-10.md`)
  is still CANDIDATE: its 15-minute white extension, expert reset and expanded activation
  rules are not inherited; only the accepted 15-minute kill block is.
- First World is Optional (no wars from this task); Hardcore branches qualify
  independently and no World is configured Hardcore.

## Reference behaviour from the draft (re-implement, do not copy)

### Lethal attribution ledger

- One runtime ledger per victim, at most 64 sources, five-minute pruning, oldest-`last_at_us`
  eviction. Each entry keeps `{hp_total, last_at_us}` plus oldest-first one-second
  contribution buckets (at most 301 per source); every bucket expires on its own latest
  hit, so `hp_total` sums only in-window hits (Codex P1 "track the window per hit").
- `LethalAttribution` carries the killer, `killer_hp` (the killer's retained five-minute
  total including the final blow; Codex P1) and the HP-ordered other sources, so
  `killer_hp` + assists is the all-source denominator.
- Refusals leave the ledger unchanged; a failed lethal mint does not publish staged pruning;
  clock regression leaves HP/ledger/replay unchanged; a removed source may replay its retained
  result but cannot deal fresh damage.
- Source Character or actual summon master comes from admitted source lineage at hit time,
  never from a client payload. The final blow is always represented even after eviction.
- Current main has a 252-line `player_lethal.rs` whose `PlayerLethalReceipt` holds victim
  facts only; the draft ledger is not on main.

### Outcome leaf

- `pvp_death` = at least 5 % PvP of all damage in the window (checked widened-integer
  comparison, no float, no zero-total default) **or** a player/summon final blow. A monster
  final blow with 5 % PvP is still a PvP death; a justified kill is still a PvP death; an
  assist-only record invents no damage. Empty or inconsistent totals refuse.
- Outcome table:

| Victim state at death | Result |
|---|---|
| Effective red/black skull (PvP or PvE) | All equipment/backpack lost, regular blessings consumed; Amulet/Twist do not protect and Twist is not consumed; no AoL, no draw. D65 no-loss through level 8 and vocation exemptions apply first. |
| Other PvP death with regular blessings + Twist | Regular blessings kept, Twist consumed; otherwise ordinary D62/AoL selection. With no regular blessings, Twist stays. Twist is not an 8th regular blessing. |
| Open PvP, level ≤ 20, not forfeited | Adventurer: no XP/item/blessing loss, evaluated before AoL/draw (no amulet consumed). A conflict with red/black facts refuses; the caller reconciles the real skull/forfeiture preimage. |
| Other ordinary outcome | Existing D58/D59/D68 XP and D62 item rules; promotion/Premium needs a real provider (the source hardcodes false). |
| Black skull at death, respawn | 40 HP / 0 mana from the committed receipt; later skull expiry does not change it. |

- The leaf returns semantic receipt data with `unfair_fight_milli = 0` (PARITY_PENDING;
  Death Redemption and unfair-fight formulas are out of scope), not a persisted receipt,
  authority, mutation permit or respawn grant.
- Draft tests (15) covered the classification threshold, each outcome row and the
  Adventurer/skull conflict.

### Classification contract (consumes PVP-1b)

- PVP-RT supplies all-source and PvP totals over the five-minute window, 10 s buckets
  (max 31), 16 full + 64 compact contributors, damage/assist type and `last_at`. All-source
  includes monster, field and condition damage; summon damage belongs to its owner;
  friendly fire never counts. Ledger caps never grant immunity: at most 80 contributors plus
  the final blow are processed.
- Each contributor is classified at death from the victim's marks toward that contributor,
  war relation or Hardcore (no global `marked` cache, no current skull read after an await).
- Open unjustified damage contributor: 1000 milli; with n > 5 damage contributors,
  floor(5000/n); assist-only 500. Keyed by (contributor, PlayerDeathOccurrence); never
  updated or refunded. Justified/Hardcore: no points, but every direct/assisting contributor
  gets kill block DB+15 min on every World type (never lowers a longer deadline).
- Skull evaluation reuses PVP-1b's evaluator with history **excluding** the new increment.
- An unjustified death creates a 7-day revenge mark; an avenging death closes the oldest
  unexpired, unavenged reverse mark in its own death transaction. No XP/money/item/point
  refund.
- First aggression forfeits Adventurer before harm under the acting Character's fence
  (PARTY §10.4, PVP-RT owner); DEATH-COMPLETE §4.6 also names same-transaction forfeiture.
  Do not defer forfeiture to death.

### Integration sequence (owning victim transaction)

1. Lethal turn (RT): commit vitals/occurrence once, capture the admitted death-time
   attribution snapshot; no harm on a dead actor; do not hold the runtime lock across the
   durable transaction.
2. Earlier training flush (build owner) through `commit_character_build` in its own earlier
   transaction; an unknown flush reconciles before the death binding. Death build loss rides
   in the death receipt's nullable all-or-none fields (A13), not a second build receipt.
3. Victim sequencing: keep `settle_player_death` → victim `RevisionSlot` →
   `commit_character_death`; receipt-first replay with the original binding; no
   reclassification on replay.
4. NEW-write: victim root (gameplay fence), then victim + contributor PvP state rows in
   CharacterId order, then point/mark rows; no contributor progression locks; offline
   contributors still receive consequences.
5. One Character commit: root/progression/build once, receipt with before/after loss and
   PvP/item/blessing evidence, delete only blessing keys absent from the after-set, then in
   the same SQL transaction points/skulls/kill blocks/marks while ledger rows still exist,
   then delete the victim ledger, then the pending respawn.
6. Ambiguous COMMIT: cursor invalidated, victim stays dead/retry, reconcile the retained
   receipt; never recompute after a ledger reset or consume a second blessing.
7. Post-commit: publish committed per-contributor skull/blocks/marks to current Channels;
   C's PlayerDeathDrop/AoL burns are separate idempotent DUR-03 transactions in receipt order.
8. Placement: resume unfinished item causes, place under the current fence, consume the
   pending row and write last position, then receipt-selected vitals in one owner revision,
   on both live respawn and admission.

## Storage blockers (resolve with DEATH-3 / PVP-1b before integration)

1. `apps/game-server/migrations/0016_character_death_receipts.sql` enforces a NULL amulet and
   an empty lost-item set; no native PvP outcome columns exist. DEATH-COMPLETE §4.2 gives the
   receipt change to batch C (DEATH-3), not to this task.
2. `apps/game-server/migrations/0081_pvp_state.sql` `game_character_pvp_receipt_guard` requires
   the receipt's `created_xact_id = pg_current_xact_id()`: points/marks go after the receipt
   INSERT and before the ledger DELETE in the same transaction. A later transaction is
   rejected by design; do not bypass it.
3. The same guard checks contributor `last_at >= clock_timestamp() - 5 min` at settlement,
   while classification uses the window at death: delayed settlement or cleanup can drop a
   valid contributor. A death-time snapshot/guard integration is required; never forge
   `last_at`, derive death time from the UUID or trust a caller flag.
4. The guard rejects every revenge-mark UPDATE; avenging needs a narrow guarded UPDATE with
   same-transaction avenging-death provenance and immutable keys.
5. 0081 row checks are not a complete consequence binding; any guard/provenance extension is
   routed with DEATH-3/PVP-1b and, if needed, a migration (next free number at allocation time
   (only one migration PR in the queue)). Never restore 0016's two-kind chain; the latest full
   progression guard is `0073_character_progression_guard_wheel_union.sql`.

## Dependencies (re-verified against main)

| Dependency | State |
|---|---|
| DEATH-1 (death writer) | MERGED (#1278) |
| PVP-1a (state tables, migration 0081) | MERGED (#2032) |
| PVP-RT-1 pure legality/lock; fenced lock restore | MERGED (#2029, #2204) |
| DEATH-ML-LOSS-1 (sole writer of `character_death.rs` until merge) | MERGED (#2193) |
| DEATH-3 (death module C: receipt item/PvP fields, item causes) | BLOCKING — open, no PR merged (DEATH-WIRE-1 #2028 only recorded the waiver) |
| PVP-1b (skull evaluator, `apply_pvp_death_in_transaction`, cleanup job) | BLOCKING — re-implementation pending (draft #2203 closing) |
| PARTY-1 (party/friendly-fire membership reads) | PARTIAL: bounded checkpoint MERGED (#2201) |
| PVP-RT-1b (live killer classification, real harm) | not started; required for go-live, not for the leaf |

The leaf and the ledger may proceed now; the shared death integration waits for DEATH-3 and
PVP-1b.

## Review dispositions carried forward

- Codex P1 "Track the five-minute window per hit": fixed by per-source one-second buckets.
- Codex P1 "Retain the killer's damage total in the lethal snapshot": fixed by `killer_hp`.
- Draft CI saw strict Clippy failures from the Foundation owner-time type at damage/ledger
  boundaries, a missing condition carrier in `OwnerCombatChange::PlayerConditions` and a
  `None` carrier in the condition-ordering fixture; and unit failures where native-source
  fixtures used the synthetic movement context instead of the pinned position context.
  The re-implementation must use pinned admission helpers in fixtures from the start.
- An unrelated `monk_save` `RootUnavailable` library test failure was seen once and never
  classified; recheck, do not assume independence.

## Target layout (ADR-0022)

| Concern | Path |
|---|---|
| Pure classification + outcome table | `crates/gameplay/combat-rules/src/pvp/death.rs` (new); tests `crates/gameplay/combat-rules/src/pvp/death_tests.rs` (new); registration in `crates/gameplay/combat-rules/src/pvp/mod.rs` (wiring-only) |
| Lethal attribution ledger | `crates/gameplay/combat/src/ability/player_lethal.rs` (exists, 252 lines) or a child `crates/gameplay/combat/src/ability/player_lethal_ledger.rs` (new) with its own tests file (new) |
| Ledger consumers in real HP commits | `crates/gameplay/combat/src/ability/creature_bite.rs`, `crates/gameplay/combat/src/creature_auto_attack.rs`, `crates/gameplay/spell/src/creature_damage_spell/composed.rs`, `crates/gameplay/spell/src/creature_damage_spell/mod.rs`, `crates/gameplay/monster-ai/src/ai_monster_melee.rs`, `crates/gameplay/monster-ai/src/monster_combat_lane.rs`, `crates/gameplay/encounters/src/bosses/smelly_cheese/smelly_damage.rs`, `crates/gameplay/spell/src/spell/{cast,companion_execution,periodic_execution,source_player_conditions,actor_conditions}.rs`, `crates/gameplay/foundation/src/foundation/{condition,condition_spell}.rs`, `crates/gameplay/dispatch/src/gameplay_transport/{actor_spell,ordinary_combat,actor_conditions,condition_snapshots}.rs` (call sites/carriers only) |
| Runtime PvP composition | `crates/gameplay/combat/src/combat/pvp/mod.rs` (wiring-only) |
| Death integration | `crates/gameplay/durability/src/durability/character_death.rs` (2,764 lines: call sites only) calling PVP-1b's `crates/gameplay/durability/src/durability/pvp_death_consequences.rs` (new, PVP-1b) and `crates/gameplay/durability/src/durability/pvp_state.rs` (exists) |
| Death request / respawn | `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` (8,167 lines: call sites only; `settle_session_death`, `respawn_in_order`); respawn vitals in `crates/gameplay/spell/src/spell/cast.rs` (`PlayerSpellState::respawned`, call site) |
| Native calculators reused | `crates/gameplay/domain/src/domain/death.rs`, `crates/gameplay/domain/src/domain/progression.rs`, `crates/gameplay/durability/src/durability/character_build.rs` |
| PG cases | `apps/game-server/tests/support/character_death_writer_postgres_cases.rs`, `apps/game-server/tests/support/pvp_state_postgres_cases.rs` (exist) or a new child case file `apps/game-server/tests/support/pvp_death_postgres_cases.rs` (new), mounted from `character_authority_postgres.rs` (the writer cases mount via `character_progression_postgres_cases.rs`) |

Layer note: `combat-rules` is L3 and must not depend on `entities`, durability, sqlx or
tokio; the ledger lives in the L4 `combat` runtime.

## Acceptance criteria

- [ ] PvP death and its one receipt commit in one victim transaction; reconcile after restart.
- [ ] Exact 5 % / final-blow classification with checked integers; outcome table above covered.
- [ ] Lethal ledger: 64-source cap, per-hit five-minute expiry, `killer_hp`, refusals leave it unchanged.
- [ ] Points/marks/kill blocks/skulls written in the same transaction under the 0081 guards; no later writer.
- [ ] Black skull at death respawns at 40 HP / 0 mana from the receipt on live and admission paths.
- [ ] Replay returns the original outcome; no reclassification under current policy; historical receipts stay decodable.
- [ ] Qualification matrix below executed against the final integrated source and PostgreSQL 17.6.
- [ ] Builder confirms the residue against main before code (plan §4).
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`character_death.rs`, `gameplay_transport/mod.rs`, `actor_spell.rs`, `cast.rs` take call sites only).

## Qualification matrix (summary)

- Victim session/lease/scope/World-vs-Channel mismatch, node revoke, retired actor: refuse without mutation.
- Same-occurrence replay versus substituted victim/cell/source/ruleset/marks: original once or exact conflict.
- Concurrent XP/build/death, stale revision, unknown training flush: one chain, flush reconciled first.
- Incomplete ledger, missing final blow, summon reparent/despawn, duplicate compact contributors, friendly fire.
- 4.999 %/5 %, small-ratio player final blow, monster final blow at 5 %, zero total, overflow.
- Delayed settlement past 5 min, cleanup races, lost write-ahead.
- Private marks, expired skulls at death, party/guild/war change.
- n = 5/6/7, assist-only 500, 16 full / 64 compact / 81st contributor, duplicate point.
- Skull history endpoints 24 h / 7 d / 30 d, new point counted twice, future stamp, overflow.
- Points in a later transaction, ledger deleted first, wrong World/contributor, forged xact.
- Avenging the wrong/newer/expired mark, duplicate avenging death.
- Twist as 8th, zero regular + Twist, red/black with Twist, Adventurer level 20/21/forfeited, low-level exemption.
- Any write failure or unknown COMMIT rolls back all staged effects; child item failure never rolls back the parent.
- Restart between receipt and item causes; black skull expiring before placement.

## Open questions

- Owner of the D1128 ledger/points adapter: this task or PVP-1b (confirm at allocation).
- Whether storage blockers 3–5 need a migration despite D1105 (CP decision with DEATH-3/PVP-1b).
- PVP0 gap-closure candidate acceptance (white extension, expert reset).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-combat-rules pvp
cargo test -p oteryn-gameplay-combat player_lethal
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres character_death_
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres pvp_state_postgres_cases::
python3 tools/agents/validate_governance.py
```
