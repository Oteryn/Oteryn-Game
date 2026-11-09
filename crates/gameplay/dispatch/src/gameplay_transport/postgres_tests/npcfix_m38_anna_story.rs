//! Anna's real literal dialogue, source regions and source anchor through native owners.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_tutorial::TutorialProgressOutcome;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const ANNA: &str = "oteryn:npc.anna";
#[path = "npcfix_m38_anna_story/world.rs"]
mod world;

#[test]
fn npcfix_m38_targuna_anna_literal_story_postgres() -> TestResult {
    anna_story(false)
}

#[test]
fn npcfix_m38_targuna_anna_fresh_owner_arrival_recovery_postgres() -> TestResult {
    anna_story(true)
}

fn anna_story(recover: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-anna-story", move || {
        run(async move |admin| scenario(admin, recover).await)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_ANNA_NATIVE recovery={recover} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn scenario(admin: String, recover: bool) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "m38_anna_story", true, false).await?;
    let result = exercise(&h, recover).await;
    h.cleanup().await?;
    result
}

async fn exercise(h: &Harness, recover: bool) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let id = super::fixture::fence(1)?.runtime_scope.world_id();
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_source(id, "content-1", None)?;
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let (bundle, npcs) = world::load(
        native,
        id,
        crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?,
    )?;
    let cells = bundle
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = pin
        .qualified_room()
        .bundle_view(
            cells.clone(),
            bundle.frame_binding_digest(id),
            bundle.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 32534,
                y: 32513,
                floor: 7,
            },
        )
        .ok_or("source room")?;
    let channel_pin = ChannelContentPin::from_activation(
        id,
        1,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (32534, 32513, 7),
    );
    let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&deps.room), Some(&deps.room));
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let session = deps.admitted.game_session_id;
    // This fixture's existing interpreted Character owns level50. The travel
    // Root reads that durable level; native progression initialization is separate.
    let admitted = deps.admitted;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let approach = {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .place_npcs(&npcs, |key| key == ANNA, true)
            .map_err(debug)?;
        let mut found = None;
        for radius in 1_i32..=2 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    let cell = MovementLocalPosition {
                        x: 32552 + dx,
                        y: 32497 + dy,
                        floor: 7,
                    };
                    if dx.abs().max(dy.abs()) != radius
                        || !crate::content::newhaven_tutorial_land::contains(
                            cell.x, cell.y, cell.floor,
                        )
                    {
                        continue;
                    }
                    if crate::gameplay_transport::respawn_cell_admissible(
                        cells.index(),
                        cells.scope(),
                        id,
                        runtime.content_pin().server_artifact_digest(),
                        &std::collections::BTreeSet::new(),
                        cell,
                    ) {
                        found = Some(cell);
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        let at = found.ok_or("admitted source Anna approach")?;
        let actual = runtime
            .place_admitted_respawn(deps.actor, session, Some(at), |cell| cell == at)
            .map_err(debug)?;
        assert_eq!(actual.position(), at);
        at
    };
    deps.initialize(&owner).await;
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &[ANNA]),
    ));
    owner
        .states
        .chat
        .admit(session, "Anna travel witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc = {
        let runtime = deps.runtime.lock().await;
        let n = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| runtime.npc_key(n.actor) == Some(ANNA))
            .ok_or("Anna entity")?;
        assert_eq!(
            (n.position.x, n.position.y, n.position.floor),
            (32552, 32497, 7)
        );
        EntityRef {
            identity: n.actor.placement_identity(),
            generation: n.generation,
        }
    };
    let before = owner
        .root
        .character_tutorial_progress(owner.character, owner.holder, fence, None)
        .await
        .map_err(debug)?;
    let version = match before {
        TutorialProgressOutcome::Read(Some(row)) => row.version,
        TutorialProgressOutcome::Read(None) => 0,
        _ => return Err("tutorial read".into()),
    };
    owner
        .root
        .character_tutorial_progress(
            owner.character,
            owner.holder,
            fence,
            Some((
                version,
                TutorialProgress {
                    step: TutorialStep::Island,
                    hunting: true,
                    corruptor: true,
                },
            )),
        )
        .await
        .map_err(debug)?;
    eprintln!("POSTGRES_STARTED NPCFIX_M38_ANNA_NATIVE recovery={recover} approach={approach:?}");
    super::classic_quests::talk(&mut owner, &admitted, npc, 49001, "hi").await?;
    let default = super::classic_quests::talk(&mut owner, &admitted, npc, 49002, "passage").await?;
    assert!(
        default.iter().any(|line| line.contains("thais peninsula")),
        "preserve owner's default: {default:?}"
    );
    super::classic_quests::talk(&mut owner, &admitted, npc, 49003, "no").await?;
    let offer = super::classic_quests::talk(&mut owner, &admitted, npc, 49004, "targuna").await?;
    assert!(
        offer.iter().any(|line| line
            .to_ascii_lowercase()
            .contains("free passage to targuna")),
        "explicit Targuna offer missing: {offer:?}"
    );
    let at = if recover {
        // Interrupt after placement and after the arrival owners ran, before citizenship
        // can commit. The pending receipt must remain the sole recovery obligation.
        sqlx::query("CREATE FUNCTION m38_anna_interrupt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'M38 Anna interrupted citizenship'; END $$").execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER m38_anna_interrupt BEFORE INSERT OR UPDATE ON game_character_home_towns FOR EACH ROW EXECUTE FUNCTION m38_anna_interrupt()").execute(&h.pool).await?;
        let current = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let interrupted = owner
            .npc_talk(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: session,
                    command_id: 49005,
                    item_fence: Some(current.item_fence()),
                },
                oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .await;
        assert!(
            interrupted.unknown,
            "interrupted arrival must retain its outcome"
        );
        let unfinished:(i64,i64,i32,i16,bool,bool)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_newhaven_travel_receipts WHERE route='targuna' AND arrived_at IS NOT NULL),COALESCE((SELECT town_id FROM game_character_home_towns LIMIT 1),33),step,hunting,corruptor FROM game_character_tutorial").fetch_one(&h.pool).await?;
        assert_eq!(
            unfinished,
            (1, 0, 33, 2, true, true),
            "late failure rolls back every durable arrival owner"
        );
        sqlx::query("DROP TRIGGER m38_anna_interrupt ON game_character_home_towns")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP FUNCTION m38_anna_interrupt()")
            .execute(&h.pool)
            .await?;
        super::maintain_ready(owner.root).await?;
        let pin = deps.runtime.lock().await.content_pin().clone();
        let room = deps.room.clone();
        let spells = deps.spells.clone();
        drop(owner);
        drop(deps);
        // A fresh owner has no old conversation, prompt, placement or travel hold.
        let fresh = Dependencies::with_content(pin, room, spells)?;
        let mut recovered = fresh.owner(h, &authority);
        recovered.active_generation = Some(active);
        recovered.movement_cells = (&cells).into();
        recovered.rooms =
            crate::gameplay_transport::SeamRooms::new(Some(&fresh.room), Some(&fresh.room));
        assert_eq!(fresh.admitted.game_session_id, session);
        let fence = recovered
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let pending = recovered
            .root
            .read_npc_pending_arrival(recovered.character, recovered.holder, fence)
            .await
            .map_err(debug)?
            .ok_or("pending source arrival")?;
        {
            let mut runtime = fresh.runtime.lock().await;
            runtime
                .initialize_first_entry_position(fresh.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    fresh.actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            let target = MovementLocalPosition {
                x: pending.destination.x,
                y: pending.destination.y,
                floor: pending.destination.floor,
            };
            let digest = runtime.content_pin().server_artifact_digest();
            let placed = runtime
                .place_admitted_respawn(fresh.actor, session, Some(target), |cell| {
                    crate::gameplay_transport::respawn_cell_admissible(
                        cells.index(),
                        cells.scope(),
                        id,
                        digest,
                        &std::collections::BTreeSet::new(),
                        cell,
                    )
                })
                .map_err(debug)?;
            assert_eq!(placed.position(), target);
        }
        fresh.initialize(&recovered).await;
        let decided = crate::gameplay_transport::last_position::AdmissionPosition::Arrival(
            pending, None, None,
        );
        let budget = crate::gameplay_transport::AdmissionRetryBudget::for_root(&session, &h.root);
        assert!(
            recovered
                .consume_admitted_newhaven_arrival(&budget, session, fresh.actor, Some(&decided))
                .await
        );
        let committed = recovered
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        assert!(
            recovered
                .consume_admitted_newhaven_arrival(&budget, session, fresh.actor, Some(&decided))
                .await
        );
        let replay = recovered
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        assert_eq!(
            committed, replay,
            "arrival replay does not write another citizenship revision"
        );
        fresh
            .runtime
            .lock()
            .await
            .read_actor_position(fresh.actor)
            .map_err(debug)?
            .position()
    } else {
        super::classic_quests::talk(&mut owner, &admitted, npc, 49005, "yes").await?;
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position()
    };
    assert_eq!((at.x, at.y, at.floor), (31938, 31928, 7));
    let result:(i64,i64,i32,i16,bool,bool)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_pending_arrivals),(SELECT count(*) FROM game_newhaven_travel_receipts WHERE route='targuna' AND arrived_at IS NOT NULL),(SELECT town_id FROM game_character_home_towns LIMIT 1),step,hunting,corruptor FROM game_character_tutorial").fetch_one(&h.pool).await?;
    assert_eq!(result, (0, 1, 34, 3, false, false));
    Ok(())
}
