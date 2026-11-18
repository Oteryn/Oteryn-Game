use super::*;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{ExactActorRef, MovementFacing, MovementLocalPosition, NpcPlacement};
use crate::gameplay_transport::{connection::UseCommand, npc_talk::NpcTalkBook};
use oteryn_protocol_oteryn::{
    npc_service::{NpcIntentDisposition, NpcTalkIntent},
    world_object::{UseDisposition, WorldObjectTarget},
    world_spatial::StepDirection,
    world_spatial_entities::EntityRef,
};

#[test]
fn barbarian_arena_halvar_payment_opens_door_and_activates_entry_on_step_in() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_route(true, Some(NativeScenario::Admission)).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Halvar arena worker panicked")??;
    Ok(())
}

pub(super) async fn verify_admission(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    let session = admitted.game_session_id;
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
        .install_npc_talk(std::sync::Arc::new(NpcTalkBook::from_model(&model)));
    owner
        .states
        .chat
        .admit(session, "Arena competitor", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc = {
        let mut runtime = owner.runtime.lock().await;
        let halvar = runtime
            .place_dynamic_npc(
                "oteryn:npc.halvar",
                NpcPlacement {
                    x: 32252,
                    y: 31098,
                    floor: -6,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let visible = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|npc| npc.actor == halvar)
            .ok_or("Halvar placement")?;
        EntityRef {
            identity: halvar.placement_identity(),
            generation: visible.generation,
        }
    };
    let use_command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: admitted.item_fence,
    };
    let relocate = |(x, y, floor)| async move {
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .relocate_arena_player(
                actor,
                session,
                before,
                MovementLocalPosition { x, y, floor },
            )
            .map_err(debug)?;
        runtime.read_actor_position(actor).map_err(debug)
    };
    let map = owner.bundle_map().ok_or("arena bundle")?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, 32233, 31098, 1)
        .ok_or("arena storage door")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };

    relocate((32232, 31098, 7)).await?;
    assert_eq!(
        owner
            .use_object(actor, use_command(800), target.clone())
            .await
            .disposition,
        UseDisposition::Locked
    );
    let start = relocate((32238, 31100, 7)).await?;
    let _ = owner
        .ordinary_paced_step(actor, session, StepDirection::South, Some(start), None)
        .await;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        at,
        start.position(),
        "unpaid forcefield returns the player to the origin"
    );

    relocate((32251, 31098, 6)).await?;
    let talk = |id, text: &str| {
        let intent = NpcTalkIntent {
            npc_actor: npc,
            text: text.to_owned(),
        };
        owner.npc_talk(actor, use_command(id), intent)
    };
    assert_eq!(talk(801, "hi").await.disposition, NpcIntentDisposition::Ok);
    let quote = talk(802, "fight")
        .await
        .conversation
        .ok_or("Halvar quote")?;
    assert!(quote.lines.iter().any(|line| line.contains("1000 gold")));
    assert!(talk(803, "yes").await.inventory_changed);
    assert!(!talk(804, "yes").await.inventory_changed);
    let (paid, phase): (i32, i32) = sqlx::query_as(
        "SELECT (body->>'paid_rank')::int,(body->>'phase')::int FROM game_character_arena_matches",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!((paid, phase), (1, 0));

    relocate((32232, 31098, 7)).await?;
    assert_eq!(
        owner
            .use_object(actor, use_command(805), target)
            .await
            .disposition,
        UseDisposition::Committed
    );
    let start = relocate((32238, 31100, 7)).await?;
    let _ = owner
        .ordinary_paced_step(actor, session, StepDirection::South, Some(start), None)
        .await;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (32210, 31098, 7));
    let (rank, pit, phase): (i32, i32, i32) = sqlx::query_as(
        "SELECT (body->>'rank')::int,(body->>'pit')::int,(body->>'phase')::int FROM game_character_arena_matches"
    ).fetch_one(&harness.pool).await?;
    assert_eq!((rank, pit, phase), (1, 1, 1));
    let fees: i64 = sqlx::query_scalar("SELECT count(*) FROM game_arena_entry_receipts")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(
        fees, 1,
        "Halvar charges once and StepIn consumes the paid attempt"
    );
    Ok(())
}
