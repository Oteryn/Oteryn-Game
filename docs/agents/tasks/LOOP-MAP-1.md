# LOOP-MAP-1 — server Cyclopedia discovery, donations and respawn

```yaml
task_id: LOOP-MAP-1
title: "LOOP-MAP-1: map loop: Cyclopedia discovery, donations and respawn"
mode: IMPLEMENT
status: implementing
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row LOOP-MAP-1; §7 'LOOP-MAP-1' (CYCLOPEDIA0 child graph)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md
  - docs/architecture/reviews/OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md
base_branch: main
base_sha: cb5b3de92d2e19df53e05a7e0f5b9d98fa11d299
branch: claude/happy-davinci-ijz3wy-srv-loopmap
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Historical audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL** source checkpoint. The owner-authorized basic server follow-up is recorded below; no published lifecycle completion is claimed.

## Server wiring gap list — server-first follow-up, 2026-10-24

Audited local `HEAD cb5b3de92d2e19df53e05a7e0f5b9d98fa11d299` and the existing
working-tree card before implementation. CARDS-AUDIT-1's **PARTIAL** residue was
confirmed: only source masks, a donation amount validator and a closed Reference
runtime port existed. No earlier server implementation was discarded.

Owner option A (2026-10-23) supersedes the earlier activation question. The basic
Oteryn path is separate from the still-unqualified Reference runtime port. This
is local AUTHORING with no Git mutation, frozen candidate, PR or publication.
Bound META 3.1.0 was refreshed read-only at its pinned commit. Client paths and
shared CI routing are untouched.

| Layer | Before this follow-up | Working-tree server resolution |
|---|---|---|
| L3 rules | Amount bounds only; closed Reference port | Premium start/shuffle, occurrence-bound deterministic POIs outside the current viewport, private same-floor finds, seventh-find completion |
| Content admission | Source masks had no native membership | Authored `basic-area.json`; native served/walkable connected Entry environs, immutable door exclusion, bounded pool; withheld without supporting bundle/native coin definitions |
| L4 storage | No map state/writer | Character progress and immutable command receipts, revision slot and live session/lease/node/recovery fences; World pool and immutable epoch selection |
| Migration | No Cyclopedia tables | Reserved 0176; fee cause 4, Character receipt-chain union, transaction guards, runtime grants, bundle/catalogue digest pins |
| Dispatch | No caller | Mechanic-owned handler under `gameplay_transport/cyclopedia`; reads committed runtime position, invokes rules/storage, emits retained result, refreshes negotiated inventory after donation |
| Protocol/registry | No map endpoint | Commands 72–73, capability 57, bounded request/result codecs; append-only registry fragments and limits 0870–0879 |
| Capability | No offer | Offered by fresh admission only after native authored-area admission; both negotiation and handler enforce selection |
| Runtime | No map consumer | Selection initialized before authenticated fresh admission and the independent respawn loop; normal home-spawn delays receive the persisted factor, with current scope proof |
| PostgreSQL E2E | None | Existing dispatch `--lib` auto-mount: admitted session → real command bytes → durable progress/fee/pool → result bytes and inventory delta; fresh session-state/sequence owner reload and lost-ACK replay |

### Reference parity

The owner authorized basic defaults, not a Reference parity qualification.

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Seven finds complete a character's subarea; resettable progress | Pinned manual [§3.2.2](../../reference/tibia-manual/interface.md); accepted CYCLOPEDIA0 §§4.1–4.3 | Durable single authored subarea; Premium start/shuffle resets; seventh distinct find completes; replay preserves the original result |
| Area, membership and POI catalogue | CYCLOPEDIA0 §3; decoded source masks remain source-only | **Oteryn default, not Reference-qualified:** area/subarea 1, Entry environs, same-floor ±32 entry square intersected with reachable served tiles; doors excluded regardless of state; at most 4,225 tiles and the first 128 eligible positions in coordinate order |
| Generated POIs and find radius | CYCLOPEDIA0 §8 records unresolved values | **Oteryn default, not Reference-qualified:** eight generated POIs, exact-tile radius zero; deterministic SHA-256 occurrence ranking; current 18×14 viewport excluded at start/shuffle |
| Player-visible POI observation | CYCLOPEDIA0 §6 limits visibility to the owning character, floor and range | **Oteryn default, not Reference-qualified:** explicit position-check intent and polling query; only own unfound same-floor in-window points returned; no automatic effect/message stream |
| NPC/passage/creature unlocks and speed | Manual §3.2.2; CYCLOPEDIA0 §§4.4–4.5 | One subarea makes completion 100%; no presentation/outfit/achievement/raid-warning consumers. **Oteryn default, not Reference-qualified:** speed contribution remains zero |
| Single donation 1..=10,000,000 gp | Owner D1230a/b; CYCLOPEDIA0 §5.3; BANK-FEE-0 §3 | Existing coin-first fee writer and bank shortfall, fee audit and pool increment in one fenced Character transaction; junior/activation rules preserved; no double charge on replay |
| Donation eligibility/Premium | CYCLOPEDIA0 §8 leaves full eligibility and Premium qualification pending | **Oteryn default, not Reference-qualified:** only authored area 1 eligible, no donation Premium requirement; current improved area refuses donation and rolls back any provisional fee |
| Reset selection and pool carry | Manual §3.2.2; CYCLOPEDIA0 §5.4 | Winner requires 10,000,000 gp; only winner pool resets; prior improved area excluded. **Oteryn default, not Reference-qualified:** donation before deterministic 33/100 random draw; unary catalogue has no tie |
| Improved home respawn | CYCLOPEDIA0 §5.5; SPAWN-1b native per-point timing | Persisted epoch selects factor 2; native 60 s home delay becomes 30 s in E2E. **Oteryn default, not Reference-qualified:** standalone factor only; normal Bestiary creatures with complete details eligible; bosses, reward creatures, familiars, unknown profiles and non-home actors excluded |
| Content changes and recovery | CYCLOPEDIA0 §4.2a; existing Character recovery/fence contracts | Bundle and authored catalogue digests pin state; incompatible changes refuse admission/writes and preserve all value. Normalization/reconfiguration remains outside this stable basic catalogue |
| Older clients | CYCLOPEDIA0 §6 | No selected capability means no Cyclopedia result; malformed request or stale generation fails closed |

### Validation evidence

Basic server implementation and local qualification PASS. `implementing` retains
the required pre-PR lifecycle state; it is not a publication/merge claim.

All tests used `CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked`.

| Package / test scope | Exact filter | Passed |
|---|---|---:|
| `oteryn-gameplay-dispatch --lib` PostgreSQL E2E | `cyclopedia_postgres_real_dispatch` | 1 |
| `oteryn-gameplay-domain --lib` | `cyclopedia` | 2 |
| `oteryn-gameplay-durability --lib` | `cyclopedia` | 1 |
| `oteryn-gameplay-durability --lib` | `writer_registration_tests` | 2 |
| `oteryn-gameplay-entities --features test-support --lib` | `respawn` | 13 |
| `oteryn-gameplay-world --lib` | `map::boot::tests` | 18 |
| `oteryn-protocol-oteryn --lib` | all unit tests (no PG) | 393 |

Final PG run emitted `POSTGRES_STARTED LOOP-MAP-1 cyclopedia_postgres_real_dispatch`
and passed in 23.66 s; no route-less early return qualified this result.

Clippy `--all-targets -- -D warnings` PASS for those six packages and
`oteryn-game-server`; cargo fmt/explicit changed-file rustfmt PASS. Existing
vendored Tokio/SQLx warnings remain outside product crates. Governance, its 129
unit tests, repository policy and diff whitespace checks PASS.

[Exact commands, file inventory and review packet](../evidence/LOOP-MAP-1-server-validation-20261024.md).
No unfiltered PostgreSQL suite or unrelated mechanic case ran. Standalone entity
tests require their existing `test-support` feature; no manifest change was needed.

### Unfinished / qualification limits

- Full Reference catalogue/parity, dynamic catalogue normalization and structural
  reconfiguration are unqualified. Existing durable values fail closed on a
  changed bundle/catalogue; no downgrade or silent reinterpretation is admitted.
- Automatic POI effect/message delivery, achievements, outfit/title/raid-warning
  consumers and nonzero speed bonuses are not part of this basic server wire.
  Client UI is outside the owner's scope.
- Reset E2E seeds an already-activated next-epoch snapshot, matching the existing
  map-consumer fixtures; it qualifies selection/consumption, not the reset
  control-plane publication itself. Restart qualification replaces the composed
  session-state/sequencer owner and reloads PostgreSQL; it is not an OS process kill.
- Independent exact-candidate review and repository CI/publication remain for the
  control plane. This worker has no commit/push authority; no review trigger sent.

Historical source-only scope, evidence and allocation are retained in
[the source history](../evidence/LOOP-MAP-1-source-history-20261024.md).
