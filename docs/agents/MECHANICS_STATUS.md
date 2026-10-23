# Mechaniki Tibii — status planowania w Oteryn Game

**Status dokumentu: DRAFT — indeks planów, nie deklaracja gotowości produktu.**

- Data zestawienia: 2026-10-23, Europe/Warsaw.
- Zakres: cooking, mining, boss encounters i Store; nie jest to pełny katalog mechanik.
- `docs/agents/MECHANICS_STATUS.md` nie istniał w bazowym drzewie. Format tabeli
  pochodzi z `docs/architecture/reviews/OTERYN_GAME_TIBIA_MECHANICS_STATUS_DRAFT_2026-10-16.md`;
  historyczne statusy klienta z tamtego audytu nie są tu aktualizowane.
- Sprawdzono aktywne i archiwalne karty przed dodaniem planów. CRAFT-1 obejmuje
  cooking, MONSTER-META-1 obejmuje BOSS-1, BOSS-ROOM-1, BOSS-REWARD-1 i BOSSTIARY-1.
  Nie dodano duplikatów COOK-1.md ani BOSS-1.md; poniżej rozszerzono ich plany.
- Archiwalny Store catalogue authoring dotyczy wyłącznie narzędzi offline, a nie
  zakupów runtime. CRAFT1 C5 wyłącza digging yields; MINING-1 nie powiela otwierania dziur.
- 🔵 oznacza plan; stan wdrożenia i zależności trzeba zweryfikować przy alokacji.
  Live GitHub Issue, PR i checks pozostają autorytetem cyklu zadania.

## Tabela mechanik

| Mechanika / funkcja | Serwer | Klient | Co pozostaje |
|---|---|---|---|
| Boss encounters / Bosstiary (BOSS-1) | 🔵 Plan w [MONSTER-META-1](tasks/MONSTER-META-1.md) | Do weryfikacji przy alokacji | Cooldown postaci, lever/room entry, kill credit, loot i Bosstiary; rozszerzenie poniżej, bez nowej karty. |
| Cooking / food crafting (COOK-1) | 🔵 Plan w [CRAFT-1](tasks/CRAFT-1.md) | Do weryfikacji przy alokacji | Składniki, przepisy, oven/fire i jedzenie z regen; rozszerzenie poniżej, bez nowej karty. |
| Mining / gathering (MINING-1) | 🔵 [waiting](tasks/MINING-1.md) | Do weryfikacji przy alokacji | Pick na stone/ore, resource cooldown/respawn i drops; osobna decyzja o yields. |
| In-game Store (STORE-1) | 🔵 [waiting](tasks/STORE-1.md) | Do weryfikacji przy alokacji | Balance, catalogue, durable fenced purchase i history przez kontrakt Platform; bez real payments. |

## COOK-1 — rozszerzenie istniejącego CRAFT-1

Planning state: `waiting` — allocation and current dependency verification pending.
The accepted CRAFT1 brief already covers cooking conversions and CRAFT-FOOD-1 dish
semantics; this extension introduces no second conversion owner or recipe store.

### Scope and layering

- Ingredients and admitted recipes, carried ingredient combinations, oven/fire target
  eligibility and output food items. Reuse native item-use food regeneration;
  consumption of a produced food must reach the existing regeneration semantics.
- Pure recipe/ingredient rules in an L3 existing rules module, proposed
  `crates/gameplay/items/src/cooking/`; storage in its own
  `crates/gameplay/durability/src/durability/item_craft/` submodule, reusing the
  CRAFT writer, receipts and native consume/MINT transaction rather than duplicating it.
- Dispatch wiring is a separate later slice in
  `crates/gameplay/dispatch/src/gameplay_transport/item_craft/`. Any migration and
  its writer remain inseparable. Small child files separate planning, decoding,
  storage and tests; shared registrations are wiring only.
- Proposed owned_paths: the three directories above and
  `apps/game-server/src/lifted_tests/auto/cooking_postgres_cases.rs`; exact content
  inputs, shared seams and updates to the existing card require lane allocation.
- Migration numbers and resource-limit ranges: **allocated at lane start**; no
  reservations. Reuse existing CRAFT schema where sufficient.

Excluded scope: new hunger economy, replacement food/condition runtime, mining,
NPC cooking services already owned by HIRE-1, client redesign and Soul War files.
Accepted CRAFT1 requirements remain binding; this extension does not defer them.

### Reference parity

Canary (`opentibiabr/canary`) paths to consult as evidence:

- `data-otservbr-global/scripts/actions/other/baking.lua`

Follow its output item definitions to food semantics at the pinned revision.

TibiaWiki page names: **Cooking, Baking, Food, Oven, Bread**.
TODO: verify recipe operands/counts, eligible heat sources, resulting item identities,
regeneration durations, tool/container transformations and any special dish cooldowns
against the accepted CRAFT1 brief and pinned upstream. Do not invent missing values.

### Acceptance and validation

