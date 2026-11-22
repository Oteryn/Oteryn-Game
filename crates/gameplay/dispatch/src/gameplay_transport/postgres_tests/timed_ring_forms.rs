//! Timed ring forms through the composed equipment, logout and re-entry owners.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, WorldId};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, UseCommand,
};
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt as _;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::time::Duration;

#[test]
fn stealth_ring_changes_form_and_keeps_worn_budget_across_logout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(
        "stealth_ring_forms",
        "oteryn:item.tibia.i3049",
        "oteryn:item.tibia.i3086",
        600_000,
    )
}

#[test]
fn life_ring_changes_form_and_keeps_worn_budget_across_logout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(
        "life_ring_forms",
        "oteryn:item.tibia.i3052",
        "oteryn:item.tibia.i3089",
        1_200_000,
    )
}

#[test]
fn healing_ring_changes_form_and_keeps_worn_budget_across_logout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(
        "healing_ring_forms",
        "oteryn:item.tibia.i3098",
        "oteryn:item.tibia.i3100",
        450_000,
    )
}

const RING_FIXTURE_ID: u8 = 71;

fn scenario(tag: &'static str, base: &'static str, worn: &'static str, full: i64) -> TestResult {
    run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, tag, false, false).await?;
        let result = qualify(&h, base, worn, full).await;
        h.cleanup().await?;
        result
    })
}

async fn qualify(h: &Harness, base: &str, worn: &str, full: i64) -> TestResult {
    let request = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (generation, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    super::tutorial_vitals_fixture::publish_native_activation(h, &pin).await?;
    let active = generation.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("item index")?;
    let room = pin.qualified_room().clone();
    let (channel_pin, _, _, _) = pin.into_channel_parts();
    let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::item_look_fixtures::seed_main_backpack(h, &[(base, 1)]).await?;
    super::tutorial_vitals_fixture::native_asset_revisions(h, active).await?;
    sqlx::query(
        "INSERT INTO game_character_equipment_state(character_id, revision) \
         VALUES (encode($1, 'hex')::uuid, 1) ON CONFLICT (character_id) DO NOTHING",
    )
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;

    let store = FreshAdmissionStore::from_root(h.root.clone());
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = item_fence_of(
        store
            .current_session(admitted.game_session_id)
            .await
            .map_err(debug)?,
    );
    let actor = deps.actor;
    let session = admitted.game_session_id;
    let position_fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "position fence unavailable")?
        .ok_or("position fence absent")?;
    assert!(owner.track_restored(
        session,
        actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        position_fence
    ));
    let item = ItemKey::Instance(id(RING_FIXTURE_ID));
    let equip = EquipmentMove::Equip {
        item,
        to: EquipmentSlot::Ring,
    };
    let unequip = EquipmentMove::Unequip {
        item,
        from: EquipmentSlot::Ring,
    };

    move_ring(&owner, actor, admitted, 1, equip).await;
    let wire_ref = |key: &str| index.definition_ref("Item", key, index.revision(key)?);
    let worn_ref = wire_ref(worn).ok_or("worn wire definition")?;
    let base_ref = wire_ref(base).ok_or("base wire definition")?;
    assert_eq!(
        worn_ring(&owner, actor, session).await?.item_definition_ref,
        worn_ref
    );

    owner.clock_origin -= Duration::from_secs(10);
    move_ring(&owner, actor, admitted, 2, unequip).await;
    let inventory = owner
        .observe_character_inventory(actor, session)
        .await
        .ok_or("backpack")?;
    let stored = inventory
        .entries
        .iter()
        .find(|entry| entry.key == item)
        .ok_or("unequipped ring")?;
    assert_eq!(stored.item_definition_ref, base_ref);
    let paused = budget(h).await?;
    assert!(
        (full - 30_000..=full - 10_000).contains(&paused),
        "{paused}"
    );
    owner.clock_origin -= Duration::from_secs(60);
    assert!(owner.checkpoint_timed_equipment(actor, session, true).await);
    assert_eq!(budget(h).await?, paused, "backpack time does not count");

    move_ring(&owner, actor, admitted, 3, equip).await;
    owner.clock_origin -= Duration::from_secs(10);
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let saved = budget(h).await?;
    assert!(
        (paused - 30_000..=paused - 10_000).contains(&saved),
        "{saved}"
    );

    owner.clock_origin -= Duration::from_secs(300);
    let (session, request) = super::full_world_support::relogin_request(h).await?;
    let current = store.current_session(session).await.map_err(debug)?;
    let transport = current.current_transport().ok_or("relog transport")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let (outcome, _, _) =
        Box::pin(owner.initialize_first_entry(&request, session, transport, actor)).await;
    assert!(
        matches!(
            outcome,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{outcome:?}"
    );
    let reloaded = worn_ring(&owner, actor, session).await?;
    assert_eq!(
        (reloaded.key, reloaded.item_definition_ref),
        (item, worn_ref)
    );
    assert_eq!(
        budget(h).await?,
        saved,
        "admission preserves the saved budget"
    );
    assert!(owner.checkpoint_timed_equipment(actor, session, true).await);
    let resumed = budget(h).await?;
    assert!(
        (saved - 10_000..=saved).contains(&resumed),
        "offline time does not count: {resumed}"
    );

    owner.clock_origin -= Duration::from_millis(u64::try_from(resumed)?);
    assert!(owner.checkpoint_timed_equipment(actor, session, true).await);
    let equipment = owner
        .observe_character_equipment(actor, session)
        .await
        .ok_or("expired equipment")?;
    assert!(
        equipment
            .iter()
            .all(|(slot, _)| *slot != EquipmentSlot::Ring)
    );
    let lifecycle: i16 = sqlx::query_scalar(
        "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(RING_FIXTURE_ID).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifecycle, 2, "Crystal decayTo 0 removes the spent ring");
    Ok(())
}

async fn move_ring(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    admitted: AdmittedSession,
    command_id: u64,
    request: EquipmentMove,
) {
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    assert_eq!(
        owner.move_equipment(actor, command, request).await,
        EquipmentMoveResult::Committed
    );
}

async fn worn_ring(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<crate::gameplay_transport::item_view::ViewItem> {
    owner
        .observe_character_equipment(actor, session)
        .await
        .ok_or("equipment view")?
        .into_iter()
        .find_map(|(slot, ring)| (slot == EquipmentSlot::Ring).then_some(ring))
        .ok_or_else(|| "worn ring".into())
}

async fn budget(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT remaining_ms FROM game_item_timed_states \
         WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(RING_FIXTURE_ID).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
