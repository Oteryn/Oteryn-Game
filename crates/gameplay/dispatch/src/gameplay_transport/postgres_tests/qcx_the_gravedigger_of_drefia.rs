use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, world_with_objects};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, NodeId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    world_callbacks::{self, Position},
};
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, WorldObjectTarget};

const CALLBACK: &str = "oteryn:quest-callback/gravedigger-statue";
const QUEST: &str = "oteryn:quest.the_gravedigger_of_drefia_quest";
const ANOINTED: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission24";
const TOLLED: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission25";
const SEED: &str = "oteryn:quest-transition/qcx74/anointed";

#[test]
fn bloodgong_uses_native_statue_without_donor_action_id() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::super::kill_reward_e2e::scenario_thread::run("qcx74-bloodgong", bloodgong)?;
    eprintln!("qcx74 Bloodgong: {:.2}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn bloodgong() -> TestResult {
    let trigger = Position { x: 32998, y: 32416, floor: -11 };
    // The imported tile contains i17469 followed by i19112, without placement tags.
    let mut map_source = world_callbacks::load()?.into_iter()
        .find(|callback| callback.key == CALLBACK).ok_or("Bloodgong callback")?;
    map_source.selector.action = None;
    map_source.selector.appearance = Some(17469);
    let world = world_with_objects(&map_source, trigger, &[(trigger, 19112)], &[])?;
    let ordinal = 2;
    let seed_transition = QuestTransition {
        key: SEED.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: ANOINTED.into(),
            from: QuestComparison::Between(-1, 0),
            effect: QuestEffectKind::Set(1),
        }],
        completes: false,
        world_change: None,
        experience: None,
    };
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx74_bloodgong", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        let catalogue =
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .with_fixture_transition(seed_transition).map_err(debug)?;
        owner.quest_catalogue = Some(std::sync::Arc::new(catalogue));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seeded = slot.commit_quest_transition(
            owner.root, owner.character, owner.holder, fence,
            QuestTransitionRequest {
                transition_key: SEED.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id, CommandId::new(900).map_err(debug)?,
                )),
                catalogue: catalogue.clone(),
            },
        ).await.map_err(debug)?;
        assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let key = oteryn_world_bundle::bundle::placement_key(
            trigger.floor,
            trigger.x,
            trigger.y,
            ordinal,
        )
        .ok_or("key")?;
        let mut placement = owner
            .bundle_map()
            .ok_or("map")?
            .overlay
            .base()
            .digest()
            .to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let reply = wire(
            &owner, &mut admitted, 1, 2,
            world_object::encode_use_intent(&target).map_err(debug)?,
        ).await?;
        assert_eq!(world_object::decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
        let state = h.root.read_character_quest_state(&authority, fence.character_id)
            .await.map_err(debug)?;
        assert_eq!(state.tracks().get(TOLLED), Some(&1));
        Ok(())
    })
}
