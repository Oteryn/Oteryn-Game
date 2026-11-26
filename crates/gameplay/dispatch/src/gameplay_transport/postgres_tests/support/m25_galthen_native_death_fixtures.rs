use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};

use crate::foundation::{CommandId, CommandRef};

use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};

use std::sync::Arc;

pub(super) const TRACK: &str = "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/";

const QUEST: &str = "oteryn:quest.adventures_of_galthen_quest";

async fn value(h: &Harness, stage: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(format!("{TRACK}{stage}"))
    .fetch_one(&h.pool)
    .await?)
}

async fn death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    command: u64,
) -> TestResult {
    let target =
        super::qengine_kill_stories_fixtures::death_actor(owner, admitted, creature, command).await?;
    retire_death_actor(owner, target).await
}

async fn retire_death_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    target: crate::foundation::ExactActorRef,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .companion_snapshot_including_dead(target)
            .map_err(debug)?
            .health,
        0
    );
    // The fixture drives lethal settlement without the AI lifecycle loop.
    // Retire its exact dead generation after both durable drains; corpse and
    // reward occurrences intentionally outlive the actor slot.
    runtime.remove_arena_opponent(target).map_err(debug)?;
    Ok(())
}

async fn predecessor(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    stage: &str,
    ordinal: u64,
) -> TestResult {
    let canonical = owner
        .quest_catalogue
        .as_ref()
        .ok_or("canonical Galthen catalogue")?;
    let key = format!("oteryn:quest-transition/m25-fixture/galthen-{stage}");
    let mut transitions = ["s5", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .transition(&format!(
                    "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}"
                ))
                .cloned()
                .ok_or("canonical M25 kill transition")
        })
        .collect::<Result<Vec<_>, _>>()?;
    for gate in ["s4", "s7"] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/m25-fixture/galthen-{gate}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: format!("{TRACK}{gate}"),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
    }
    let tracks = ["s4", "s5", "s7", "s8"]
        .into_iter()
        .map(|stage| {
            canonical
                .track(&format!("{TRACK}{stage}"))
                .cloned()
                .ok_or("selected authored M25 track")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let catalogue = Arc::new(
        QuestStateCatalogue::new(canonical.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "actual current Quest fence")?
        .ok_or("actual Quest fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(ordinal).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    // The shared quest scenario replaces the admitted runtime. Rebind only
    // its current durable Character lease before exercising native combat.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current M25 combat fence")?
        .ok_or("current M25 combat fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("M25 combat actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mitmah = "oteryn:creature.mitmah_vanguard";
    if name.ends_with("-xp") || name.ends_with("-departed") {
        return assert_mitmah_award_after_reward_settlement(
            owner,
            admitted,
            h,
            mitmah,
            name.ends_with("-xp"),
        )
        .await;
    }
    if name.ends_with("-takeover") || name.ends_with("-idle") {
        return assert_mitmah_reward_grant_boundary(
            owner,
            admitted,
            h,
            mitmah,
            60_010,
            Some(name.ends_with("-idle")),
        )
        .await;
    }
    for (index, (creature, gate, stage)) in [
        ("oteryn:creature.ahau", "s4", "s5"),
        ("oteryn:creature.mitmah_vanguard", "s7", "s8"),
    ]
    .into_iter()
    .enumerate()
    {
        let command = 60000 + u64::try_from(index)? * 10;
        assert_eq!(value(h, gate).await?, 0);
        if stage == "s8" {
            assert_mitmah_reward_grant_boundary(owner, admitted, h, creature, command, None)
                .await?;
        } else {
            death(owner, admitted, creature, command).await?;
        }
        super::m25_corpse_chain::assert_native_corpse_chain(&h.pool, creature).await?;
        assert_eq!(
            value(h, stage).await?,
            0,
            "actual death without the exact authored predecessor never counts"
        );
        predecessor(owner, admitted, gate, command + 1).await?;
        assert_eq!(value(h, gate).await?, 1);
        eprintln!(
            "M25_REAL_NATIVE_DEATH_REACHED creature={creature} predecessor={gate} target={stage}"
        );
        death(owner, admitted, creature, command + 2).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "real native death progresses its selected authored stage once"
        );
        if stage == "s8" {
            // Emulate a pre-achievement character with the completed, retained s8 receipt.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            for delete in [
                "DELETE FROM game_account_achievements WHERE achievement_key='oteryn:achievement/museum_goer'",
                "DELETE FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/museum_goer'",
            ] {
                let removed = sqlx::query(delete)
                    .execute(&mut *tx).await?;
                assert_eq!(removed.rows_affected(), 1);
            }
            tx.commit().await?;
        }
        death(owner, admitted, creature, command + 3).await?;
        assert_eq!(
            value(h, stage).await?,
            1,
            "duplicate projection/replay and another death cannot exceed the stage bound"
        );
        let completed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)").bind(QUEST).fetch_one(&h.pool).await?;
        assert!(
            !completed,
            "native boss death never completes the quest or performs the final report"
        );
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1 AND NOT completes").bind(format!("oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}")).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, 1,
            "one actual death owns one non-completing stage receipt"
        );
        let museum_goer: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievement_grant_requests
             WHERE achievement_key='oteryn:achievement/museum_goer'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(museum_goer, i64::from(stage == "s8"));
    }
    Ok(())
}

