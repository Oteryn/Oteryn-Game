//! M37 free source teleport receipts never fabricate quest progress or paid travel.
use crate::bestiary_postgres_harness as f;

#[test]
fn m37_npc_teleport_postgres_receipt_requires_atomic_arrival() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc8_npc_teleport", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M37_POSTGRES_ATOMIC_SOURCE_ARRIVAL_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M37 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('game_quest_npc_teleport_receipts') IS NOT NULL")
            .fetch_one(&h.pool)
            .await?;
    assert!(
        exists,
        "M37 needs a response receipt independent of quest mutation"
    );
    let receipt = "INSERT INTO game_quest_npc_teleport_receipts(character_id,game_session_id,command_id,transition_key,request_binding,program_digest,occurrence_id,world_id,tile_x,tile_y,tile_floor,reply,next_topic,end_conversation,end_delay_ms,quest_changed) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,9701,'oteryn:quest-npc/in_service_of_yalahar/peter/direct-factory',$3,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,32859,31302,-7,ARRAY['Go ahead.'],0,false,0,false)";
    let arrival = "INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor,source_npc_teleport) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,32859,31302,-7,true)";
    for (write_receipt, write_arrival) in [(true, false), (false, true), (true, true)] {
        let mut tx = h.pool.begin().await?;
        if write_receipt {
            sqlx::query(receipt)
                .bind(f::id(f::CHARACTER).as_slice())
                .bind(f::id(f::SESSION).as_slice())
                .bind([1u8; 33].as_slice())
                .bind([2u8; 32].as_slice())
                .bind(f::id(97).as_slice())
                .bind(f::id(f::WORLD).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        if write_arrival {
            sqlx::query(arrival)
                .bind(f::id(f::CHARACTER).as_slice())
                .bind(f::id(97).as_slice())
                .bind(f::id(f::WORLD).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        let committed = tx.commit().await;
        if write_receipt && write_arrival {
            committed?;
        } else {
            let error = committed
                .err()
                .ok_or("unpaired NPC teleport was accepted")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
        }
    }
    for query in [
        "SELECT count(*) FROM game_character_quest_receipts",
        "SELECT count(*) FROM game_character_quest_states",
        "SELECT count(*) FROM game_character_quest_tracks",
        "SELECT count(*) FROM game_newhaven_travel_receipts",
    ] {
        let count: i64 = sqlx::query_scalar(query).fetch_one(&h.pool).await?;
        assert_eq!(
            count, 0,
            "free teleport must not fabricate value or progress"
        );
    }
    assert_eq!(h.root_revision().await?, "1");
    sqlx::query("UPDATE game_character_pending_arrivals SET occurrence_id=occurrence_id")
        .execute(&h.pool)
        .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_character_pending_arrivals SET tile_x=1 WHERE character_id=encode($1,'hex')::uuid").bind(f::id(f::CHARACTER).as_slice()).execute(&mut *tx).await?;
    assert_eq!(
        tx.commit()
            .await
            .err()
            .ok_or("source arrival retargeted")?
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("23514")
    );
    sqlx::query("DELETE FROM game_character_pending_arrivals")
        .execute(&h.pool)
        .await?;
    sqlx::query("DELETE FROM game_quest_npc_teleport_receipts")
        .execute(&h.pool)
        .await?;
    exercise_peter(h).await?;
    Ok(())
}

struct NoItems;
impl crate::durability::item_transfer::ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Err(crate::durability::item_transfer::ItemWeightRefused::Unknown)
    }
}
impl crate::durability::item_loop_move::ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        None
    }
}

