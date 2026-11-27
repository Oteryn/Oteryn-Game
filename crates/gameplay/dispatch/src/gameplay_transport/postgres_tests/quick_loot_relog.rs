//! A fresh GameSession can write Quick Loot destinations while the holder is contended.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandStatus, WorldId};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, UseCommand,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt as _;
use oteryn_protocol_oteryn::{logout::LogoutResult, quick_loot as wire};

#[test]
fn quick_loot_food_to_gold_pouch_on_first_command_after_relog() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-relog", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(admin))
    })
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "quick_loot_relog", false, false).await?;
    let result = exercise(&h).await;
    let cleanup = h.cleanup().await;
    result?;
    cleanup
}

async fn exercise(h: &Harness) -> TestResult {
    let request = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active Content")?;
    let native = active.native_gameplay().ok_or("native Content")?;
    super::tutorial_vitals_fixture::publish_native_activation(h, &pin).await?;
    super::tutorial_vitals_fixture::native_asset_revisions(h, active).await?;
    let room = pin.qualified_room().clone();
    let deps =
        Dependencies::with_content(pin.into_channel_pin(), room, native.spell_book().clone())?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    admitted.item_fence = crate::gameplay_transport::item_fence_of(
        store.current_session(admitted.game_session_id).await?,
    );
    owner.admit_progression_session(&mut admitted).await;
    super::track(&owner, &deps)?;
    let pouch = id(71);
    let pouch_key = "oteryn:item.tibia.i23721";
    let revision = native
        .item_index()
        .and_then(|index| index.revision(pouch_key))
        .ok_or("Gold Pouch definition")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid)")
            .bind(pouch.as_slice()).bind(id(42).as_slice()).bind(pouch_key).bind(revision)
            .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_character_inbox_locations(character_id,item_instance_id,world_id,ordinal)
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)")
            .bind(id(41).as_slice()).bind(pouch.as_slice()).bind(id(42).as_slice())
            .execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );

    let (session, relog_request) = super::full_world_support::relogin_request(h).await?;
    let current = store.current_session(session).await?;
    let transport = current.current_transport().ok_or("relog transport")?;
    let actor = {
        let mut runtime = deps.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reserved).map_err(debug)?
    };
    let (outcome, fence, _) =
        Box::pin(owner.initialize_first_entry(&relog_request, session, transport, actor)).await;
    assert!(
        matches!(
            outcome,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{outcome:?}"
    );
    assert!(fence.is_some());
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await?;
    // A cold coordinator makes the first Premium check read its durable latch.
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    owner.premium_coordinator = Some(&premium);
    owner.admit_premium(id(40), session);
    admitted.game_session_id = session;
    admitted.runtime_actor = Some(actor);
    admitted.first_entry = outcome;
    admitted.item_fence = fence;
    admitted.controller = Some(ControllerBinding {
        transport,
        account_id: id(40),
    });
    admitted.continuity.house_input_pending = Some(false);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 98, 607])
            .ok_or("Quick Loot destination capability")?;

    let command = wire::QuickLootRequest {
        action: wire::QuickLootAction::AssignLoot,
        corpse: None,
        category: Some(wire::QuickLootCategory::Food),
        recipient: Some(wire::QuickLootRecipient::GoldPouch),
        recipient_id: Some(pouch),
        fallback_to_main: true,
        filter_mode: None,
        filter_definition_ref: None,
    };
    let payload = wire::encode_quick_loot_request(&command).map_err(debug)?;
    let command_type = command.action.command_type();
    let reply = {
        let holder = h.root.try_issue_semantic_pass()?;
        let mut routed = std::pin::pin!(owner.quick_loot_destinations(
            admitted,
            UseCommand {
                game_session_id: session,
                command_id: admitted.continuity.next_command_id,
                item_fence: fence,
            },
            (command_type, &payload),
            Some(command),
        ));
        // Let the command encounter the held slot before releasing it. A synchronous
        // refusal or disconnect here is the original failure.
        let early = std::future::poll_fn(|cx| {
            use std::future::Future as _;
            std::task::Poll::Ready(match routed.as_mut().poll(cx) {
                std::task::Poll::Ready(result) => Some(result),
                std::task::Poll::Pending => None,
            })
        })
        .await;
        drop(holder);
        match early {
            Some(reply) => reply,
            None => routed.await,
        }
    };
    eprintln!(
        "QUICK_LOOT_RELOG first assignment accepted={}",
        matches!(reply, Some((CommandStatus::Accepted, _)))
    );
    assert!(
        matches!(
            reply,
            Some((CommandStatus::Accepted, ref payload))
                if payload.is_empty()
        ),
        "first Food assignment after relog: {reply:?}"
    );
    let stale = wire::encode_quick_loot_request(&wire::QuickLootRequest {
        recipient_id: Some(id(72)),
        ..command
    })
    .map_err(debug)?;
    let review = wire::encode_quick_loot_request(&wire::QuickLootRequest {
        action: wire::QuickLootAction::ReviewDestinations,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        ..command
    })
    .map_err(debug)?;
    let (replies, _) = super::parameter_spell_fixtures::exchange(
        &owner,
        &mut admitted,
        &[
            (command_type, payload),
            (command_type, stale),
            (command_type, review),
        ],
        false,
        false,
    )
    .await?;
    assert_eq!(replies.len(), 3);
    assert_eq!(
        replies[0],
        (CommandStatus::Accepted, Vec::new()),
        "durable replay"
    );
    assert_eq!(replies[1].0, CommandStatus::Rejected);
    assert_eq!(
        wire::decode_quick_loot_refusal(&replies[1].1),
        Ok(wire::QuickLootRefusal::Stale)
    );
    assert_eq!(
        replies[2].0,
        CommandStatus::Accepted,
        "the refused command keeps the stream open"
    );
    let destinations = wire::decode_quick_loot_destinations(&replies[2].1).map_err(debug)?;
    assert!(destinations.loot.contains(&wire::QuickLootDestination {
        category: command.category,
        recipient: wire::QuickLootRecipient::GoldPouch,
        recipient_id: Some(pouch),
    }));
    Ok(())
}
