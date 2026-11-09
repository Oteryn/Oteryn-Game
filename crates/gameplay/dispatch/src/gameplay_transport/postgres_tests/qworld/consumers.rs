//! Real native rat death settles the shared counter, then unlocks the fenced quest writer.
use super::super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

#[allow(
    clippy::too_many_arguments,
    reason = "The fixture passes independent owner, actor and occurrence proofs explicitly."
)]
pub(crate) async fn qualify(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    pin: crate::content::channel_content_pin::ChannelContentPin,
    room: &crate::content::QualifiedNativeEntryRoom,
    epoch: u64,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let world = admitted.world_id;
    let session = admitted.game_session_id;
    super::rules::qualify(h, owner, world, epoch).await?;
    let key = "qworld_rat_cull";
    let rule = crate::durability::world_event::WorldChangeCounter {
        key: key.into(),
        creature: "oteryn:creature.rat".into(),
        goal: 2,
    };
    h.root
        .bind_world_change_counter(owner.character, world, epoch, rule.clone())
        .await?;
    h.root
        .bind_world_change_counter(owner.character, world, epoch, rule)
        .await?;
    assert!(!h.root.world_change_active(world, key).await?);
    owner.admit_progression_session(admitted).await;
    // The shared admission fixture declares this interpretation; progression is
    // initialized by the real native policy above, never by a synthetic XP row.
    let revision = "content-1";
    let track = QuestTrack {
        key: "oteryn:quest-progress/qworld/cull".into(),
        quest: "oteryn:quest.qworld_cull".into(),
        initial: 0,
        min: 0,
        max: 1,
    };
    let transition = QuestTransition {
        key: "oteryn:quest-transition/qworld/cull".into(),
        quest: track.quest.clone(),
        effects: vec![QuestEffect {
            track: track.key.clone(),
            from: QuestComparison::Eq(0),
            effect: QuestEffectKind::Set(1),
        }],
        completes: true,
        experience: None,
        world_change: Some(key.into()),
    };
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(revision, vec![track], vec![transition]).map_err(debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: "oteryn:quest-transition/qworld/cull".into(),
        cause: QuestCause::Command(crate::foundation::CommandRef::new(
            session,
            crate::foundation::CommandId::new(901).map_err(debug)?,
        )),
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                owner.character,
                owner.holder,
                fence,
                travel_route(key),
                &NoPremium
            )
            .await
            .map_err(debug)?,
        Err(crate::durability::npc_travel::TravelRefusal::UnavailableRoute)
    );
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let refused = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request.clone(),
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            refused,
            QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch
            )
        ));
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let active = owner.active_generation.ok_or("native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
    }
    let mut combat_clock = owner.owner_now();
    for death in 1_u16..=2 {
        let rat = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .realize_native_qualification_spawn_for_source(
                    death,
                    pin.clone(),
                    "oteryn:creature.rat",
                    crate::foundation::MovementLocalPosition {
                        x: 1,
                        y: 11,
                        floor: -7,
                    },
                )
                .map_err(debug)?;
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .ok_or("real rat")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        let start = combat_clock;
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(room),
                    actor,
                    session,
                    start,
                    Some(rat),
                    u64::from(death)
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        for turn in 0..128 {
            let now = start.checked_add(turn * 3_000_000).map_err(debug)?;
            owner.refresh_attack_weapon_at(actor, session, now).await;
            owner.drain_auto_attacks_at(now).await;
            combat_clock = now.checked_add(3_000_000).map_err(debug)?;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
        }
        let replay = {
            let attack = owner.states.attack.lock().await;
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                attack.kills().take(session, 2)
            else {
                return Err("real swings did not queue the kill".into());
            };
            let entry = taken.entry().ok_or("real sealed kill")?;
            let replay = (entry.facts, entry.target, entry.activity_observed_at);
            taken.requeue();
            replay
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if owner.drain_kill_rewards(session).await
                == crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("kill settlement deadline".into());
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(h.root.world_change_active(world, key).await?, death == 2);
        assert_eq!(
            h.root
                .world_change_active(world, "qworld_rat_exhaustion")
                .await?,
            death != 2
        );
        {
            let mut runtime = owner.runtime.lock().await;
            let attack = owner.states.attack.lock().await;
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    prey: None,
                    activity: &[],
                    queue: attack.kills(),
                    rewards: native.reward_table(),
                    charms: None,
                },
                replay.1,
                replay.0.principal,
                replay.0.death_at_ms,
                replay.2,
            );
        }
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
        let count: i64 = sqlx::query_scalar("SELECT counter FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key=$3")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(key).fetch_one(&h.pool).await?;
        assert_eq!(
            count,
            i64::from(death),
            "sealed death replay must not increment twice"
        );
        assert_eq!(
            h.root.world_change_active(world, key).await?,
            death == 2,
            "the first replay cannot cross the two-kill threshold"
        );
    }
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("current quest fence")?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                owner.character,
                owner.holder,
                fence,
                travel_route(key),
                &NoPremium
            )
            .await
            .map_err(debug)?,
        Ok(100)
    );
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let committed = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            catalogue.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(committed, QuestTransitionOutcome::Committed(_)),
        "active world change unlocks quest: {committed:?}"
    );
    assert!(matches!(
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            catalogue
        )
        .await
        .map_err(debug)?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    Ok(())
}

struct NoPremium;
impl crate::durability::npc_travel::NpcTravelGate for NoPremium {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(false)
    }
}

fn travel_route(key: &str) -> oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute {
    use oteryn_gameplay_content::content::{ProjectV2TravelDestination, ProjectV2TravelRoute};
    oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute {
        service: "oteryn:npc-service.qworld".into(),
        route: ProjectV2TravelRoute {
            key: "qworld".into(),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x: 2,
                y: 11,
                floor: -7,
            },
            price: 100,
            premium: false,
            min_level: None,
        },
        keyword: "qworld".into(),
        postman: false,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
        quest_clear_gate: None,
        ticket: None,
        world_change: Some(key.into()),
    }
}