async fn exercise_peter(h: &f::Harness) -> f::TestResult {
    use crate::durability::character_last_position::LastPositionCell;
    use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
    use crate::durability::quest_state::npc_program::{
        NpcProgramRequest,
        teleport::{self, NpcTeleportAdmission},
    };
    use crate::durability::spell_owner_commit::SpellLanePermit;
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
        npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
    };
    use std::sync::Arc;
    let source = NpcRuleCatalog::runtime().map_err(f::debug)?;
    let program = source
        .programs()
        .iter()
        .find(|p| p.document.quest == "oteryn:quest.in_service_of_yalahar_quest")
        .ok_or("Yalahar source programme")?;
    let tracks: Vec<_> = program
        .document
        .tracks
        .iter()
        .map(|t| QuestTrack {
            key: t.key.clone(),
            quest: t.quest.clone(),
            initial: t.initial,
            min: t.min,
            max: t.max,
        })
        .collect();
    let first = tracks.first().ok_or("Yalahar first track")?;
    let digest = program
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let pin = QuestTransition {
        key: format!("oteryn:quest-npc-pin/{digest}"),
        quest: program.document.quest.clone(),
        effects: vec![QuestEffect {
            track: first.key.clone(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Computed,
        }],
        completes: false,
        experience: None,
        world_change: None,
    };
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, vec![pin]).map_err(f::debug)?);
    let request = NpcProgramRequest {
        command: command(9702)?,
        npc: "oteryn:npc.peter".into(),
        trigger: NpcRuleTrigger::Say,
        message: "factory".into(),
        topic: 0,
        transition_key: "oteryn:quest-npc/in_service_of_yalahar_quest/peter/direct-factory".into(),
    };
    let preview = teleport::preview_teleport(
        &request,
        &Default::default(),
        &catalogue,
        1_700_000_000,
        &Default::default(),
    )
    .map_err(f::debug)?
    .ok_or("Peter teleport preview")?;
    let mut request = request;
    request.transition_key = preview
        .source
        .selected_rule_keys
        .first()
        .ok_or("Peter branch")?
        .clone();
    assert!(preview.source.track_changes.is_empty());
    let fence = f::fence(1)?;
    let arrival = PendingArrival {
        occurrence: TravelOccurrence::from_bytes(f::id(98)).map_err(f::debug)?,
        world_id: fence.runtime_scope.world_id(),
        destination: LastPositionCell {
            x: 32859,
            y: 31302,
            floor: -7,
        },
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                stale,
                request.clone(),
                catalogue.clone(),
                &NoItems,
                NpcTeleportAdmission { arrival }
            )
            .await
            .is_err()
    );
    let mut wrong = arrival;
    wrong.destination.x += 1;
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                fence,
                request.clone(),
                catalogue.clone(),
                &NoItems,
                NpcTeleportAdmission { arrival: wrong }
            )
            .await
            .is_err()
    );
    let result = h
        .root
        .commit_character_npc_teleport(
            &authority,
            &h.node,
            &permit,
            fence,
            request.clone(),
            catalogue.clone(),
            &NoItems,
            NpcTeleportAdmission { arrival },
        )
        .await
        .map_err(f::debug)?;
    assert!(result.read_only);
    assert_eq!(result.reply, preview.source.reply);
    assert_eq!(h.root_revision().await?, "1");
    let pending = h
        .root
        .read_npc_pending_arrival(&authority, &h.node, fence)
        .await
        .map_err(f::debug)?;
    assert_eq!(pending, Some(arrival));
    let replay = h
        .root
        .commit_character_npc_teleport(
            &authority,
            &h.node,
            &permit,
            fence,
            request.clone(),
            catalogue.clone(),
            &NoItems,
            NpcTeleportAdmission { arrival },
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(replay.reply, result.reply);
    let mut changed = request.clone();
    changed.message = "trade".into();
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                fence,
                changed,
                catalogue,
                &NoItems,
                NpcTeleportAdmission { arrival }
            )
            .await
            .is_err()
    );
    let mut cross_branch = request.clone();
    cross_branch.message = "trade".into();
    cross_branch.transition_key =
        "oteryn:quest-npc/in_service_of_yalahar_quest/peter/direct-trade".into();
    assert!(matches!(
        h.root.reconcile_npc_program(&authority, fence.character_id, cross_branch).await,
        Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)
    ), "one NPC command cannot reconcile as a different source branch");
    let reconciled = h
        .root
        .reconcile_npc_program(&authority, fence.character_id, request)
        .await
        .map_err(f::debug)?
        .ok_or("durable free response")?;
    assert_eq!(reconciled.reply, result.reply);
    for query in [
        "SELECT count(*) FROM game_character_quest_receipts",
        "SELECT count(*) FROM game_character_quest_states",
        "SELECT count(*) FROM game_character_quest_tracks",
        "SELECT count(*) FROM game_newhaven_travel_receipts",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&h.pool)
                .await?,
            0
        );
    }
    Ok(())
}

fn command(ordinal: u64) -> f::TestResult<crate::foundation::CommandRef> {
    use crate::foundation::{CommandId, CommandRef, GameSessionId};
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(ordinal).map_err(f::debug)?,
    ))
}
//! M37 free source teleport receipts never fabricate quest progress or paid travel.
use crate::bestiary_postgres_harness as f;

