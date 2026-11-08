# questpc3 n3 current handoff

Base main: `a16f196762d66ef27f76a02545960ec8b4283295`. Branch: `claude/happy-davinci-ijz3wy-questpc3b-n3`.

This branch starts from current main. Earlier branches are frozen for collector packaging; no earlier feature was recreated or merged independently. The local loop ledger is `.lane/rows.md` and remains uncommitted.

All six retained quests were checked against current main and current engine branches. Four need the shared owners below; two were qualified on the previous lane branch but are absent from main. No quest is newly marked DONE. Sixteen other-owner packet rows remain excluded.

- **The Beginning Quest — WAITING-ENGINE**: Source ground-branch birth and five physical USE_WITH cart deliveries; retain prior NPC/reward work when collector delivers it.
- **The First Dragon Quest — WAITING-ENGINE**: Recurring Jan14-Feb12 NPC date guard and named per-player boss cooldown/room cleanup; current delayed-spawn timing is value-free and excludes players.
- **The Tainted Souls Quest — WAITING-ENGINE**: Atomic whole-target-stack talisman conversion, live-creature cleansing including summons, 100-soul gate and20h Ekatrix reset. Never replace cleansing with kills.
- **Threatened Dreams Quest — WAITING-ENGINE**: Fenced repeating world callback transaction clocks for harvest/cooldown, plus scheduled NPC appearances. QENGINE2 branch adds state_operations/has_repeating_clock; main has neither.
- **Pilgrimage of Ashes Quest — BLOCKED**: Previous qualified implementation is not yet on main; coordinator is packaging earlier lane branches. Do not duplicate or drop that work.
- **Stampede Mini World Change — BLOCKED**: Previous qualified implementation is not yet on main; coordinator is packaging earlier lane branches. Do not duplicate or drop that work.

## Resume order

Merge origin/main first. Reconcile the collector delivery from previous head `dc9e763c856503bfd897e50238b99002f5791005`, then consume the assigned engines. Continue one quest at a time through start, every step, physical reward and completion with a real PG case. Historical qualified slices do not establish a fresh end-to-end result.

QENGINE2 is actively implementing repeating-clock callbacks (`790996f8e`, branch head `925d8480e`). ENGITEM currently adds the authoritative quest clock and atomic track capacity (`bbe018482`); ENGENC currently adds private boss-loot admission (`4ffd19f3a`). These changes are not evidence that the outstanding quest-specific contracts are delivered.

No gameplay/source/generated files changed in this audit. Required changed-crate tests and own new PG cases have no applicable changes; no test was weakened, skipped, or deleted. Workspace check/fmt are recorded after completion.

Existing encounter timing is acknowledged rather than requested twice: `crates/gameplay/quests/src/quest/world_callbacks/effects.rs:189` defines delayed spawn/reset, while line300 refuses nonempty players for that timing mode. First Dragon still needs admission plus the source named personal cooldown and room cleanup; a value-free delayed spawn is not equivalent. The exact month-only NPC guard is at `npc_rules.rs:149`; the source season crosses day boundaries inside January and February.

Final checks: `cargo check --workspace --all-targets` PASS7m31s; `cargo fmt --all --check` PASS; repository duplicate-migration check PASS; focused scope/evidence audit PASS6 terminal owned rows (0DONE,4WAITING-ENGINE,2BLOCKED). Final origin/main fetch/merge is unchanged107625470. No runtime/content/schema/generated input changed, so no generated refresh, changed-crate unit suite, or new gameplay PG claim applies to this documentation-only handoff. `.lane/rows.md` is locally excluded and is not committed. Earlier branches remain untouched.

MAINRED1108 delivery3b4696be3: reused migration1553 and both native reset-read call sites unchanged. Boss-clock/reset access is repaired; First Dragon still needs the source-specific admission/cooldown/cleanup binding and annual NPC gate. Other engine/collector dependencies remain open. Requalifying the upstream focused PG case, workspace and fmt before publication. No content/generated input changed.

SEASON6ef6b0cad arrived during the final main sync and was merged cleanly. Rechecked all6 rows: unchanged four feature-gap source blobs, earlier collector head still unmerged, seasonal NPC adapter fixed to Santa/Brutus/bakers. No quest is newly actionable. Previous reset-lock PG8.450s evidence belongs to3b4696be3; case/migration unchanged, but no current-head PG result is asserted. Refreshing workspace/fmt on the new main. No lane-authored content or serve change was introduced; all seasonal implementation remains main-owned.

SEASON PUBLICATION QUALIFIED: `cargo check --workspace --all-targets` PASS7m52s and fmt PASS on fixed main6ef6b0cad snapshot; duplicate check PASS. Current branch runtime/content/serve tree equals merged main; lane-authored delta is only handoff records. No additional source generation or boot/content-activation modification was authored. Prior selected reset-lock PG on3b4696be3 PASS8.450s remains explicitly ancestor-scoped, not a current-head gameplay qualification. Collector callback/test artefacts for Pilgrimage/Stampede remain absent, verified independently of Git ancestry. All6 retained rows remain4WAITING-ENGINE/2BLOCKED; .lane rows stay uncommitted. Original questpc3/questpc3b branches were not pushed.

Client settings and native login challenge main delivery e9276ad7e merged unchanged. ENGNPC e22dff551 paid-travel ownership fix inspected; date/day guard still absent. All four engine-gap source blobs unchanged; collector callbacks absent. Six terminal dispositions remain valid. Rechecking workspace/fmt; no lane-authored runtime or content change.

CLIENT-MAIN PUBLICATION QUALIFIED: maine9276ad7e integrated unchanged; workspace all-targets PASS7m 05s; fmt and duplicate check PASS. All six retained quest dispositions remain4WAITING-ENGINE/2BLOCKED. Main client settings/login changes and ENGNPC paid-travel confirmation fix do not close a missing quest contract. Only lane-owned handoff records changed relative to main; no runtime/content/schema input authored and no new gameplay qualification or PG result asserted. Current local ledger remains uncommitted.

ENGITEM ROUTING AUDIT:183f49bc1 active queue omits questpc3 and retains its requests in legacy audit. QENGINE2 already queues six owned features; do not duplicate those. New ENGINE-REQUESTS.md makes the current requests discoverable, includes the Items/Content B4 boundary and Beginning bridge request18/B1/B7. Star-herb source capture independently matches committed inventory blobd76b2fcd9c9411ee9839f55e9d5d9c563a73e50d and SHA256f3ffe07cf0538c6a072cc0145e340ea9bf1a74a83601e3e7c42c6a90d03f13e4. Maina16f19676 client parity changes merged unchanged; required push verification will include the raw exact-ref ls-remote line. No new gameplay completion.

ROUTING/PUBLICATION QUALIFIED: workspace all-targets PASS7m 08s; fmt/duplicate checks PASS; locked cargo metadata PASS; tracked pins/lock/generated content identical to maina16f19676. Request packet preserves six existing QENGINE2 entries and explicitly routes frontendB4/bridgeB1-B7 instead of treating ENGITEM legacy audit as active implementation. All six quests remain4WAITING-ENGINE/2BLOCKED, no new gameplay result. Main merge was already in progress when rule changed; finish this publication, then no more autonomous main merges. Future Cargo commands use one build job; no concurrent Cargo jobs. Every push must be followed by raw exact-ref git ls-remote output.
