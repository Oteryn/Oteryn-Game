# OTV2-20261004-monster-main-reconciliation

```yaml
task_id: OTV2-20261004-monster-main-reconciliation
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: reconcile/monster-main-20261004
base_sha: 95c93340dd43eb7e52ef6e29e766ec94929a206f
pr: null
owner: root
owned_paths:
  - apps/game-server/src/content/project/bestiary.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/abilities/definitions/abilities-00000-00499.json
  - content/abilities/definitions/abilities-00500-00999.json
  - content/abilities/definitions/abilities-01000-01499.json
  - content/abilities/definitions/abilities-01500-01999.json
  - content/abilities/definitions/abilities-02000-02499.json
  - content/abilities/definitions/abilities-02500-02999.json
  - content/abilities/definitions/abilities-03000-03499.json
  - content/abilities/definitions/abilities-03500-03999.json
  - content/abilities/definitions/abilities-04000-04499.json
  - content/abilities/definitions/abilities-04500-04999.json
  - content/abilities/definitions/abilities-05000-05499.json
  - content/abilities/definitions/abilities-05500-05999.json
  - content/abilities/definitions/abilities-06000-06499.json
  - content/abilities/effects/effects-00000-00499.json
  - content/abilities/effects/effects-00500-00999.json
  - content/abilities/effects/effects-01000-01499.json
  - content/abilities/effects/effects-01500-01999.json
  - content/abilities/effects/effects-02500-02999.json
  - content/abilities/effects/effects-03000-03499.json
  - content/abilities/effects/effects-03500-03999.json
  - content/abilities/effects/effects-04000-04499.json
  - content/abilities/effects/effects-04500-04999.json
  - content/abilities/effects/effects-05000-05499.json
  - content/abilities/formulas/formulas-00000-00499.json
  - content/abilities/formulas/formulas-00500-00999.json
  - content/abilities/formulas/formulas-01000-01499.json
  - content/abilities/formulas/formulas-01500-01999.json
  - content/abilities/formulas/formulas-02000-02499.json
  - content/abilities/formulas/formulas-02500-02999.json
  - content/abilities/formulas/formulas-03000-03499.json
  - content/abilities/formulas/formulas-03500-03999.json
  - content/abilities/formulas/formulas-04000-04499.json
  - content/abilities/formulas/formulas-04500-04999.json
  - content/abilities/formulas/formulas-05000-05499.json
  - content/behaviors/behaviors-01500-01999.json
  - content/behaviors/behaviors-02000-02499.json
  - content/content.lock.json
  - content/cosmetics/mounts/index.json
  - content/creatures/definitions/creatures-00000-00499.json
  - content/creatures/definitions/creatures-00500-00999.json
  - content/creatures/definitions/creatures-01000-01499.json
  - content/dialogues/definitions/index.json
  - content/documents/index.json
  - content/items/definitions/items-00000-00499.json
  - content/items/definitions/items-00500-00999.json
  - content/items/definitions/items-01000-01499.json
  - content/items/definitions/items-01500-01999.json
  - content/items/definitions/items-02000-02499.json
  - content/items/definitions/items-02500-02999.json
  - content/items/definitions/items-03000-03499.json
  - content/items/definitions/items-03500-03999.json
  - content/items/definitions/items-04000-04499.json
  - content/items/definitions/items-04500-04999.json
  - content/items/definitions/items-05000-05499.json
  - content/items/definitions/items-05500-05999.json
  - content/items/definitions/items-06000-06499.json
  - content/items/definitions/items-06500-06999.json
  - content/items/definitions/items-07000-07499.json
  - content/items/definitions/items-07500-07999.json
  - content/items/definitions/items-08000-08499.json
  - content/items/definitions/items-08500-08999.json
  - content/items/definitions/items-09000-09499.json
  - content/items/definitions/items-09500-09999.json
  - content/items/definitions/items-10000-10499.json
  - content/items/definitions/items-10500-10999.json
  - content/items/definitions/items-11000-11499.json
  - content/items/definitions/items-11500-11999.json
  - content/items/definitions/items-12000-12499.json
  - content/items/definitions/items-12500-12999.json
  - content/items/definitions/items-13000-13499.json
  - content/items/definitions/items-14000-14499.json
  - content/items/definitions/items-14500-14999.json
  - content/items/definitions/items-15000-15499.json
  - content/items/definitions/items-15500-15999.json
  - content/items/definitions/items-16000-16499.json
  - content/items/definitions/items-16500-16999.json
  - content/items/definitions/items-17000-17499.json
  - content/items/definitions/items-17500-17999.json
  - content/items/definitions/items-18000-18499.json
  - content/items/definitions/items-18500-18999.json
  - content/items/definitions/items-19000-19499.json
  - content/items/definitions/items-19500-19999.json
  - content/items/definitions/items-20000-20499.json
  - content/items/definitions/items-20500-20999.json
  - content/items/definitions/items-21000-21499.json
  - content/items/definitions/items-21500-21999.json
  - content/items/definitions/items-22000-22499.json
  - content/items/definitions/items-22500-22999.json
  - content/items/definitions/items-23000-23499.json
  - content/items/definitions/items-23500-23999.json
  - content/items/definitions/items-24000-24499.json
  - content/items/definitions/items-24500-24999.json
  - content/items/definitions/items-25000-25499.json
  - content/items/definitions/items-25500-25999.json
  - content/items/definitions/items-26000-26499.json
  - content/items/definitions/items-26500-26999.json
  - content/items/definitions/items-27000-27499.json
  - content/items/definitions/items-27500-27999.json
  - content/items/definitions/items-28000-28499.json
  - content/items/definitions/items-28500-28999.json
  - content/items/definitions/items-29000-29499.json
  - content/items/definitions/items-29500-29999.json
  - content/items/definitions/items-30000-30499.json
  - content/items/definitions/items-30500-30999.json
  - content/items/definitions/items-31000-31499.json
  - content/items/definitions/items-31500-31999.json
  - content/items/definitions/items-32000-32499.json
  - content/items/definitions/items-32500-32999.json
  - content/items/definitions/items-33000-33499.json
  - content/items/definitions/items-33500-33999.json
  - content/loot/loot-00500-00999.json
  - content/manifest.json
  - content/npcs/definitions/index.json
  - content/presentations/definitions/presentations-00000-00499.json
  - content/presentations/definitions/presentations-00500-00999.json
  - content/presentations/definitions/presentations-01000-01499.json
  - content/presentations/definitions/presentations-01500-01999.json
  - content/presentations/definitions/presentations-02000-02499.json
  - content/project.json
  - content/services/trade/index.json
  - content/services/travel/index.json
  - content/world/assets/catalog.json
  - content/world/content.lock.json
  - content/world/editor/author.json
  - content/world/manifest.json
  - content/world/presentations/bindings.json
  - content/world/project.json
  - content/world/worlds/world.json
  - imports/canary/batches.json
  - imports/canary/bindings/creatures.json
  - imports/canary/sources.json
  - imports/crystalserver/batches.json
  - imports/crystalserver/bindings/creatures.json
  - imports/crystalserver/sources.json
  - apps/game-server/src/ai_monster_melee.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - apps/game-server/src/gameplay_transport/condition_snapshots.rs
  - apps/game-server/src/spell/mod.rs
  - apps/game-server/src/spell/source_player_conditions.rs
  - apps/game-server/src/spell/actor_conditions.rs
  - apps/game-server/src/canonical_monster_successor_tests.rs
  - apps/game-server/src/ability/
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/actor_conditions.rs
  - apps/game-server/src/gameplay_transport/actor_invisibility.rs
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/runtime_actor_conditions.rs
  - apps/game-server/src/foundation/runtime_actor_conditions_tests.rs
  - apps/game-server/src/movement/pacing.rs
  - apps/game-server/src/spell/plan.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/foundation/channel_owner_creature_bite_tests.rs
  - apps/game-server/src/foundation/runtime_actor_death.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/movement.rs
  - apps/game-server/src/movement/speed.rs
  - apps/game-server/src/creature_auto_attack.rs
  - apps/game-server/src/creature_damage_spell.rs
  - apps/game-server/src/creature_damage_composed.rs
  - apps/game-server/src/monster_owner_cycle.rs
  - apps/game-server/src/monster_combat_lane.rs
  - apps/game-server/src/monster_summon.rs
  - apps/game-server/src/creature_condition_content.rs
  - apps/game-server/src/condition_content_tests.rs
  - apps/game-server/src/source_critical_baseline.rs
  - apps/game-server/src/creature_attack_geometry.rs
  - docs/agents/evidence/monster-runtime-project-20261004/lanes/special/condition-fixtures.json
  - apps/game-server/src/creature_auto_attack_test_data.json
  - apps/game-server/src/source_threshold_heal.rs
  - apps/game-server/src/creature_chain_attack.rs
  - apps/game-server/src/creature_area_heal.rs
  - apps/game-server/src/self_heal_qualification.rs
  - apps/game-server/src/creature_defense_presentation.rs
  - apps/game-server/src/creature_defense_presentation_test_data.json
  - apps/game-server/src/crystal_death_composition.rs
  - apps/game-server/src/pinned-death-registry.json
  - apps/game-server/src/crystal_callbacks.rs
  - apps/game-server/src/smelly_cheese.rs
  - apps/game-server/src/smelly_damage.rs
  - apps/game-server/src/weak_spot_speech.rs
  - apps/game-server/src/welter_consume.rs
  - apps/game-server/src/bone_phase_registry.rs
  - apps/game-server/src/creature_damage_spell_delayed.rs
  - apps/game-server/src/creature_defense_variant_tests.rs
  - apps/game-server/src/source_threshold_heal_tests.rs
  - apps/game-server/src/creature_area_heal_tests.rs
  - apps/game-server/src/creature_damage_composed_tests.rs
  - apps/game-server/src/world_runtime.rs
  - content/abilities/definitions/abilities-06500-06999.json
  - content/abilities/definitions/abilities-07000-07391.json
  - content/abilities/definitions/index.json
  - content/abilities/effects/effects-02000-02499.json
  - content/abilities/effects/effects-05500-05796.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/formulas-05500-05999.json
  - content/abilities/formulas/formulas-06000-06125.json
  - content/abilities/formulas/index.json
  - content/behaviors/behaviors-00000-00499.json
  - content/behaviors/behaviors-00500-00999.json
  - content/behaviors/behaviors-01000-01499.json
  - content/behaviors/behaviors-02500-02972.json
  - content/behaviors/index.json
  - content/creatures/definitions/creatures-01500-01762.json
  - content/creatures/definitions/creatures-01500-01862.json
  - content/creatures/definitions/index.json
  - content/encounters/definitions/encounters-00000-00107.json
  - content/encounters/definitions/index.json
  - content/items/definitions/items-13500-13999.json
  - content/items/definitions/items-34000-34041.json
  - content/items/index.json
  - content/loot/index.json
  - content/loot/loot-00000-00499.json
  - content/loot/loot-01000-01289.json
  - content/presentations/definitions/index.json
  - content/presentations/definitions/presentations-02500-02972.json
  - content/world/definitions/declarations.json
  - content/world/definitions/reference.json
  - content/world/provenance/imports.json
  - content/world/provenance/sources.json
  - imports/crystalserver/creature-completion-evidence-20261004.json
  - content/behaviors/behaviors-02500-02872.json
  - content/presentations/definitions/presentations-02500-02872.json
  - content/abilities/definitions/abilities-07000-07059.json
  - content/abilities/formulas/formulas-05500-05822.json
  - content/abilities/effects/effects-05500-05506.json
  - content/loot/loot-01000-01253.json
  - content/items/definitions/items-34000-34031.json
  - content/encounters/definitions/encounters-00000-00103.json
  - apps/game-server/src/lib.rs
  - docs/agents/evidence/monster-full-mechanics-20261004/reconcile-main/
```