async fn assert_mitmah_reward_grant_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    creature: &str,
    command: u64,
    takeover: Option<bool>,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;

    let rewards = async || -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_party_kill_xp_receipts WHERE creature_key=$1),
                    (SELECT count(*) FROM game_item_mint_receipts
                     WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL)",
        )
        .bind(creature)
        .fetch_one(&h.pool)
        .await?)
    };
    let before = rewards().await?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_mitmah_corpse() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             IF NEW.loot_purpose_key = 'CORPSE_MATERIALIZATION' THEN
                 RAISE EXCEPTION 'injected Mitmah corpse settlement failure';
             END IF;
             RETURN NEW;
         END $$;
         CREATE TRIGGER reject_mitmah_corpse BEFORE INSERT ON game_item_mint_receipts
             FOR EACH ROW EXECUTE FUNCTION reject_mitmah_corpse()",
    )
    .execute(&h.pool)
    .await?;
    let target =
        super::qengine_kill_stories_fixtures::queue_death(owner, admitted, creature, command).await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Deferred
    );
    let awards: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements
         WHERE achievement_key='oteryn:achievement/museum_goer'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rewards().await?.1, before.1);
    assert_eq!(awards, 0, "failed corpse settlement left an orphan boss achievement");
    sqlx::query("DROP TRIGGER reject_mitmah_corpse ON game_item_mint_receipts")
        .execute(&h.pool)
        .await?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_mitmah_award() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             IF NEW.achievement_key = 'oteryn:achievement/museum_goer' THEN
                 RAISE EXCEPTION 'injected Mitmah achievement failure';
             END IF;
             RETURN NEW;
         END $$;
         CREATE TRIGGER reject_mitmah_award BEFORE INSERT ON game_account_achievement_grant_requests
             FOR EACH ROW EXECUTE FUNCTION reject_mitmah_award()",
    )
    .execute(&h.pool)
    .await?;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Deferred
    );
    let settled = rewards().await?;
    assert_eq!(settled.1, before.1 + 1, "corpse commits before the grant attempt");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/museum_goer'",
        ).fetch_one(&h.pool).await?,
        0,
        "the interrupted grant is invisible"
    );
    sqlx::query("DROP TRIGGER reject_mitmah_award ON game_account_achievement_grant_requests")
        .execute(&h.pool)
        .await?;
    if let Some(idle) = takeover {
        return assert_mitmah_award_after_takeover(owner, admitted, h, idle).await;
    }
    // Discard the process-local kill queue before retrying under the same generation.
    *owner.states.attack.lock().await = Default::default();
    super::qengine_kill_stories_fixtures::drain_deaths(owner, admitted, &[]).await?;
    assert_eq!(rewards().await?, settled, "grant retry cannot duplicate death rewards");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/museum_goer'",
        ).fetch_one(&h.pool).await?,
        1
    );
    retire_death_actor(owner, target).await
}

