# E2E rows: remaining work (handoff 2026-11-09, resume Wednesday 2026-11-11)

PR #3023, branch `claude/happy-davinci-ijz3wy-e2e-rows`. This file is the
resume point; live PR and check state win over it.

## State at handoff

- Head `df0a6962c`: `main` merged up to #3069, rows run in batches
  (`OTERYN_E2E_ROW_BATCH=start:len`, default `0:15`, see
  `client_phases/e2e_row_batch.rs`), every row waits for both characters'
  release, and C20a/C20d take batch indices 228/229. Indices 0..229 cover the
  row calls in `client_phases/world.rs` `journeys`.
- Node boot on `df0a6962c` failed in the default dev-client phase
  (`disconnect_resume`: `game durability root is not ready`, then
  `resume_unavailable`). The same signature is red on `main` (Merge Queue node
  boot at `91786a101`). This is a main defect, not this PR.
- The re-run (job 129322268460) passed the default phase. In the row phase it
  cancelled at the 60-minute limit with 28 row results, 0 PASS. Batch 0:15
  ended at W23 as expected, but C19a, C20d, S16a, C20a, C20b and P16 still
  ran after it. Find their unwrapped callers (probably inside `full_e2ef5` or
  the base journey) and put them behind `e2e_row_selected`.
- Batch 0:20 on `115fac74b`: 30 row results, 0 PASS; the job hit its
  60-minute limit. Most rows lost the connection on their first command
  (connection reset, broken pipe or TLS EOF), and each failed row then waited
  about 75 s for its session release.
- PostgreSQL legs: the only reds were foreign (S17 `s17a`/`s17b`, encounter
  deadlock in PG 24). Each was commented on #3023 and re-run once.

## To finish

1. Check whether `main` has a fix for the
   durability-root loss (`durability root is not ready`, `control_lost
   path=liveness_missed`). If so, merge `main` (merge drivers, `cargo fmt`,
   `tools/merge-driver/regen.sh`) and push.
2. When batch 0:15 runs, read the `session close events behind failed e2e
   rows` block in the node boot log. Fix the test defects of the batch in one
   push: C01 equipment, C06 seller, C11 Oressa, W10/W15d/W23 routes, W15a NPC
   command, W15b reward chest, W22b prey slot, W24 cue, C04 PZ cast, C05
   timeout.
3. Advance `E2E_ROW_BATCH_DEFAULT` by one batch per push (15:15, 30:15, ...)
   until index 229.
4. Write journeys for the 71 new rows below, in our own test files only (no
   lane paths), and give each a batch index after 229.
5. Write the per-row report (PASS / FAIL / MISSING-SERVER / MISSING-CLIENT)
   and send the coordinator ONE message with the FAIL/MISSING-SERVER and
   MISSING-CLIENT lists.
6. Merge #3023 when its own CI is green, then report "zmergowane #3023".
7. Keep the hourly check of `main`: no lane is skipped any more (owner
   2026-11-09). Add each new mechanic as a row here.

## Owner decisions (2026-11-09)

- No lane is skipped: quest, encounter, season, Soulpit, player trade and S17
  rows are tested too. Tests go only into our own test files.
- Excluded: P23a/P23b (Prey Hunting Tasks) and W14a-e (OT config rows).
  X01-X05 are owner decisions; X06, S12 and E11 are out of scope.

## New rows (71)

`N*`: mechanics merged to `main` after the scenario matrix. `A-*`: audit rows
that were MISSING in `docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`
and are outside `SCENARIO_MATRIX.md`. A row whose mechanic the server still
lacks is reported as MISSING-SERVER, not skipped.

