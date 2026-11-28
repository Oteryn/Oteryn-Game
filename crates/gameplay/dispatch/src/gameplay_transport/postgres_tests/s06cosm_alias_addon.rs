//! A retained Citizen outfit key can wear an addon after an actual item acquisition.
use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::item_look_fixtures::seed_character_backpack;
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::quest_world_callbacks_fixtures::wire;
use super::rust_lottery_fixtures::{bundle_action_room, exchange};
use super::{
    Dependencies, FirstEntryOutcome, Harness, TestResult, debug, fence, id, prepare_current_session,
};
use crate::content::ReferenceItemField;
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, CharacterLease, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_items::item_cosmetics;
use oteryn_protocol_oteryn::appearance::{self, Change, ChangeResult, Selection};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn alias_keyed_outfit_addon_is_wearable_after_its_grant() -> TestResult {
    const ALIAS: &str = "oteryn:content.outfit.citizen_outfits";
    const DOLL: &str = "oteryn:item.tibia.i6579";
    const GRANT: &str = "oteryn:item-grant.outfit.128-136.addon1";
    const BACKPACK: u8 = 81;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let harness = Harness::create_appearance(admin, "s06cosm_alias_addon").await?;
        eprintln!("s06cosm-marker: alias_keyed_outfit_addon_is_wearable_after_its_grant db ready");
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) =
            production_generation_with_pin(fence(1)?.runtime_scope.world_id(), "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let items = native.item_index().ok_or("native item index")?;
        let doll = items
            .definition_ref("Item", DOLL, "definition-r1")
            .ok_or("doll item reference")?;
        let reference = |key: &str| items.definition_ref("Item", key, "definition-r1");
        let bag = "oteryn:item.tibia.i2854";
        let oven = "oteryn:item.tibia.i2535";
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let ReferenceItemField::Known(physical) = &policy.semantics.physical else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
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
        let room = bundle_action_room(pin.qualified_room(), &map, deps.admitted.world_id, &cells)?;
        let channel_pin = super::ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (6, 0, 7),
        );
        let frame = pin.frame_binding().digest();
        let mut admitted = deps.admitted;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            admitted.world_id,
            admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current fence")?;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(current.item_fence());
        deps.initialize(&owner).await;
        owner.states.chat.admit(
            admitted.game_session_id,
            "Citizen witness",
            owner.owner_now(),
        );
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &offered,
            &[3, 4, 6, 11, 15, 18, appearance::CAPABILITY_APPEARANCE_V1],
        )
        .ok_or("item and appearance capabilities")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&harness.pool).await?;
        seed_character_backpack(&harness, 41, BACKPACK, &[(DOLL, 1)]).await?;

        let initial = owner
            .appearance_observation(deps.actor, admitted.game_session_id, vec![])
            .await
            .ok_or("initial appearance catalogue")?;
        let alias = initial
            .catalogue
            .outfits
            .iter()
            .find(|o| o.key == ALIAS)
            .ok_or("starter alias")?;
        assert_eq!(alias.permitted_addons, 0);

        // This carried instance gives a deterministic Citizen draw on the first
        // ordinary USE. The production selector still decides the grant.
        let command = admitted.continuity.next_command_id;
        let rule = item_cosmetics::rule(DOLL).ok_or("stock addon doll")?;
        let choice = item_cosmetics::select_for_command(
            rule,
            *admitted.game_session_id.as_bytes(),
            command,
            id(BACKPACK + 1),
        )
        .ok_or("doll choice")?;
        assert!(choice.grants.iter().any(|key| key == GRANT));
        let addon = 1;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("item handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let (mut admitted, disposition, _) = exchange(
            &owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(doll),
            None,
        )
        .await?;
        assert_eq!(disposition, UseDisposition::Committed);
        let source: (String, String) = sqlx::query_as(
            "SELECT r.source_kind, r.content_revision FROM game_account_appearance_unlocks u \
             JOIN game_account_appearance_unlock_receipts r USING (grant_occurrence_id, account_id, unlock_key) \
             WHERE u.account_id=encode($1,'hex')::uuid AND u.unlock_key=$2",
        ).bind(id(40).as_slice()).bind(GRANT).fetch_one(&harness.pool).await?;
        assert_eq!(
            (source.0.as_str(), source.1.as_str()),
            ("item_action", item_cosmetics::PROVENANCE)
        );

        let observation = owner
            .appearance_observation(deps.actor, admitted.game_session_id, vec![])
            .await
            .ok_or("appearance after the grant")?;
        let change = Change {
            context: observation.catalogue.context,
            expected_root_revision: observation.catalogue.root_revision,
            prior: observation.catalogue.selected,
            selection: Selection {
                outfit_key: ALIAS.into(),
                colours: [0; 4],
                addons: addon,
                mount_key: None,
            },
            content_revision: owner
                .appearance_context()
                .ok_or("appearance context")?
                .1
                .content_revision()
                .into(),
        };
        let command = admitted.continuity.next_command_id;
        let reply = wire(
            &owner,
            &mut admitted,
            command,
            appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
            change.encode().map_err(debug)?,
        )
        .await?;
        assert_eq!(
            ChangeResult::decode(&reply).map_err(debug)?,
            ChangeResult::Committed(change.expected_root_revision + 1),
            "the earned Citizen addon must be wearable on the retained key",
        );
        let stored: (String, i16) = sqlx::query_as(
            "SELECT outfit_key, addons FROM game_character_appearance WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!((stored.0.as_str(), stored.1), (ALIAS, i16::from(addon)));
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