#[test]
fn m37_npc_teleport_postgres_receipt_requires_atomic_arrival() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc8_npc_teleport", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M37_POSTGRES_ATOMIC_SOURCE_ARRIVAL_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M37 PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('game_quest_npc_teleport_receipts') IS NOT NULL")
            .fetch_one(&h.pool)
            .await?;
    assert!(
        exists,
        "M37 needs a response receipt independent of quest mutation"
    );
    let receipt = "INSERT INTO game_quest_npc_teleport_receipts(character_id,game_session_id,command_id,transition_key,request_binding,program_digest,occurrence_id,world_id,tile_x,tile_y,tile_floor,reply,next_topic,end_conversation,end_delay_ms,quest_changed) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,9701,'oteryn:quest-npc/in_service_of_yalahar/peter/direct-factory',$3,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,32859,31302,-7,ARRAY['Go ahead.'],0,false,0,false)";
    let arrival = "INSERT INTO game_character_pending_arrivals(character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor,source_npc_teleport) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,32859,31302,-7,true)";
    for (write_receipt, write_arrival) in [(true, false), (false, true), (true, true)] {
        let mut tx = h.pool.begin().await?;
        if write_receipt {
            sqlx::query(receipt)
                .bind(f::id(f::CHARACTER).as_slice())
                .bind(f::id(f::SESSION).as_slice())
                .bind([1u8; 33].as_slice())
                .bind([2u8; 32].as_slice())
                .bind(f::id(97).as_slice())
                .bind(f::id(f::WORLD).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        if write_arrival {
            sqlx::query(arrival)
                .bind(f::id(f::CHARACTER).as_slice())
                .bind(f::id(97).as_slice())
                .bind(f::id(f::WORLD).as_slice())
                .execute(&mut *tx)
                .await?;
        }
        let committed = tx.commit().await;
        if write_receipt && write_arrival {
            committed?;
        } else {
            let error = committed
                .err()
                .ok_or("unpaired NPC teleport was accepted")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
        }
    }
    for query in [
        "SELECT count(*) FROM game_character_quest_receipts",
        "SELECT count(*) FROM game_character_quest_states",
        "SELECT count(*) FROM game_character_quest_tracks",
        "SELECT count(*) FROM game_newhaven_travel_receipts",
    ] {
        let count: i64 = sqlx::query_scalar(query).fetch_one(&h.pool).await?;
        assert_eq!(
            count, 0,
            "free teleport must not fabricate value or progress"
        );
    }
    assert_eq!(h.root_revision().await?, "1");
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let recovered = h
        .root
        .read_npc_pending_arrival(&authority, &h.node, f::fence(1)?)
        .await
        .map_err(f::debug)?
        .ok_or("legacy source arrival")?;
    assert_eq!(
        recovered.destination.floor, 7,
        "v1 source receipt recovers in movement coordinates"
    );
    sqlx::query("UPDATE game_character_pending_arrivals SET occurrence_id=occurrence_id")
        .execute(&h.pool)
        .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_character_pending_arrivals SET tile_x=1 WHERE character_id=encode($1,'hex')::uuid").bind(f::id(f::CHARACTER).as_slice()).execute(&mut *tx).await?;
    assert_eq!(
        tx.commit()
            .await
            .err()
            .ok_or("source arrival retargeted")?
            .as_database_error()
            .and_then(|e| e.code())
            .as_deref(),
        Some("23514")
    );
    sqlx::query("DELETE FROM game_character_pending_arrivals")
        .execute(&h.pool)
        .await?;
    sqlx::query("DELETE FROM game_quest_npc_teleport_receipts")
        .execute(&h.pool)
        .await?;
    exercise_peter(h).await?;
    Ok(())
}

struct NoItems;
impl crate::durability::item_transfer::ItemWeightLookup for NoItems {
    fn generation_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn weight_of(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        Err(crate::durability::item_transfer::ItemWeightRefused::Unknown)
    }
}
impl crate::durability::item_loop_move::ItemLoopContent for NoItems {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn loop_facts(
        &self,
        _: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Option<crate::durability::item_transfer::ItemDefinitionFacts> {
        None
    }
}

async fn exercise_peter(h: &f::Harness) -> f::TestResult {
    use crate::durability::character_last_position::LastPositionCell;
    use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
    use crate::durability::quest_state::npc_program::{
        NpcProgramRequest,
        teleport::{self, NpcTeleportAdmission},
    };
    use crate::durability::spell_owner_commit::SpellLanePermit;
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
        npc_rules::{NpcRuleCatalog, NpcRuleTrigger},
    };
    use std::sync::Arc;
    let source = NpcRuleCatalog::runtime().map_err(f::debug)?;
    let program = source
        .programs()
        .iter()
        .find(|p| p.document.quest == "oteryn:quest.in_service_of_yalahar_quest")
        .ok_or("Yalahar source programme")?;
    let tracks: Vec<_> = program
        .document
        .tracks
        .iter()
        .map(|t| QuestTrack {
            key: t.key.clone(),
            quest: t.quest.clone(),
            initial: t.initial,
            min: t.min,
            max: t.max,
        })
        .collect();
    let first = tracks.first().ok_or("Yalahar first track")?;
    let digest = program
        .digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let pin = QuestTransition {
        key: format!("oteryn:quest-npc-pin/{digest}"),
        quest: program.document.quest.clone(),
        effects: vec![QuestEffect {
            track: first.key.clone(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Computed,
        }],
        completes: false,
        experience: None,
        world_change: None,
    };
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, vec![pin]).map_err(f::debug)?);
    let request = NpcProgramRequest {
        command: command(9702)?,
        npc: "oteryn:npc.peter".into(),
        trigger: NpcRuleTrigger::Say,
        message: "factory".into(),
        topic: 0,
        transition_key: "oteryn:quest-npc/in_service_of_yalahar_quest/peter/direct-factory".into(),
    };
    let preview = teleport::preview_teleport(
        &request,
        &Default::default(),
        &catalogue,
        1_700_000_000,
        &Default::default(),
    )
    .map_err(f::debug)?
    .ok_or("Peter teleport preview")?;
    let mut request = request;
    request.transition_key = preview
        .source
        .selected_rule_keys
        .first()
        .ok_or("Peter branch")?
        .clone();
    assert!(preview.source.track_changes.is_empty());
    let fence = f::fence(1)?;
    let arrival = PendingArrival {
        occurrence: TravelOccurrence::from_bytes(f::id(98)).map_err(f::debug)?,
        world_id: fence.runtime_scope.world_id(),
        destination: LastPositionCell {
            x: 32859,
            y: 31302,
            floor: 7,
        },
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope)
        .await
        .map_err(f::debug)?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                stale,
                request.clone(),
                catalogue.clone(),
                &NoItems,
                NpcTeleportAdmission { arrival }
            )
            .await
            .is_err()
    );
    let mut wrong = arrival;
    wrong.destination.x += 1;
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                fence,
                request.clone(),
                catalogue.clone(),
                &NoItems,
                NpcTeleportAdmission { arrival: wrong }
            )
            .await
            .is_err()
    );
    let result = h
        .root
        .commit_character_npc_teleport(
            &authority,
            &h.node,
            &permit,
            fence,
            request.clone(),
            catalogue.clone(),
            &NoItems,
            NpcTeleportAdmission { arrival },
        )
        .await
        .map_err(f::debug)?;
    assert!(result.read_only);
    assert_eq!(result.reply, preview.source.reply);
    assert_eq!(h.root_revision().await?, "1");
    let pending = h
        .root
        .read_npc_pending_arrival(&authority, &h.node, fence)
        .await
        .map_err(f::debug)?;
    assert_eq!(pending, Some(arrival));
    let runtime_frame: bool = sqlx::query_scalar("SELECT runtime_coordinates FROM game_quest_npc_teleport_receipts WHERE character_id=encode($1,'hex')::uuid AND command_id=9702").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(runtime_frame, "new commands persist movement coordinates");
    let replay = h
        .root
        .commit_character_npc_teleport(
            &authority,
            &h.node,
            &permit,
            fence,
            request.clone(),
            catalogue.clone(),
            &NoItems,
            NpcTeleportAdmission { arrival },
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(replay.reply, result.reply);
    let mut changed = request.clone();
    changed.message = "trade".into();
    assert!(
        h.root
            .commit_character_npc_teleport(
                &authority,
                &h.node,
                &permit,
                fence,
                changed,
                catalogue,
                &NoItems,
                NpcTeleportAdmission { arrival }
            )
            .await
            .is_err()
    );
    let mut cross_branch = request.clone();
    cross_branch.message = "trade".into();
    cross_branch.transition_key =
        "oteryn:quest-npc/in_service_of_yalahar_quest/peter/direct-trade".into();
    assert!(matches!(
        h.root.reconcile_npc_program(&authority, fence.character_id, cross_branch).await,
        Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence)
    ), "one NPC command cannot reconcile as a different source branch");
    let reconciled = h
        .root
        .reconcile_npc_program(&authority, fence.character_id, request)
        .await
        .map_err(f::debug)?
        .ok_or("durable free response")?;
    assert_eq!(reconciled.reply, result.reply);
    for query in [
        "SELECT count(*) FROM game_character_quest_receipts",
        "SELECT count(*) FROM game_character_quest_states",
        "SELECT count(*) FROM game_character_quest_tracks",
        "SELECT count(*) FROM game_newhaven_travel_receipts",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(query)
                .fetch_one(&h.pool)
                .await?,
            0
        );
    }
    Ok(())
}

fn command(ordinal: u64) -> f::TestResult<crate::foundation::CommandRef> {
    use crate::foundation::{CommandId, CommandRef, GameSessionId};
    Ok(CommandRef::new(
        GameSessionId::decode(&f::id(f::SESSION)).map_err(f::debug)?,
        CommandId::new(ordinal).map_err(f::debug)?,
    ))
}