Użytkownik polecił kontynuować po audycie aktualnego main. Zabezpieczony poprzedni worktree `/workspace/monster-resource-plan` i źródła/cache nie są zastępowane. One writer root; subagenci zwracają guardowane pakiety. Bez PR/commitów/push/aktywacji serwera. Aktualny main ma już slot-owned conditions, lethal/death/respawn, kanoniczną szybkość i czyste kernels Attack. Portowane adaptery źródłowe muszą konsumować te istniejące ownery; nie utrzymywać równoległej HP/condition/target/timer authority. Poprzednie testy kwalifikują starszą bazę31ee0a, nie dowodzą integracji z tymmain.


2026-10-04 integration checkpoint:
- origin/main refreshed again at owner request: FETCH_HEAD=c4ccaefdbeb8e7fa3b274f8f16a0b0b5e98c8007; no newer commits, local work preserved.
- Conditions01-06, Foundation1/2/3a/3b/4/5, native HP/MP bridge, DoT fullpass/spatial adapter, native SPEED and actual Movement cadence integrated locally. Private source consumers imported for reconciliation; no transport/server activation.
- First current-base compile PASS. Exact initial source freeze and logs under reconcile-main/integration. Full initial run1805PASS/4FAIL/3ignored:2 missing tracked native-input cases materialized; native source lethal test position fixture repaired; native SPEED fixture target-vs-delta repaired. These fixes await current-source compile/test; no retroactive PASS claim.
- Current additional source imports compile work in progress; native aggregate and atomic condition/HP hooks have dedicated subagent proposals, not ready-to-import evidence.
- Real main defect discovered: player native conditions survive HP0/respawn. Assigned completion_wiki for existing-owner death-clear composition and failed-mint rollback proof, without an alternate death queue.
- SourceSpeed verified against already cached pinned Canary/Crystal C++ and importer: coefficients represent TARGET speed, native delta subtraction correct. Added source golden vectors; f32/RNG differences remain explicitly marked.
- Previous1515 tests qualify preserved old31ee worktree, not current c4cc integration. Final current-source qualification pending.