async fn assert_mitmah_award_after_takeover(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    idle: bool,
) -> TestResult {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
        RuntimeScopeAssignmentWriter,
    };
    use crate::foundation::ScopeOwnershipGeneration;

    let fence = owner.current_quest_fence(admitted.game_session_id)
        .await.map_err(debug)?.ok_or("Mitmah earning fence")?;
    let scope = fence.runtime_scope;
    let old_generation = fence.scope_ownership_generation;
    let buyer = super::fixture::id(104);
    let seller: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    // Apply the ownership write of a completed sale while the award is retained.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(buyer.as_slice()).execute(&mut *tx).await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    assert_eq!(sqlx::query(
        "UPDATE game_character_roots SET account_id=encode($1,'hex')::uuid
         WHERE character_id=encode($2,'hex')::uuid",
    ).bind(buyer.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .execute(&mut *tx).await?.rows_affected(), 1);
    tx.commit().await?;

    sqlx::query(
        "INSERT INTO game_control_scope_grants VALUES
         (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 2)",
    ).bind(owner.world_id.as_bytes().as_slice())
        .bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let predecessor = h.root.read_runtime_scope_predecessor(scope)
        .await.map_err(debug)?.ok_or("Mitmah predecessor")?;
    let successor = super::fixture::registered_node(&h.root, 2).await?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await.map_err(debug)?;
    let AssignmentOutcome::Committed(moved) = writer.submit(&AssignmentRequest {
        operation_key: OperationKey::from_bytes([204; 32]),
        actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
        command: AssignmentCommand::Replace { scope, predecessor, target: successor.fact() },
    }).await.map_err(debug)? else {
        return Err("Mitmah scope replacement refused".into());
    };
    let generation = ScopeOwnershipGeneration::new(moved.assignment.ownership_generation)
        .map_err(debug)?;
    assert_eq!(generation.get(), 2);
    super::source_boss_handoff_fixtures::publish_source_boss_successor_readiness(
        h, &successor, &moved,
    ).await?;
    assert!(h.root.reconcile_boss_death_achievement(
        owner.character, owner.holder, scope, old_generation,
    ).await.is_err());
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_death_achievement_pending")
        .fetch_one(&h.pool).await?, 1, "the old owner must leave the pending row intact");
    if idle {
        let content = owner.runtime.lock().await.content_pin().clone();
        let fact = successor.fact();
        let runtime = crate::foundation::ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            fact.node_id(),
            fact.registration_revision(),
            generation.get(),
            moved.assignment.source_revision,
            &moved.assignment.decision_identity,
            2,
            content,
        )
        .map_err(debug)?;
        assert!(runtime.positioned_actor_census().map_err(debug)?.is_empty());
        serve_idle_successor(owner, h, &successor, tokio::sync::Mutex::new(runtime)).await?;
    } else {
    assert!(!h.root.reconcile_boss_death_achievement(
        owner.character, &successor, scope, generation,
    ).await?);
    assert!(h.root.reconcile_boss_death_achievement(
        owner.character, &successor, scope, generation,
    ).await?);
    }
    assert_eq!(sqlx::query_scalar::<_, String>(
        "SELECT account_id::text FROM game_account_achievements
         WHERE achievement_key='oteryn:achievement/museum_goer'",
    ).fetch_all(&h.pool).await?, [seller]);
    Ok(())
}

async fn assert_mitmah_award_after_reward_settlement(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    creature: &str,
    xp_failure: bool,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::DrainOutcome;

    if xp_failure {
        sqlx::raw_sql(
            "CREATE FUNCTION reject_mitmah_xp() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
                 RAISE EXCEPTION 'injected Mitmah XP failure';
             END $$;
             CREATE TRIGGER reject_mitmah_xp BEFORE INSERT ON game_party_kill_xp_receipts
                 FOR EACH ROW EXECUTE FUNCTION reject_mitmah_xp()",
        )
        .execute(&h.pool)
        .await?;
    }
    let departed = if xp_failure {
        None
    } else {
        h.seed_pvp_target().await?;
        super::appearance_change_postgres_fixtures::observer_visibility_tests::admit(h).await?;
        let session =
            crate::foundation::GameSessionId::decode(&super::fixture::id(110)).map_err(debug)?;
        Some(
            owner
                .current_quest_fence(session)
                .await
                .map_err(debug)?
                .ok_or("departing top-damager fence")?,
        )
    };
    let target = super::qengine_kill_stories_fixtures::queue_death_with_departure(
        owner, admitted, creature, 60_010, departed,
    )
    .await?;
    if xp_failure {
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Deferred
        );
        assert_eq!(museum_grant_count(h).await?, 0);
        sqlx::query("DROP TRIGGER reject_mitmah_xp ON game_party_kill_xp_receipts")
            .execute(&h.pool)
            .await?;
    }
    super::qengine_kill_stories_fixtures::drain_deaths(owner, admitted, &[]).await?;
    assert_eq!(
        museum_grant_count(h).await?,
        1,
        "settled Mitmah death grants the remaining contributor"
    );
    let reward_counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_party_kill_xp_receipts WHERE creature_key=$1),
                (SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION')",
    ).bind(creature).fetch_one(&h.pool).await?;
    assert_eq!(
        reward_counts,
        (i64::from(xp_failure), i64::from(xp_failure))
    );
    retire_death_actor(owner, target).await
}

