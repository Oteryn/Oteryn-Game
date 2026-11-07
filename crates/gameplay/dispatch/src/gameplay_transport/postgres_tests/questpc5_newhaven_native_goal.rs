//! Native Troll goals share the existing lethal queue; source tour flags are seeded only in this component case.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CharacterLease, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::native_kill_goals;
const COUNTER: &str = "oteryn:quest-progress/newhaven/troll_kills";
#[test]
fn questpc5_newhaven_native_troll_goal_is_bound_to_the_selected_activation() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-newhaven-native-goal")
}
pub(super) async fn value(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<i64> {
    let copy = owner
        .root
        .read_character_quest_state(
            owner.character,
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        )
        .await
        .map_err(debug)?;
    Ok(copy.tracks().get(COUNTER).copied().unwrap_or(0))
}

pub(super) async fn capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    position: MovementLocalPosition,
    serial: u16,
) -> TestResult<ExactActorRef> {
    capture_origin(owner, admitted, creature, position, serial, true).await
}

async fn capture_origin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    position: MovementLocalPosition,
    serial: u16,
    static_origin: bool,
) -> TestResult<ExactActorRef> {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let map = owner.bundle_map().ok_or("qualified map")?;
    assert!(
        map.overlay
            .base()
            .tile(
                u16::try_from(position.x)?,
                u16::try_from(position.y)?,
                -i8::try_from(position.floor)?
            )
            .is_some(),
        "the spawn's physical tile must exist"
    );
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        let pin = runtime.content_pin().clone();
        let target = if static_origin {
            runtime
                .realize_native_qualification_spawn_for_source(serial, pin, creature, position)
                .map_err(debug)?
                .actor
        } else {
            let policy = runtime.companion_policy(creature).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(creature, policy.maximum_health, position)
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, creature)
                .map_err(debug)?;
            actor
        };
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(
                    session,
                    CommandId::new(10000 + u64::from(serial)).map_err(debug)?,
                ),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"quest-native-goal",
                    damage: health,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                now,
                Some(observed),
            );
        }
        target
    };
    assert!(
        queue.has_session(session),
        "native death must enter the retained settlement queue"
    );
    let lease = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
        queue.take(session, lease.generation())
    else {
        return Err("retained native occurrence".into());
    };
    drop(taken); // Cancellation returns the same frozen predecessor to the existing queue.
    Ok(target)
}

pub(super) async fn drain(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
) -> TestResult {
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let mut runtime = owner.runtime.lock().await;
    assert!(
        !runtime.corpse_projection_pinned(target),
        "settlement must release the native corpse pin"
    );
    runtime.remove_test_actor(target).map_err(debug)?;
    Ok(())
}

