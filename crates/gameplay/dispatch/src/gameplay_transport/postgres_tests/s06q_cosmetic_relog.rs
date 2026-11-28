//! Source-bound item acquisition followed by terminal logout and fresh admission.
use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::full_world_support::{FreshAdmissionStore, relogin};
use super::item_look_fixtures::seed_character_backpack;
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::quest_world_callbacks_fixtures::wire;
use super::rust_lottery_fixtures::{bundle_action_room, exchange};
use super::{Dependencies, Harness, TestResult, debug, fence, id, prepare_current_session};
use crate::content::ReferenceItemField;
use crate::foundation::CharacterLease;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply,
    SessionContinuity,
};
use oteryn_gameplay_items::item_cosmetics;
use oteryn_protocol_oteryn::appearance::{self, Catalogue, Change, ChangeResult, Selection};
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1;
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;

#[test]
fn s06q_pegasus_feather_mount_selection_survives_fresh_admission() -> TestResult {
    acquire_and_relog(
        "oteryn:item.tibia.i48424",
        "oteryn:item-grant.mount.227",
        "oteryn:content.mount.pegasus",
        None,
    )
}

#[test]
fn s06q_admitted_addon_doll_alias_selection_survives_fresh_admission() -> TestResult {
    acquire_and_relog(
        "oteryn:item.tibia.i6579",
        "oteryn:item-grant.outfit.128-136.addon1",
        "oteryn:content.outfit.citizen_outfits",
        Some(1),
    )
}

