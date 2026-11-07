//! Cumulative mixed fish and final reward through the production NPC_TALK route.
use super::fixture::{Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn sinatuki_mixed_fish_then_final_fish_atomic_replay() -> TestResult {
    execute(&[
        ("oteryn:item.tibia.i7159", 40),
        ("oteryn:item.tibia.i7158", 59),
        ("oteryn:item.tibia.i7159", 1),
        ("oteryn:item.tibia.i7158", 1),
    ])
}

#[test]
fn sinatuki_whole_hundred_stack_still_needs_final_fish() -> TestResult {
    execute(&[
        ("oteryn:item.tibia.i7158", 100),
        ("oteryn:item.tibia.i7159", 1),
        ("oteryn:item.tibia.i7158", 1),
    ])
}

fn execute(fish: &'static [(&'static str, i64)]) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(fish).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Sinatuki scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED sinatuki stacks={} duration_ms={}",
        fish.len(),
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario(fish: &[(&str, i64)]) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "sinatuki", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let dependencies = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, _activation, _proof) = purchase_fixture::current(id(40));
        let coordinator =
            SpellPremiumCoordinator::new(consumer.clone(), Some(clock), None, Some(h.root.clone()));
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        dependencies.initialize(&owner).await;
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = dependencies.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.sinatuki".into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        dependencies
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                dependencies.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        // Carried inventory projects the visible top stack first (ordinal DESC).
        let entries: Vec<_> = fish.iter().copied().rev().collect();
        seed_main_backpack(&h, &entries).await?;
        // Access is external prerequisite state; every fish, cumulative step and
        // reward below is driven by the real NPC dispatcher, never seeded.
        let mut next = 1000;
        let speak = |text: &str| NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        };
        let outcome = owner
            .npc_talk(dependencies.actor, command(next), speak("hi"))
            .await;
        next += 1;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        let total: i64 = fish.iter().map(|(_, count)| count).sum();
        // Missing access refuses the debit and cannot create cumulative credit.
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let denied = owner
            .npc_talk(dependencies.actor, command(next), speak("nupi"))
            .await;
        next += 1;
        assert!(!denied.unknown);
        assert!(!denied.inventory_changed);
        assert_eq!(
            quantity(&h, "oteryn:item.tibia.i7159").await?
                + quantity(&h, "oteryn:item.tibia.i7158").await?,
            total
        );
        assert_eq!(receipts(&h).await?, 0);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (track, value) in [
            (
                "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
                3_i64,
            ),
            (
                "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
                5_i64,
            ),
        ] {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest.the_ice_islands_quest',$2,$3,2)")
                .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let mut remaining = total;
        for (round, (_, count)) in fish[..fish.len() - 1].iter().enumerate() {
            owner.clock_origin -= std::time::Duration::from_secs(2);
            let (commit, outcome) = if round + 2 == fish.len() {
                let mut admitted = dependencies.admitted;
                admitted.first_entry =
                    crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
                admitted.item_fence = Some(fence);
                let commit = super::rookgaard_exchanges::wire_talk(
                    &owner,
                    admitted,
                    npc,
                    &["hi", "chuqua", "nupi"],
                )
                .await?;
                let outcome = owner
                    .resume_npc_bank(dependencies.actor, command(commit))
                    .await
                    .ok_or("wire fish receipt")?;
                (commit, outcome)
            } else {
                let commit = next;
                next += 1;
                let outcome = owner
                    .npc_talk(dependencies.actor, command(commit), speak("nupi"))
                    .await;
                (commit, outcome)
            };
            assert!(!outcome.unknown, "fish round {round}");
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
            assert!(
                outcome.inventory_changed,
                "fish must actually be consumed round {round}"
            );
            remaining -= count;
            assert_eq!(
                quantity(&h, "oteryn:item.tibia.i7159").await?
                    + quantity(&h, "oteryn:item.tibia.i7158").await?,
                remaining
            );
            let reward = if round + 2 == fish.len() { 5 } else { 0 };
            assert_eq!(quantity(&h, "oteryn:item.tibia.i7290").await?, reward);
            let line = if reward == 5 {
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>"
            } else {
                "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>"
            };
            assert_eq!(outcome.conversation.ok_or("fish reply")?.lines, [line]);
            assert_eq!(receipts(&h).await?, i64::try_from(round + 1)?);
            let before = footprint(&h).await?;
            let replay = owner
                .npc_talk(dependencies.actor, command(commit), speak("nupi"))
                .await;
            assert!(!replay.unknown);
            assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
            assert_eq!(footprint(&h).await?, before);
            let conflict = owner
                .npc_talk(dependencies.actor, command(commit), speak("chuqua"))
                .await;
            assert_eq!(conflict.disposition, NpcIntentDisposition::Rejected);
            assert_eq!(footprint(&h).await?, before);
            owner.states.npc_conversations.forget(session);
            owner.states.chat.forget(session);
            let cold = owner
                .resume_npc_bank(dependencies.actor, command(commit))
                .await
                .ok_or("cold fish receipt")?;
            assert!(!cold.unknown);
            assert_eq!(cold.disposition, NpcIntentDisposition::Ok);
            assert_eq!(cold.conversation.ok_or("retained reply")?.open, None);
            assert_eq!(footprint(&h).await?, before);
            owner
                .states
                .chat
                .admit(session, "Fixture Hero", owner.owner_now());
            owner.states.chat.set_npc_service(session);
            owner.clock_origin -= std::time::Duration::from_secs(2);
            let hi = owner
                .npc_talk(dependencies.actor, command(next), speak("hi"))
                .await;
            assert_eq!(hi.disposition, NpcIntentDisposition::Ok);
            next += 1;
        }
        let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/jamjam'").fetch_one(&h.pool).await?;
        assert_eq!(grants, 1);
        let before = footprint(&h).await?;
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let complete = owner
            .npc_talk(dependencies.actor, command(next), speak("nupi"))
            .await;
        assert!(!complete.unknown);
        assert!(
            !complete.inventory_changed,
            "one quest reward per character"
        );
        assert_eq!(footprint(&h).await?, before);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid AND definition_production_key=$2 AND lifecycle=1").bind(id(42).as_slice()).bind(key).fetch_one(&h.pool).await?)
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_craft_records WHERE recipe='oteryn:npc-quest-exchange.sinatuki'").fetch_one(&h.pool).await?)
}
async fn footprint(h: &Harness) -> TestResult<Vec<(String, String, i64, i16)>> {
    Ok(sqlx::query_as("SELECT item_instance_id::text,definition_production_key,quantity,lifecycle FROM game_item_instances ORDER BY item_instance_id LIMIT 32").fetch_all(&h.pool).await?)
}