async fn seed_visits(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let mut tracks = std::collections::BTreeMap::new();
    let mut transitions = Vec::new();
    for c in callbacks
        .iter()
        .filter(|c| c.quest == "oteryn:quest.newhaven_quest")
    {
        for s in &c.storage {
            tracks.insert(
                s.track.clone(),
                original.track(&s.track).cloned().ok_or("track")?,
            );
        }
        transitions.push(
            original
                .transition(&c.key)
                .cloned()
                .ok_or("callback transition")?,
        );
    }
    let goal = native_kill_goals::load()?.first().ok_or("goal")?;
    transitions.push(
        original
            .transition(&goal.key)
            .cloned()
            .ok_or("native goal transition")?,
    );
    assert_eq!(
        (
            tracks[COUNTER].initial,
            tracks[COUNTER].min,
            tracks[COUNTER].max
        ),
        (0, 0, 10)
    );
    let setup = "oteryn:quest-transition/fixture-newhaven-tour";
    transitions.push(QuestTransition {
        key: setup.into(),
        quest: goal.quest.clone(),
        effects: ["school", "supply"]
            .into_iter()
            .map(|suffix| QuestEffect {
                track: format!("oteryn:quest-progress/newhaven/{suffix}"),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            })
            .collect(),
        completes: false,
        experience: None,
    });
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            original.content_revision(),
            tracks.into_values().collect(),
            transitions,
        )
        .map_err(debug)?,
    );
    native_kill_goals::validate_closed(&catalogue)?;
    owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: setup.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let goal = native_kill_goals::load()?.first().ok_or("goal")?;
    assert_eq!(goal.count, 10);
    assert_eq!(
        goal.activation,
        "oteryn:quest-reward/newhaven-training-ready"
    );
    let source = super::source_npc_placements::declaration("oteryn:npc.gustavo_the_guard")?;
    assert_eq!(
        source["cell"],
        serde_json::json!({"x":32560,"y":32488,"floor":7})
    );
    assert_eq!(source["direction"], "north");
    let [npc] =
        super::classic_quests::install(owner, admitted, ["oteryn:npc.gustavo_the_guard"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    let first = goal.origins.first().ok_or("origin")?;
    let origin = MovementLocalPosition {
        x: i32::from(first.x),
        y: i32::from(first.y),
        floor: i16::from(first.floor),
    };
    // Five real pre-activation Troll deaths earn base20XP with the existing100% low-level bonus, never quest credit.
    for serial in 1000..1005 {
        let target = capture(owner, admitted, "oteryn:creature.troll", origin, serial).await?;
        drain(owner, admitted, target).await?;
        assert_eq!(
            value(owner).await?,
            0,
            "pre-activation native kills are ineligible"
        );
    }
    let earned: i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        earned, 200,
        "five ordinary native Troll deaths earn20XP each plus the existing100% low-level bonus"
    );
    seed_visits(owner, admitted).await?;
    let say = super::qengine_rewards::talk;
    let mut command = admitted.continuity.next_command_id;
    assert_eq!(
        say(owner, admitted, npc, command, "hi").await?,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    command += 1;
    assert_eq!(
        say(owner, admitted, npc, command, "welcome").await?,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    command += 1;
    assert_eq!(
        say(owner, admitted, npc, command, "mission").await?,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    command += 1;
    assert_eq!(value(owner).await?, 0);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let bypass = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: goal.key.clone(),
                cause: crate::durability::quest_state::QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(9999).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await;
    assert!(
        bypass.is_err(),
        "a command cannot manufacture a native death"
    );
    for (serial, creature, position, native_source) in [
        (
            1010,
            "oteryn:creature.troll",
            MovementLocalPosition {
                x: 32558,
                y: 32488,
                floor: 7,
            },
            true,
        ),
        (1011, "oteryn:creature.dragon", origin, true),
        (1012, "oteryn:creature.troll", origin, false),
    ] {
        let target =
            capture_origin(owner, admitted, creature, position, serial, native_source).await?;
        drain(owner, admitted, target).await?;
        assert_eq!(
            value(owner).await?,
            0,
            "wrong origin/species or encounter copy is ineligible"
        );
    }
    for (index, source) in goal.origins.iter().enumerate() {
        let position = MovementLocalPosition {
            x: i32::from(source.x),
            y: i32::from(source.y),
            floor: i16::from(source.floor),
        };
        let target = capture(
            owner,
            admitted,
            &source.creature,
            position,
            1020 + u16::try_from(index)?,
        )
        .await?;
        drain(owner, admitted, target).await?;
        assert_eq!(value(owner).await?, i64::try_from(index + 1)?);
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            value(owner).await?,
            i64::try_from(index + 1)?,
            "replay never credits twice"
        );
    }
    let target = capture(owner, admitted, "oteryn:creature.troll", origin, 1040).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(value(owner).await?, 10, "native goal is capped at ten");
    assert_eq!(
        say(owner, admitted, npc, command, "report").await?,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
    );
    command += 1;
    let rings:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i52732'").fetch_one(&h.pool).await?;
    assert_eq!(
        rings, 1,
        "ten actual native kills permit the guarded ring reward"
    );
    let _ = say(owner, admitted, npc, command, "training").await?;
    let target = capture(owner, admitted, "oteryn:creature.troll", origin, 1041).await?;
    drain(owner, admitted, target).await?;
    assert_eq!(value(owner).await?, 10, "terminal native run never reopens");
    eprintln!(
        "POSTGRES_CASE_COMPLETED newhaven_native_kills duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn source_world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette = vec![bundle::PaletteEntry {
        key: "oteryn:terrain.tibia.i51914".into(),
        family: bundle::Family::Terrain,
        id: 0,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(120),
        }),
    }];
    data.manifest.world = bundle::Extent {
        min_x: 32558,
        min_y: 32450,
        max_x: 32610,
        max_y: 32490,
        floors: vec![-7],
    };
    let mut positions = std::collections::BTreeSet::new();
    for y in 32487..=32489 {
        for x in 32558..=32561 {
            positions.insert((y, x));
        }
    }
    for origin in native_kill_goals::load()?
        .first()
        .ok_or("goal")?
        .origins
        .iter()
    {
        for y in origin.y - 1..=origin.y + 1 {
            for x in origin.x - 1..=origin.x + 1 {
                positions.insert((y, x));
            }
        }
    }
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for (y, x) in positions {
        grouped
            .entry((y / 32, x / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                }],
            });
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: -7,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32559,
                y: 32488,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
