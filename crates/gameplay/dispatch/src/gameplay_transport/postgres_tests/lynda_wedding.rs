//! Actual two-Character reciprocal wedding conversation must grant the source outfit.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::domain::appearance::AppearanceContent;
use crate::foundation::{
    CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;
#[test]
fn srv7_lynda_two_real_characters_reciprocally_acquire_wedding_outfit() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Lynda wedding thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let started = std::time::Instant::now();
        let h = Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_lynda_wedding",
            false,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        super::item_look::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i3035", 100),
                ("oteryn:item.tibia.i3035", 100),
            ],
        )
        .await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(id(41).as_slice()).execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world_id,
            "content-1",
            None,
        )?;
        let active = controller.active().ok_or("active generation")?;
        let (bundle, source_npcs) = source_world::world(
            world_id,
            active.native_gameplay().ok_or("source wedding native")?,
        )?;
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 32408,
                    y: 32221,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (32408, 32221, 7),
        );
        // The in-transaction custody owner independently reads this real activation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(channel_pin.server_artifact_digest().as_slice())
            .bind(channel_pin.client_artifact_digest().as_slice())
            .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
            5,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.admit_progression_session(&mut admitted).await;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        let (crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        }) = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            &authority,
            current.character_id,
        )
        .await
        else {
            return Err("durable NPC customer build".into());
        };
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = Arc::new(NpcTalkBook::from_model(&model));
        owner.states.chat.install_npc_talk(Arc::clone(&book));
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Citizen", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .place_npcs(&source_npcs, |key| book.get(key).is_some(), true)
                .map_err(debug)?;
        }
        let npc = source_actor(&owner, "oteryn:npc.lynda").await?;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[3, 4, 6, 11, 15, 25, 350],
            )
            .ok_or("NPC capability")?;
        let mut other = super::scenarios::peer_at(
            &h,
            &mut owner,
            admitted,
            crate::foundation::MovementLocalPosition {
                x: 32407,
                y: 32221,
                floor: 7,
            },
        )
        .await?;
        other.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[3, 4, 6, 11, 15, 25, 350],
            )
            .ok_or("peer actual offered native NPC/trade capabilities")?;
        assert!(other.continuity.selected_capabilities.contains(3));
        let other_actor = other.runtime_actor.ok_or("real peer actor")?;
        seed_other_cash(&h, other).await?;
        owner
            .states
            .chat
            .admit(other.game_session_id, "Scenario Peer", owner.owner_now());
        owner.states.chat.set_npc_service(other.game_session_id);
        let other_character = other
            .item_fence
            .ok_or("independently fresh peer fence")?
            .character_id;
        assert_ne!(other_character.as_bytes(), &id(41));
        assert!(
            owner
                .current_quest_fence(other.game_session_id)
                .await
                .map_err(|_| "peer current fence")?
                .is_some()
        );
        buy_source_materials(&mut owner, &mut admitted, &h, &book, 2).await?;
        buy_source_materials(&mut owner, &mut other, &h, &book, 1).await?;
        position(&owner, actor, admitted.game_session_id, (32330, 32199, 7)).await?;
        position(
            &owner,
            other_actor,
            other.game_session_id,
            (32331, 32200, 7),
        )
        .await?;
        let own_start = admitted.continuity.next_command_id;
        let actor_name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        // A real Mage keyword switches away from the unconfirmed wedding-name
        // graph. No synthetic quest track or partner is installed by this test.
        super::grizzly_npc::wire::talk_paced(
            &mut owner,
            &mut admitted,
            npc,
            &[
                (own_start, "hi"),
                (own_start + 1, "marry"),
                (own_start + 2, "yes"),
                (own_start + 3, "wand"),
                (own_start + 4, "yes"),
            ],
        )
        .await?;
        assert_eq!(
            own_material_count(&h).await?,
            4,
            "a stale yes cannot remove wedding materials or bind a partner"
        );
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[
                (own_start + 5, "hi"),
                (own_start + 6, "marry"),
                (own_start + 7, "yes"),
                (own_start + 8, "Scenario Peer"),
            ],
        )
        .await?;
        assert!(
            !has_outfit(&h, &owner, admitted, &authority).await?,
            "first proposal alone cannot grant the wedding outfit"
        );
        assert_eq!(
            own_material_count(&h).await?,
            2,
            "the first genuine proposal consumes exactly one ring and box"
        );
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[
                (own_start + 9, "hi"),
                (own_start + 10, "remove"),
                (own_start + 11, "yes"),
                (own_start + 11, "yes"),
            ],
        )
        .await?;
        let refunded:Vec<(String,i64)>=sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_reward_claim_inbox_mint_lines l JOIN game_item_instances i USING(item_instance_id) JOIN game_reward_claim_inbox_mint_receipts r USING(transaction_id) WHERE r.character_id=encode($1,'hex')::uuid ORDER BY i.definition_production_key").bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            refunded,
            vec![
                ("oteryn:item.tibia.i3004".into(), 1),
                ("oteryn:item.tibia.i9586".into(), 1)
            ],
            "fresh pending cancellation refunds the exact two source items once; native replay mints nothing"
        );
        assert!(
            !has_outfit(&h, &owner, admitted, &authority).await?,
            "cancelled consent cannot grant outfits"
        );
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[
                (own_start + 12, "hi"),
                (own_start + 13, "marry"),
                (own_start + 14, "yes"),
                (own_start + 15, "Scenario Peer"),
            ],
        )
        .await?;
        let command = other.continuity.next_command_id;
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut other,
            npc,
            &[
                (command, "hi"),
                (command + 1, "marry"),
                (command + 2, "yes"),
                (command + 3, &actor_name),
            ],
        )
        .await?;
        assert!(
            has_outfit(&h, &owner, admitted, &authority).await?,
            "actual reciprocal proposal must grant the original real Character's Account outfit"
        );
        assert!(
            has_outfit(&h, &owner, other, &authority).await?,
            "actual accepting Character's separate Account must also receive its own source outfit"
        );
        assert_eq!(
            own_material_count(&h).await?,
            0,
            "actual wedding costs consumed only by native source receipts"
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_srv7_item_costs k CROSS JOIN LATERAL jsonb_array_elements(k.inputs) consumed JOIN game_item_instances i ON i.item_instance_id=(consumed->>'item')::uuid JOIN game_npc_trades purchased ON purchased.transaction_id=i.minted_transaction_id AND purchased.character_id=k.character_id WHERE k.cause_kind='lynda_wedding' AND i.definition_production_key IN('oteryn:item.tibia.i3004','oteryn:item.tibia.i9586') AND i.lifecycle=2").fetch_one(&h.pool).await?,6,"all six wedding cost instances originated in each real Character's native merchant BUY; no ring or box was fixture-minted");
        eprintln!(
            "SRV7_LYNDA_TWO_CHARACTER_NATIVE_EXECUTED elapsed={:.3}s real_fresh_peers=2 fake_quest_flags=0 reciprocal_outfit_accounts=2",
            started.elapsed().as_secs_f64()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn has_outfit(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<bool> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current actual wedding Character")?
        .ok_or("current Character fence")?;
    let content = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|g| g.cosmetics())
        .ok_or("actual native cosmetics")?;
    let state = h
        .root
        .read_current_character_appearance_state(
            authority,
            &h.node,
            fence,
            content.earned_grant_keys(),
        )
        .await?;
    Ok(state
        .earned
        .iter()
        .any(|(key, _)| key == "oteryn:appearance-grant/npc/lynda/newly_wed/base"))
}
async fn own_material_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key IN('oteryn:item.tibia.i3004','oteryn:item.tibia.i9586')").bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}
async fn seed_other_cash(
    h: &Harness,
    other: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let character = other.item_fence.ok_or("real peer Character")?.character_id;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let bag: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)VALUES($1,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)").bind(bag).bind(id(42).as_slice()).bind(id(199).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id)VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(character.as_bytes().as_slice()).bind(bag).bind(id(42).as_slice()).bind(id(199).as_slice()).execute(&mut *tx).await?;
    for (n, key) in ["oteryn:item.tibia.i3035"].into_iter().enumerate() {
        let item: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)VALUES($1,encode($2,'hex')::uuid,'Item',$3,'definition-r1',100,1,encode($4,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(key).bind(id(199).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,encode($6,'hex')::uuid)").bind(item).bind(id(42).as_slice()).bind(character.as_bytes().as_slice()).bind(bag).bind(i64::try_from(n+1)?).bind(id(199).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[path = "lynda_wedding/source_world.rs"]
mod source_world;
async fn source_actor(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    let runtime = owner.runtime.lock().await;
    let n = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|n| runtime.npc_key(n.actor) == Some(key))
        .ok_or("actual source acquisition NPC actor")?;
    Ok(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: n.actor.placement_identity(),
        generation: n.generation,
    })
}
async fn position(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    at: (i32, i32, i16),
) -> TestResult {
    owner
        .runtime
        .lock()
        .await
        .stage_pinned_player_test_position(
            actor,
            session,
            crate::foundation::MovementLocalPosition {
                x: at.0,
                y: at.1,
                floor: at.2,
            },
        )
        .map_err(debug)?;
    Ok(())
}
async fn buy_source_materials(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    h: &Harness,
    book: &Arc<NpcTalkBook>,
    quantity: u32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actual source buyer")?;
    for (npc_key, item, price, at) in [
        (
            "oteryn:npc.hanna",
            "oteryn:item.tibia.i3004",
            990,
            (32408, 32221, 7),
        ),
        (
            "oteryn:npc.norbert",
            "oteryn:item.tibia.i9586",
            5000,
            (32953, 32103, 6),
        ),
    ] {
        position(owner, actor, admitted.game_session_id, at).await?;
        let npc = source_actor(owner, npc_key).await?;
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("actual native source")?;
        let index = native.item_index().ok_or("actual Item index")?;
        let offers = oteryn_gameplay_npc::npc_command::projected(
            book.get(npc_key).ok_or("actual source merchant")?.offers(),
            index,
        )
        .collect::<Vec<_>>();
        let offer = offers
            .iter()
            .position(|(view, source)| {
                source.item.key == item
                    && view.side == oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy
            })
            .ok_or("actual source material offer")?;
        let request = oteryn_protocol_oteryn::npc_service::NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: book.catalogue_revision(),
            offer_index: u32::try_from(offer)?,
            side: oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: price,
        };
        // The existing native shop owner deliberately trades one non-stackable
        // per transaction. Acquire each genuine item through that real caller;
        // exact replay of every purchase must still mint no second copy.
        for _ in 0..quantity {
            let command = admitted.continuity.next_command_id;
            let got =
                super::grizzly_npc::wire::trade(owner, admitted, h, command, &request, None, true)
                    .await?;
            assert_eq!(
                got,
                vec![oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok; 2]
            );
        }
    }
    Ok(())
}