async fn museum_grant_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievement_grant_requests
         WHERE achievement_key='oteryn:achievement/museum_goer'",
    )
    .fetch_one(&h.pool)
    .await?)
}

async fn serve_idle_successor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    successor: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    runtime: tokio::sync::Mutex<crate::foundation::ChannelRuntimeV1>,
) -> TestResult {
    use crate::gameplay_transport::{
        GameplayListenerConfig, GameplaySeamOwners, composed_test_owner::empty_test_rooms,
        quest_log::QuestLogLines,
        serve_gameplay, source_item_cycle::SOURCE_OWNER_SWEEP_COMPLETE,
    };
    use std::time::Duration;

    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let tls = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let quest_log = QuestLogLines::new(journal.content).ok_or("quest log line bounds")?;
    let stop = oteryn_foundation::CancellationToken::new();
    let swept = Arc::new(tokio::sync::Notify::new());
    let mut serving = Box::pin(
        SOURCE_OWNER_SWEEP_COMPLETE.scope(
            Arc::clone(&swept),
            serve_gameplay(
                &listener,
                GameplayListenerConfig {
                    bulletin_source: None,
                    server_motd: String::new(),
                    certificates: vec![tls.cert.der().clone()],
                    private_key: rustls::pki_types::PrivatePkcs8KeyDer::from(
                        tls.signing_key.serialize_der(),
                    )
                    .into(),
                    max_connections: 1,
                    max_handshake_units: 1,
                    entry_deadline: Duration::from_secs(1),
                },
                GameplaySeamOwners {
                    platform_enforcement_enabled: false,
                    cast: None,
                    activity_templates: &[],
                    soulpit_template: None,
                    gameplay_idle_policy: Default::default(),
                    store_purchases: Default::default(),
                    store: None,
                    emit: owner.emit,
                    root: owner.root,
                    character: owner.character,
                    holder: successor,
                    evidence: owner.evidence,
                    world_id: owner.world_id,
                    channel_id: owner.channel_id,
                    premium_only: false,
                    required_capabilities: Arc::clone(&owner.required_capabilities),
                    runtime: &runtime,
                    movement_cells: &owner.movement_cells,
                    door: &owner.door,
                    chest: owner.chest,
                    spells: owner.spells,
                    active_generation: owner.active_generation,
                    premium_coordinator: None,
                    rooms: empty_test_rooms(),
                    achievements: owner.achievements,
                    npc_talk: None,
                    imported_charms: owner.imported_charms,
                    quest_catalogue: owner
                        .quest_catalogue
                        .as_ref()
                        .ok_or("Mitmah quest catalogue")?,
                    quest_log_content: &quest_log,
                    last_position_flush: None,
                },
                &stop,
                None,
            ),
        ),
    );
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(15), async {
            tokio::select! {
                result = &mut serving => Err(format!("successor stopped before an owner sweep: {result:?}")),
                () = swept.notified() => Ok(()),
            }
        }).await??;
    }
    assert_eq!(
        museum_grant_count(h).await?,
        1,
        "idle serving must recover the retained boss award"
    );
    assert!(
        runtime
            .lock()
            .await
            .positioned_actor_census()
            .map_err(debug)?
            .is_empty()
    );
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(15), serving)
        .await?
        .map_err(debug)?;
    Ok(())
}