fn acquire_and_relog(
    item: &'static str,
    grant: &'static str,
    cosmetic: &'static str,
    addon: Option<u8>,
) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async move |admin| {
        let harness = Harness::create_appearance(admin, "s06q_cosmetic_relog").await?;
        eprintln!("S06-Q {item} POSTGRES_STARTED");
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence(1)?.runtime_scope.world_id();
        let channel = fence(1)?.runtime_scope.channel_id().ok_or("channel")?;
        let (controller, pin) = production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let items = native.item_index().ok_or("item index")?;
        let reference = |key| items.definition_ref("Item", key, "definition-r1");
        let bag = "oteryn:item.tibia.i2854";
        let oven = "oteryn:item.tibia.i2535";
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("backpack")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 3,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let ReferenceItemField::Known(physical) = &policy.semantics.physical else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: items.definition_ref("Item", key, "definition-r1")?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = bundle_action_room(pin.qualified_room(), &map, world, &cells)?;
        let entry = room.entry_start();
        let channel_pin = super::ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (entry.x, entry.y, entry.floor),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        let mut admitted = deps.admitted;
        let store = FreshAdmissionStore::from_root(harness.root.clone());
        let current = store.current_session(admitted.game_session_id).await?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    current.current_character_lease(),
                )
                .map_err(debug)?;
        }
        let original = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        admitted.controller = Some(ControllerBinding {
            transport: current.current_transport().ok_or("initial transport")?,
            account_id: id(40),
        });
        admitted.item_fence = Some(original);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        deps.initialize(&owner).await;
        super::track(&owner, &deps)?;
        owner.states.chat.admit(
            admitted.game_session_id,
            "Cosmetic witness",
            owner.owner_now(),
        );
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        let selected = SelectedCapabilities::select(
            &offered,
            &[
                CAPABILITY_NPC_SERVICE_V1,
                CAPABILITY_ITEM_VIEW_MOVE_V1,
                CAPABILITY_WORLD_SPATIAL_ENTITIES,
                CAPABILITY_WORLD_INTERACTION_V1,
                CAPABILITY_ITEM_USE_V1,
                CAPABILITY_WORLD_MAP_VIEW_V1,
                appearance::CAPABILITY_APPEARANCE_V1,
            ],
        )
        .ok_or("item and appearance capabilities")?;
        admitted.continuity.selected_capabilities = selected;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_loop_training(&mut admitted).await;

        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        // This retained instance identity produces the Citizen addon from the
        // production doll selector. No unlock or receipt is seeded.
        seed_character_backpack(&harness, 41, 81, &[(item, 1)]).await?;
        assert!(!owns(&catalogue(&owner, &admitted).await?, cosmetic, addon));

        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let (next, result, _) = exchange(
            &owner,
            admitted,
            command,
            &payload,
            Some(reference(item).ok_or("source item")?),
            None,
        )
        .await?;
        admitted = next;
        assert_eq!(result, UseDisposition::Committed);
        let receipt: (String, String) = sqlx::query_as(
            "SELECT r.source_kind, r.content_revision FROM game_account_appearance_unlocks u \
             JOIN game_account_appearance_unlock_receipts r USING (grant_occurrence_id, account_id, unlock_key) \
             WHERE u.account_id=encode($1,'hex')::uuid AND u.unlock_key=$2",
        )
        .bind(id(40).as_slice()).bind(grant).fetch_one(&harness.pool).await?;
        assert_eq!(
            (receipt.0.as_str(), receipt.1.as_str()),
            ("item_action", item_cosmetics::PROVENANCE)
        );

        let earned = catalogue(&owner, &admitted).await?;
        assert!(owns(&earned, cosmetic, addon));
        let mut selection = Selection {
            outfit_key: match addon {
                Some(_) => cosmetic.to_owned(),
                None => earned.outfits.first().ok_or("starter outfit")?.key.clone(),
            },
            colours: [0; 4],
            addons: addon.unwrap_or_default(),
            mount_key: addon.is_none().then(|| cosmetic.to_owned()),
        };
        select(&owner, &mut admitted, earned, selection.clone()).await?;
        let before = catalogue(&owner, &admitted)
            .await?
            .selected
            .ok_or("committed appearance")?;
        assert_eq!(before.selection, selection);

        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let successor = relogin(&harness).await?;
        assert_ne!(successor, admitted.game_session_id);
        let current = store.current_session(successor).await?;
        let lease: CharacterLease = current.current_character_lease();
        let successor_fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("successor fence")?;
        assert!(successor_fence.character_lease_generation > original.character_lease_generation);
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(successor).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(actor, successor, lease)
                .map_err(debug)?;
            actor
        };
        let gameplay = owner
            .current_quest_fence(successor)
            .await
            .map_err(debug)?
            .ok_or("successor gameplay fence")?;
        let position = owner
            .read_admission_position(gameplay)
            .await
            .map_err(debug)?;
        {
            let mut runtime = owner.runtime.lock().await;
            owner
                .initialize_admitted_position(&mut runtime, actor, successor, &position)
                .await
                .map_err(debug)?;
        }
        assert!(owner.track_restored(successor, actor, &position, gameplay));
        deps.initialize_player(&owner, actor, successor).await;
        let mut relogged = AdmittedSession {
            game_session_id: successor,
            runtime_actor: Some(actor),
            item_fence: Some(successor_fence),
            controller: Some(ControllerBinding {
                transport: current.current_transport().ok_or("successor transport")?,
                account_id: id(40),
            }),
            continuity: SessionContinuity {
                selected_capabilities: selected,
                ..SessionContinuity::FRESH
            },
            ..admitted
        };
        owner.admit_progression_session(&mut relogged).await;
        owner.admit_loop_training(&mut relogged).await;
        let restored = catalogue(&owner, &relogged).await?;
        assert!(owns(&restored, cosmetic, addon));
        assert_eq!(restored.selected.as_ref(), Some(&before));
        selection.colours[0] = 1;
        select(&owner, &mut relogged, restored, selection.clone()).await?;
        let after = catalogue(&owner, &relogged)
            .await?
            .selected
            .ok_or("successor appearance")?;
        assert_eq!(after.selection, selection);
        eprintln!("S06-Q {item} POSTGRES_CASE_COMPLETED");
        drop(store);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn catalogue(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Catalogue> {
    owner
        .appearance_observation(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            vec![],
        )
        .await
        .map(|observation| observation.catalogue)
        .ok_or_else(|| "current appearance catalogue".into())
}

fn owns(catalogue: &Catalogue, key: &str, addon: Option<u8>) -> bool {
    match addon {
        Some(bit) => catalogue
            .outfits
            .iter()
            .any(|option| option.key == key && option.permitted_addons & bit == bit),
        None => catalogue.mounts.iter().any(|mount| mount == key),
    }
}

async fn select(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    catalogue: Catalogue,
    selection: Selection,
) -> TestResult {
    let change = Change {
        context: catalogue.context,
        expected_root_revision: catalogue.root_revision,
        prior: catalogue.selected,
        selection,
        content_revision: owner
            .appearance_context()
            .ok_or("appearance content")?
            .1
            .content_revision()
            .into(),
    };
    let command = admitted.continuity.next_command_id;
    let reply = wire(
        owner,
        admitted,
        command,
        appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
        change.encode().map_err(debug)?,
    )
    .await?;
    assert_eq!(
        ChangeResult::decode(&reply).map_err(debug)?,
        ChangeResult::Committed(change.expected_root_revision + 1)
    );
    Ok(())
}
