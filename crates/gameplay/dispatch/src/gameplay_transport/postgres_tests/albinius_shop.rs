use super::boss_encounter_fixtures::maintain_ready;
use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::npc_catalogue::quest_shops::{
    ALBINIUS, BLANK_IMBUEMENT_SCROLL, HEAVY_OLD_TOME, ORDINARY_UNLOCK_VALUE, ordinary_requirement,
};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::npc_command::npc_bank_talk;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[test]
fn albinius_greet_and_trade_quotes_the_blank_scroll() -> TestResult {
    scenario("albinius_hi_trade", "hi trade", false)
}

#[test]
fn albinius_greet_and_offers_quotes_the_blank_scroll() -> TestResult {
    scenario("albinius_hi_offers", "hi offers", false)
}

#[test]
fn albinius_buy_opens_the_shop_and_quotes_the_blank_scroll() -> TestResult {
    scenario("albinius_buy", "buy", true)
}

#[test]
fn albinius_rate_limited_trade_never_reads_the_backpack() -> TestResult {
    scenario("albinius_rate", "trade", true)
}

fn scenario(tag: &'static str, phrase: &'static str, greet_first: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |admin| {
                let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let world = WorldId::decode(&id(42)).map_err(debug)?;
                let (controller, pin) = activated_with_loot_tables(world, None);
                let active = controller.active().ok_or("active content")?;
                let index = active.native_gameplay().and_then(|native| native.item_index())
                    .ok_or("item index")?;
                let channel_pin = ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(),
                    (2, 0, 7),
                );
                let deps = Dependencies::with_content(
                    channel_pin,
                    crate::content::qualify_native_entry_room(world).map_err(debug)?,
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                )?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state(
                    "content-1",
                )
                .map_err(debug)?;
                let (quest, track) = ordinary_requirement(ALBINIUS).ok_or("Albinius quest")?;
                let hash = catalogue.catalogue().definition_hash(quest).ok_or("quest hash")?;
                owner.quest_catalogue = Some(Arc::new(catalogue.catalogue().clone()));
                deps.initialize(&owner).await;
                seed_main_backpack(&h, &[(HEAVY_OLD_TOME, 1); 5]).await?;
                let mut tx = h.pool.begin().await?;
                // Install the prior quest progress in this fixture. Admission loads the copy.
                sqlx::query("SET LOCAL session_replication_role = replica")
                    .execute(&mut *tx).await?;
                sqlx::query(
                    "INSERT INTO game_character_quest_states (character_id, quest_key, \
                     pinned_content_revision, definition_hash, started_character_revision, \
                     completed_character_revision, committed_character_revision) \
                     VALUES (encode($1,'hex')::uuid, $2, 'content-1', $3, 2, 2, 2)",
                )
                .bind(id(41).as_slice()).bind(quest).bind(hash.as_slice())
                .execute(&mut *tx).await?;
                sqlx::query(
                    "INSERT INTO game_character_quest_tracks (character_id, track_key, quest_key, \
                     value, committed_character_revision) VALUES (encode($1,'hex')::uuid, $2, $3, $4, 2)",
                )
                .bind(id(41).as_slice()).bind(track).bind(quest).bind(ORDINARY_UNLOCK_VALUE)
                .execute(&mut *tx).await?;
                tx.commit().await?;
                let catalogue = NpcDataCatalogue::load(
                    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
                    std::ffi::OsStr::new("world"),
                    npc_catalogue_preproduction_limits(),
                ).map_err(debug)?;
                let model = catalogue.service_model_for_project().map_err(debug)?;
                owner.states.chat.install_npc_talk(Arc::new(
                    NpcTalkBook::from_model_with_native_quests(&model, &[ALBINIUS]),
                ));
                let mut admitted = deps.admitted;
                let session = admitted.game_session_id;
                owner.states.chat.admit(session, "Fixture Hero", owner.owner_now());
                owner.states.chat.set_npc_service(session);
                let npc = {
                    let mut runtime = owner.runtime.lock().await;
                    runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                    let spot = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                    runtime.place_npcs(
                        &NpcPlacementTable {
                            npcs: vec![NpcPlacements {
                                key: ALBINIUS.into(),
                                placements: vec![NpcPlacement {
                                    x: u16::try_from(spot.x + 1)?,
                                    y: u16::try_from(spot.y)?,
                                    floor: -i8::try_from(spot.floor)?,
                                    direction: MovementFacing::South,
                                }],
                            }],
                        },
                        |_| true,
                        true,
                    ).map_err(debug)?;
                    let placed = runtime.visible_entities().npcs[0];
                    EntityRef { identity: placed.actor.placement_identity(), generation: placed.generation }
                };
                let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(session).await?.0;
                admitted.item_fence = Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
                owner.admit_quest_session(&admitted).await;
                {
                    let sessions = owner.states.quest_sessions.lock().map_err(debug)?;
                    let copy = sessions.get(&session).and_then(|entry| entry.copy.as_ref())
                        .ok_or("admitted quest copy")?;
                    assert_eq!(copy.tracks().get(track), Some(&ORDINARY_UNLOCK_VALUE));
                    assert!(copy.states().get(quest).is_some_and(|state| state.completed));
                }
                owner.states.test_owner_time.store(1, Ordering::Relaxed);
                let talk = |command_id, text: &str| {
                    npc_bank_talk(
                        &owner, deps.actor,
                        UseCommand { game_session_id: session, command_id, item_fence: admitted.item_fence },
                        NpcTalkIntent { npc_actor: npc, text: text.into() },
                        owner.owner_now(),
                    )
                };
                if greet_first {
                    maintain_ready(&owner).await?;
                    assert_eq!(talk(1, "hi").await.disposition, NpcIntentDisposition::Ok);
                }
                if tag == "albinius_rate" {
                    for id in 2..=4 {
                        maintain_ready(&owner).await?;
                        assert_eq!(talk(id, "trade").await.disposition, NpcIntentDisposition::Ok);
                    }
                    maintain_ready(&owner).await?;
                    let mut lock = h.pool.begin().await?;
                    sqlx::query("LOCK TABLE game_item_container_slots IN ACCESS EXCLUSIVE MODE")
                        .execute(&mut *lock).await?;
                    let waiting = async {
                        loop {
                            let blocked: bool = sqlx::query_scalar(
                                "SELECT EXISTS (SELECT FROM pg_locks WHERE \
                                 relation = 'game_item_container_slots'::regclass AND NOT granted)",
                            )
                            .fetch_one(&h.pool).await?;
                            if blocked {
                                return Ok::<(), sqlx::Error>(());
                            }
                            tokio::task::yield_now().await;
                        }
                    };
                    let reply = tokio::time::timeout(Duration::from_secs(5), async {
                        tokio::select! {
                            reply = talk(5, phrase) => Ok::<_, sqlx::Error>(Some(reply)),
                            blocked = waiting => blocked.map(|()| None),
                        }
                    }).await??;
                    lock.rollback().await?;
                    let reply = reply.ok_or("fifth talk tried to read the locked backpack")?;
                    assert_eq!(reply.disposition, NpcIntentDisposition::RateLimited);
                } else {
                    maintain_ready(&owner).await?;
                    let reply = talk(2, phrase).await;
                    assert_eq!(reply.disposition, NpcIntentDisposition::Ok);
                    let window = reply.window.filter(|window| window.open.is_some())
                        .ok_or("Albinius did not open the shop")?;
                    assert!(window.offers.iter().any(|offer| {
                        NonZeroU32::new(offer.item_definition_ref).and_then(|reference| index.definition_at(reference))
                            .is_some_and(|(key, _)| key == BLANK_IMBUEMENT_SCROLL)
                    }), "Albinius omitted the eligible blank imbuement scroll for {phrase}");
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await?;
                eprintln!("POSTGRES_CASE_COMPLETED {tag}");
                Ok(())
            }).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Albinius shop test panicked")??;
    Ok(())
}