| Row | Source | Mechanic | Side |
|---|---|---|---|
| N01 | #3047 | house controls in production session (apps/client 0049-house) | server+client |
| N02 | #3047 | named Wheel builds + code exchange (0148-wheel-allocation presets/settings) | server+client |
| N03 | #3047 | depot search result list + retrieval (0153-depot-search) | server+client |
| N04 | #3047 | player bug report (0076-player-report bug.rs) | server+client |
| N05 | #3049 | settings import/export/backup/defaults (local-reference-preferences) | server+client |
| N06 | #3048 | native login challenge before admission (native_entry/challenge, platform-client) | server+client |
| N07 | #3039 | W18 house furniture rotation/decorations client (0445) -> fold into W18 | server+client |
| N33 | #3039 | W05d Cake/Santa seasonal | server+client |
| N10 | #3046 | Cyclopedia character titles, analyser, boss tracker, minimap markers (apps/client 0010 0097 0117 ...) | server+client |
| N34 | #3050 | ENGNPC NPC/quest receipts | server+client |
| N08 | #3054 | taming mounts: use taming item on source creature grants mount; construction self-use | server+client |
| N09 | #3054 | pocket watch use shows world time | server+client |
| N35 | #3054 | Soul War/Rotten Blood/Primal-Hazard selection, private boss loot, Svargrond arena (encounter lane) | - |
| N11 | #3058 | imbuement details query + panel (cap 0447, cmd 0731) | server+client |
| N12 | #3058 | weapon proficiency state + perks (cap 0448, cmd 0732) | server+client |
| N13 | #3058 | spellbook details query + spell augments (cap 0449, cmd 0733, mig 1710) | server+client |
| N14 | #3058 | server modal dialog query/answer (cap 0450, cmd 0736/0737) | server+client |
| N15 | #3058 | analyser item + party hunt queries, loot tracker (cmd 0734/0735) | server+client |
| N16 | #3058 | daily reward details, picker, pick claim (cmd 0738/6122/6123, mig 1711) | server+client |
| N17 | #3058 | returner reward query/claim (cmd 6124/6125, mig 1712) | server+client |
| N18 | #3058 | house podium preview (cmd 0739) | server+client |
| N19 | #3058 | battle list metadata query (cmd 0740) | server+client |
| N20 | #3058 | forge preview (cmd 6116) | server+client |
| N21 | #3058 | chat console query (cmd 6117) | server+client |
| N22 | #3058 | store browse + offer popularity (cmd 6118, mig 2505; payment sandbox only) | server+client |
| N23 | #3058 | wheel presentation + gem presentation (cmd 6119/6121) | server+client |
| N24 | #3058 | market details + statistics (cmd 6126, mig 1713) | server+client |
| N25 | #3058 | coin transfer quotes + coin market (cmd 6127/6128, mig 2507/2508) | server+client |
| N26 | #3058 | house sale escrow/authority, disposition recipients, charged furniture wrapping (cmd 6129, mig 2601-2604/2703/2704) | server+client |
| N27 | #3058 | account friends + badge visibility (mig 2503/2504) | server+client |
| N28 | #3058 | Character Bazaar + world transfer, login challenge fences (cg-account) | server+client |
| N29 | #3058 | quest tracker + filters, achievements, titles, defence stats, blessings (cg-qtrack) | client |
| N30 | #3058 | Cyclopedia character misc skills + resource bars (cg-cyclo) | client |
| N31 | #3058 | options pages, HP/MP arcs, minimap memory, action bar, settings (local-reference-preferences, local-spell-action-bar) | client |
| N32 | #3058 | shrine quotes, bug report flow (cg-systems) | server+client |
| N36 | #3058 | soulseal fight entry (cmd 6120, mig 2506) – soulpit lane | - |
| A-C19b | audit | Starting items: per-vocation equipment and backpack contents | server+client |
| A-I17a | audit | Hotkey equip: worn item by id from backpack/loot pouch | server+client |
| A-I17b | audit | Hotkey equip: toggle unequip to managed container | server+client |
| A-I17c | audit | Hotkey equip: auto two-hand/shield swap and ammo-to-quiver | server+client |
| A-I17d | audit | Hotkey equip: ring/necklace delay and exhaust/feared gates | server+client |
| A-I18a | audit | Browse field: open tile browse-field container with walk-to | server+client |
| A-I18b | audit | Browse field: move items out of/into the browse field | server+client |
| A-I18c | audit | Browse field: move up from a ground container | server+client |
| A-I19 | audit | Gem/weapon enchanting at elemental shrines | server+client |
| A-P12b | audit | Cyclopedia badges | server+client |
| A-P22a | audit | Cyclopedia tracker: bestiary add/remove | server+client |
| A-P22b | audit | Cyclopedia tracker: bosstiary add/remove (kill-gated) | server+client |
| A-P22c | audit | Cyclopedia tracker: list persistence across login | server+client |
| A-P22d | audit | Cyclopedia tracker: live refresh on kill progress | server+client |
| A-S14a | audit | Team Finder: leader listing create/edit/remove | server+client |
| A-S14b | audit | Team Finder: browse list and join request/cancel | server+client |
| A-S14c | audit | Team Finder: leader accept/deny/invite | server+client |
| A-S15a | audit | Login protection: monsters ignore a freshly logged-in player (10 s) | server+client |
| A-S16c | audit | Vocation choice: trial tiles before level 20 | server+client |
| A-S16d | audit | Vocation choice: vocation room teleport/doors and mainland item removal | server+client |
| A-S17a | audit | Livestream (Canary only): caster on/off, password, description | server+client |
| A-S17b | audit | Livestream (Canary only): read-only viewer login and fan-out | server+client |
| A-S17c | audit | Livestream (Canary only): viewer chat and moderation | server+client |
| A-S19 | audit | Player typing indicator | server+client |
| A-W05b | audit | Daily world changes | server+client |
| A-W05d | audit | Seasonal events / events scheduler | server+client |
| A-W06 | audit | Echo Warden kill trigger, portal, raid and first-species charm reward | server+client |
| A-W07 | audit | Soulpit waves, Soul Core/prism/seal custody and survival rewards | server+client |
| A-W09 | audit | Concoction effects/cooldowns and Drome Cube consume/reward | server+client |
| A-W15c | audit | Killing in the Name Of tasks (Grizzly Adams) | server+client |
| A-W19b | audit | Planting/seeds | server+client |
| A-W19c | audit | Novelty items (fireworks, toys) | server+client |
| A-W19d | audit | Outfit/mount usable items | server+client |
| A-W19e | audit | Rust remover, lottery ticket, anniversary items | server+client |
| A-W21 | audit | Boss difficulty selection | server+client |