## Live main refresh 18:32 UTC

Fetched95c93340 (#1534), advanced local branch by fast-forward after preserving tracked work in local stash and54source-file SHA backups. Four conflicting owners await reviewed candidate integration. Previous1940+9 test pass qualifiesc4cc only. The32data proposal is stale in6index documents and must be rebased preserving spell_imports. No PR/push/server changes.

## Main95 reconciliation — current verification

Integrated against95c93340, preserving upstream spell imports, presentation metadata, native Player conditions and sole combat/death authorities. Source adapters consume existing main owners; private full source aggregate is not activated by a server transport hook. No PR, commit, push or server activation.

Canonical writer/re-admission/deterministic roundtrip PASS:1863Creature,108Encounter,34042Item,62456reference rows. Guarded165-file apply;9 obsolete tails archived and retired. Independent data audit PASS: zero unresolved references, prior accepted stats/mitigation/loot retained,34031 pre-existing Item rows unchanged, main95 spell manifest/index metadata retained. Evidence: reconcile-main/integration/main95-sealed-data-apply.json and lanes/data-three-way/final-main95-data-readonly-audit.json.

Final native static review:0P1/0P2; nine actual cached native capture tests PASS on main95. Production cargo check PASS after restoring unconditional EngineeringStaticCellIndex import. Full lib run2244PASS/3FAIL/17ignored identified one missing exact-HEAD provisioning input (restored) and two Bestiary corpus-budget failures. Guarded bounded Bestiary candidate applied:823 verified blocks,1863Creature, total source budget4900000 against measured4892414 bytes; per-document/depth/field/string guards unchanged. Final recompilation and full tests are in progress; previous counts are not a final PASS. Repository inventory integration test also remains to execute.

Final main95 library qualification:2248PASS/0FAIL/17ignored (`main95-final-all-tests.log`, frozen63 serverfiles). Productioncheck PASS and9realcachednativecapture PASS. Repositoryinventory first runtime:1PASS/2FAIL; exactHEAD missing treecontract/5markers restored, source-qualified portal25051 description must be checked as a narrow additional provenance cohort rather than violating older closed wiki-description subset. Follow-up guard in review; no claim of inventoryPASS yet.

Fresh main95 owner census corrected previous historical gap assumptions: appearance registry +temporary snapshots, current dynamicfield/item owners, NativeGameplay Creature compiler already exist. Production spell-native catalog actually contains1508Creature profiles, not only2 testfixture rows. Workers auditing/reusing current builders/owners for remaining source-specific entries; no substitute namespace/revision aliases or server activation.

## Main95 existing-owner follow-up ownership

Root solewriter; independent native_source_successor review0P1/0P2, exact reviewed guards. Callable preparation +qualified snapshots only, no server activation.
- apps/game-server/src/spell/native_items.rs
- apps/game-server/src/gameplay_transport/ordinary_field_items.rs
- apps/game-server/src/foundation/runtime_actor_companion.rs
- apps/game-server/src/content/native_spell_appearances.rs
- apps/game-server/src/gameplay_transport/condition_snapshots.rs

Root owns Prophet guarded source edits after independent0P1/0P2review; exactsource names/topstack, PROJECT_LOAD_SNAPSHOT +PROJECT_ATOMIC_SELF_PLUS_ALLIES explicit.
- apps/game-server/src/creature_area_heal.rs
- apps/game-server/src/creature_area_heal_tests.rs

Root owns maximal native profile union and local producerpin updates after independent datareviewPASS. Preserve1508 servingrows exact, append362canonicalmissing;1870 includes7servingfixtures. No stats/wiki1:1claim for743existing divergent actors.
- content/creatures/definitions/spell-native-profiles.json
- content/presentations/definitions/spell-native-profiles.json
- content/spells.manifest.json

Root owns40bounded cached47look records and native appearance lowerer; independent9opreviewPASS_STATIC0P1/0P2, existing268preserved. Localdefinition qualification only, no activation.
- apps/game-server/src/content/native_spell_appearances.rs
- content/presentations/bindings/spell-appearances.json
- content/spells.manifest.json
- content/manifest.json
- content/content.lock.json
- apps/game-server/src/creature_appearance_content.rs
- apps/game-server/src/creature_appearance_content_tests.rs
- apps/game-server/src/creature_appearance_test_data.json
- apps/game-server/src/lib.rs

## Guarded follow-up integrated; awaiting actual qualification

Root applied reviewed sourceItem selectors/Player-owned conversion, nativeexactkey variantpolicy, completeappearance selection reader, Prophet PROJECT load snapshot+atomic heal, fullnative1870profileunion, bounded40Canary47recordacceptance and data lowerer. All268 earlier appearance rows retained;308 total. SourceCreature definitionqualification110ops prepared,10sourceItem appearances requireexactnativeItem admission and remainexplicitrefusal. Metadata updated only corresponding SHA/count overlays inoutermanifest/lock/Creature+Presentation indices; canonical11docs unchanged.

Independent static reviews0P1/0P2, exactguarded packets+applyreceipts underintegration/main95-*-apply.json. Rustfmt changes no semantics. Final current-generation compile/library/nativefullmanifest tests pending; earlier2248+3+9PASS applies to preceding checkpoint only. No PR/commit/push/serveractivation.

Root final metadata repair ownership: content/presentations/bindings/index.json; only308appearance descriptorSHA/count, directorycontract/cues unchanged.

Root owns exactObject sourceflag compatibility and narrow Bestiary quota test repair, independentownerreview0P1/0P2.
- apps/game-server/src/content/native_gameplay.rs
- apps/game-server/src/foundation/runtime_actor_companion.rs
- apps/game-server/src/creature_appearance_content_tests.rs
- apps/game-server/src/content/project/bestiary.rs

## Zamknięcie odświeżenia i lokalnego scalenia main95 — VERIFIED

Aktualne dowody: `reconcile-main/integration/main95-final-validation-receipts.json` oraz `main95-final-source-freeze.json` (71serverfiles). Produkcyjny kod i dane ostatniej generacji:2263PASS/0FAIL/17ignored; actualfullnative manifest/compiler/carrier/readback1PASS; productioncheckPASS (241warnings, bez strictClippyPASSclaim); inventory3PASS;8cachedsourcecasesPASS oraz poprawionyactualArea census1PASS; affectedArea4PASS. Po pełnym2263PASS zmieniono wyłącznie ignored census test:19dawnych primitive+1jawnyPROJECTProphet; actualRED i poprawionyPASS zachowane. Production/data unchanged.

Canonical1863Creature/108Encounter/34042Item/62456reference. Native1870Creature+1870Presentation=1863canonical+7existingfixtures; wszystkie1508 wcześniejsze profile zachowane exact,362dodane.334appearance=268main+40cachedsourceTransformationtargets+26missingillusionableOutfits, każda nowa definicja ma exactLua/Git/headerproof;329Creaturelinks+5Avatar. NiezależnycurrentmetadataauditPASS; canonical11+129shards unchanged.

Zamknięty zakres: pobranie aktualnego main, zachowanie lokalnej pracy, ponowne użycie istniejących ownerów, lokalne scalenie danych/adapterów oraz faktyczna kwalifikacja importera. Cała mechanika runtime nadal nie jest oznaczona kompletna. Osobnyjawny ledger743aktorów/1156native-vs-canonicalstatfield differences; zachowane przyjęte servingvalues. SourceAppearance scheduledcast dispatch (110qualifiedCreaturedefinitions,10Itemmissingpolicy),82fieldentry physicaldurableCreatureItem dispatch,18Attack+15DefenseEncounter/quest writers oraz Icicle rawHP/AIclear pozostają dokładnie opisane. Lokalne #162: `docs/agents/evidence/monster-full-mechanics-20261004/decisions-162.md`. BezPR/commit/push/serveractivation.


## Wiki priority data reconciliation — qualified local successor

Explicit owner instruction: wiki decides stat conflicts. Six workers prepared1156 scoped input rows, root applied1139native field edits across735actors.607mitigation rows reconciled preserving579accepted non-Global estimates,27activeWiki values and1donorfallback; no estimate relabeledGlobal. Actual cachedproofs corrected MagmaBubble localeXP, DragonHoard wrongactorWiki mapping and Eliz4resistance cells;5healing response entries for4actors retained as typed data with runtimeexecutionUNCONFIRMED.17Wiki speeds use authorizedPROJECT base-speed mapping, without donor/globalunit equivalence.

Authoritative current receipt: `docs/agents/evidence/monster-wiki-priority-20261004/integration-applied.json`. Canonical11 writer+re-admission1PASS; nativefullartifact/compiler/carrier/readback1PASS; repositoryinventory3PASS. ActualRustvalidator caught23unsortednative resistance arrays; valuesunchanged sorting repair and GREEN evidence retained.16semanticdifferences remain UNKNOWNfield/identity confirmation and preserve current serving values;111neutral representations are semantically equivalent. Earlier743/1156pending-statcheckpoint is superseded by this scopedresult. Whole source mechanics/scheduledAppearance/fields/encountercallbacks/Icicle/audio/loot/respawn integrations remain outside this data completion result. Localbase95 retained; latestremoteacd700 fetched/read but not merged. NoPR/commit/push/serveractivation.
