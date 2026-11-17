//! Actual carried essence USE reaches the production composed owner and paired writers.
#[path = "srx3_rotten_essence/native_house.rs"]
mod native_house;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
pub(super) use native_house::assert_native_house_use;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn srx3_rotten_essence_channel_production_use_burns_and_sets_quest_once_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-essence-use", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "srx3_essence_use", false, false)
                .await?;
            prepare_current_session(&h).await?;
            // Historical inventory only; all new USE/Quest/Item guards remain active.
            super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i43501", 2)]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            let active = controller.active().ok_or("active generation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let room = pin.qualified_room().clone();
            let channel_pin = ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                room.map_revision_digest(),
                (0, 0, 0),
            );
            let deps = Dependencies::with_content(
                channel_pin,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let index = native.item_index().ok_or("source item index")?;
            let definition = index
                .definition_ref("Item", "oteryn:item.tibia.i43501", "definition-r1")
                .ok_or("real carried essence definition")?;
            let command = UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1,
                item_fence: admitted.item_fence,
            };
            let first = owner
                .use_item_by_definition(deps.actor, command, definition)
                .await;
            assert_eq!(
                first.disposition,
                UseDisposition::Committed,
                "actual source essence USE must reach its paired owner"
            );
            let replay = owner
                .use_item_by_definition(deps.actor, command, definition)
                .await;
            assert_eq!(replay.disposition, first.disposition);
            let changed = index
                .definition_ref("Item", "oteryn:item.tibia.i43502", "definition-r1")
                .ok_or("other source essence")?;
            assert_eq!(
                owner
                    .use_item_by_definition(deps.actor, command, changed)
                    .await
                    .disposition,
                UseDisposition::Rejected,
                "same CommandRef cannot replay another essence"
            );

            // A new USE while the source flag is already held must not burn
            // another unit (rottenEssences.lua:16-20), independently of replay.
            let _already_held = owner
                .use_item_by_definition(
                    deps.actor,
                    UseCommand {
                        command_id: 2,
                        ..command
                    },
                    definition,
                )
                .await;
            let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($2,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'), (SELECT count(*) FROM game_item_use_records WHERE cause_kind=4)")
                .bind(id(71).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(state, (1, 1, 1));
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/rotten-blood/essence-murcion'").fetch_one(&h.pool).await?,1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ESSENCE_CHANNEL_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// The surrounding cases establish a real Activity or named-House handoff.
/// Their catalogue data is only scope-fixture data, not boss qualification.
pub(super) async fn assert_scoped_use(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    house: Option<&str>,
) -> TestResult {
    use crate::foundation::RuntimeScopeRefV1;
    let started = std::time::Instant::now();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("current native source")?;
    let index = native.item_index().ok_or("native item index")?;
    let definition = index
        .definition_ref("Item", "oteryn:item.tibia.i43501", "definition-r1")
        .ok_or("actual essence")?;
    let fence = admitted.item_fence.ok_or("actual transferred item fence")?;
    assert!(matches!(
        fence.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: Some(fence),
    };
    assert_eq!(
        owner
            .use_item_by_definition(actor, command, definition)
            .await
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(
        owner
            .use_item_by_definition(actor, command, definition)
            .await
            .disposition,
        UseDisposition::Committed
    );
    let changed = index
        .definition_ref("Item", "oteryn:item.tibia.i43502", "definition-r1")
        .ok_or("other source essence")?;
    assert_eq!(
        owner
            .use_item_by_definition(actor, command, changed)
            .await
            .disposition,
        UseDisposition::Rejected
    );
    let mut stale = fence;
    stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
        fence.scope_ownership_generation.get() + 1,
    )
    .map_err(debug)?;
    assert_eq!(
        owner
            .use_item_by_definition(
                actor,
                UseCommand {
                    command_id: command.command_id + 1,
                    item_fence: Some(stale),
                    ..command
                },
                definition
            )
            .await
            .disposition,
        UseDisposition::Rejected
    );
    let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT value FROM game_character_quest_tracks WHERE character_id=encode($2,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'),(SELECT count(*) FROM game_item_use_records WHERE cause_kind=4)")
        .bind(id(71).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (1, 1, 1));
    let scope:(Option<Vec<u8>>,Option<Vec<u8>>,Option<String>)=sqlx::query_as("SELECT uuid_send(activity_instance_id),uuid_send(channel_id),source_house_key FROM game_item_use_records WHERE cause_kind=4")
        .fetch_one(&h.pool).await?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("real transferred Instance".into());
    };
    if let Some(house) = house {
        assert_eq!(
            scope,
            (
                None,
                Some(owner.channel_id.as_bytes().to_vec()),
                Some(house.into())
            )
        );
    } else {
        assert_eq!(scope, (Some(instance_id.to_vec()), None, None));
    }
    let audit:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_item_use_records u USING(transaction_id) WHERE u.cause_kind=4")
        .fetch_one(&h.pool).await?;
    let (_, body) =
        crate::durability::item_use_audit::decode_item_use_envelope(&audit).map_err(debug)?;
    assert_eq!(body.house_key.as_deref(), house);
    assert_eq!(
        body.instance_id,
        house.is_none().then(|| instance_id.to_vec())
    );
    assert_eq!(
        body.channel_id,
        if house.is_some() {
            owner.channel_id.as_bytes().to_vec()
        } else {
            Vec::new()
        }
    );
    assert_eq!(body.source_before.as_ref().map(|v| v.quantity), Some(2));
    assert_eq!(body.source_after.as_ref().map(|v| v.quantity), Some(1));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/rotten-blood/essence-murcion'").fetch_one(&h.pool).await?,1);
    if house.is_some() {
        let state = {
            let runtime = owner.runtime.lock().await;
            assert!(
                runtime.read_actor_position(actor).is_err(),
                "a private House resident has no Channel position"
            );
            runtime
                .house_actor_state(actor, admitted.game_session_id)
                .map_err(debug)?
        };
        assert_eq!(state.scope, fence.runtime_scope);
        assert_eq!(state.generation, fence.scope_ownership_generation);
        assert_eq!(state.lease.generation(), fence.character_lease_generation);
        let private = crate::map::overlay::TilePos {
            x: state.position.x.try_into()?,
            y: state.position.y.try_into()?,
            floor: i8::try_from(state.position.floor)?
                .checked_neg()
                .ok_or("House private floor")?,
        };
        let events = owner
            .observe_current_actor_movement_effects(actor, admitted.game_session_id, 0)
            .await;
        assert_eq!(
            events.len(),
            1,
            "a real committed House essence emits blue once at its own private position"
        );
        assert_eq!((events[0].1, events[0].2), (private, 13));
        assert!(
            owner
                .observe_current_actor_movement_effects(
                    actor,
                    admitted.game_session_id,
                    events[0].0
                )
                .await
                .is_empty(),
            "an observer cursor cannot replay the committed effect"
        );
        assert!(
            owner
                .observe_current_actor_movement_effects(
                    actor,
                    crate::foundation::GameSessionId::decode(&id(250)).map_err(debug)?,
                    0
                )
                .await
                .is_empty(),
            "a foreign session cannot read private House presentation"
        );
        assert!(
            owner.observe_movement_effects(0).is_empty(),
            "the origin Channel never receives the House-private blue effect"
        );
        eprintln!(
            "SRX3_HOUSE_ESSENCE_SCOPED_FX_VERIFIED duration={:?}",
            started.elapsed()
        );
        let presentation =
            crate::gameplay_transport::connection::fresh_current_actor_capability_selection(
                owner,
                &[6, 23],
                admitted,
            )
            .await
            .ok_or("current House optional presentation selection")?;
        assert!(
            presentation.contains(oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1),
            "the real current House actor must qualify optional native effect23 before a wire timeout"
        );
        assert_eq!(
            owner
                .native_current_actor_map_cue_frame(actor, admitted.game_session_id)
                .await,
            Some(false),
            "the actual House keeps its qualified existing wire frame"
        );
        assert!(
            owner
                .native_current_actor_map_cue_frame(
                    actor,
                    crate::foundation::GameSessionId::decode(&id(250)).map_err(debug)?
                )
                .await
                .is_none(),
            "a current-actor frame proof cannot be borrowed by a foreign session"
        );
    }
    eprintln!(
        "SRX3_ESSENCE_TRANSFERRED_SCOPE_CASE_EXECUTED house={} duration={:?}",
        house.is_some(),
        started.elapsed()
    );
    Ok(())
}