One conversion consumes exactly the admitted ingredients and mints the admitted food
atomically, or changes nothing. Wrong recipe/heat source, insufficient ingredients,
full output capacity, stale session-generation and duplicate command are tested.
Concurrent use cannot consume the same units twice. Eat the output through real
dispatch and assert the existing food regeneration effect. Property tests cover
ingredient conservation, partial stacks, checked arithmetic and replay equivalence.

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-gameplay-items -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-items --lib cooking::
cargo test --locked -p oteryn-gameplay-items --lib cooking::property_
cargo test --locked -p oteryn-gameplay-durability --lib item_craft::
cargo test --locked -p oteryn-gameplay-dispatch --lib item_craft::
cargo test --locked -p oteryn-game-server --lib lifted_tests_auto::cooking_postgres_cases::
```

These are planned filters; require actual matching tests at implementation. PG cases
are auto-mounted with no shared mount edit, use the installed migrator/runtime role,
and cover rollback, replay, restart and stale fences. They return `Ok(())` when
admin routing is absent, without claiming PG qualification. Run only this mechanic's
PG filter locally. Every new public function requires a production caller and a
real-dispatch end-to-end test.

Risks: competing CRAFT ownership, ingredient/container identity, atomic retirement
and minting, accidentally redefining existing food regeneration and content admission.

## BOSS-1 — rozszerzenie istniejącego MONSTER-META-1

Planning state: `waiting` — family lane allocation and current dependency verification
pending. The existing family card and accepted BOSS_RAID0 brief own this mechanic;
its historical partial spawn-clock work is not evidence of full encounter completion.

### Scope and layering

- Per-Character boss cooldown, lever/room eligibility and bounded entry/exit, durable
  death occurrence with frozen kill-credit set, loot delivery rules and Bosstiary
  progress. Follow the accepted BOSS-1 → BOSS-ROOM-1 → BOSS-REWARD-1 → BOSSTIARY-1
  graph and real encounter outcome/death owner; recheck dependencies at allocation.
- Pure L3 encounter rules in `crates/gameplay/encounter-rules/src/boss/`, separated
  into eligibility, cooldown and credit plans with child tests.
- Durable state remains in its own existing
  `crates/gameplay/durability/src/durability/boss_encounter/` submodule (including
  the existing `boss_encounter.rs` facade) and
  `crates/gameplay/durability/src/durability/character_bosstiary.rs` with child tests.
  Extend existing owners; no duplicate spawn-clock or kill-credit store.
- Dispatch wiring is a separate later slice in
  `crates/gameplay/dispatch/src/gameplay_transport/boss/`, with actor/runtime
  integration owned by `crates/gameplay/encounters/src/bosses/` and scope handoff.
  Allocate exact touched runtime child files before writing; this is not ownership
  of the entire bosses tree.
- Proposed owned_paths: the rules, storage and dispatch paths above, plus
  `apps/game-server/src/lifted_tests/auto/boss_postgres_cases.rs`. Keep small
  mechanic-specific files; shared registration changes are wiring only.
- Migration numbers and resource-limit ranges: **allocated at lane start**; none
  reserved. Reuse existing schema and accepted limits where applicable.

Excluded scope: scheduled raids already owned by EVENTS-RAID-1, a replacement encounter
engine, a second monster death/loot owner, speculative boss content, Soul War files
and redistribution of reference assets.

### Reference parity

Canary (`opentibiabr/canary`) paths to consult as evidence:

- `data/libs/functions/boss_lever.lua`
- `data/scripts/creaturescripts/monster/boss_lever_death.lua`
- `data/libs/systems/reward_boss.lua`
- `src/io/io_bosstiary.cpp`

TibiaWiki page names: **Bosses, Boss Cooldowns, Reward System, Bosstiary**.
TODO: verify boss-specific cooldowns, lever participant limits, room occupancy/timeout,
credit eligibility, reward contribution/distribution and Bosstiary thresholds/points.
Use accepted BOSS_RAID0 decisions where binding; classify reference differences rather
than replacing them with guessed numbers.

### Acceptance and validation

Two lever attempts cannot admit overlapping occupancy or bypass a Character cooldown.
Entry failure cannot strand a room reservation. Death credit, loot and Bosstiary
updates reconcile by the retained death occurrence through restart/reconnect, with
no duplicate mint/progress. Character writes remain session-generation fenced;
WorldId and ChannelId stay distinct. Property tests cover cooldown boundaries,
credit-set ordering/deduplication and one reward/progress update per death identity.

```sh
cargo fmt --all --check
cargo clippy --locked -p oteryn-gameplay-encounter-rules -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-encounter-rules --lib boss::
cargo test --locked -p oteryn-gameplay-encounter-rules --lib boss::property_
cargo test --locked -p oteryn-gameplay-durability --lib boss_encounter::
cargo test --locked -p oteryn-gameplay-dispatch --lib boss::
cargo test --locked -p oteryn-game-server --lib lifted_tests_auto::boss_postgres_cases::
```

These are planned filters; require actual matching tests at implementation. Auto-mounted
PG cases exercise installed migrations/runtime grants, competing entry/death consumers,
rollback, replay, scope isolation, stale fences and restart reconciliation. No new
PG target or shared mount edit. Missing admin routing returns `Ok(())` without PG
qualification. Only the mechanic's PG cases run locally. Every new public function
needs a production caller and an end-to-end test through real dispatch, including
lever entry, actual death, loot and progress observations.

Risks: room handoff versus cooldown transaction ordering, duplicate death consumers,
ambiguous reward commits, disconnected credited characters, and treating durable
spawn metadata as proof of a physically realized boss.
