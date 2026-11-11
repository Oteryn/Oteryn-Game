# QUESTPC run8 Viola checkpoint

HEAD: 1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7; local AUTHORING, no Git writes.

Source decision: currentWiki Newhaven Quest/Spoiler transcript requires mission/yes both accepting and returning Lost Wisdom. CurrentWiki book51977, scarf3572,800XP supersede donor44774 and vocationgear. Viola retired spellteaching15.22. Freshcaptures in .questpc/wiki-r8, donor lua fetched immutable201189f.

Ownfiles: durability/tutorial_viola.rs; dispatch/npc_command/viola.rs; dispatch/postgres_tests/viola.rs; migration1002; item_craft/audit.rs one visibility widening. Root mounted dispatcher and durablemodule hooks. Handoff uses explicit recipe `oteryn:npc-handoff.viola` in existing itemCraft auditowner, exact carried51977 debit and3572 mint, native XPowner800 in one physicaltransaction. Samexact receipt/recipe/source/output/stage proof SQL; replayreturns historicaloutcome only. Strict pending topic binds exactNPC context/session/itemfence/30s age. No arbitrary item reward trigger.

Compile: absolute cargo check -p oteryn-gameplay-dispatch --locked --offline --quiet slot5 completed output no ownerrors, vendorwarnings only; pipeline originally lackedpipefail; robust no-run compile pending.

Pending: root model/generation refresh; bookworker source_world()/target() for exact realbookUSE; complete mechanicPG incl refusal/restart/fences + clippy; record full exactcommands POSTGRES_CASE_COMPLETED durations. Test replaced fabricatedbookfixture with liveUSE helper integration (not yetcompiled), do not claim pathqualified.

## Source stable handoff

Robust compile proof: `QPC_SLOT=5 python3 .questpc/run_timed.py r8-viola-check-confirmed /home/mole/oteryn-cache/codex-bin/cargo check --locked --offline -p oteryn-gameplay-dispatch --quiet` exited0,172.724s (see resultJSON). Earlier clippy snapshot caught equipmenterror conversion, repaired explicit Rejected/Durability/Database handling and EquipmentAuthority success.

Full source test batch now consumes authored Viola placement32571,32507,rawz7/nativefloor-7; fixture actor relocation to real Book chest32614,32419 and back does not qualify the intervening walking path. `viola_mission_yes` selects2mechanicPG cases: plain book full source flow with deferredCOMMIT rejection and whole rollback; nestedbook fullbag refusal→freeentry→completion. Both cold-recreate ownerstate and re-read completion; stalelease, unofferedyes, no, missingbook, one-time completion and current retirement dialogue asserted. Book minted only through real command2 USE; nestedbag/coins are burden fixtures through existing mint/move owners.

Shared root owns central finalclippy/PG in slot6 to avoid simultaneous huge testlinkbuilds. Ownoldslot5 PG interrupted by explicit leadrequest with CtrlC/tool exit130, no POSTGRES_CASE_COMPLETED marker: NO qualification. Files SOURCE_STABLE; no more Rust/SQL writes until compiler/runtime finding.

Review: read whole own module/test/migration and checklist /home/mole/oteryn-cache/CODEX-CHECKLIST.md; enum/constructor/impl rg sweep across apps/,crates/,tools/ found only own new types' instances and productionhooks. SQL FK leadingindexes, bounded reads, exact craft source/target, current recovery/node/session/lease/equipment/revision fences, same-xact800XP, strict state successor and immutable receipts checked. Native xmin cast matches existing fee migrations.

Unfinished: central exact-tree clippy and the two actual mechanicPG executions with duration markers; no runtime completion claim yet.

## Central actual RED and repair

Central16case PG reached realPostgreSQL and rejected migration1002 before mechanicexecution: TRUNCATE cannot use FOR EACH ROW (centralPGredlog retained bylead). ReturnedAUTHORING for narrowbatch. Both receipt/state TRUNCATE guards now explicitly FOR EACH STATEMENT; regexsweep of every0969/1000..1003 CREATE TRIGGER found no remaining row-level TRUNCATE. Othermodules unchanged. Added required narrow too_many_arguments reason to commit_viola; no otherproductionlogic change. SOURCE_STABLE again; no ownCargo, leadrerunswarmcentral.

## Shared NPC book class repair

CentralPG established Father/scopednative programme hi→NoConversation because the generic servicebook excludes NPCs with no ordinary service. Viola is deliberately services[] under currentWiki retirement and has the same reachable productioncause. Under lead-authorized sharedbatch, NPCworker added closed `from_model_with_native_quests` and productionbundlecaller includes programme NPCs + exactViolakey; own sourceplacementfixture now passes only `["oteryn:npc.viola"]` to this constructor. No default servicebook broadening, content changes, fallback grant, or ownCargo. SourceStable after targetedfmt.

## Framed command-sequence fixture repair

Read-only fence/XP audit confirmed item-only exchange and equipment/capacity reads never advance CharacterRevision; nativeXP sees the slot's sameexpectedrevision then advances root/progression once inside samephysicaltx. Sourcefixture defect: directNPC dispatch does not populate SessionIngress, so firstactual framedBookUSE must becommand1, not100. Preserved everyingress/runtimeguard: leadgranted minimalfixturecorrection BookUSE1 and initialdirectHi900 (uniquecommandidentity). No productionchange/noCargo; SourceStable afterfmt. Bookworker notified of its separate reconnectcontinuity issue.

## Trigger function resolution hardening

Lead SQLreview identified missing pinned search_path/explicitfunctiongrants on new1002guards. Added exactbounded four-function DO matching0969/1003: game_viola_complete, game_viola_handoff_proven, game_viola_state_guard, game_viola_state_proven pin current_schema then pg_temp; REVOKE ALL FROM PUBLIC and EXECUTE TO runtime only. No SECURITY DEFINER, controlrole execution, tablegrant expansion or gameplaylogic changes. SourceStable; leadownsPG/review.

## First mission SQL decode repair

Actual closedbook-repairedPG reached Viola mission11 then Rejected beforewrites. Leadtraced stage query: COALESCE(SMALLINT, integer0) resolvesINT4 but sqlxexpectedi16. Cast default0::smallint to retainexactstoredstage type. Swept every ownquery_scalar/query_as and try_get: action/reply directSMALLINT→i16; UUIDsend→Vecu8; UUID/text ordinals/max→String; count/biginttimestamp→i64; to_jsonb→Value allmatch. No genericdecodercoercion or testweakening. LeadownsactualPG verification, no ownCargo.

## Deferred rollback reach proof

Lead requested a concrete trigger-hit witness because an unrelated early Unknown must not qualify rollback. The handoff failure trigger now advances a fixture sequence immediately before raising; sequence state survives rollback, and the test asserts is_called before checking unchanged book/scarf/XP/stage/receipts. Added the same two-second semantic clock advance as the established NPC talk helper before direct failure and reconciliation calls. No production gate bypass or weaker assertion. Narrow test-only DDL grants sequence usage/select to runtime, then drops trigger/function/sequence after the proof. Targeted rustfmt passed; no own Cargo.
